use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio::sync::{OnceCell, Semaphore};

use super::{FetchReport, ReviewProvider, MAX_CONCURRENCY};
use crate::{
    domain::{CiStatus, MergeState, MergeStatus, ProviderKind, PullRequest},
    error::{AppError, AppResult},
    http,
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
    /// Hard cap on requests in flight for this account, across every repo and PR. Buffering
    /// alone would allow repositories × PRs at once.
    permits: Semaphore,
}

impl Bitbucket {
    pub fn new(
        client: reqwest::Client,
        account_id: String,
        workspace: String,
        token: String,
    ) -> Self {
        Self {
            client,
            account_id,
            workspace,
            token,
            user_id: OnceCell::new(),
            use_q_filter: AtomicBool::new(true),
            permits: Semaphore::new(MAX_CONCURRENCY),
        }
    }

    /// Sends a request, waiting for a free slot first.
    async fn send(&self, request: reqwest::RequestBuilder) -> AppResult<reqwest::Response> {
        let _permit = self
            .permits
            .acquire()
            .await
            .expect("the provider semaphore is never closed");
        Ok(request.send().await?)
    }

    /// JSON call that keeps the slot while the transient-error retry runs.
    async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> AppResult<T> {
        let _permit = self
            .permits
            .acquire()
            .await
            .expect("the provider semaphore is never closed");
        http::send_json(&self.client, request).await
    }

    async fn user_id(&self) -> AppResult<&str> {
        self.user_id
            .get_or_try_init(|| async {
                let request = self
                    .client
                    .get(format!("{API}/user"))
                    .bearer_auth(&self.token);
                let user: BitbucketUser = self.send_json(request).await?;
                user.account_id.ok_or_else(|| {
                    AppError::Provider("Missing account_id in /user response".into())
                })
            })
            .await
            .map(String::as_str)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> AppResult<T> {
        self.send_json(self.client.get(url).bearer_auth(&self.token))
            .await
    }

    /// Pipeline/build status of the PR's head commit. Best effort: a token without the
    /// pipeline/status scope, or any API error, just means "no status".
    async fn ci_status(&self, repo: &str, pr_id: u64) -> Option<CiStatus> {
        let url = format!(
            "{API}/repositories/{}/{repo}/pullrequests/{pr_id}/statuses?pagelen=100&fields=values.state",
            self.workspace
        );
        match self.get_json::<PagedResponse<RawStatus>>(&url).await {
            Ok(page) => CiStatus::combine(page.values.iter().map(RawStatus::ci_status)),
            Err(e) => {
                log::debug!("No CI status for {repo}#{pr_id}: {e}");
                None
            }
        }
    }

    /// Bitbucket's own verdict on every merge rule of the PR (approvals, builds, conflicts…).
    /// Best effort: `None` makes the caller fall back to the heuristic.
    async fn merge_checks(&self, repo: &str, pr_id: u64) -> Option<Vec<MergeCheck>> {
        let url = format!(
            "{API}/repositories/{}/{repo}/pullrequests/{pr_id}/mergeability/checks",
            self.workspace
        );
        match self.get_json::<PagedResponse<MergeCheck>>(&url).await {
            Ok(page) => Some(page.values),
            Err(e) => {
                log::debug!("No mergeability checks for {repo}#{pr_id}: {e}");
                None
            }
        }
    }

    async fn fetch_repo(
        &self,
        repo: &str,
        user_id: &str,
        role: Role,
    ) -> AppResult<Vec<PullRequest>> {
        let mut raws = Vec::new();
        let mut use_q = self.use_q_filter.load(Ordering::Relaxed);
        let workspace = &self.workspace;
        // Same identifier GitHub uses (`owner/name`), so the list and notifications match.
        let full_repo = format!("{workspace}/{repo}");
        let repo_id = full_repo.as_str();

        let mut url = Some(if use_q {
            // state="OPEN" must be inside q — Bitbucket ignores a standalone state= when q is present.
            format!(
                "{API}/repositories/{workspace}/{repo}/pullrequests?pagelen=50&fields={FIELDS}&q=state%3D%22OPEN%22+AND+{}%3D%22{user_id}%22",
                role.q_field()
            )
        } else {
            open_prs_url(workspace, repo)
        });

        while let Some(next_url) = url {
            log::debug!("GET {next_url}");
            let request = self.client.get(&next_url).bearer_auth(&self.token);
            let response = self.send(request).await?;
            let status = response.status();
            log::debug!("  → HTTP {status}");

            if use_q && status == 400 {
                log::info!(
                    "  q filter not supported, switching to full fetch + client-side filter"
                );
                use_q = false;
                self.use_q_filter.store(false, Ordering::Relaxed);
                url = Some(open_prs_url(workspace, repo));
                continue;
            }

            let page: PagedResponse<RawPullRequest> = http::json(response).await?;

            for raw in page.values {
                let wanted = match role {
                    // Once approved there is nothing left to do, so it is not "pending review".
                    Role::Reviewer => {
                        !raw.is_approved_by(user_id) && (use_q || raw.is_reviewer(user_id))
                    }
                    Role::Author => use_q || raw.is_author(user_id),
                };
                if wanted {
                    raws.push(raw);
                }
            }
            url = page.next;
        }

        Ok(stream::iter(raws)
            .map(|raw| async move {
                let (ci_status, checks) = if role == Role::Author {
                    tokio::join!(
                        self.ci_status(repo, raw.id),
                        self.merge_checks(repo, raw.id)
                    )
                } else {
                    (self.ci_status(repo, raw.id).await, None)
                };
                let merge =
                    (role == Role::Author).then(|| raw.merge_status(ci_status, checks.as_deref()));
                let mut pr = raw.into_pr(&self.account_id, repo_id);
                pr.ci_status = ci_status;
                pr.merge = merge;
                pr
            })
            .buffered(MAX_CONCURRENCY)
            .collect()
            .await)
    }
}

/// Which PRs of the user to look for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Reviewer,
    Author,
}

