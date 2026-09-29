use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{
    bitbucket::{self, RepoInfo},
    credentials::{self, Config},
    state::PrState,
};

#[derive(serde::Serialize)]
pub struct ConfigResponse {
    pub workspace: String,
    pub repos: Vec<String>,
    pub polling_interval_secs: u64,
    pub has_token: bool,
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> Result<Option<ConfigResponse>, String> {
    match credentials::load_config(&app)? {
        Some(c) => Ok(Some(ConfigResponse {
            has_token: !c.token.is_empty(),
            workspace: c.workspace,
            repos: c.repos,
            polling_interval_secs: c.polling_interval_secs,
        })),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn save_credentials(
    app: AppHandle,
    workspace: String,
    token: String,
    repos: Vec<String>,
    polling_interval_secs: Option<u64>,
) -> Result<(), String> {
    log::info!("[save_credentials] token_len={} workspace={} repos={}", token.len(), workspace, repos.len());
    // If the caller sent an empty token, keep whatever is already stored.
    let resolved_token = if token.is_empty() {
        credentials::load_config(&app)?
            .map(|c| c.token)
            .unwrap_or_default()
    } else {
        token
    };
    log::info!("[save_credentials] resolved_token_len={}", resolved_token.len());
    let config = Config {
        workspace,
        token: resolved_token,
        repos,
        polling_interval_secs: polling_interval_secs.unwrap_or(120),
    };
    credentials::save_config(&app, &config)?;

    Ok(())
}

#[tauri::command]
pub async fn hide_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}


#[tauri::command]
pub async fn delete_credentials(app: AppHandle) -> Result<(), String> {
    credentials::delete_token();
    credentials::save_config(&app, &Config::default())?;
    Ok(())
}

#[tauri::command]
pub async fn validate_credentials(token: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    bitbucket::fetch_user_account_id(&client, &token).await
}

#[tauri::command]
pub async fn get_pull_requests(state: State<'_, PrState>) -> Result<Vec<bitbucket::PullRequest>, String> {
    Ok(state.prs.lock().await.clone())
}

#[tauri::command]
pub async fn test_notification(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title("New PR to review")
        .body("Fix login bug by John Doe")
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    // Hide the popover first (it has alwaysOnTop=true and would cover settings)
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }

    let win = match app.get_webview_window("settings") {
        Some(w) => w,
        None => {
            // Window was closed/destroyed — recreate it
            WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("/".into()))
                .title("peekr — Settings")
                .inner_size(440.0, 520.0)
                .resizable(false)
                .decorations(true)
                .always_on_top(false)
                .skip_taskbar(false)
                .visible(false)
                .build()
                .map_err(|e| e.to_string())?
        }
    };

    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    // set_focus() briefly activates the app as foreground on macOS (Dock icon appears).
    // Re-assert Accessory policy immediately so the app never shows in the Dock.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    Ok(())
}

#[tauri::command]
pub async fn refresh_pull_requests(
    trigger: State<'_, crate::state::PollTrigger>,
) -> Result<(), String> {
    trigger
        .tx
        .send(())
        .map_err(|_| "Failed to trigger refresh".to_string())
}
