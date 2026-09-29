use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;
use tokio::sync::OnceCell;

use super::{FetchReport, ReviewProvider};
use crate::{
    domain::{ProviderKind, PullRequest},
    error::{AppError, AppResult},
};

const API: &str = "https://api.bitbucket.org/2.0";

// `+values.participants` adds participants to the default response (needed for the approved
// check). `draft` is already part of the default PR response.
const FIELDS: &str = "%2Bvalues.participants";

pub struct Bitbucket {
    client: reqwest::Client,
    account_id: String,
    workspace: String,
    token: String,
    /// Bitbucket's own account id of the token owner, resolved once.
    user_id: OnceCell<String>,
    /// Server-side `reviewers.account_id` filtering. Flipped off for good the first time
    /// Bitbucket rejects it (depends on the token type), falling back to client-side filtering.
    use_q_filter: AtomicBool,
}

impl Bitbucket {
    pub fn new(client: reqwest::Client, account_id: String, workspace: String, token: String) -> Self {
        Self {
            client,
            account_id,
            workspace,
            token,
            user_id: OnceCell::new(),
            use_q_filter: AtomicBool::new(true),
        }
    }

    async fn user_id(&self) -> AppResult<&str> {
        self.user_id
            .get_or_try_init(|| async {
                let user: BitbucketUser = self
                    .client
                    .get(format!("{API}/user"))
                    .bearer_auth(&self.token)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                user.account_id
                    .ok_or_else(|| AppError::Provider("Missing account_id in /user response".into()))
            })
            .await
            .map(String::as_str)
    }

    async fn fetch_repo(&self, repo: &str, user_id: &str) -> AppResult<Vec<PullRequest>> {
        let mut prs = Vec::new();
        let mut use_q = self.use_q_filter.load(Ordering::Relaxed);
        let workspace = &self.workspace;

        let mut url = Some(if use_q {
            // state="OPEN" must be inside q — Bitbucket ignores a standalone state= when q is present.
            format!(
                "{API}/repositories/{workspace}/{repo}/pullrequests?pagelen=50&fields={FIELDS}&q=state%3D%22OPEN%22+AND+reviewers.account_id%3D%22{user_id}%22"
            )
        } else {
            open_prs_url(workspace, repo)
        });

        while let Some(next_url) = url {
            log::info!("GET {next_url}");
            let response = self.client.get(&next_url).bearer_auth(&self.token).send().await?;
            let status = response.status();
            log::info!("  → HTTP {status}");

            if use_q && status == 400 {
                log::info!("  q filter not supported, switching to full fetch + client-side filter");
                use_q = false;
                self.use_q_filter.store(false, Ordering::Relaxed);
                url = Some(open_prs_url(workspace, repo));
                continue;
            }

            let page: PagedResponse<RawPullRequest> = response.error_for_status()?.json().await?;

            for raw in page.values {
                // Once approved there is nothing left to do, so it is not "pending review".
                if raw.is_approved_by(user_id) {
                    continue;
                }
                if use_q || raw.is_reviewer(user_id) {
                    prs.push(raw.into_pr(&self.account_id, repo));
                }
            }
            url = page.next;
        }

        Ok(prs)
    }
}

fn open_prs_url(workspace: &str, repo: &str) -> String {
    format!("{API}/repositories/{workspace}/{repo}/pullrequests?state=OPEN&pagelen=50&fields={FIELDS}")
}

#[async_trait]
impl ReviewProvider for Bitbucket {
    async fn validate(&self) -> AppResult<String> {
        self.user_id().await.map(str::to_string)
    }

    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        let user_id = self.user_id().await?;

        let results = join_all(repos.iter().map(|repo| async move {
            (repo.clone(), self.fetch_repo(repo, user_id).await)
        }))
        .await;

        let mut prs = Vec::new();
        let mut failed_repos = Vec::new();
        for (repo, result) in results {
            match result {
                Ok(repo_prs) => prs.extend(repo_prs),
                Err(e) => {
                    log::warn!("Failed to fetch PRs for repo {repo}: {e}");
                    failed_repos.push((repo, e));
                }
            }
        }

        Ok(FetchReport { prs, failed_repos })
    }
}