impl Role {
    fn q_field(self) -> &'static str {
        match self {
            Role::Reviewer => "reviewers.account_id",
            Role::Author => "author.account_id",
        }
    }
}

fn open_prs_url(workspace: &str, repo: &str) -> String {
    format!(
        "{API}/repositories/{workspace}/{repo}/pullrequests?state=OPEN&pagelen=50&fields={FIELDS}"
    )
}

#[async_trait]
impl ReviewProvider for Bitbucket {
    async fn validate(&self) -> AppResult<String> {
        self.user_id().await.map(str::to_string)
    }

    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        self.fetch_all(repos, Role::Reviewer).await
    }

    async fn fetch_my_prs(&self, repos: &[String]) -> AppResult<FetchReport> {
        self.fetch_all(repos, Role::Author).await
    }
}

impl Bitbucket {
    async fn fetch_all(&self, repos: &[String], role: Role) -> AppResult<FetchReport> {
        let user_id = self.user_id().await?;

        let results = stream::iter(repos.iter().cloned())
            .map(|repo| async move {
                let fetched = self.fetch_repo(&repo, user_id, role).await;
                (repo, fetched)
            })
            .buffered(MAX_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

        let mut prs = Vec::new();
        let mut failed_repos = Vec::new();
        for (repo, result) in results {
            match result {
                Ok(repo_prs) => prs.extend(repo_prs),
                Err(e) => {
                    let full_repo = format!("{}/{repo}", self.workspace);
                    log::warn!("Failed to fetch PRs for repo {full_repo}: {}", e.detail());
                    failed_repos.push((full_repo, e));
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
    account_id: Option<String>,
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
    /// `approved`, `changes_requested` or absent.
    state: Option<String>,
    user: ParticipantUser,
}

#[derive(Debug, Deserialize)]
struct RawStatus {
    state: String,
}

impl RawStatus {
    fn ci_status(&self) -> CiStatus {
        match self.state.as_str() {
            "SUCCESSFUL" => CiStatus::Success,
            "INPROGRESS" => CiStatus::Pending,
            // FAILED, STOPPED and anything unknown need a human look.
            _ => CiStatus::Failed,
        }
    }
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
    /// Live comments, including inline code-review ones; deleted comments are excluded.
    comment_count: Option<u32>,
    links: PrLinks,
    participants: Option<Vec<Participant>>,
    #[serde(default)]
    draft: bool,
    task_count: Option<u32>,
}

/// One entry of `pullrequests/{id}/mergeability/checks`.
#[derive(Debug, Deserialize)]
struct MergeCheck {
    status: String,
    #[serde(default)]
    blocking: bool,
    #[serde(rename = "type")]
    kind: String,
    reason: Option<String>,
    check: Option<MergeCheckDefinition>,
    requirement: Option<serde_json::Value>,
    observed: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct MergeCheckDefinition {
    kind: String,
}

impl MergeCheck {
    fn failed(&self) -> bool {
        self.blocking && self.status != "PASSED"
    }

    fn number(value: &Option<serde_json::Value>, key: &str) -> u64 {
        value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|n| n.as_u64())
            .unwrap_or(0)
    }

    /// State and message for a failed check.
    fn blocker(&self) -> (MergeState, String) {
        let kind = self
            .check
            .as_ref()
            .map_or(self.kind.as_str(), |c| c.kind.as_str());
        let progress = |required: &str, observed: &str| {
            format!(
                "{}/{}",
                Self::number(&self.observed, observed),
                Self::number(&self.requirement, required)
            )
        };
        match kind {
            "git_mergeability_check" if self.reason.as_deref() == Some("conflicts") => {
                (MergeState::Conflicts, "Merge conflicts".to_string())
            }
            "minimum_approvals" => (
                MergeState::Waiting,
                format!(
                    "Approvals {}",
                    progress("minimum_approvals", "approval_count")
                ),
            ),
            "minimum_successful_builds" => (
                MergeState::Waiting,
                format!(
                    "Successful builds {}",
                    progress("minimum_successful_builds", "successful_build_count")
                ),
            ),
            "in_progress_builds" => (MergeState::Waiting, "Pipeline running".to_string()),
            "failed_builds" => (MergeState::Blocked, "Pipeline failed".to_string()),
            other => (MergeState::Blocked, capitalized(other)),
        }
    }
}

/// `minimum_approvals` → `Minimum approvals`, for check kinds we do not know by name.
fn capitalized(kind: &str) -> String {
    let spaced = kind.replace('_', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => spaced,
    }
}

/// Readiness from Bitbucket's own merge checks. Changes requested and open tasks are kept
/// from the PR payload because they only show up as checks when the repo enforces them.
fn assess_checks(
    draft: bool,
    changes_requested: bool,
    open_tasks: u32,
    checks: &[MergeCheck],
) -> MergeStatus {
    if draft {
        return MergeStatus::new(MergeState::Draft, vec![]);
    }
    let mut blockers = Vec::new();
    let mut push = |state: MergeState, message: String, worst: &mut MergeState| {
        if !blockers.contains(&message) {
            blockers.push(message);
        }
        let rank = |s: MergeState| match s {
            MergeState::Blocked => 3,
            MergeState::Conflicts => 2,
            MergeState::Waiting => 1,
            _ => 0,
        };
        if rank(state) > rank(*worst) {
            *worst = state;
        }
    };
    let mut worst = MergeState::Ready;
    if changes_requested {
        push(
            MergeState::Blocked,
            "Changes requested".to_string(),
            &mut worst,
        );
    }
    if open_tasks > 0 {
        let noun = if open_tasks == 1 { "task" } else { "tasks" };
        push(
            MergeState::Blocked,
            format!("{open_tasks} open {noun}"),
            &mut worst,
        );
    }
    for check in checks.iter().filter(|c| c.failed()) {
        let (state, message) = check.blocker();
        push(state, message, &mut worst);
    }
    MergeStatus::new(worst, blockers)
}

/// Bitbucket exposes no "mergeable" verdict and the merge rules need admin access to read,
/// so readiness is inferred from what any user can see: build statuses, approvals, requested
/// changes and open tasks. Merge conflicts are not detectable this way.
fn assess(
    draft: bool,
    approvals: u32,
    changes_requested: bool,
    open_tasks: u32,
    ci: Option<CiStatus>,
) -> MergeStatus {
    if draft {
        return MergeStatus::new(MergeState::Draft, vec![]);
    }
    let mut blockers = Vec::new();
    if changes_requested {
        blockers.push("Changes requested".to_string());
    }
    if ci == Some(CiStatus::Failed) {
        blockers.push("Pipeline failed".to_string());
    }
    if open_tasks > 0 {
        blockers.push(format!(
            "{open_tasks} open {}",
            if open_tasks == 1 { "task" } else { "tasks" }
        ));
    }
    if !blockers.is_empty() {
        return MergeStatus::new(MergeState::Blocked, blockers);
    }
    let mut waiting = Vec::new();
    if ci == Some(CiStatus::Pending) {
        waiting.push("Pipeline running".to_string());
    }
    if approvals == 0 {
        waiting.push("Waiting for approval".to_string());
    }
    if !waiting.is_empty() {
        return MergeStatus::new(MergeState::Waiting, waiting);
    }
    MergeStatus::new(MergeState::Ready, vec![])
}

impl RawPullRequest {
    fn is_author(&self, user_id: &str) -> bool {
        self.author.account_id.as_deref() == Some(user_id)
    }

    fn merge_status(&self, ci: Option<CiStatus>, checks: Option<&[MergeCheck]>) -> MergeStatus {
        let changes_requested = self
            .participants
            .iter()
            .flatten()
            .any(|p| p.state.as_deref() == Some("changes_requested"));
        let open_tasks = self.task_count.unwrap_or(0);
        match checks {
            Some(checks) => assess_checks(self.draft, changes_requested, open_tasks, checks),
            None => assess(
                self.draft,
                self.approvals(),
                changes_requested,
                open_tasks,
                ci,
            ),
        }
    }

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

    fn approvals(&self) -> u32 {
        self.participants
            .iter()
            .flatten()
            .filter(|p| p.approved)
            .count() as u32
    }

    fn into_pr(self, account_id: &str, repo: &str) -> PullRequest {
        let approvals = self.approvals();
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
            approvals,
            ci_status: None,
            merge: None,
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
        assert_eq!(pr.approvals, 1);
    }

    #[test]
    fn unknown_check_kinds_are_readable() {
        assert_eq!(capitalized("minimum_approvals"), "Minimum approvals");
        assert_eq!(capitalized(""), "");
    }

    #[test]
    fn maps_status_states() {
        let state = |s: &str| RawStatus { state: s.into() }.ci_status();
        assert_eq!(state("SUCCESSFUL"), CiStatus::Success);
        assert_eq!(state("INPROGRESS"), CiStatus::Pending);
        assert_eq!(state("FAILED"), CiStatus::Failed);
        assert_eq!(state("STOPPED"), CiStatus::Failed);
    }

    #[test]
    fn assesses_merge_readiness() {
        use CiStatus::*;
        use MergeState::*;
        let state = |draft, approvals, changes, tasks, ci| {
            assess(draft, approvals, changes, tasks, ci).state
        };
        assert_eq!(state(true, 2, false, 0, Some(Success)), Draft);
        assert_eq!(state(false, 1, false, 0, Some(Success)), Ready);
        assert_eq!(state(false, 1, false, 0, None), Ready);
        assert_eq!(
            state(false, 0, false, 0, Some(Success)),
            MergeState::Waiting
        );
        assert_eq!(
            state(false, 1, false, 0, Some(Pending)),
            MergeState::Waiting
        );
        assert_eq!(state(false, 1, false, 0, Some(Failed)), Blocked);
        assert_eq!(state(false, 1, true, 0, Some(Success)), Blocked);
        assert_eq!(state(false, 1, false, 2, Some(Success)), Blocked);
        // A blocker wins over a missing approval.
        assert_eq!(
            assess(false, 0, false, 1, None).blockers,
            vec!["1 open task"]
        );
    }

    #[test]
    fn reads_author_and_changes_requested() {
        let pr = parse(
            r#"{"id":1,"title":"t","created_on":"a","updated_on":"b","task_count":2,
                "author":{"account_id":"me"},
                "source":{"branch":{"name":"s"}},"destination":{"branch":{"name":"d"}},
                "links":{"html":{"href":"h"}},
                "participants":[{"role":"REVIEWER","approved":false,"state":"changes_requested","user":{"account_id":"x"}}]}"#,
        );
        assert!(pr.is_author("me"));
        assert!(!pr.is_author("x"));
        let status = pr.merge_status(Some(CiStatus::Success), None);
        assert_eq!(status.state, MergeState::Blocked);
        assert_eq!(status.blockers, vec!["Changes requested", "2 open tasks"]);
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

    #[test]
    fn assesses_from_merge_checks() {
        let checks: PagedResponse<MergeCheck> = serde_json::from_str(
            r#"{"values":[
                {"type":"git_mergeability_check","status":"PASSED","blocking":false,"reason":"clean"},
                {"type":"standard_merge_check","status":"FAILED","blocking":true,
                 "check":{"kind":"minimum_approvals"},
                 "requirement":{"minimum_approvals":2},"observed":{"approval_count":1}},
                {"type":"standard_merge_check","status":"PASSED","blocking":false,
                 "check":{"kind":"failed_builds"},"observed":{"failed_build_count":0}}
            ]}"#,
        )
        .unwrap();
        let status = assess_checks(false, false, 0, &checks.values);
        assert_eq!(status.state, MergeState::Waiting);
        assert_eq!(status.blockers, vec!["Approvals 1/2"]);

        let conflicts: PagedResponse<MergeCheck> = serde_json::from_str(
            r#"{"values":[{"type":"git_mergeability_check","status":"FAILED","blocking":true,"reason":"conflicts"}]}"#,
        )
        .unwrap();
        assert_eq!(
            assess_checks(false, false, 0, &conflicts.values).state,
            MergeState::Conflicts
        );
        assert_eq!(
            assess_checks(false, true, 0, &[]).state,
            MergeState::Blocked
        );
        assert!(assess_checks(false, false, 0, &[]).is_ready());
        assert_eq!(
            assess_checks(true, false, 0, &checks.values).state,
            MergeState::Draft
        );
    }
}
