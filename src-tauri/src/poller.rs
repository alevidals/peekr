//! Background loop that polls every configured account and publishes the result
//! (state, `prs-updated` event, tray badge, notifications).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::{
    config::Account,
    domain::PullRequest,
    notifier,
    provider::{self, FetchReport, ReviewProvider},
    state::{ConfigStore, PrState, Snapshot},
    tray,
};

/// How long to wait when there is nothing to poll yet.
const IDLE_WAIT_SECS: u64 = 30;

struct ActiveAccount {
    account: Account,
    provider: Arc<dyn ReviewProvider>,
}

impl ActiveAccount {
    fn label(&self) -> String {
        format!(
            "{} ({})",
            self.account.workspace,
            self.account.kind.as_str()
        )
    }
}

pub async fn run(app: AppHandle, mut rx: watch::Receiver<()>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let mut built_revision = None;
    let mut active: Vec<ActiveAccount> = Vec::new();
    // Accounts that completed a poll before; only their new PRs trigger notifications, so
    // that the first successful poll of an account never floods the user.
    let mut seeded: HashSet<String> = HashSet::new();
    let mut seeded_mine: HashSet<String> = HashSet::new();

    loop {
        let snapshot = app.state::<ConfigStore>().snapshot();
        if built_revision != Some(snapshot.revision) {
            active = build_active(&client, &snapshot);
            built_revision = Some(snapshot.revision);
            log::info!("[peekr] {} account(s) ready to poll", active.len());
        }

        if active.is_empty() {
            publish(&app, Vec::new()).await;
            publish_mine(&app, Vec::new()).await;
            wait_for_next_poll(IDLE_WAIT_SECS, &mut rx).await;
            continue;
        }

        let (results, mine_results) =
            tokio::join!(
                join_all(active.iter().map(|a| async move {
                    (a, a.provider.fetch_review_prs(&a.account.repos).await)
                })),
                join_all(
                    active.iter().map(|a| async move {
                        (a, a.provider.fetch_my_prs(&a.account.repos).await)
                    })
                ),
            );

        poll_once(&app, results, &mut seeded).await;
        poll_mine(&app, mine_results, &mut seeded_mine).await;
        wait_for_next_poll(snapshot.config.polling_interval_secs, &mut rx).await;
    }
}

fn build_active(client: &reqwest::Client, snapshot: &Snapshot) -> Vec<ActiveAccount> {
    snapshot
        .config
        .accounts
        .iter()
        .filter(|a| a.is_complete())
        .filter_map(|account| {
            let Some(token) = snapshot.tokens.get(&account.id) else {
                log::warn!("[peekr] account {} has no token, skipping", account.id);
                return None;
            };
            Some(ActiveAccount {
                account: account.clone(),
                provider: provider::build(client.clone(), account, token),
            })
        })
        .collect()
}

async fn poll_once(
    app: &AppHandle,
    results: Vec<(&ActiveAccount, crate::error::AppResult<FetchReport>)>,
    seeded: &mut HashSet<String>,
) {
    let mut new_prs: Vec<PullRequest> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    // (account id, repo) whose fetch failed; `None` = the whole account failed.
    let mut failed: Vec<(&str, Option<&str>)> = Vec::new();
    let mut succeeded: Vec<&str> = Vec::new();

    for (account, result) in &results {
        match result {
            Ok(report) => {
                new_prs.extend(report.prs.iter().cloned());
                succeeded.push(&account.account.id);
                for (repo, e) in &report.failed_repos {
                    errors.push(format!("{}/{repo}: {e}", account.account.workspace));
                    failed.push((&account.account.id, Some(repo)));
                }
            }
            Err(e) => {
                log::error!("Polling error for {}: {e}", account.label());
                errors.push(format!("{}: {e}", account.label()));
                failed.push((&account.account.id, None));
            }
        }
    }

    let state = app.state::<PrState>();
    let mut current = state.prs.lock().await;

    // Keep what we already knew about repos we couldn't reach, so a transient failure
    // neither drops their PRs nor makes them look new on the next successful poll.
    new_prs.extend(
        current
            .iter()
            .filter(|pr| {
                failed.iter().any(|(account_id, repo)| {
                    *account_id == pr.account_id && repo.is_none_or(|r| r == pr.repo)
                })
            })
            .cloned(),
    );
    new_prs.sort_by(|a, b| b.created_on.cmp(&a.created_on));

    let added: Vec<&PullRequest> = notifier::newly_added(&current, &new_prs)
        .into_iter()
        .filter(|pr| seeded.contains(&pr.account_id))
        .collect();
    if !added.is_empty() {
        notifier::notify_new_prs(app, &added);
    }
    seeded.extend(succeeded.into_iter().map(str::to_string));

    log::info!(
        "[peekr] poll complete: {} PRs pending review",
        new_prs.len()
    );
    *current = new_prs.clone();
    drop(current);

    emit_and_badge(app, &new_prs);
    if !errors.is_empty() {
        let _ = app.emit("poll-error", errors.join("\n"));
    }
}

