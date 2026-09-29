mod bitbucket;
mod commands;
mod credentials;
mod state;
mod tray;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::{PollTrigger, PrState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            // Keep the app as a pure tray/accessory app — no Dock icon, no menu-bar
            // app-name slot.  Info.plist sets LSUIElement for the bundle, but the
            // runtime call is needed so dev mode (pnpm tauri dev) behaves the same way.
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Generation counter shared with the tray click handler (tray.rs).
            // See state::PopoverHideState for the design rationale.
            let close_gen  = Arc::new(AtomicU64::new(0));
            let focused_at = Arc::new(AtomicU64::new(0));
            app.manage(state::PopoverHideState {
                close_gen:  close_gen.clone(),
                focused_at: focused_at.clone(),
            });

            tray::create_tray(app)?;

            // Hide popover on blur (click-outside behaviour)
            if let Some(popover) = app.get_webview_window("popover") {
                let close_gen2  = close_gen.clone();
                let focused_at2 = focused_at.clone();
                let popover_ref = popover.clone();

                popover.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::Focused(true) => {
                            // Bump the generation to cancel any in-flight hide task.
                            close_gen2.fetch_add(1, Ordering::SeqCst);
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            focused_at2.store(now, Ordering::SeqCst);
                        }
                        tauri::WindowEvent::Focused(false) => {
                            // Ignore the spurious Focused(false) that macOS fires right
                            // after a programmatic window.hide() — it arrives within a
                            // few ms of the hide, well inside the 300 ms guard.
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            if now.saturating_sub(focused_at2.load(Ordering::SeqCst)) < 300 {
                                return;
                            }
                            // Capture the new generation; the hide task uses it to
                            // detect whether it has been superseded.
                            let gen = close_gen2.fetch_add(1, Ordering::SeqCst) + 1;
                            let _ = popover_ref.emit("popover-will-hide", ());
                            let w  = popover_ref.clone();
                            let cg = close_gen2.clone();
                            let fa = focused_at2.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(150)).await;
                                if cg.load(Ordering::SeqCst) == gen {
                                    let now2 = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;
                                    // Update focused_at before hiding so that the
                                    // Focused(false) fired by hide() is caught by the
                                    // 300 ms guard above and doesn't restart the cycle.
                                    fa.store(now2, Ordering::SeqCst);
                                    let _ = w.hide();
                                }
                            });
                        }
                        _ => {}
                    }
                });
            }

            app.manage(PrState::new());

            let (tx, rx) = watch::channel(());
            app.manage(PollTrigger { tx });

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                run_polling_loop(app_handle, rx).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_credentials,
            commands::delete_credentials,
            commands::validate_credentials,
            commands::get_pull_requests,
            commands::refresh_pull_requests,
            commands::open_settings_window,
            commands::hide_settings_window,
            commands::test_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn run_polling_loop(app: tauri::AppHandle, mut rx: watch::Receiver<()>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let mut account_id_cache: Option<String> = None;
    let mut use_q_filter = true;
    let mut is_first_poll = true;

    loop {
        let config = match credentials::load_config(&app) {
            Ok(Some(c)) if !c.token.is_empty() && !c.repos.is_empty() => {
                log::info!("[peekr] config ok — workspace='{}' repos={:?}", c.workspace, c.repos);
                c
            }
            Ok(Some(c)) => {
                log::warn!("[peekr] config incomplete — token_set={} repos={}", !c.token.is_empty(), c.repos.len());
                wait_for_next_poll(30, &mut rx).await;
                continue;
            }
            Ok(None) => {
                log::warn!("[peekr] no config file found, skipping poll");
                wait_for_next_poll(30, &mut rx).await;
                continue;
            }
            Err(e) => {
                log::error!("[peekr] failed to load config: {}", e);
                // wait before retrying
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }
        };

        let token = config.token.clone();

        // Cache account_id
        if account_id_cache.is_none() {
            log::info!("[peekr] fetching account_id");
            match bitbucket::fetch_user_account_id(&client, &token).await {
                Ok(id) => {
                    log::info!("[peekr] account_id = {}", id);
                    account_id_cache = Some(id);
                }
                Err(e) => {
                    log::error!("Failed to fetch account_id: {}", e);
                    let _ = app.emit("poll-error", e);
                    continue;
                }
            }
        }

        let account_id = account_id_cache.as_deref().unwrap();

        match bitbucket::fetch_all_review_prs(
            &client,
            &config.workspace,
            &config.repos,
            &token,
            account_id,
            &mut use_q_filter,
        )
        .await
        {
            Ok(new_prs) => {
                log::info!("[peekr] poll complete: {} PRs pending review", new_prs.len());
                let state = app.state::<PrState>();
                let mut current = state.prs.lock().await;

                if !is_first_poll {
                    let known_ids: HashSet<u64> = current.iter().map(|pr| pr.id).collect();
                    let added: Vec<_> = new_prs.iter().filter(|pr| !known_ids.contains(&pr.id)).collect();

                    if !added.is_empty() {
                        send_notification(&app, &added);
                    }
                }

                is_first_poll = false;
                let pr_count = new_prs.len() as u32;
                *current = new_prs.clone();
                drop(current);

                let _ = app.emit("prs-updated", &new_prs);
                tray::update_tray_badge(&app, pr_count);
            }
            Err(e) => {
                log::error!("Polling error: {}", e);
                let _ = app.emit("poll-error", &e);
            }
        }

        let interval_secs = credentials::load_config(&app)
            .ok()
            .flatten()
            .map(|c| c.polling_interval_secs)
            .unwrap_or(120);

        wait_for_next_poll(interval_secs, &mut rx).await;
    }
}

async fn wait_for_next_poll(secs: u64, rx: &mut watch::Receiver<()>) {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(secs)) => {
            log::info!("[peekr] polling triggered by timer (interval={}s)", secs);
        }
        _ = rx.changed() => {
            log::info!("[peekr] polling triggered manually");
        }
    }
}

fn send_notification(app: &tauri::AppHandle, new_prs: &[&bitbucket::PullRequest]) {
    let (title, body) = if new_prs.len() == 1 {
        let pr = &new_prs[0];
        (
            "New PR to review".to_string(),
            format!("{} by {}", pr.title, pr.author_display_name),
        )
    } else {
        (
            format!("{} new PRs to review", new_prs.len()),
            new_prs
                .iter()
                .map(|pr| pr.title.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    log::info!("[notification] sending: title=\"{}\" body=\"{}\"", title, body);
    let result = app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show();
    if let Err(e) = result {
        log::error!("[notification] failed to send: {}", e);
    }
}
