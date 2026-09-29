use std::collections::HashMap;
use std::sync::RwLock;

use tauri::AppHandle;
use tokio::sync::{watch, Mutex};

use crate::{
    config::{self, AppConfig},
    domain::PullRequest,
    error::AppResult,
    secrets,
};

/// Latest PRs shown in the popover.
#[derive(Default)]
pub struct PrState {
    pub prs: Mutex<Vec<PullRequest>>,
}

/// Wakes the poller before its timer expires (manual refresh, settings saved).
pub struct PollTrigger {
    pub tx: watch::Sender<()>,
}

/// In-memory copy of the config and tokens, so neither the disk nor the keychain
/// are touched on every poll.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub config: AppConfig,
    /// account id → token
    pub tokens: HashMap<String, String>,
    /// Bumped on every change; lets the poller know when to rebuild its providers.
    pub revision: u64,
}

#[derive(Default)]
pub struct ConfigStore {
    inner: RwLock<Snapshot>,
}

impl ConfigStore {
    pub fn load(app: &AppHandle) -> AppResult<Self> {
        let config = config::load(app)?;
        let mut tokens = HashMap::new();
        for account in &config.accounts {
            match secrets::get_token(&account.id) {
                Ok(Some(token)) => {
                    tokens.insert(account.id.clone(), token);
                }
                Ok(None) => log::warn!("[keychain] no token for account {}", account.id),
                Err(e) => log::warn!("[keychain] {e}"),
            }
        }
        Ok(Self {
            inner: RwLock::new(Snapshot { config, tokens, revision: 0 }),
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.read().unwrap().clone()
    }

    pub fn replace(&self, config: AppConfig, tokens: HashMap<String, String>) {
        let mut inner = self.inner.write().unwrap();
        inner.revision += 1;
        inner.config = config;
        inner.tokens = tokens;
    }
}
