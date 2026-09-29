use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::domain::PullRequest;

/// PRs in `current` that were not in `previous`.
pub fn newly_added<'a>(previous: &[PullRequest], current: &'a [PullRequest]) -> Vec<&'a PullRequest> {
    let known: std::collections::HashSet<String> = previous.iter().map(PullRequest::key).collect();
    current.iter().filter(|pr| !known.contains(&pr.key())).collect()
}

/// What to do when the user clicks a notification.
enum ClickTarget {
    OpenPr(String),
    ShowPopover,
}

pub fn notify_new_prs(app: &AppHandle, new_prs: &[&PullRequest]) {
    let (title, body, target) = match new_prs {
        [] => return,
        [pr] => (
            "New PR to review".to_string(),
            format!("{} by {}", pr.title, pr.author_display_name),
            ClickTarget::OpenPr(pr.html_url.clone()),
        ),
        many => (
            format!("{} new PRs to review", many.len()),
            many.iter().map(|pr| pr.title.as_str()).collect::<Vec<_>>().join(", "),
            ClickTarget::ShowPopover,
        ),
    };

    log::info!("[notification] sending: title=\"{title}\" body=\"{body}\"");
    let app = app.clone();
    // Waiting for the click blocks, so each notification gets its own thread.
    std::thread::spawn(move || {
        if show_and_wait_for_click(&app, &title, &body) {
            handle_click(&app, target);
        }
    });
}

fn handle_click(app: &AppHandle, target: ClickTarget) {
    match target {
        ClickTarget::OpenPr(url) => {
            if let Err(e) = app.opener().open_url(url, None::<&str>) {
                log::error!("[notification] failed to open PR: {e}");
            }
        }
        ClickTarget::ShowPopover => crate::popover::show(app),
    }
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
        }
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
