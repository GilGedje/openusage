//! The SSO session, kept in the OS secure storage: Keychain on macOS, Credential Manager on
//! Windows, Secret Service (GNOME Keyring / KWallet) on Ubuntu. Never written to a plain file.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const SERVICE: &str = "litellm-usage";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub team_id: Option<String>,
    /// Unix seconds when the token was issued (it expires after the proxy's configured lifetime).
    pub obtained_at: i64,
}

fn entry(proxy_url: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, proxy_url).map_err(|e| Error::Keyring(e.to_string()))
}

pub fn save(proxy_url: &str, session: &Session) -> Result<()> {
    let json = serde_json::to_string(session)?;
    entry(proxy_url)?.set_password(&json).map_err(|e| Error::Keyring(e.to_string()))
}

/// `Ok(None)` when there is no saved sign-in for this proxy.
pub fn load(proxy_url: &str) -> Result<Option<Session>> {
    match entry(proxy_url)?.get_password() {
        Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Error::Keyring(e.to_string())),
    }
}

pub fn delete(proxy_url: &str) -> Result<()> {
    match entry(proxy_url)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(Error::Keyring(e.to_string())),
    }
}
