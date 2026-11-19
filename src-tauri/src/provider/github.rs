//! GitHub via the GraphQL API.
//!
//! One search per side lists the PRs: `review-requested:@me` for the ones waiting on the user
//! (including requests made to teams they belong to) and `author:@me` for the user's own. A
//! `PullRequest` node already carries branches, the comment count, the reviews and the check
//! rollup, so no per-PR request is needed.
//!
//! For the user's own PRs GitHub computes the verdict itself (`mergeStateStatus`, which accounts
//! for the *required* checks and reviews of the branch protection), so no rule is reimplemented
//! here.

use async_trait::async_trait;
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
/// Search never returns more than 1000 results.
const MAX_PAGES: usize = 10;
const REVIEW_SCOPE: &str = "is:pr is:open review-requested:@me";

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

    fn post_graphql(&self, body: &serde_json::Value) -> RequestBuilder {
        self.client
            .post(format!("{API}/graphql"))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .json(body)
    }

    async fn login(&self) -> AppResult<&str> {
        self.login
            .get_or_try_init(|| async {
                let user: GithubUser =
                    http::send_json(&self.client, self.get(&format!("{API}/user"))).await?;
                Ok::<_, AppError>(user.login)
            })
            .await
            .map(String::as_str)
    }

    /// Runs a GitHub search page by page and returns the PR nodes it contains.
    async fn search(&self, query: &str) -> AppResult<Vec<GqlPull>> {
        let mut nodes = Vec::new();
        let mut cursor: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let request = self.post_graphql(&serde_json::json!({
                "query": SEARCH_QUERY,
                "variables": { "q": query, "cursor": cursor },
            }));
            let data = graphql_data(http::send_json(&self.client, request).await?)?;
            let GraphqlSearch {
                page_info,
                nodes: page,
            } = data.search;
            nodes.extend(page.into_iter().flatten());

            match page_info.end_cursor {
                Some(next) if page_info.has_next_page => cursor = Some(next),
                _ => break,
            }
        }

        Ok(nodes)
    }

    /// Keeps the nodes that belong to this account and maps them to the domain type.
    fn to_prs(
        &self,
        nodes: Vec<GqlPull>,
        repos: &[String],
        include_merge: bool,
    ) -> Vec<PullRequest> {
        nodes
            .into_iter()
            .filter(|node| in_scope(&node.repository.name_with_owner, &self.owner, repos))
            .map(|node| node.into_pr(&self.account_id, include_merge))
            .collect()
    }
}

#[async_trait]
impl ReviewProvider for Github {
    async fn validate(&self) -> AppResult<String> {
        self.login().await.map(str::to_string)
    }

    async fn fetch_my_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        let login = self.login().await?;
        let nodes = self
            .search(&format!("is:pr is:open archived:false author:{login}"))
            .await?;
        Ok(FetchReport {
            prs: self.to_prs(nodes, repos, true),
            failed_repos: Vec::new(),
        })
    }

    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        // The search spans every owner the token can see; `in_scope` keeps this account's.
        let nodes = self.search(REVIEW_SCOPE).await?;
        Ok(FetchReport {
            prs: self.to_prs(nodes, repos, false),
            failed_repos: Vec::new(),
        })
    }
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

// ── Search query ─────────────────────────────────────────────────────────────

