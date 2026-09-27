//! What the panel renders, the background refresh tick, and keeping the tray icon current.

use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use usage_core::cache::{self, CacheFile};
use usage_core::config::Config;
use usage_core::{account, log, refresh};

use crate::gauge;
use crate::login::LoginState;

pub const TRAY_ID: &str = "main";

#[derive(Default)]
pub struct AppState {
    pub login: Mutex<LoginState>,
    /// When the panel was last hidden by losing focus (a tray click also blurs it first).
    pub last_hidden: Mutex<Option<Instant>>,
}

/// Everything the panel needs to render.
#[derive(Debug, Clone, Serialize)]
pub struct PanelState {
    pub signed_in: bool,
    pub user_id: Option<String>,
    pub proxy_url: Option<String>,
    /// Pre-fills the sign-in form.
    pub suggested_url: Option<String>,
    /// LiteLLM's own Usage page.
    pub usage_url: Option<String>,
    pub cache: Option<CacheFile>,
    pub now: i64,
}

pub fn current() -> PanelState {
    let config = Config::load().unwrap_or_else(|e| {
        log::error("tray-config", &e.to_string());
        Config::default()
    });
    let cache = config.proxy_url.as_ref().and_then(|url| cache::read().filter(|c| &c.proxy_url == url));
    let usage_url = config.proxy_url.as_deref().map(usage_core::config::usage_page_url);
    PanelState {
        signed_in: config.user_id.is_some(),
        user_id: config.user_id,
        proxy_url: config.proxy_url,
        suggested_url: account::resolve_proxy_url(None).ok(),
        usage_url,
        cache,
        now: usage_core::now(),
    }
}

/// Refreshes when due, then pushes the latest state to the panel and tray.
pub fn tick(app: &AppHandle) {
    let state = current();
    if state.signed_in
        && refresh::is_due(state.cache.as_ref(), state.now)
        && let Err(e) = refresh::refresh_if_free()
    {
        log::error("tray-refresh", &e.to_string());
    }
    publish(app);
}

/// Sends the current state to the panel and updates the tray icon, title and tooltip.
pub fn publish(app: &AppHandle) {
    let state = current();
    let snap = state.cache.as_ref().and_then(|c| c.snapshot.as_ref()).filter(|_| state.signed_in);
    let fraction = snap.and_then(|s| s.budget.used_fraction());
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(gauge::icon(fraction)));
        let _ = tray.set_icon_as_template(cfg!(target_os = "macos"));
        let title = fraction.map(|f| format!("{:.0}%", f * 100.0));
        #[cfg(not(target_os = "windows"))]
        let _ = tray.set_title(title.as_deref());
        let tooltip = match (snap, title) {
            (Some(s), Some(pct)) => format!(
                "LiteLLM: {} of {} ({pct})",
                money(s.budget.spend),
                money(s.budget.max_budget.unwrap_or_default())
            ),
            (Some(s), None) => format!("LiteLLM: {} spent", money(s.budget.spend)),
            _ if !state.signed_in => "LiteLLM: signed out".to_string(),
            _ => "LiteLLM Usage".to_string(),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    let _ = app.emit("state", &state);
}

fn money(v: f64) -> String {
    format!("${v:.2}")
}
