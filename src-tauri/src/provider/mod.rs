//! Code-hosting integrations.
//!
//! Every service (Bitbucket today, GitHub next) implements [`ReviewProvider`] and maps its
//! API payloads to the shared [`PullRequest`] domain type. The poller, notifier, tray and UI
//! only know about this trait, so adding a provider means adding a module here plus one arm
//! in [`build`].

pub mod bitbucket;
pub mod github;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    config::Account,
    domain::{ProviderKind, PullRequest},
    error::{AppError, AppResult},
};

/// Cap on simultaneous requests to one service. Providers fan out over repositories and PRs,
/// and an unbounded burst is the fastest way to be throttled.
pub(crate) const MAX_CONCURRENCY: usize = 8;

/// Outcome of polling one account. Repos that failed are reported separately so that one bad
/// repo neither hides the others nor makes its previously known PRs look "closed". The repo
/// identifier is the same one carried by [`PullRequest::repo`].
pub struct FetchReport {
    pub prs: Vec<PullRequest>,
    pub failed_repos: Vec<(String, AppError)>,
}

#[async_trait]
pub trait ReviewProvider: Send + Sync {
    /// Checks the token and returns an identifier of the authenticated user.
    async fn validate(&self) -> AppResult<String>;

    /// Open PRs in `repos` where the authenticated user is a requested reviewer. When
    /// `include_approved` is true, PRs the user already approved are kept too.
    async fn fetch_review_prs(
        &self,
        repos: &[String],
        include_approved: bool,
    ) -> AppResult<FetchReport>;

    /// Open PRs authored by the authenticated user, each with its [`MergeStatus`].
    async fn fetch_my_prs(&self, repos: &[String]) -> AppResult<FetchReport>;
}

pub fn build(client: reqwest::Client, account: &Account, token: &str) -> Arc<dyn ReviewProvider> {
    match account.kind {
        ProviderKind::Bitbucket => Arc::new(bitbucket::Bitbucket::new(
            client,
            account.id.clone(),
            account.workspace.clone(),
            token.to_string(),
        )),
        ProviderKind::Github => Arc::new(github::Github::new(
            client,
            account.id.clone(),
            account.workspace.clone(),
            token.to_string(),
        )),
    }
}
