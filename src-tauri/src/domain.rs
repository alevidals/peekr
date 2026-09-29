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
}

impl PullRequest {
    /// Globally unique identity (numbers only repeat across repos/accounts).
    pub fn key(&self) -> String {
        format!("{}/{}#{}", self.account_id, self.repo, self.number)
    }
}
