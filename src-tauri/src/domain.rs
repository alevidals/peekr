use serde::{Deserialize, Serialize};

/// Code-hosting services peekr knows how to talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Bitbucket,
    Github,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Bitbucket => "bitbucket",
            ProviderKind::Github => "github",
        }
    }

    /// Whether the user must list the repos to watch (GitHub can search across all of them).
    pub fn requires_repos(&self) -> bool {
        match self {
            ProviderKind::Bitbucket => true,
            ProviderKind::Github => false,
        }
    }
}

/// Aggregated state of the checks (pipelines, Actions, commit statuses) on a PR's head commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
    Success,
    Failed,
    Pending,
}

impl CiStatus {
    /// Folds individual check results into one: any failure wins, then anything still
    /// running, and success only if every check passed. `None` when there are no checks.
    pub fn combine(statuses: impl IntoIterator<Item = CiStatus>) -> Option<CiStatus> {
        statuses.into_iter().reduce(|a, b| match (a, b) {
            (CiStatus::Failed, _) | (_, CiStatus::Failed) => CiStatus::Failed,
            (CiStatus::Pending, _) | (_, CiStatus::Pending) => CiStatus::Pending,
            _ => CiStatus::Success,
        })
    }
}

/// Whether one of the user's own PRs could be merged right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeState {
    /// Required checks and approvals are satisfied and nothing else blocks the merge.
    Ready,
    /// Nothing is wrong yet, but checks are running or reviews are still missing.
    Waiting,
    /// A failed check, requested changes, open tasks or another rule stops the merge.
    Blocked,
    Conflicts,
    /// The branch must be updated with its base before merging.
    Behind,
    Draft,
    /// The provider has not computed it yet.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeStatus {
    pub state: MergeState,
    /// Human-readable reasons the PR is not ready; empty when it is.
    pub blockers: Vec<String>,
}

impl MergeStatus {
    pub fn new(state: MergeState, blockers: Vec<String>) -> Self {
        Self { state, blockers }
    }

    pub fn is_ready(&self) -> bool {
        self.state == MergeState::Ready
    }
}

/// Provider-agnostic pull request. Every provider maps its API payload to this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub provider: ProviderKind,
    /// Id of the configured account this PR was fetched with.
    pub account_id: String,
    /// Repository name within the account (slug for Bitbucket, `owner/name` for GitHub).
    pub repo: String,
    /// PR number, unique only within `repo`.
    pub number: u64,
    pub title: String,
    pub author_display_name: String,
    pub author_avatar_url: String,
    pub source_branch: String,
    pub destination_branch: String,
    pub created_on: String,
    pub updated_on: String,
    pub comment_count: u32,
    pub html_url: String,
    pub draft: bool,
    /// The authenticated user approved this PR. Only review-queue PRs can be approved;
    /// own PRs carry it for completeness.
    #[serde(default)]
    pub approved_by_me: bool,
    /// Number of approvals; 0 when unknown.
    #[serde(default)]
    pub approvals: u32,
    /// `None` when the PR has no checks or they could not be read.
    #[serde(default)]
    pub ci_status: Option<CiStatus>,
    /// Only set for the user's own PRs.
    #[serde(default)]
    pub merge: Option<MergeStatus>,
}

impl PullRequest {
    /// Globally unique identity (numbers only repeat across repos/accounts).
    pub fn key(&self) -> String {
        format!("{}/{}#{}", self.account_id, self.repo, self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::CiStatus::*;
    use super::*;

    #[test]
    fn combine_prioritises_failure_then_pending() {
        assert_eq!(CiStatus::combine([]), None);
        assert_eq!(CiStatus::combine([Success, Success]), Some(Success));
        assert_eq!(CiStatus::combine([Success, Pending]), Some(Pending));
        assert_eq!(CiStatus::combine([Pending, Failed, Success]), Some(Failed));
    }
}
