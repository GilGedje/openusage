//! What a tray panel renders, independent of the UI framework — shared by the Tauri 2 tray and the
//! Tauri 1 tray for Ubuntu 20.04.

use serde::Serialize;

use crate::cache::{self, CacheFile};
use crate::config::{self, Config};
use crate::{account, log};

/// Everything the panel needs to render (sent to the web UI as the `state` event).
#[derive(Debug, Clone, Serialize)]
pub struct PanelState {
    pub signed_in: bool,
    pub user_id: Option<String>,
    pub proxy_url: Option<String>,
    /// Pre-fills the sign-in form.
    pub suggested_url: Option<String>,
    /// LiteLLM's own Usage page.
    pub usage_url: Option<String>,
    /// The organization's status page, when configured.
    pub status_url: Option<String>,
    /// Custom CA certificate file, when configured.
    pub ca_cert: Option<String>,
    pub cache: Option<CacheFile>,
    pub now: i64,
}

impl PanelState {
    pub fn current() -> PanelState {
        let config = Config::load().unwrap_or_else(|e| {
            log::error("tray-config", &e.to_string());
            Config::default()
        });
        let cache = config.proxy_url.as_ref().and_then(|url| cache::read().filter(|c| &c.proxy_url == url));
        let usage_url = config.proxy_url.as_deref().map(config::usage_page_url);
        let status_url = config.status_page();
        let ca_cert = config.ca_cert.clone();
        PanelState {
            signed_in: config.user_id.is_some(),
            user_id: config.user_id,
            proxy_url: config.proxy_url,
            suggested_url: account::resolve_proxy_url(None).ok(),
            usage_url,
            status_url,
            ca_cert,
            cache,
            now: crate::now(),
        }
    }

    /// Share of the budget used, when signed in with a limited budget.
    pub fn used_fraction(&self) -> Option<f64> {
        self.snapshot().and_then(|s| s.budget.used_fraction())
    }

    /// The tray icon's hover text.
    pub fn tooltip(&self) -> String {
        match (self.snapshot(), self.used_fraction()) {
            (Some(s), Some(f)) => format!(
                "LiteLLM: ${:.2} of ${:.2} ({:.0}%)",
                s.budget.spend,
                s.budget.max_budget.unwrap_or_default(),
                f * 100.0
            ),
            (Some(s), None) => format!("LiteLLM: ${:.2} spent", s.budget.spend),
            _ if !self.signed_in => "LiteLLM: signed out".to_string(),
            _ => "LiteLLM Usage".to_string(),
        }
    }

    fn snapshot(&self) -> Option<&crate::snapshot::Snapshot> {
        self.cache.as_ref().and_then(|c| c.snapshot.as_ref()).filter(|_| self.signed_in)
    }
}
