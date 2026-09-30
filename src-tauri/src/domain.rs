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
    /// Number of approvals; 0 when unknown.
    #[serde(default)]
    pub approvals: u32,
    /// `None` when the PR has no checks or they could not be read.
    #[serde(default)]
    pub ci_status: Option<CiStatus>,
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
