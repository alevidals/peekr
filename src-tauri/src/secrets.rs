//! Access tokens live in the OS keychain, one entry per configured account.

use crate::error::{AppError, AppResult};

const SERVICE: &str = "peekr";
/// Single entry used before multi-account support.
const LEGACY_ACCOUNT: &str = "bitbucket_token";

fn entry(name: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).map_err(|e| AppError::Keychain(e.to_string()))
}

fn account_entry(account_id: &str) -> AppResult<keyring::Entry> {
    entry(&format!("token:{account_id}"))
}

fn read(entry: keyring::Entry) -> AppResult<Option<String>> {
    match entry.get_password() {
        Ok(token) => Ok(Some(token).filter(|t| !t.is_empty())),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

pub fn get_token(account_id: &str) -> AppResult<Option<String>> {
    read(account_entry(account_id)?)
}

pub fn set_token(account_id: &str, token: &str) -> AppResult<()> {
    account_entry(account_id)?
        .set_password(token)
        .map_err(|e| AppError::Keychain(e.to_string()))?;
    log::info!("[keychain] token saved for account {account_id}");
    Ok(())
}

pub fn delete_token(account_id: &str) {
    if let Ok(entry) = account_entry(account_id) {
        let _ = entry.delete_credential();
        log::info!("[keychain] token deleted for account {account_id}");
    }
}

pub fn get_legacy_token() -> AppResult<Option<String>> {
    read(entry(LEGACY_ACCOUNT)?)
}

pub fn delete_legacy_token() {
    if let Ok(entry) = entry(LEGACY_ACCOUNT) {
        let _ = entry.delete_credential();
    }
}
