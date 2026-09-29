use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "peekr";
const KEYRING_ACCOUNT: &str = "bitbucket_token";

/// Full in-memory config (token included).
/// token is skipped during JSON serialisation — it lives in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub workspace: String,
    #[serde(skip)]
    pub token: String,
    pub repos: Vec<String>,
    pub polling_interval_secs: u64,
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub token: String, // present only in old configs — migrated to keychain on first load
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default)]
    pub polling_interval_secs: u64,
}

fn config_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    let dir = app.path().app_config_dir()?;
    Ok(dir.join("config.json"))
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|e| e.to_string())
}

pub fn save_token(token: &str) -> Result<(), String> {
    let entry = keyring_entry()?;
    if token.is_empty() {
        let _ = entry.delete_credential();
        log::info!("[keychain] token deleted");
    } else {
        entry.set_password(token).map_err(|e| e.to_string())?;
        log::info!("[keychain] token saved (len={})", token.len());
    }
    Ok(())
}

pub fn load_token() -> String {
    match keyring_entry() {
        Ok(entry) => match entry.get_password() {
            Ok(t) => {
                log::info!("[keychain] token loaded (len={})", t.len());
                t
            }
            Err(keyring::Error::NoEntry) => {
                log::warn!("[keychain] no entry found");
                String::new()
            }
            Err(e) => {
                log::warn!("[keychain] get_password error: {}", e);
                String::new()
            }
        },
        Err(e) => {
            log::warn!("[keychain] failed to open entry: {}", e);
            String::new()
        }
    }
}

pub fn delete_token() {
    if let Ok(entry) = keyring_entry() {
        let _ = entry.delete_credential();
    }
}

pub fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    // Persist token in OS keychain
    save_token(&config.token)?;

    // Persist the rest to disk (token is skipped by #[serde(skip)])
    let path = config_path(app).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    log::info!("[config] saved to {:?}", path);
    Ok(())
}

pub fn load_config(app: &AppHandle) -> Result<Option<Config>, String> {
    let path = config_path(app).map_err(|e| e.to_string())?;
    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let raw: RawConfig = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    // One-time migration: token was stored in plain JSON, move it to keychain.
    if !raw.token.is_empty() {
        log::info!("[keychain] migrating token from config.json to keychain");
        save_token(&raw.token)?;
        // Rewrite file without the token field
        let clean = Config { workspace: raw.workspace.clone(), repos: raw.repos.clone(), polling_interval_secs: raw.polling_interval_secs, ..Default::default() };
        let _ = fs::write(&path, serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?);
    }

    let token = load_token();

    Ok(Some(Config {
        workspace: raw.workspace,
        token,
        repos: raw.repos,
        polling_interval_secs: raw.polling_interval_secs,
    }))
}
