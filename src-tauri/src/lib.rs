mod commands;
mod config;
mod domain;
mod error;
mod http;
mod notifier;
mod poller;
mod popover;
mod provider;
mod secrets;
mod state;
mod tray;
mod update;

use tauri::Manager;
use tokio::sync::watch;

use crate::state::{ConfigStore, PollTrigger, PrState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // Keep one tray process. A second launch brings the existing popover forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            popover::show(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .max_file_size(2 * 1024 * 1024)
                .level(log::LevelFilter::Info)
                // Keep dependency noise down: suppaftp logs raw PASS at trace.
                .level_for(
                    "peekr_lib",
                    if cfg!(debug_assertions) {
                        log::LevelFilter::Debug
                    } else {
                        log::LevelFilter::Info
                    },
                )
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ));

    // Turns the popover into an NSPanel (see popover.rs); macOS only.
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(|app| {
            // Keep the app as a pure tray/accessory app — no Dock icon, no menu-bar
            // app-name slot.  Info.plist sets LSUIElement for the bundle, but the
            // runtime call is needed so dev mode (bun run tauri dev) behaves the same way.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let store = ConfigStore::load(app.handle()).unwrap_or_else(|e| {
                log::error!("[config] failed to load, starting empty: {e}");
                ConfigStore::default()
            });
            app.manage(store);
            app.manage(PrState::default());
            // Shared by the poller and the settings validation so every call has the same
            // timeout and connection pool.
            app.manage(http::client());

            popover::setup(app);
            tray::create_tray(app)?;

            let (tx, rx) = watch::channel(());
            app.manage(PollTrigger { tx });
            tauri::async_runtime::spawn(poller::run(app.handle().clone(), rx));

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing settings with the window's own close button also drops the Dock icon.
            if window.label() == "settings"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                commands::set_dock_visible(window.app_handle(), false);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::validate_credentials,
            commands::get_pull_requests,
            commands::get_my_pull_requests,
            commands::refresh_pull_requests,
            commands::open_settings_window,
            commands::hide_settings_window,
            commands::hide_popover,
            commands::open_logs,
            commands::check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
