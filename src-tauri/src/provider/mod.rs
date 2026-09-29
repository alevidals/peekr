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

/// Outcome of polling one account. Repos that failed are reported separately so that one bad
/// repo neither hides the others nor makes its previously known PRs look "closed".
pub struct FetchReport {
    pub prs: Vec<PullRequest>,
    pub failed_repos: Vec<(String, AppError)>,
}

#[async_trait]
pub trait ReviewProvider: Send + Sync {
    /// Checks the token and returns an identifier of the authenticated user.
    async fn validate(&self) -> AppResult<String>;

    /// Open PRs in `repos` where the authenticated user is a requested reviewer.
    async fn fetch_review_prs(&self, repos: &[String]) -> AppResult<FetchReport>;
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
