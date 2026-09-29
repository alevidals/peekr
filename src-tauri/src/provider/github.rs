//! GitHub via the REST API.
//!
//! One search (`review-requested:@me`) lists every open PR waiting on the user — including
//! requests made to teams they belong to, and dropping PRs once a review has been submitted.
//! Search results lack branches and full comment counts, so each PR is then fetched once.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::RequestBuilder;
use serde::Deserialize;

use super::{FetchReport, ReviewProvider};
use crate::{
    domain::{ProviderKind, PullRequest},
    error::{AppError, AppResult},
};

const API: &str = "https://api.github.com";
const PER_PAGE: usize = 100;
/// Search never returns more than 1000 results.
const MAX_PAGES: usize = 10;
const DETAIL_CONCURRENCY: usize = 8;

pub struct Github {
    client: reqwest::Client,
    account_id: String,
    /// Organisation or user the account is scoped to.
    owner: String,
    token: String,
}

impl Github {
    pub fn new(client: reqwest::Client, account_id: String, owner: String, token: String) -> Self {
        Self { client, account_id, owner, token }
    }

    fn get(&self, url: &str) -> RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            // GitHub rejects requests without a User-Agent.
            .header("User-Agent", "peekr")
    }

    async fn search_review_requests(&self) -> AppResult<Vec<SearchItem>> {
        let mut items = Vec::new();
        for page in 1..=MAX_PAGES {
            let response: SearchResponse = self
                .get(&format!("{API}/search/issues"))
                .query(&[
                    ("q", "is:pr is:open review-requested:@me"),
                    ("per_page", &PER_PAGE.to_string()),
                    ("page", &page.to_string()),
                ])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let received = response.items.len();
            items.extend(response.items);
            if received < PER_PAGE {
                break;
            }
        }
        Ok(items)
    }

    async fn fetch_pull(&self, pull_url: &str, repo: &str) -> AppResult<PullRequest> {
        let raw: RawPull = self.get(pull_url).send().await?.error_for_status()?.json().await?;
        Ok(raw.into_pr(&self.account_id, repo))
    }
}

#[async_trait]
impl ReviewProvider for Github {
    async fn validate(&self) -> AppResult<String> {
        let user: GithubUser = self
            .get(&format!("{API}/user"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(user.login)
    }

    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        let items = self.search_review_requests().await?;

        // The search spans every owner the token can see; keep only this account's scope.
        let in_scope: Vec<(String, String)> = items
            .into_iter()
            .filter_map(|item| {
                let repo = repo_full_name(&item.repository_url)?.to_string();
                let pull_url = item.pull_request?.url;
                in_scope(&repo, &self.owner, repos).then_some((repo, pull_url))
            })
            .collect();

        let results: Vec<_> = stream::iter(in_scope)
            .map(|(repo, pull_url)| async move {
                let result = self.fetch_pull(&pull_url, &repo).await;
                (repo, result)
            })
            .buffered(DETAIL_CONCURRENCY)
            .collect()
            .await;

        let mut prs = Vec::new();
        let mut failed_repos: Vec<(String, AppError)> = Vec::new();
        for (repo, result) in results {
            match result {
                Ok(pr) => prs.push(pr),
                Err(e) => {
                    log::warn!("Failed to fetch PR in {repo}: {e}");
                    if !failed_repos.iter().any(|(r, _)| *r == repo) {
                        failed_repos.push((repo, e));
                    }
                }
            }
        }
        // A repo with any failed PR is reported as failed so its previous PRs are kept.
        prs.retain(|pr| !failed_repos.iter().any(|(r, _)| *r == pr.repo));

        Ok(FetchReport { prs, failed_repos })
    }
}

/// `https://api.github.com/repos/owner/name` → `owner/name`.
fn repo_full_name(repository_url: &str) -> Option<&str> {
    repository_url.split("/repos/").nth(1)
}

/// Whether `full_name` (`owner/name`) belongs to the account: same owner and, if `repos` is
/// non-empty, one of them. Entries in `repos` may be `name` or `owner/name`.
fn in_scope(full_name: &str, owner: &str, repos: &[String]) -> bool {
    let Some((repo_owner, name)) = full_name.split_once('/') else {
        return false;
    };
    if !repo_owner.eq_ignore_ascii_case(owner) {
        return false;
    }
    repos.is_empty()
        || repos.iter().any(|wanted| {
            let wanted = wanted.rsplit_once('/').map_or(wanted.as_str(), |(_, n)| n);
            wanted.eq_ignore_ascii_case(name)
        })
}

// ── API payloads ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubUser {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    repository_url: String,
    /// Absent for plain issues; the query only asks for PRs.
    pull_request: Option<PullLink>,
}

