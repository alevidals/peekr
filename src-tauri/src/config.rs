//! Non-secret settings, persisted as JSON in the app config dir.
//!
//! Format history:
//! - v1 (implicit): `{ workspace, repos, polling_interval_secs }` + a single Bitbucket token
//!   (first in the JSON itself, later in the keychain).
//! - v2: `{ version, polling_interval_secs, accounts: [...] }`, tokens keyed by account id.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{
    domain::ProviderKind,
    error::{AppError, AppResult},
    secrets,
};

const CONFIG_VERSION: u32 = 2;
pub const DEFAULT_POLL_SECS: u64 = 120;
pub const MIN_POLL_SECS: u64 = 30;
pub const MAX_POLL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub kind: ProviderKind,
    /// Bitbucket workspace / GitHub organisation or user.
    pub workspace: String,
    pub repos: Vec<String>,
}

impl Account {
    /// An account is only polled once it has everything needed to make requests.
    pub fn is_complete(&self) -> bool {
        !self.workspace.is_empty() && (!self.repos.is_empty() || !self.kind.requires_repos())
    }
}

/// UI colour scheme; `System` follows the OS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Colour preset; each one defines its own light and dark variants in the frontend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Palette {
    #[default]
    Indigo,
    Graphite,
    Green,
    Amber,
    Contrast,
    // Named themes shared with sftply; each pins its own light/dark scheme.
    OneDark,
    GruvboxDark,
    GruvboxLight,
    Nord,
    Dracula,
    SolarizedLight,
    TokyoNight,
    CatppuccinMocha,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Look of the popover on top of the light/dark [`Theme`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub palette: Palette,
    /// `#rrggbb` overriding the palette's accent; `None` keeps the palette's own.
    pub accent: Option<String>,
    pub density: Density,
    pub text_size: TextSize,
}

impl Appearance {
    /// Drops an accent that is not a `#rrggbb` colour instead of failing the whole save.
    pub fn sanitized(mut self) -> Self {
        self.accent = self.accent.filter(|c| {
            c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub polling_interval_secs: u64,
    pub accounts: Vec<Account>,
    /// Added after v2 shipped: older files simply lack it.
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub appearance: Appearance,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            polling_interval_secs: DEFAULT_POLL_SECS,
            accounts: Vec::new(),
            theme: Theme::default(),
            appearance: Appearance::default(),
        }
    }
}

pub fn clamp_interval(secs: u64) -> u64 {
    secs.clamp(MIN_POLL_SECS, MAX_POLL_SECS)
}

/// Trims, drops empty entries and removes duplicates while keeping order.
pub fn clean_repos(repos: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for repo in repos.into_iter().map(|r| r.trim().to_string()) {
        if !repo.is_empty() && !out.contains(&repo) {
            out.push(repo);
        }
    }
    out
}

fn config_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_config_dir()?.join("config.json"))
}

pub fn save(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, serde_json::to_string_pretty(config)?)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    log::info!("[config] saved to {path:?}");
    Ok(())
}

/// Loads the config, migrating older formats in place. Missing file → defaults.
pub fn load(app: &AppHandle) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;

    let mut config = if value.get("accounts").is_some() {
        serde_json::from_value::<AppConfig>(value)?
    } else {
        migrate_v1(app, serde_json::from_value(value)?)?
    };
    config.polling_interval_secs = clamp_interval(config.polling_interval_secs);
    Ok(config)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigV1 {
    workspace: String,
    token: String, // only present in the oldest configs
    repos: Vec<String>,
    polling_interval_secs: u64,
}

