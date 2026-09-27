//! Non-secret settings (the proxy URL) and the app's config/cache folders. The token itself lives
//! in secure storage (see `secret`), never here.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const APP_DIR: &str = "litellm-usage";

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Proxy the saved sign-in belongs to, normalized (see `normalize_url`).
    pub proxy_url: Option<String>,
    /// Who signed in (the token itself is in secure storage).
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    /// Unix seconds when the token was issued (it expires after the proxy's configured lifetime).
    #[serde(default)]
    pub signed_in_at: Option<i64>,
}

impl Config {
    pub fn load() -> Result<Config> {
        let path = config_dir().join("config.json");
        match fs::read(&path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("config.json"), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join(APP_DIR)
}

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join(APP_DIR)
}

/// Turns what users paste (or what Claude Code's `ANTHROPIC_BASE_URL` holds) into the proxy root:
/// trims whitespace and trailing slashes, and drops a trailing `/v1` or `/anthropic` route suffix.
/// Any other path is kept, since a proxy can be served under a sub-path.
pub fn normalize_url(raw: &str) -> Result<String> {
    let mut url = raw.trim().trim_end_matches('/').to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(Error::Invalid(format!("Proxy URL must start with http:// or https:// (got `{raw}`)")));
    }
    for suffix in ["/v1", "/anthropic"] {
        if let Some(stripped) = url.strip_suffix(suffix) {
            url = stripped.trim_end_matches('/').to_string();
        }
    }
    if url.split("://").nth(1).is_none_or(str::is_empty) {
        return Err(Error::Invalid(format!("Proxy URL has no host (got `{raw}`)")));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::normalize_url;

    #[test]
    fn normalizes_common_forms() {
        assert_eq!(normalize_url("http://localhost:4000").unwrap(), "http://localhost:4000");
        assert_eq!(normalize_url(" http://localhost:4000/ ").unwrap(), "http://localhost:4000");
        assert_eq!(normalize_url("https://llm.example.com/v1").unwrap(), "https://llm.example.com");
        assert_eq!(normalize_url("https://llm.example.com/anthropic/").unwrap(), "https://llm.example.com");
        assert_eq!(normalize_url("https://example.com/litellm").unwrap(), "https://example.com/litellm");
    }

    #[test]
    fn rejects_bad_urls() {
        assert!(normalize_url("localhost:4000").is_err());
        assert!(normalize_url("http://").is_err());
    }
}
