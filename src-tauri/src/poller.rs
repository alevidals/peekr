//! Background loop that polls every configured account and publishes the result
//! (state, `prs-updated` event, tray badge, notifications).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::{
    config::Account,
    domain::PullRequest,
    error::AppResult,
    notifier,
    provider::{self, FetchReport, ReviewProvider},
    state::{ConfigStore, PrState, Snapshot},
    tray,
};

/// How long to wait when there is nothing to poll yet.
const IDLE_WAIT_SECS: u64 = 30;
/// A run of failed polls backs off up to this many times the configured interval.
const MAX_BACKOFF: u32 = 3;

struct ActiveAccount {
    account: Account,
    provider: Arc<dyn ReviewProvider>,
}

impl ActiveAccount {
    fn label(&self) -> String {
        account_label(&self.account)
    }
}

fn account_label(account: &Account) -> String {
    format!("{} ({})", account.workspace, account.kind.as_str())
}

/// Accounts the poller can use, plus why the others cannot be polled.
#[derive(Default)]
struct ActiveSet {
    accounts: Vec<ActiveAccount>,
    problems: Vec<String>,
}

pub async fn run(app: AppHandle, mut rx: watch::Receiver<()>) {
    let client = app.state::<reqwest::Client>().inner().clone();
    let mut built_revision = None;
    let mut active = ActiveSet::default();
    // Accounts that completed a poll before; only their new PRs trigger notifications, so
    // that the first successful poll of an account never floods the user.
    let mut seeded: HashSet<String> = HashSet::new();
    let mut seeded_mine: HashSet<String> = HashSet::new();
    // PRs already announced. A search is eventually consistent: a known PR can vanish for a
    // poll and come back, and that must not notify twice.
    let mut notified: HashSet<String> = HashSet::new();
    let mut failures = 0;

    loop {
        let snapshot = app.state::<ConfigStore>().snapshot();
        if built_revision != Some(snapshot.revision) {
            active = build_active(&client, &snapshot);
            built_revision = Some(snapshot.revision);
            log::info!("[peekr] {} account(s) ready to poll", active.accounts.len());
        }

        if active.accounts.is_empty() {
            clear_prs(&app).await;
            clear_mine(&app).await;
            if !active.problems.is_empty() {
                let _ = app.emit("poll-error", active.problems.join("\n"));
            }
            wait_for_next_poll(IDLE_WAIT_SECS, &mut rx).await;
            continue;
        }

        // Diagnostic detail: the trigger line above says whether the timer or a manual
        // refresh started this round.
        log::debug!("[poll] start: accounts={}", active.accounts.len());

        let (results, mine_results) =
            tokio::join!(
                join_all(active.accounts.iter().map(|a| async move {
                    (a, a.provider.fetch_review_prs(&a.account.repos).await)
                })),
                join_all(
                    active.accounts.iter().map(|a| async move {
                        (a, a.provider.fetch_my_prs(&a.account.repos).await)
                    })
                ),
            );
        // Only a whole account failing means the service (or the network) is a problem worth
        // slowing down for; one broken repository must not stretch the interval.
        let failed_round =
            results.iter().any(|(_, r)| r.is_err()) || mine_results.iter().any(|(_, r)| r.is_err());

        poll_once(&app, results, &active.problems, &mut seeded, &mut notified).await;
        poll_mine(&app, mine_results, &mut seeded_mine).await;

        failures = if failed_round {
            (failures + 1).min(MAX_BACKOFF)
        } else {
            0
        };
        let interval = backoff_interval(snapshot.config.polling_interval_secs, failures);
        wait_for_next_poll(interval, &mut rx).await;
    }
}

fn build_active(client: &reqwest::Client, snapshot: &Snapshot) -> ActiveSet {
    let mut set = ActiveSet::default();
    for account in &snapshot.config.accounts {
        let label = account_label(account);
        if !account.is_complete() {
            // The settings form prevents this; a hand-edited file can still get here.
            set.problems.push(format!("{label}: account is incomplete"));
            continue;
        }
        let Some(token) = snapshot.tokens.get(&account.id) else {
            set.problems.push(format!("{label}: no token stored"));
            continue;
        };
        set.accounts.push(ActiveAccount {
            account: account.clone(),
            provider: provider::build(client.clone(), account, token),
        });
    }
    set
}

