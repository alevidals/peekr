//! GitHub via the REST API.
//!
//! One search (`review-requested:@me`) lists every open PR waiting on the user — including
//! requests made to teams they belong to, and dropping PRs once a review has been submitted.
//! Search results lack branches and full comment counts, so each PR is then fetched once,
//! together with its reviews (approvals) and its checks (Actions check-runs + commit statuses).
//! The reviews/checks calls are best effort: without the token permission they just yield no data.
//!
//! The user's own PRs come from one GraphQL query instead: GitHub itself computes whether
//! each PR is mergeable (`mergeStateStatus`, which accounts for the *required* checks and
//! reviews of the branch protection), so no rule has to be reimplemented here.

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::RequestBuilder;
use serde::Deserialize;
use tokio::sync::OnceCell;

use super::{FetchReport, ReviewProvider};
use crate::{
    domain::{CiStatus, MergeState, MergeStatus, ProviderKind, PullRequest},
    error::{AppError, AppResult},
    http,
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
    /// Login of the token owner, resolved once.
    login: OnceCell<String>,
}

impl Github {
    pub fn new(client: reqwest::Client, account_id: String, owner: String, token: String) -> Self {
        Self {
            client,
            account_id,
            owner,
            token,
            login: OnceCell::new(),
        }
    }

    fn get(&self, url: &str) -> RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    async fn login(&self) -> AppResult<&str> {
        self.login
            .get_or_try_init(|| async {
                let user: GithubUser =
                    http::json(self.get(&format!("{API}/user")).send().await?).await?;
                Ok::<_, AppError>(user.login)
            })
            .await
            .map(String::as_str)
    }

    async fn search_review_requests(&self) -> AppResult<Vec<SearchItem>> {
        let mut items = Vec::new();
        for page in 1..=MAX_PAGES {
            let response: SearchResponse = http::json(
                self.get(&format!("{API}/search/issues"))
                    .query(&[
                        ("q", "is:pr is:open review-requested:@me"),
                        ("per_page", &PER_PAGE.to_string()),
                        ("page", &page.to_string()),
                    ])
                    .send()
                    .await?,
            )
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
        let raw: RawPull = http::json(self.get(pull_url).send().await?).await?;
        let (approvals, ci_status) = tokio::join!(
            self.approvals(repo, raw.number),
            self.ci_status(repo, raw.head.sha.as_deref())
        );
        let mut pr = raw.into_pr(&self.account_id, repo);
        pr.approvals = approvals;
        pr.ci_status = ci_status;
        Ok(pr)
    }

    /// GET + JSON that maps any failure to `None` (logged), for the optional extras.
    async fn get_optional<T: serde::de::DeserializeOwned>(&self, url: String) -> Option<T> {
        let result = async { http::json(self.get(&url).send().await?).await }.await;
        result
            .map_err(|e| log::debug!("Optional GitHub call {url} failed: {e}"))
            .ok()
    }

    async fn approvals(&self, repo: &str, number: u64) -> u32 {
        let url = format!("{API}/repos/{repo}/pulls/{number}/reviews?per_page=100");
        self.get_optional::<Vec<RawReview>>(url)
            .await
            .map_or(0, |reviews| count_approvals(&reviews))
    }

    async fn ci_status(&self, repo: &str, sha: Option<&str>) -> Option<CiStatus> {
        let sha = sha?;
        let (runs, status) = tokio::join!(
            self.get_optional::<CheckRuns>(format!(
                "{API}/repos/{repo}/commits/{sha}/check-runs?per_page=100"
            )),
            self.get_optional::<CombinedStatus>(format!("{API}/repos/{repo}/commits/{sha}/status")),
        );
        let from_runs = runs
            .into_iter()
            .flat_map(|r| r.check_runs)
            .map(|r| r.ci_status());
        let from_status = status.and_then(|s| s.ci_status());
        CiStatus::combine(from_runs.chain(from_status))
    }
}

#[async_trait]
impl ReviewProvider for Github {
    async fn validate(&self) -> AppResult<String> {
        self.login().await.map(str::to_string)
    }

