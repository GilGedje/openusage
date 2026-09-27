//! Signing in and out, shared by `ccline` and the tray app. The interactive parts (showing the
//! code, choosing a team) stay in each app; see `sso` for the login protocol.

use crate::config::{Config, normalize_url};

use crate::{Error, Result, cache, secret};

/// The proxy to sign in to: an explicit URL, else `ANTHROPIC_BASE_URL`, else the last one used.
pub fn resolve_proxy_url(explicit: Option<&str>) -> Result<String> {
    let raw = explicit
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok().filter(|s| !s.trim().is_empty()))
        .or(Config::load()?.proxy_url)
        .ok_or(Error::NotConfigured)?;
    normalize_url(&raw)
}

/// Stores a finished sign-in: token in secure storage, who/where in the config file.
pub fn save_sign_in(proxy_url: &str, token: &str, user_id: &str, team_id: Option<String>) -> Result<()> {
    secret::save(proxy_url, token)?;
    Config {
        proxy_url: Some(proxy_url.to_string()),
        user_id: Some(user_id.to_string()),
        team_id,
        signed_in_at: Some(crate::now()),
        ..Config::load()?
    }
    .save()?;
    cache::clear()
}

/// Forgets the token and cached usage; keeps the proxy URL so signing in again needs no URL.
pub fn sign_out() -> Result<()> {
    let config = Config::load()?;
    if let Some(url) = &config.proxy_url {
        secret::delete(url)?;
    }
    // Keep everything that isn't the sign-in itself.
    Config { user_id: None, team_id: None, signed_in_at: None, ..config }.save()?;
    cache::clear()
}

/// What an installer passes in. `None` leaves a setting unchanged; an empty string clears it.
#[derive(Debug, Default)]
pub struct Setup<'a> {
    pub url: Option<&'a str>,
    pub status_url: Option<&'a str>,
    /// Custom CA file to trust (PEM). Used in place unless `copy_ca`.
    pub ca_cert: Option<&'a str>,
    /// Copy the CA file next to the settings instead of pointing at it.
    pub copy_ca: bool,
}

/// Saves what an installer was given. A new proxy address drops the old sign-in details (they
/// belong to the old proxy).
pub fn configure(setup: Setup) -> Result<Config> {
    let Setup { url, status_url, ca_cert, copy_ca } = setup;
    let mut config = Config::load()?;
    if let Some(url) = url {
        let url = normalize_url(url)?;
        if config.proxy_url.as_deref() != Some(url.as_str()) {
            config.user_id = None;
            config.team_id = None;
            config.signed_in_at = None;
        }
        config.proxy_url = Some(url);
    }
    if let Some(status) = status_url {
        let status = status.trim().trim_end_matches('/').to_string();
        if status.is_empty() {
            config.status_url = None;
        } else if status.starts_with("https://") || status.starts_with("http://") {
            config.status_url = Some(status);
        } else {
            return Err(Error::Invalid(format!("Status page must start with http:// or https:// (got `{status}`)")));
        }
    }
    if let Some(ca) = ca_cert.map(str::trim) {
        config.ca_cert = if ca.is_empty() {
            None
        } else {
            Some(crate::tls::prepare_ca(std::path::Path::new(ca), copy_ca)?.to_string_lossy().into_owned())
        };
    }
    config.save()?;
    Ok(config)
}

/// Saves the budget alert levels and colors (Settings → Alerts).
pub fn save_alerts(alerts: crate::alerts::Alerts) -> Result<Config> {
    alerts.validate()?;
    let config = Config { alerts, ..Config::load()? };
    config.save()?;
    Ok(config)
}