const SEARCH_QUERY: &str = r#"
query($q: String!, $cursor: String) {
  search(query: $q, type: ISSUE, first: 100, after: $cursor) {
    pageInfo { hasNextPage endCursor }
    nodes {
      ... on PullRequest {
        number title url isDraft createdAt updatedAt headRefName baseRefName
        repository { nameWithOwner }
        author { login name avatarUrl }
        # Issue and review comments together, as GitHub's own PR list counts them.
        totalCommentsCount
        comments { totalCount }
        reviewDecision
        mergeable
        mergeStateStatus
        # Approvals above 20 and checks above 20 are rare, and every nested node costs
        # rate-limit points: the rollup state still reports the aggregate.
        latestOpinionatedReviews(first: 20) { nodes { state } }
        commits(last: 1) {
          nodes {
            commit {
              statusCheckRollup {
                state
                contexts(first: 20) {
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

// ── GraphQL payloads ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GraphqlResponse {
    data: Option<GraphqlData>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

/// A response with errors and no data cannot be used; with partial data the errors only warn.
fn graphql_data(response: GraphqlResponse) -> AppResult<GraphqlData> {
    if let Some(error) = response.errors.as_ref().and_then(|errors| errors.first()) {
        if response.data.is_none() {
            return Err(AppError::Provider(format!(
                "GitHub GraphQL: {}",
                error.message
            )));
        }
        log::warn!("GitHub GraphQL returned errors: {}", error.message);
    }
    response
        .data
        .ok_or_else(|| AppError::Provider("GitHub GraphQL returned no data".into()))
}

#[derive(Debug, Deserialize)]
struct GraphqlData {
    search: GraphqlSearch,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphqlSearch {
    page_info: PageInfo,
    /// Entries are `null` when the token cannot see the item.
    nodes: Vec<Option<GqlPull>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
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
    /// Issue + review comments; null when GitHub cannot compute it.
    total_comments_count: Option<u32>,
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
    /// Display name; falls back to the login when the profile does not set one.
    name: Option<String>,
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlCount {
    total_count: u32,
}

#[derive(Debug, Deserialize)]
struct GqlNodes<T> {
    // `default = "Vec::new"` rather than `default`: serde's derive would otherwise demand
    // `T: Default` for every node type.
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
    /// `include_merge` is false for PRs the user only reviews: peekr shows merge readiness only
    /// for their own PRs, and another author's blockers are not actionable here.
    fn into_pr(self, account_id: &str, include_merge: bool) -> PullRequest {
        let rollup = self
            .commits
            .nodes
            .iter()
            .find_map(|n| n.commit.status_check_rollup.as_ref());
        let merge = include_merge.then(|| assess(&self, rollup));
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
                let name = a
                    .name
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or(a.login);
                (name, a.avatar_url)
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
            comment_count: self
                .total_comments_count
                .unwrap_or(self.comments.total_count),
            html_url: self.url,
            draft: self.is_draft,
            approvals,
            ci_status,
            merge,
        }
    }
}

// ── Login payload ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubUser {
    login: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn gql_pull(merge_state: &str, review_decision: Option<&str>, rollup: &str) -> GqlPull {
        let json = format!(
            r#"{{"number":3,"title":"t","url":"u","isDraft":false,"createdAt":"a","updatedAt":"b",
                "headRefName":"f","baseRefName":"main","repository":{{"nameWithOwner":"acme/web"}},
                "author":{{"login":"me","name":"Me","avatarUrl":"av"}},
                "totalCommentsCount":9,"comments":{{"totalCount":4}},
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
        let pr = gql_pull("CLEAN", Some("APPROVED"), PASSING).into_pr("acc", true);
        let merge = pr.merge.clone().unwrap();
        assert!(merge.is_ready() && merge.blockers.is_empty());
        assert_eq!(pr.key(), "acc/acme/web#3");
        assert_eq!((pr.approvals, pr.comment_count), (1, 9));
        assert_eq!(pr.ci_status, Some(CiStatus::Success));
    }

    #[test]
    fn comment_count_falls_back_to_issue_comments() {
        let mut pull = gql_pull("CLEAN", None, PASSING);
        pull.total_comments_count = None;
        assert_eq!(pull.into_pr("acc", true).comment_count, 4);
    }

    #[test]
    fn author_uses_the_display_name_and_falls_back_to_the_login() {
        let pr = gql_pull("CLEAN", None, PASSING).into_pr("acc", true);
        assert_eq!(pr.author_display_name, "Me");

        let mut pull = gql_pull("CLEAN", None, PASSING);
        pull.author.as_mut().unwrap().name = None;
        assert_eq!(pull.into_pr("acc", true).author_display_name, "me");

        let mut pull = gql_pull("CLEAN", None, PASSING);
        pull.author.as_mut().unwrap().name = Some("  ".into());
        assert_eq!(pull.into_pr("acc", true).author_display_name, "me");
    }

    #[test]
    fn reviewed_prs_carry_no_merge_status() {
        let pr = gql_pull("CLEAN", None, PASSING).into_pr("acc", false);
        assert!(pr.merge.is_none());
        assert_eq!(pr.ci_status, Some(CiStatus::Success));
    }

    #[test]
    fn blocked_pr_names_what_fails() {
        let pr = gql_pull("BLOCKED", Some("CHANGES_REQUESTED"), FAILING).into_pr("acc", true);
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
        let state = |d, r| gql_pull("BLOCKED", d, r).into_pr("a", true).merge.unwrap();
        assert_eq!(state(None, running).blockers, vec!["Checks running"]);
        assert_eq!(
            state(Some("REVIEW_REQUIRED"), PASSING).blockers,
            vec!["Review required"]
        );
        assert_eq!(state(None, PASSING).state, MergeState::Blocked);
    }

    #[test]
    fn other_merge_states() {
        let state = |m| {
            gql_pull(m, None, PASSING)
                .into_pr("a", true)
                .merge
                .unwrap()
                .state
        };
        assert_eq!(state("UNSTABLE"), MergeState::Ready);
        assert_eq!(state("BEHIND"), MergeState::Behind);
        assert_eq!(state("DIRTY"), MergeState::Conflicts);
        assert_eq!(state("UNKNOWN"), MergeState::Unknown);
    }

    #[test]
    fn search_nodes_tolerate_nulls_and_missing_rollup() {
        let data: GraphqlData = serde_json::from_str(
            r#"{"search":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[null]}}"#,
        )
        .unwrap();
        assert!(data.search.nodes[0].is_none());
        let pr = gql_pull("CLEAN", None, "null").into_pr("a", true);
        assert_eq!(pr.ci_status, None);
    }

    #[test]
    fn graphql_errors_are_fatal_only_without_data() {
        let no_data: GraphqlResponse =
            serde_json::from_str(r#"{"data":null,"errors":[{"message":"Bad credentials"}]}"#)
                .unwrap();
        assert!(graphql_data(no_data).is_err());

        let partial: GraphqlResponse = serde_json::from_str(
            r#"{"data":{"search":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}},
                "errors":[{"message":"something timed out"}]}"#,
        )
        .unwrap();
        assert!(graphql_data(partial).is_ok());
    }
}