// ── API payloads ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PagedResponse<T> {
    values: Vec<T>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BitbucketUser {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Link {
    href: String,
}

#[derive(Debug, Deserialize)]
struct UserLinks {
    avatar: Option<Link>,
}

#[derive(Debug, Deserialize)]
struct Author {
    display_name: Option<String>,
    links: Option<UserLinks>,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SourceDest {
    branch: BranchRef,
}

#[derive(Debug, Deserialize)]
struct PrLinks {
    html: Link,
}

#[derive(Debug, Deserialize)]
struct ParticipantUser {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Participant {
    role: String,
    #[serde(default)]
    approved: bool,
    user: ParticipantUser,
}

#[derive(Debug, Deserialize)]
struct RawPullRequest {
    id: u64,
    title: String,
    author: Author,
    source: SourceDest,
    destination: SourceDest,
    created_on: String,
    updated_on: String,
    comment_count: Option<u32>,
    links: PrLinks,
    participants: Option<Vec<Participant>>,
    #[serde(default)]
    draft: bool,
}

impl RawPullRequest {
    fn participants_of<'a>(&'a self, user_id: &'a str) -> impl Iterator<Item = &'a Participant> {
        self.participants
            .iter()
            .flatten()
            .filter(move |p| p.user.account_id.as_deref() == Some(user_id))
    }

    fn is_reviewer(&self, user_id: &str) -> bool {
        self.participants_of(user_id).any(|p| p.role == "REVIEWER")
    }

    fn is_approved_by(&self, user_id: &str) -> bool {
        self.participants_of(user_id).any(|p| p.approved)
    }

    fn into_pr(self, account_id: &str, repo: &str) -> PullRequest {
        PullRequest {
            provider: ProviderKind::Bitbucket,
            account_id: account_id.to_string(),
            repo: repo.to_string(),
            number: self.id,
            title: self.title,
            author_display_name: self
                .author
                .display_name
                .unwrap_or_else(|| "Unknown".to_string()),
            author_avatar_url: self
                .author
                .links
                .and_then(|l| l.avatar)
                .map(|a| a.href)
                .unwrap_or_default(),
            source_branch: self.source.branch.name,
            destination_branch: self.destination.branch.name,
            created_on: self.created_on,
            updated_on: self.updated_on,
            comment_count: self.comment_count.unwrap_or(0),
            html_url: self.links.html.href,
            draft: self.draft,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> RawPullRequest {
        serde_json::from_str(json).unwrap()
    }

    const PR: &str = r#"{
        "id": 12, "title": "Fix login", "created_on": "2026-01-01T00:00:00Z",
        "updated_on": "2026-01-02T00:00:00Z", "comment_count": 3, "draft": true,
        "author": {"display_name": "John", "links": {"avatar": {"href": "http://a/x.png"}}},
        "source": {"branch": {"name": "feat"}}, "destination": {"branch": {"name": "main"}},
        "links": {"html": {"href": "http://pr/12"}},
        "participants": [
            {"role": "PARTICIPANT", "approved": false, "user": {"account_id": "other"}},
            {"role": "REVIEWER", "approved": true, "user": {"account_id": "me"}}
        ]
    }"#;

    #[test]
    fn maps_fields_and_detects_approval() {
        let pr = parse(PR);
        assert!(pr.is_reviewer("me"));
        assert!(!pr.is_reviewer("other"));
        assert!(!pr.is_reviewer("nobody"));

        assert!(pr.is_approved_by("me"));
        assert!(!pr.is_approved_by("other"));

        let pr = pr.into_pr("acc", "repo");
        assert_eq!(pr.key(), "acc/repo#12");
        assert!(pr.draft);
        assert_eq!(pr.comment_count, 3);
        assert_eq!(pr.source_branch, "feat");
        assert_eq!(pr.author_avatar_url, "http://a/x.png");
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let pr = parse(
            r#"{"id":1,"title":"t","created_on":"a","updated_on":"b","author":{},
                "source":{"branch":{"name":"s"}},"destination":{"branch":{"name":"d"}},
                "links":{"html":{"href":"h"}}}"#,
        )
        .into_pr("acc", "repo");
        assert_eq!(pr.author_display_name, "Unknown");
        assert!(!pr.draft);
        assert_eq!(pr.comment_count, 0);
    }
}
