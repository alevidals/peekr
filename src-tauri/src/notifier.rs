use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::domain::{CiStatus, PullRequest};

/// PRs in `current` that were not in `previous`.
pub fn newly_added<'a>(previous: &[PullRequest], current: &'a [PullRequest]) -> Vec<&'a PullRequest> {
    let known: std::collections::HashSet<String> = previous.iter().map(PullRequest::key).collect();
    current.iter().filter(|pr| !known.contains(&pr.key())).collect()
}

/// Something worth telling the user about one of their own PRs.
#[derive(Debug, PartialEq, Eq)]
pub enum MyPrEvent {
    BecameReady,
    ChecksFailed,
}

/// Transitions between two polls of the user's PRs. PRs that are new in `current` raise
/// nothing: the first sighting of a PR is not a change.
pub fn my_pr_events<'a>(previous: &[PullRequest], current: &'a [PullRequest]) -> Vec<(&'a PullRequest, MyPrEvent)> {
    let before: std::collections::HashMap<String, &PullRequest> =
        previous.iter().map(|pr| (pr.key(), pr)).collect();
    let is_ready = |pr: &PullRequest| pr.merge.as_ref().is_some_and(|m| m.is_ready());

    current
        .iter()
        .filter_map(|pr| {
            let old = before.get(&pr.key())?;
            if is_ready(pr) && !is_ready(old) {
                Some((pr, MyPrEvent::BecameReady))
            } else if pr.ci_status == Some(CiStatus::Failed) && old.ci_status != Some(CiStatus::Failed) {
                Some((pr, MyPrEvent::ChecksFailed))
            } else {
                None
            }
        })
        .collect()
}

/// One notification per event, so each can be clicked through to its own PR.
pub fn notify_my_pr_events(app: &AppHandle, events: &[(&PullRequest, MyPrEvent)]) {
    for (pr, event) in events {
        let (title, body) = match event {
            MyPrEvent::BecameReady => ("Ready to merge", pr.title.clone()),
            MyPrEvent::ChecksFailed => ("Checks failed", pr.title.clone()),
        };
        send(app, title, format!("{body} ({})", pr.repo), &pr.html_url);
    }
}

/// One notification per new PR, so each can be clicked through to its own PR.
pub fn notify_new_prs(app: &AppHandle, new_prs: &[&PullRequest]) {
    for pr in new_prs {
        send(
            app,
            "New PR to review",
            format!("{} by {} ({})", pr.title, pr.author_display_name, pr.repo),
            &pr.html_url,
        );
    }
}

/// Clicking the notification opens `url`.
fn send(app: &AppHandle, title: &str, body: String, url: &str) {
    log::info!("[notification] sending: title=\"{title}\" body=\"{body}\"");
    let (app, title, url) = (app.clone(), title.to_string(), url.to_string());
    // Waiting for the click blocks, so each notification gets its own thread.
    std::thread::spawn(move || {
        if show_and_wait_for_click(&app, &title, &body) {
            if let Err(e) = app.opener().open_url(url, None::<&str>) {
                log::error!("[notification] failed to open PR: {e}");
            }
        }
    });
}

/// Shows the notification and blocks until it is clicked. Returns true on click.
#[cfg(target_os = "macos")]
fn show_and_wait_for_click(app: &AppHandle, title: &str, body: &str) -> bool {
    use mac_notification_sys::{Notification, NotificationResponse};

    // In dev the app has no bundle of its own, so the notification is attributed to Terminal.
    let bundle = if tauri::is_dev() { "com.apple.Terminal" } else { app.config().identifier.as_str() };
    let _ = mac_notification_sys::set_application(bundle);

    match Notification::new().title(title).message(body).wait_for_click(true).send() {
        Ok(response) => matches!(response, NotificationResponse::Click),
        Err(e) => {
            log::error!("[notification] failed to send: {e}");
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn show_and_wait_for_click(app: &AppHandle, title: &str, body: &str) -> bool {
    let mut notification = notify_rust::Notification::new();
    notification.summary(title).body(body);
    #[cfg(windows)]
    if !tauri::is_dev() {
        notification.app_id(&app.config().identifier);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    notification.action("default", "Open");
    let _ = app;

    let handle = match notification.show() {
        Ok(handle) => handle,
        Err(e) => {
            log::error!("[notification] failed to send: {e}");
            return false;
        }
    };
    let mut clicked = false;
    handle.wait_for_action(|action| clicked = action == "default");
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MergeState, MergeStatus, ProviderKind};

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

    fn mine(number: u64, state: MergeState, ci: Option<CiStatus>) -> PullRequest {
        PullRequest { merge: Some(MergeStatus::new(state, vec![])), ci_status: ci, ..pr("a", "r", number) }
    }

    #[test]
    fn reports_ready_and_failed_transitions_only() {
        use MergeState::*;
        let previous = vec![
            mine(1, Waiting, Some(CiStatus::Pending)),
            mine(2, Waiting, Some(CiStatus::Pending)),
            mine(3, Ready, Some(CiStatus::Success)),
            mine(4, Blocked, Some(CiStatus::Failed)),
        ];
        let current = vec![
            mine(1, Ready, Some(CiStatus::Success)),
            mine(2, Blocked, Some(CiStatus::Failed)),
            mine(3, Ready, Some(CiStatus::Success)),
            mine(4, Blocked, Some(CiStatus::Failed)),
            mine(5, Ready, Some(CiStatus::Success)),
        ];
        let events: Vec<_> = my_pr_events(&previous, &current)
            .into_iter()
            .map(|(pr, e)| (pr.number, e))
            .collect();
        assert_eq!(events, vec![(1, MyPrEvent::BecameReady), (2, MyPrEvent::ChecksFailed)]);
    }

    #[test]
    fn same_number_in_another_repo_is_a_new_pr() {
        let previous = vec![pr("a", "repo-a", 12)];
        let current = vec![pr("a", "repo-a", 12), pr("a", "repo-b", 12), pr("b", "repo-a", 12)];
        let added = newly_added(&previous, &current);
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|p| p.key() != "a/repo-a#12"));
    }
}
