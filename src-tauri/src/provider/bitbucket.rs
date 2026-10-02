use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;
use tokio::sync::OnceCell;

use super::{FetchReport, ReviewProvider};
use crate::{
    domain::{CiStatus, MergeState, MergeStatus, ProviderKind, PullRequest},
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

    /// Pipeline/build status of the PR's head commit. Best effort: a token without the
    /// pipeline/status scope, or any API error, just means "no status".
    async fn ci_status(&self, repo: &str, pr_id: u64) -> Option<CiStatus> {
        let url = format!(
            "{API}/repositories/{}/{repo}/pullrequests/{pr_id}/statuses?pagelen=100&fields=values.state",
            self.workspace
        );
        let result: Result<PagedResponse<RawStatus>, reqwest::Error> = async {
            self.client
                .get(&url)
                .bearer_auth(&self.token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }
        .await;
        match result {
            Ok(page) => CiStatus::combine(page.values.iter().map(RawStatus::ci_status)),
            Err(e) => {
                log::debug!("No CI status for {repo}#{pr_id}: {e}");
                None
            }
        }
    }

    async fn fetch_repo(&self, repo: &str, user_id: &str, role: Role) -> AppResult<Vec<PullRequest>> {
        let mut raws = Vec::new();
        let mut use_q = self.use_q_filter.load(Ordering::Relaxed);
        let workspace = &self.workspace;

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
                let wanted = match role {
                    // Once approved there is nothing left to do, so it is not "pending review".
                    Role::Reviewer => !raw.is_approved_by(user_id) && (use_q || raw.is_reviewer(user_id)),
                    Role::Author => use_q || raw.is_author(user_id),
                };
                if wanted {
                    raws.push(raw);
                }
            }
            url = page.next;
        }

        Ok(join_all(raws.into_iter().map(|raw| async move {
            let ci_status = self.ci_status(repo, raw.id).await;
            let merge = (role == Role::Author).then(|| raw.merge_status(ci_status));
            let mut pr = raw.into_pr(&self.account_id, repo);
            pr.ci_status = ci_status;
            pr.merge = merge;
            pr
        }))
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
    format!("{API}/repositories/{workspace}/{repo}/pullrequests?state=OPEN&pagelen=50&fields={FIELDS}")
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

        let results = join_all(repos.iter().map(|repo| async move {
            (repo.clone(), self.fetch_repo(repo, user_id, role).await)
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
    comment_count: Option<u32>,
    links: PrLinks,
    participants: Option<Vec<Participant>>,
    #[serde(default)]
    draft: bool,
    task_count: Option<u32>,
}

/// Bitbucket exposes no "mergeable" verdict and the merge rules need admin access to read,
/// so readiness is inferred from what any user can see: build statuses, approvals, requested
/// changes and open tasks. Merge conflicts are not detectable this way.
fn assess(draft: bool, approvals: u32, changes_requested: bool, open_tasks: u32, ci: Option<CiStatus>) -> MergeStatus {
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
        blockers.push(format!("{open_tasks} open {}", if open_tasks == 1 { "task" } else { "tasks" }));
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

    fn merge_status(&self, ci: Option<CiStatus>) -> MergeStatus {
        let changes_requested = self
            .participants
            .iter()
            .flatten()
            .any(|p| p.state.as_deref() == Some("changes_requested"));
        assess(self.draft, self.approvals(), changes_requested, self.task_count.unwrap_or(0), ci)
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
        self.participants.iter().flatten().filter(|p| p.approved).count() as u32
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
        let state = |draft, approvals, changes, tasks, ci| assess(draft, approvals, changes, tasks, ci).state;
        assert_eq!(state(true, 2, false, 0, Some(Success)), Draft);
        assert_eq!(state(false, 1, false, 0, Some(Success)), Ready);
        assert_eq!(state(false, 1, false, 0, None), Ready);
        assert_eq!(state(false, 0, false, 0, Some(Success)), MergeState::Waiting);
        assert_eq!(state(false, 1, false, 0, Some(Pending)), MergeState::Waiting);
        assert_eq!(state(false, 1, false, 0, Some(Failed)), Blocked);
        assert_eq!(state(false, 1, true, 0, Some(Success)), Blocked);
        assert_eq!(state(false, 1, false, 2, Some(Success)), Blocked);
        // A blocker wins over a missing approval.
        assert_eq!(assess(false, 0, false, 1, None).blockers, vec!["1 open task"]);
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
        let status = pr.merge_status(Some(CiStatus::Success));
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
}