/// Which side of the poll is running; the two differ in wording and in what is kept.
#[derive(Clone, Copy)]
enum Scope {
    Reviews,
    Mine,
}

impl Scope {
    fn error_prefix(self) -> &'static str {
        match self {
            Scope::Reviews => "",
            Scope::Mine => "My PRs, ",
        }
    }

    fn log_label(self) -> &'static str {
        match self {
            Scope::Reviews => "review requests",
            Scope::Mine => "own PRs",
        }
    }

    /// A failed repo keeps its PRs in the review list. The own list only recovers whole
    /// accounts, because a partial list there is only ever missing PRs.
    fn keeps_repo_failures(self) -> bool {
        matches!(self, Scope::Reviews)
    }
}

/// Sources that failed this round; their previously known PRs are kept.
#[derive(Default)]
struct Failures {
    accounts: HashSet<String>,
    repos: HashSet<(String, String)>,
}

impl Failures {
    fn add_account(&mut self, account_id: &str) {
        self.accounts.insert(account_id.to_string());
    }

    fn add_repo(&mut self, account_id: &str, repo: &str) {
        self.repos
            .insert((account_id.to_string(), repo.to_string()));
    }

    fn covers(&self, pr: &PullRequest) -> bool {
        self.accounts.contains(&pr.account_id)
            || self
                .repos
                .contains(&(pr.account_id.clone(), pr.repo.clone()))
    }

    fn covers_account(&self, pr: &PullRequest) -> bool {
        self.accounts.contains(&pr.account_id)
    }
}

/// Everything one side of a poll produced.
#[derive(Default)]
struct PollBatch {
    prs: Vec<PullRequest>,
    errors: Vec<String>,
    failures: Failures,
    succeeded: Vec<String>,
}