    async fn fetch_my_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        let login = self.login().await?;
        let response = self
            .client
            .post(format!("{API}/graphql"))
            .bearer_auth(&self.token)
            .header("User-Agent", "peekr")
            .json(&serde_json::json!({
                "query": MY_PRS_QUERY,
                "variables": { "q": format!("is:pr is:open archived:false author:{login}") },
            }))
            .send()
            .await?
            .error_for_status()?
            .json::<GraphqlResponse>()
            .await?;

        if let Some(error) = response.errors.as_ref().and_then(|e| e.first()) {
            // Partial data is still usable, but with no data at all the error is the story.
            if response.data.is_none() {
                return Err(AppError::Provider(format!(
                    "GitHub GraphQL: {}",
                    error.message
                )));
            }
            log::warn!("GitHub GraphQL returned errors: {}", error.message);
        }

        let prs = response
            .data
            .into_iter()
            .flat_map(|d| d.search.nodes)
            .flatten()
            .filter(|node| in_scope(&node.repository.name_with_owner, &self.owner, repos))
            .map(|node| node.into_pr(&self.account_id))
            .collect();
        Ok(FetchReport {
            prs,
            failed_repos: Vec::new(),
        })
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

// ── GraphQL (own PRs) ────────────────────────────────────────────────────────

const MY_PRS_QUERY: &str = r#"
query($q: String!) {
  search(query: $q, type: ISSUE, first: 100) {
    nodes {
      ... on PullRequest {
        number title url isDraft createdAt updatedAt headRefName baseRefName
        repository { nameWithOwner }
        author { login avatarUrl }
        comments { totalCount }
        reviewDecision
        mergeable
        mergeStateStatus
        latestOpinionatedReviews(first: 50) { nodes { state } }
        commits(last: 1) {
          nodes {
            commit {
              statusCheckRollup {
                state
                contexts(first: 100) {
                  nodes {
                    __typename
                    ... on CheckRun { name status conclusion }
                    ... on StatusContext { context state }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}"#;

#[derive(Debug, Deserialize)]
struct GraphqlResponse {
    data: Option<GraphqlData>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlData {
    search: GraphqlSearch,
}

#[derive(Debug, Deserialize)]
struct GraphqlSearch {
    /// Entries are `null` when the token cannot see the item, and `{}` for non-PR nodes.
    nodes: Vec<Option<GqlPull>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlPull {
    number: u64,
    title: String,
    url: String,
    is_draft: bool,
    created_at: String,
    updated_at: String,
    head_ref_name: String,
    base_ref_name: String,
    repository: GqlRepo,
    author: Option<GqlAuthor>,
    comments: GqlCount,
    review_decision: Option<String>,
    mergeable: String,
    merge_state_status: String,
    latest_opinionated_reviews: Option<GqlNodes<GqlReview>>,
    commits: GqlNodes<GqlCommitNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlRepo {
    name_with_owner: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlAuthor {
    login: String,
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlCount {
    total_count: u32,
}

#[derive(Debug, Deserialize)]
struct GqlNodes<T> {
    #[serde(default = "Vec::new")]
    nodes: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct GqlReview {
    state: String,
}

#[derive(Debug, Deserialize)]
struct GqlCommitNode {
    commit: GqlCommit,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlCommit {
    status_check_rollup: Option<GqlRollup>,
}

#[derive(Debug, Deserialize)]
struct GqlRollup {
    state: String,
    contexts: GqlNodes<GqlContext>,
}

/// A check run or a legacy commit status; fields of the other kind are absent.
#[derive(Debug, Default, Deserialize)]
struct GqlContext {
    // CheckRun
    name: Option<String>,
    conclusion: Option<String>,
    // StatusContext
    context: Option<String>,
    state: Option<String>,
}

impl GqlContext {
    fn failed_name(&self) -> Option<&str> {
        let failed = match (&self.conclusion, &self.state) {
            (Some(c), _) => matches!(
                c.as_str(),
                "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
            ),
            (None, Some(s)) => matches!(s.as_str(), "FAILURE" | "ERROR"),
            (None, None) => false,
        };
        failed.then(|| {
            self.name
                .as_deref()
                .or(self.context.as_deref())
                .unwrap_or("check")
        })
    }
}

/// Folds GitHub's own verdict into a [`MergeStatus`]. `BLOCKED` is explained from the review
/// decision and the failing checks so the user knows what to do about it.
fn assess(pull: &GqlPull, rollup: Option<&GqlRollup>) -> MergeStatus {
    if pull.is_draft {
        return MergeStatus::new(MergeState::Draft, vec![]);
    }
    let failing: Vec<&str> = rollup
        .into_iter()
        .flat_map(|r| &r.contexts.nodes)
        .filter_map(GqlContext::failed_name)
        .collect();
    let checks_running = rollup.is_some_and(|r| matches!(r.state.as_str(), "PENDING" | "EXPECTED"));

    let mut reasons = Vec::new();
    if pull.review_decision.as_deref() == Some("CHANGES_REQUESTED") {
        reasons.push("Changes requested".to_string());
    }
    if !failing.is_empty() {
        let shown: Vec<&str> = failing.iter().copied().take(3).collect();
        let more = failing.len() - shown.len();
        let extra = if more > 0 {
            format!(" +{more}")
        } else {
            String::new()
        };
        reasons.push(format!("Failing: {}{extra}", shown.join(", ")));
    }

    if pull.mergeable == "CONFLICTING" || pull.merge_state_status == "DIRTY" {
        return MergeStatus::new(MergeState::Conflicts, vec!["Merge conflicts".into()]);
    }
    match pull.merge_state_status.as_str() {
        // UNSTABLE: only non-required checks are failing; HAS_HOOKS: mergeable with pre-receive hooks.
        "CLEAN" | "UNSTABLE" | "HAS_HOOKS" => MergeStatus::new(MergeState::Ready, vec![]),
        "BEHIND" => MergeStatus::new(MergeState::Behind, vec!["Branch is behind its base".into()]),
        "BLOCKED" => {
            if !reasons.is_empty() {
                return MergeStatus::new(MergeState::Blocked, reasons);
            }
            if checks_running {
                return MergeStatus::new(MergeState::Waiting, vec!["Checks running".into()]);
            }
            if pull.review_decision.as_deref() == Some("REVIEW_REQUIRED") {
                return MergeStatus::new(MergeState::Waiting, vec!["Review required".into()]);
            }
            MergeStatus::new(
                MergeState::Blocked,
                vec!["Blocked by branch protection".into()],
            )
        }
        _ => MergeStatus::new(MergeState::Unknown, vec![]),
    }
}

impl GqlPull {
    fn into_pr(self, account_id: &str) -> PullRequest {
        let rollup = self
            .commits
            .nodes
            .iter()
            .find_map(|n| n.commit.status_check_rollup.as_ref());
        let merge = assess(&self, rollup);
        let ci_status = rollup.and_then(|r| match r.state.as_str() {
            "SUCCESS" => Some(CiStatus::Success),
            "FAILURE" | "ERROR" => Some(CiStatus::Failed),
            "PENDING" | "EXPECTED" => Some(CiStatus::Pending),
            _ => None,
        });
        let approvals = self
            .latest_opinionated_reviews
            .iter()
            .flat_map(|r| &r.nodes)
            .filter(|r| r.state == "APPROVED")
            .count() as u32;
        let (author, avatar) = self
            .author
            .map_or(("Unknown".to_string(), String::new()), |a| {
                (a.login, a.avatar_url)
            });
        PullRequest {
            provider: ProviderKind::Github,
            account_id: account_id.to_string(),
            repo: self.repository.name_with_owner,
            number: self.number,
            title: self.title,
            author_display_name: author,
            author_avatar_url: avatar,
            source_branch: self.head_ref_name,
            destination_branch: self.base_ref_name,
            created_on: self.created_at,
            updated_on: self.updated_at,
            comment_count: self.comments.total_count,
            html_url: self.url,
            draft: self.is_draft,
            approvals,
            ci_status,
            merge: Some(merge),
        }
    }
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
    /// Head commit; only read for `head`.
    sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawReview {
    user: Option<ReviewUser>,
    state: String,
}

#[derive(Debug, Deserialize)]
struct ReviewUser {
    login: String,
}

/// Reviews come oldest first; a reviewer's last approve/request-changes/dismissal decides,
/// while plain comments don't change their standing.
fn count_approvals(reviews: &[RawReview]) -> u32 {
    let mut standing: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for review in reviews {
        let Some(user) = &review.user else { continue };
        match review.state.as_str() {
            "APPROVED" => standing.insert(&user.login, true),
            "CHANGES_REQUESTED" | "DISMISSED" => standing.insert(&user.login, false),
            _ => None,
        };
    }
    standing.values().filter(|approved| **approved).count() as u32
}

#[derive(Debug, Deserialize)]
struct CheckRuns {
    check_runs: Vec<CheckRun>,
}

#[derive(Debug, Deserialize)]
struct CheckRun {
    status: String,
    conclusion: Option<String>,
}

impl CheckRun {
    fn ci_status(&self) -> CiStatus {
        if self.status != "completed" {
            return CiStatus::Pending;
        }
        match self.conclusion.as_deref() {
            Some("success" | "neutral" | "skipped") => CiStatus::Success,
            _ => CiStatus::Failed,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CombinedStatus {
    state: String,
    #[serde(default)]
    total_count: u32,
}

impl CombinedStatus {
    /// `None` when the commit has no legacy statuses (GitHub reports "pending" for that).
    fn ci_status(&self) -> Option<CiStatus> {
        if self.total_count == 0 {
            return None;
        }
        Some(match self.state.as_str() {
            "success" => CiStatus::Success,
            "pending" => CiStatus::Pending,
            _ => CiStatus::Failed,
        })
    }
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
            approvals: 0,
            ci_status: None,
            merge: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_repo_from_api_url() {
        assert_eq!(
            repo_full_name("https://api.github.com/repos/acme/web"),
            Some("acme/web")
        );
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
    fn approvals_use_each_reviewers_latest_verdict() {
        let review = |login: &str, state: &str| RawReview {
            user: Some(ReviewUser {
                login: login.into(),
            }),
            state: state.into(),
        };
        let reviews = [
            review("a", "APPROVED"),
            review("a", "COMMENTED"),
            review("b", "APPROVED"),
            review("b", "CHANGES_REQUESTED"),
            review("c", "APPROVED"),
        ];
        assert_eq!(count_approvals(&reviews), 2);
    }

    #[test]
    fn maps_check_runs_and_statuses() {
        let run = |status: &str, conclusion: Option<&str>| {
            CheckRun {
                status: status.into(),
                conclusion: conclusion.map(Into::into),
            }
            .ci_status()
        };
        assert_eq!(run("completed", Some("success")), CiStatus::Success);
        assert_eq!(run("completed", Some("skipped")), CiStatus::Success);
        assert_eq!(run("completed", Some("failure")), CiStatus::Failed);
        assert_eq!(run("in_progress", None), CiStatus::Pending);

        let status = |state: &str, total_count| {
            CombinedStatus {
                state: state.into(),
                total_count,
            }
            .ci_status()
        };
        assert_eq!(status("pending", 0), None);
        assert_eq!(status("failure", 2), Some(CiStatus::Failed));
    }

    fn gql_pull(merge_state: &str, review_decision: Option<&str>, rollup: &str) -> GqlPull {
        let json = format!(
            r#"{{"number":3,"title":"t","url":"u","isDraft":false,"createdAt":"a","updatedAt":"b",
                "headRefName":"f","baseRefName":"main","repository":{{"nameWithOwner":"acme/web"}},
                "author":{{"login":"me","avatarUrl":"av"}},"comments":{{"totalCount":4}},
                "reviewDecision":{},"mergeable":"MERGEABLE","mergeStateStatus":"{merge_state}",
                "latestOpinionatedReviews":{{"nodes":[{{"state":"APPROVED"}},{{"state":"CHANGES_REQUESTED"}}]}},
                "commits":{{"nodes":[{{"commit":{{"statusCheckRollup":{rollup}}}}}]}}}}"#,
            review_decision.map_or("null".to_string(), |d| format!("\"{d}\"")),
        );
        serde_json::from_str(&json).unwrap()
    }

    const FAILING: &str = r#"{"state":"FAILURE","contexts":{"nodes":[
        {"__typename":"CheckRun","name":"build","status":"COMPLETED","conclusion":"FAILURE"},
        {"__typename":"CheckRun","name":"lint","status":"COMPLETED","conclusion":"SUCCESS"},
        {"__typename":"StatusContext","context":"ci/legacy","state":"ERROR"}]}}"#;
    const PASSING: &str = r#"{"state":"SUCCESS","contexts":{"nodes":[]}}"#;

    #[test]
    fn clean_pr_is_ready_and_mapped() {
        let pr = gql_pull("CLEAN", Some("APPROVED"), PASSING).into_pr("acc");
        let merge = pr.merge.clone().unwrap();
        assert!(merge.is_ready() && merge.blockers.is_empty());
        assert_eq!(pr.key(), "acc/acme/web#3");
        assert_eq!((pr.approvals, pr.comment_count), (1, 4));
        assert_eq!(pr.ci_status, Some(CiStatus::Success));
    }

    #[test]
    fn blocked_pr_names_what_fails() {
        let pr = gql_pull("BLOCKED", Some("CHANGES_REQUESTED"), FAILING).into_pr("acc");
        let merge = pr.merge.unwrap();
        assert_eq!(merge.state, MergeState::Blocked);
        assert_eq!(
            merge.blockers,
            vec!["Changes requested", "Failing: build, ci/legacy"]
        );
        assert_eq!(pr.ci_status, Some(CiStatus::Failed));
    }

    #[test]
    fn blocked_without_failures_is_waiting_or_protected() {
        let running = r#"{"state":"PENDING","contexts":{"nodes":[]}}"#;
        let state = |d, r| gql_pull("BLOCKED", d, r).into_pr("a").merge.unwrap();
        assert_eq!(state(None, running).blockers, vec!["Checks running"]);
        assert_eq!(
            state(Some("REVIEW_REQUIRED"), PASSING).blockers,
            vec!["Review required"]
        );
        assert_eq!(state(None, PASSING).state, MergeState::Blocked);
    }

    #[test]
    fn other_merge_states() {
        let state = |m| gql_pull(m, None, PASSING).into_pr("a").merge.unwrap().state;
        assert_eq!(state("UNSTABLE"), MergeState::Ready);
        assert_eq!(state("BEHIND"), MergeState::Behind);
        assert_eq!(state("DIRTY"), MergeState::Conflicts);
        assert_eq!(state("UNKNOWN"), MergeState::Unknown);
    }

    #[test]
    fn search_nodes_tolerate_nulls_and_missing_rollup() {
        let data: GraphqlData = serde_json::from_str(r#"{"search":{"nodes":[null]}}"#).unwrap();
        assert!(data.search.nodes[0].is_none());
        let pr = gql_pull("CLEAN", None, "null").into_pr("a");
        assert_eq!(pr.ci_status, None);
    }

    #[test]
    fn search_item_and_pull_parse() {
        let item: SearchItem = serde_json::from_str(
            r#"{"repository_url":"https://api.github.com/repos/acme/web",
                "pull_request":{"url":"https://api.github.com/repos/acme/web/pulls/7"},"extra":1}"#,
        )
        .unwrap();
        assert_eq!(
            item.pull_request.unwrap().url,
            "https://api.github.com/repos/acme/web/pulls/7"
        );

        let pr = serde_json::from_str::<RawPull>(
            r#"{"number":7,"title":"Add x","html_url":"https://github.com/acme/web/pull/7",
                "created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-02T00:00:00Z","draft":true,
                "user":{"login":"octo","avatar_url":"http://a/o.png"},
                "head":{"ref":"feat","sha":"abc"},"base":{"ref":"main"},"comments":2,"review_comments":3}"#,
        )
        .unwrap()
        .into_pr("acc", "acme/web");
        assert_eq!(pr.key(), "acc/acme/web#7");
        assert_eq!((pr.comment_count, pr.draft), (5, true));
        assert_eq!(
            (pr.source_branch.as_str(), pr.destination_branch.as_str()),
            ("feat", "main")
        );
        assert_eq!(pr.author_display_name, "octo");
    }
}