/// Same as [`poll_once`] for the user's own PRs: no badge, and notifications only for
/// readiness/failure transitions.
async fn poll_mine(
    app: &AppHandle,
    results: Vec<(&ActiveAccount, crate::error::AppResult<FetchReport>)>,
    seeded: &mut HashSet<String>,
) {
    let mut fresh: Vec<PullRequest> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut failed: Vec<&str> = Vec::new();
    let mut succeeded: Vec<&str> = Vec::new();

    for (account, result) in &results {
        match result {
            Ok(report) => {
                // A repo that failed is not retained: the whole account failing is what we
                // can recover from, and a partial list here is only ever missing PRs.
                fresh.extend(report.prs.iter().cloned());
                succeeded.push(&account.account.id);
                for (repo, e) in &report.failed_repos {
                    errors.push(format!("My PRs, {}/{repo}: {e}", account.account.workspace));
                }
            }
            Err(e) => {
                log::error!("Polling own PRs failed for {}: {e}", account.label());
                errors.push(format!("My PRs, {}: {e}", account.label()));
                failed.push(&account.account.id);
            }
        }
    }

    let state = app.state::<PrState>();
    let mut current = state.mine.lock().await;
    fresh.extend(
        current
            .iter()
            .filter(|pr| failed.contains(&pr.account_id.as_str()))
            .cloned(),
    );
    fresh.sort_by(|a, b| b.updated_on.cmp(&a.updated_on));

    let events: Vec<_> = notifier::my_pr_events(&current, &fresh)
        .into_iter()
        .filter(|(pr, _)| seeded.contains(&pr.account_id))
        .collect();
    notifier::notify_my_pr_events(app, &events);
    seeded.extend(succeeded.into_iter().map(str::to_string));

    *current = fresh.clone();
    drop(current);
    let _ = app.emit("my-prs-updated", &fresh);
    if !errors.is_empty() {
        let _ = app.emit("poll-error", errors.join("\n"));
    }
}

async fn publish_mine(app: &AppHandle, prs: Vec<PullRequest>) {
    let state = app.state::<PrState>();
    let mut current = state.mine.lock().await;
    if current.len() == prs.len() {
        return;
    }
    *current = prs.clone();
    drop(current);
    let _ = app.emit("my-prs-updated", &prs);
}

/// Replaces the shared PR list without notifying (used when there is nothing to poll).
async fn publish(app: &AppHandle, prs: Vec<PullRequest>) {
    let state = app.state::<PrState>();
    let mut current = state.prs.lock().await;
    if current.len() == prs.len() {
        return;
    }
    *current = prs.clone();
    drop(current);
    emit_and_badge(app, &prs);
}

fn emit_and_badge(app: &AppHandle, prs: &[PullRequest]) {
    let _ = app.emit("prs-updated", prs);
    tray::update_badge(app, prs.len() as u32);
}

async fn wait_for_next_poll(secs: u64, rx: &mut watch::Receiver<()>) {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(secs)) => {
            log::info!("[peekr] polling triggered by timer (interval={secs}s)");
        }
        _ = rx.changed() => {
            log::info!("[peekr] polling triggered manually");
        }
    }
}
