use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{
    config::{self, Account, AppConfig, Appearance, Theme},
    domain::{ProviderKind, PullRequest},
    error::{AppError, AppResult},
    provider, secrets,
    state::{ConfigStore, PollTrigger, PrState},
};

#[derive(Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub kind: ProviderKind,
    pub workspace: String,
    pub repos: Vec<String>,
    pub has_token: bool,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub polling_interval_secs: u64,
    pub theme: Theme,
    pub appearance: Appearance,
    pub accounts: Vec<AccountResponse>,
}

/// An account as edited in the settings window.
#[derive(Deserialize)]
pub struct AccountInput {
    /// `None` for accounts that have just been added.
    pub id: Option<String>,
    pub kind: ProviderKind,
    pub workspace: String,
    pub repos: Vec<String>,
    /// Empty/absent keeps the stored token.
    pub token: Option<String>,
}

#[tauri::command]
pub fn get_config(store: State<'_, ConfigStore>) -> ConfigResponse {
    let snapshot = store.snapshot();
    ConfigResponse {
        polling_interval_secs: snapshot.config.polling_interval_secs,
        theme: snapshot.config.theme,
        appearance: snapshot.config.appearance,
        accounts: snapshot
            .config
            .accounts
            .into_iter()
            .map(|a| AccountResponse {
                has_token: snapshot.tokens.contains_key(&a.id),
                id: a.id,
                kind: a.kind,
                workspace: a.workspace,
                repos: a.repos,
            })
            .collect(),
    }
}

#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    trigger: State<'_, PollTrigger>,
    polling_interval_secs: u64,
    theme: Theme,
    appearance: Appearance,
    accounts: Vec<AccountInput>,
) -> AppResult<()> {
    let previous = store.snapshot();
    let mut new_accounts = Vec::with_capacity(accounts.len());
    let mut new_tokens = HashMap::new();

    for input in accounts {
        // Only reuse ids we know about; anything else is treated as a new account.
        let id = input
            .id
            .filter(|id| previous.config.accounts.iter().any(|a| &a.id == id))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let account = Account {
            id: id.clone(),
            kind: input.kind,
            workspace: input.workspace.trim().to_string(),
            repos: config::clean_repos(input.repos),
        };
        config::validate_account(&account)?;

        let token = match input.token.filter(|t| !t.is_empty()) {
            Some(token) => {
                secrets::set_token(&id, &token)?;
                Some(token)
            }
            None => previous.tokens.get(&id).cloned(),
        };
        if let Some(token) = token {
            new_tokens.insert(id, token);
        }
        new_accounts.push(account);
    }

    for old in &previous.config.accounts {
        if !new_accounts.iter().any(|a| a.id == old.id) {
            secrets::delete_token(&old.id);
        }
    }

    let new_config = AppConfig {
        polling_interval_secs: config::clamp_interval(polling_interval_secs),
        accounts: new_accounts,
        theme,
        appearance: appearance.sanitized(),
        ..AppConfig::default()
    };
    config::save(&app, &new_config)?;
    store.replace(new_config, new_tokens);

    trigger
        .tx
        .send(())
        .map_err(|_| AppError::Config("Failed to trigger refresh".into()))
}

/// Checks a token and returns the identifier of its owner. When `token` is empty, the token
/// already stored for `account_id` is checked instead.
#[tauri::command]
pub async fn validate_credentials(
    store: State<'_, ConfigStore>,
    kind: ProviderKind,
    account_id: Option<String>,
    token: Option<String>,
) -> AppResult<String> {
    let token = token
        .filter(|t| !t.is_empty())
        .or_else(|| account_id.and_then(|id| store.snapshot().tokens.get(&id).cloned()))
        .ok_or_else(|| AppError::Config("No token to validate".into()))?;

    let account = Account {
        id: String::new(),
        kind,
        workspace: String::new(),
        repos: Vec::new(),
    };
    provider::build(reqwest::Client::new(), &account, &token)
        .validate()
        .await
}

#[tauri::command]
pub async fn get_pull_requests(state: State<'_, PrState>) -> Result<Vec<PullRequest>, ()> {
    Ok(state.prs.lock().await.clone())
}

#[tauri::command]
pub async fn get_my_pull_requests(state: State<'_, PrState>) -> Result<Vec<PullRequest>, ()> {
    Ok(state.mine.lock().await.clone())
}

#[tauri::command]
pub fn refresh_pull_requests(trigger: State<'_, PollTrigger>) -> AppResult<()> {
    trigger
        .tx
        .send(())
        .map_err(|_| AppError::Config("Failed to trigger refresh".into()))
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) -> AppResult<()> {
    if let Some(win) = app.get_webview_window(crate::popover::LABEL) {
        crate::popover::schedule_hide(&win);
    }
    Ok(())
}

#[tauri::command]
pub fn hide_settings_window(app: AppHandle) -> AppResult<()> {
    if let Some(win) = app.get_webview_window("settings") {
        win.hide()?;
    }
    set_dock_visible(&app, false);
    Ok(())
}

/// Shows the app in the Dock (and app switcher) while the settings window is open,
/// and goes back to a pure tray app when it is closed. No-op outside macOS.
pub fn set_dock_visible(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    });
    #[cfg(not(target_os = "macos"))]
    let _ = (app, visible);
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> AppResult<()> {
    // Hide the popover first (it has alwaysOnTop=true and would cover settings)
    if let Some(popover) = app.get_webview_window(crate::popover::LABEL) {
        let _ = popover.hide();
    }

    let win = match app.get_webview_window("settings") {
        Some(w) => w,
        // Window was closed/destroyed — recreate it
        None => {
            let builder = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("/".into()))
                .title("peekr — Settings")
                .inner_size(480.0, 640.0)
                .resizable(false)
                .decorations(true)
                .always_on_top(false)
                .skip_taskbar(false)
                .visible(false);
            // Content extends under the title bar; only the traffic lights remain.
            #[cfg(target_os = "macos")]
            let builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true);
            builder.build()?
        }
    };

    // Regular policy must be set before focusing so the app activates with a Dock icon.
    set_dock_visible(&app, true);
    win.show()?;
    win.set_focus()?;
    Ok(())
}