fn migrate_v1(app: &AppHandle, old: ConfigV1) -> AppResult<AppConfig> {
    log::info!("[config] migrating v1 config to multi-account format");

    let mut config = AppConfig {
        polling_interval_secs: clamp_interval(old.polling_interval_secs),
        ..AppConfig::default()
    };

    let had_legacy_keychain_entry;
    if old.workspace.is_empty() && old.repos.is_empty() {
        had_legacy_keychain_entry = false;
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let legacy_keychain = secrets::get_legacy_token()?;
        had_legacy_keychain_entry = legacy_keychain.is_some();
        let token = Some(old.token)
            .filter(|t| !t.is_empty())
            .or(legacy_keychain);
        if let Some(token) = token {
            secrets::set_token(&id, &token)?;
        }
        config.accounts.push(Account {
            id,
            kind: ProviderKind::Bitbucket,
            workspace: old.workspace,
            repos: clean_repos(old.repos),
        });
    }

    save(app, &config)?;
    if had_legacy_keychain_entry {
        secrets::delete_legacy_token();
    }
    Ok(config)
}

/// Rejects accounts that could never work.
pub fn validate_account(account: &Account) -> AppResult<()> {
    if account.workspace.trim().is_empty() {
        return Err(AppError::Config("Workspace is required".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_polling_interval() {
        assert_eq!(clamp_interval(0), MIN_POLL_SECS);
        assert_eq!(clamp_interval(120), 120);
        assert_eq!(clamp_interval(99_999), MAX_POLL_SECS);
    }

    #[test]
    fn cleans_repo_lists() {
        let repos = vec![" a ".into(), "".into(), "b".into(), "a".into(), "  ".into()];
        assert_eq!(clean_repos(repos), vec!["a", "b"]);
    }

    #[test]
    fn v2_roundtrip() {
        let mut config = AppConfig::default();
        config.accounts.push(Account {
            id: "x".into(),
            kind: ProviderKind::Github,
            workspace: "org".into(),
            repos: vec!["r".into()],
        });
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"github\""));
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.accounts[0].kind, ProviderKind::Github);
    }

    #[test]
    fn theme_defaults_to_system_for_older_files() {
        let config: AppConfig =
            serde_json::from_str(r#"{"version":2,"polling_interval_secs":60,"accounts":[]}"#)
                .unwrap();
        assert_eq!(config.theme, Theme::System);

        let dark: AppConfig = serde_json::from_str(
            r#"{"version":2,"polling_interval_secs":60,"accounts":[],"theme":"dark"}"#,
        )
        .unwrap();
        assert_eq!(dark.theme, Theme::Dark);
    }

    #[test]
    fn appearance_defaults_and_validates_accent() {
        let config: AppConfig =
            serde_json::from_str(r#"{"version":2,"polling_interval_secs":60,"accounts":[]}"#)
                .unwrap();
        assert_eq!(config.appearance, Appearance::default());

        let partial: Appearance =
            serde_json::from_str(r##"{"palette":"green","accent":"#12ab9F"}"##).unwrap();
        assert_eq!(
            (partial.palette, partial.density),
            (Palette::Green, Density::Comfortable)
        );
        assert_eq!(partial.sanitized().accent.as_deref(), Some("#12ab9F"));

        for bad in ["red", "#12345", "#12345g", "12ab9f1", "url(x)"] {
            let a = Appearance {
                accent: Some(bad.into()),
                ..Default::default()
            };
            assert_eq!(a.sanitized().accent, None, "{bad}");
        }
    }

    #[test]
    fn named_palettes_use_camel_case() {
        let appearance: Appearance =
            serde_json::from_str(r#"{"palette":"catppuccinMocha"}"#).unwrap();
        assert_eq!(appearance.palette, Palette::CatppuccinMocha);
        assert!(serde_json::to_string(&appearance)
            .unwrap()
            .contains("\"catppuccinMocha\""));
    }

    #[test]
    fn v1_defaults_fill_missing_fields() {
        let old: ConfigV1 = serde_json::from_str(r#"{"workspace":"w","repos":["r"]}"#).unwrap();
        assert_eq!(old.polling_interval_secs, 0); // clamped to the minimum by the migration
        assert!(old.token.is_empty());
    }
}
