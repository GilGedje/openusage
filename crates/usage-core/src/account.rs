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
        status_url: Config::load()?.status_url,
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
    Config { proxy_url: config.proxy_url, status_url: config.status_url, ..Config::default() }.save()?;
    cache::clear()
}