#[derive(Debug, Deserialize)]
struct PullLink {
    url: String,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawPull {
    number: u64,
    title: String,
    html_url: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    draft: bool,
    user: Option<GithubUser>,
    head: GitRef,
    base: GitRef,
    comments: Option<u32>,
    review_comments: Option<u32>,
}

impl RawPull {
    fn into_pr(self, account_id: &str, repo: &str) -> PullRequest {
        let (author, avatar) = match self.user {
            Some(user) => (user.login, user.avatar_url.unwrap_or_default()),
            None => ("Unknown".to_string(), String::new()),
        };
        PullRequest {
            provider: ProviderKind::Github,
            account_id: account_id.to_string(),
            repo: repo.to_string(),
            number: self.number,
            title: self.title,
            author_display_name: author,
            author_avatar_url: avatar,
            source_branch: self.head.name,
            destination_branch: self.base.name,
            created_on: self.created_at,
            updated_on: self.updated_at,
            comment_count: self.comments.unwrap_or(0) + self.review_comments.unwrap_or(0),
            html_url: self.html_url,
            draft: self.draft,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_repo_from_api_url() {
        assert_eq!(repo_full_name("https://api.github.com/repos/acme/web"), Some("acme/web"));
        assert_eq!(repo_full_name("https://example.com/other"), None);
    }

    #[test]
    fn scope_matches_owner_case_insensitively() {
        assert!(in_scope("Acme/web", "acme", &[]));
        assert!(!in_scope("other/web", "acme", &[]));
    }

    #[test]
    fn scope_filters_by_repo_name_or_full_name() {
        let repos = vec!["web".to_string(), "acme/api".to_string()];
        assert!(in_scope("acme/web", "acme", &repos));
        assert!(in_scope("acme/API", "acme", &repos));
        assert!(!in_scope("acme/docs", "acme", &repos));
    }

    #[test]
    fn search_item_and_pull_parse() {
        let item: SearchItem = serde_json::from_str(
            r#"{"repository_url":"https://api.github.com/repos/acme/web",
                "pull_request":{"url":"https://api.github.com/repos/acme/web/pulls/7"},"extra":1}"#,
        )
        .unwrap();
        assert_eq!(item.pull_request.unwrap().url, "https://api.github.com/repos/acme/web/pulls/7");

        let pr = serde_json::from_str::<RawPull>(
            r#"{"number":7,"title":"Add x","html_url":"https://github.com/acme/web/pull/7",
                "created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-02T00:00:00Z","draft":true,
                "user":{"login":"octo","avatar_url":"http://a/o.png"},
                "head":{"ref":"feat"},"base":{"ref":"main"},"comments":2,"review_comments":3}"#,
        )
        .unwrap()
        .into_pr("acc", "acme/web");
        assert_eq!(pr.key(), "acc/acme/web#7");
        assert_eq!((pr.comment_count, pr.draft), (5, true));
        assert_eq!((pr.source_branch.as_str(), pr.destination_branch.as_str()), ("feat", "main"));
        assert_eq!(pr.author_display_name, "octo");
    }
}
