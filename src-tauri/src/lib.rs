mod commands;
mod config;
mod domain;
mod error;
mod notifier;
mod poller;
mod popover;
mod provider;
mod secrets;
mod state;
mod tray;

use tauri::Manager;
use tokio::sync::watch;

use crate::state::{ConfigStore, PollTrigger, PrState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
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

            let store = ConfigStore::load(app.handle()).unwrap_or_else(|e| {
                log::error!("[config] failed to load, starting empty: {e}");
                ConfigStore::default()
            });
            app.manage(store);
            app.manage(PrState::default());

            popover::setup(app);
            tray::create_tray(app)?;

            let (tx, rx) = watch::channel(());
            app.manage(PollTrigger { tx });
            tauri::async_runtime::spawn(poller::run(app.handle().clone(), rx));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::validate_credentials,
            commands::get_pull_requests,
            commands::refresh_pull_requests,
            commands::open_settings_window,
            commands::hide_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