fn collect_batch(
    results: Vec<(&ActiveAccount, AppResult<FetchReport>)>,
    scope: Scope,
) -> PollBatch {
    let mut batch = PollBatch::default();
    for (account, result) in results {
        let id = &account.account.id;
        match result {
            Ok(report) => {
                batch.prs.extend(report.prs);
                batch.succeeded.push(id.clone());
                for (repo, e) in report.failed_repos {
                    batch.errors.push(format!(
                        "{}{}/{repo}: {e}",
                        scope.error_prefix(),
                        account.account.workspace
                    ));
                    if scope.keeps_repo_failures() {
                        batch.failures.add_repo(id, &repo);
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "Polling {} for {} failed: {e}",
                    scope.log_label(),
                    account.label()
                );
                batch
                    .errors
                    .push(format!("{}{}: {e}", scope.error_prefix(), account.label()));
                batch.failures.add_account(id);
            }
        }
    }
    batch
}

/// Keys that entered, left or changed between two consecutive polls.
#[derive(Debug, Default)]
struct ListDiff {
    new: Vec<String>,
    gone: Vec<String>,
    updated: Vec<String>,
}

fn list_diff(previous: &[PullRequest], current: &[PullRequest]) -> ListDiff {
    let before: HashMap<String, &PullRequest> = previous.iter().map(|pr| (pr.key(), pr)).collect();
    let after: HashSet<String> = current.iter().map(PullRequest::key).collect();

    ListDiff {
        new: current
            .iter()
            .filter(|pr| !before.contains_key(&pr.key()))
            .map(PullRequest::key)
            .collect(),
        gone: previous
            .iter()
            .filter(|pr| !after.contains(&pr.key()))
            .map(PullRequest::key)
            .collect(),
        updated: current
            .iter()
            .filter(|pr| {
                before
                    .get(&pr.key())
                    .is_some_and(|old| old.updated_on != pr.updated_on)
            })
            .map(PullRequest::key)
            .collect(),
    }
}

/// Fresh PRs plus the ones we already knew from sources we could not reach this round.
fn merge_keeping_stale(
    known: &[PullRequest],
    fresh: Vec<PullRequest>,
    keep: impl Fn(&PullRequest) -> bool,
) -> Vec<PullRequest> {
    let mut merged = fresh;
    merged.extend(known.iter().filter(|pr| keep(pr)).cloned());
    merged
}

async fn poll_once(
    app: &AppHandle,
    results: Vec<(&ActiveAccount, AppResult<FetchReport>)>,
    problems: &[String],
    seeded: &mut HashSet<String>,
    notified: &mut HashSet<String>,
) {
    let batch = collect_batch(results, Scope::Reviews);

    let state = app.state::<PrState>();
    let mut current = state.prs.lock().await;

    let fresh = batch.prs.len();
    let mut prs = merge_keeping_stale(&current, batch.prs, |pr| batch.failures.covers(pr));
    prs.sort_by(|a, b| b.created_on.cmp(&a.created_on));

    let added = notifier::newly_added(&current, &prs);
    if log::log_enabled!(log::Level::Debug) {
        let diff = list_diff(&current, &prs);
        log::debug!(
            "[poll] review update: total={} fresh={fresh} new={:?} gone={:?} updated={:?}",
            prs.len(),
            diff.new,
            diff.gone,
            diff.updated
        );
    }
    let to_notify: Vec<&PullRequest> = added
        .iter()
        .copied()
        .filter(|pr| seeded.contains(&pr.account_id) && !notified.contains(&pr.key()))
        .collect();
    notified.extend(added.iter().map(|pr| pr.key()));
    seeded.extend(batch.succeeded.iter().cloned());

    *current = prs.clone();
    drop(current);

    if !to_notify.is_empty() {
        notifier::notify_new_prs(app, &to_notify);
    }
    log::info!("[peekr] poll complete: {} PRs pending review", prs.len());
    emit_and_badge(app, &prs);

    let mut errors = batch.errors;
    errors.extend_from_slice(problems);
    if !errors.is_empty() {
        let _ = app.emit("poll-error", errors.join("\n"));
    }
}

/// Same as [`poll_once`] for the user's own PRs: no badge, and notifications only for
/// readiness/failure transitions.
async fn poll_mine(
    app: &AppHandle,
    results: Vec<(&ActiveAccount, AppResult<FetchReport>)>,
    seeded: &mut HashSet<String>,
) {
    let batch = collect_batch(results, Scope::Mine);

    let state = app.state::<PrState>();
    let mut current = state.mine.lock().await;

    let fresh = batch.prs.len();
    let mut prs = merge_keeping_stale(&current, batch.prs, |pr| batch.failures.covers_account(pr));
    prs.sort_by(|a, b| b.updated_on.cmp(&a.updated_on));

    let events: Vec<_> = notifier::my_pr_events(&current, &prs)
        .into_iter()
        .filter(|(pr, _)| seeded.contains(&pr.account_id))
        .collect();
    if log::log_enabled!(log::Level::Debug) {
        let diff = list_diff(&current, &prs);
        log::debug!(
            "[poll] mine update: total={} fresh={fresh} new={:?} gone={:?} updated={:?} events={:?}",
            prs.len(),
            diff.new,
            diff.gone,
            diff.updated,
            events
                .iter()
                .map(|(pr, event)| (pr.key(), format!("{event:?}")))
                .collect::<Vec<_>>()
        );
    }
    seeded.extend(batch.succeeded.iter().cloned());

    *current = prs.clone();
    drop(current);

    notifier::notify_my_pr_events(app, &events);
    let _ = app.emit("my-prs-updated", &prs);
    if !batch.errors.is_empty() {
        let _ = app.emit("poll-error", batch.errors.join("\n"));
    }
}

async fn clear_mine(app: &AppHandle) {
    let state = app.state::<PrState>();
    let mut current = state.mine.lock().await;
    if current.is_empty() {
        return;
    }
    current.clear();
    drop(current);
    let _ = app.emit("my-prs-updated", &[] as &[PullRequest]);
}

/// Clears the shared PR list without notifying (used when there is nothing to poll).
async fn clear_prs(app: &AppHandle) {
    let state = app.state::<PrState>();
    let mut current = state.prs.lock().await;
    if current.is_empty() {
        return;
    }
    current.clear();
    drop(current);
    emit_and_badge(app, &[]);
}

fn emit_and_badge(app: &AppHandle, prs: &[PullRequest]) {
    let _ = app.emit("prs-updated", prs);
    tray::update_badge(app, prs.len() as u32);
}

/// Multiplies the configured interval after consecutive failed polls, so a service that is
/// down or throttling us is not hammered at the normal rate.
fn backoff_interval(base_secs: u64, failures: u32) -> u64 {
    base_secs.saturating_mul(1u64 << failures.min(MAX_BACKOFF))
}

async fn wait_for_next_poll(secs: u64, rx: &mut watch::Receiver<()>) {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(secs)) => {
            log::info!("[peekr] polling triggered by timer (interval={secs}s)");
        }
        changed = rx.changed() => {
            if changed.is_err() {
                // No sender left: keep the timer instead of spinning.
                tokio::time::sleep(Duration::from_secs(secs)).await;
            } else {
                log::info!("[peekr] polling triggered early (manual refresh or settings saved)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProviderKind;

    fn pr(account: &str, repo: &str, number: u64) -> PullRequest {
        PullRequest {
            provider: ProviderKind::Bitbucket,
            account_id: account.into(),
            repo: repo.into(),
            number,
            title: String::new(),
            author_display_name: String::new(),
            author_avatar_url: String::new(),
            source_branch: String::new(),
            destination_branch: String::new(),
            created_on: String::new(),
            updated_on: String::new(),
            comment_count: 0,
            html_url: String::new(),
            draft: false,
            approvals: 0,
            ci_status: None,
            merge: None,
        }
    }

    #[test]
    fn backoff_grows_after_failures_and_caps() {
        assert_eq!(backoff_interval(120, 0), 120);
        assert_eq!(backoff_interval(120, 1), 240);
        assert_eq!(backoff_interval(120, 3), 960);
        assert_eq!(backoff_interval(120, 9), 960);
    }

    #[test]
    fn failures_cover_whole_accounts_and_single_repos() {
        let mut failures = Failures::default();
        failures.add_account("a");
        failures.add_repo("b", "api");

        assert!(failures.covers(&pr("a", "web", 1)));
        assert!(failures.covers(&pr("b", "api", 1)));
        assert!(!failures.covers(&pr("b", "web", 1)));
        assert!(!failures.covers(&pr("c", "api", 1)));

        assert!(failures.covers_account(&pr("a", "web", 1)));
        assert!(!failures.covers_account(&pr("b", "api", 1)));
    }

    fn pr_updated(account: &str, repo: &str, number: u64, updated_on: &str) -> PullRequest {
        PullRequest {
            updated_on: updated_on.into(),
            ..pr(account, repo, number)
        }
    }

    #[test]
    fn diff_reports_new_gone_and_updated_prs() {
        let previous = vec![
            pr_updated("a", "web", 1, "t1"),
            pr_updated("a", "web", 2, "t1"),
            pr_updated("a", "web", 3, "t1"),
        ];
        let current = vec![
            pr_updated("a", "web", 1, "t1"),
            pr_updated("a", "web", 2, "t2"),
            pr_updated("a", "web", 4, "t1"),
        ];

        let diff = list_diff(&previous, &current);
        assert_eq!(diff.new, vec!["a/web#4"]);
        assert_eq!(diff.gone, vec!["a/web#3"]);
        assert_eq!(diff.updated, vec!["a/web#2"]);
    }

    #[test]
    fn merge_keeps_stale_prs_without_duplicating_fresh_ones() {
        let known = vec![pr("a", "web", 1), pr("a", "api", 2)];
        let fresh = vec![pr("a", "web", 1)];

        let merged = merge_keeping_stale(&known, fresh, |pr| pr.repo == "api");
        let keys: Vec<_> = merged.iter().map(PullRequest::key).collect();
        assert_eq!(keys, vec!["a/web#1", "a/api#2"]);
    }
}
