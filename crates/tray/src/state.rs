//! App state, the background refresh tick, and keeping the tray icon current. What the panel shows
//! comes from `usage_core::panel`.

use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use usage_core::panel::PanelState;
use usage_core::{log, refresh};

use crate::login::LoginState;

pub const TRAY_ID: &str = "main";

#[derive(Default)]
pub struct AppState {
    pub login: Mutex<LoginState>,
    /// When the panel was last hidden by losing focus (a tray click also blurs it first).
    pub last_hidden: Mutex<Option<Instant>>,
    /// Test mode (`LITELLM_USAGE_OPEN_PANEL=pin`): keep the panel open when it loses focus, e.g.
    /// on a Windows machine where a script-started app isn't allowed to take focus.
    pub pinned: std::sync::atomic::AtomicBool,
    /// Where the tray was clicked (Linux reports it); the panel opens next to it.
    pub anchor: Mutex<Option<(i32, i32)>>,
    /// The StatusNotifierItem icon, when used (Linux).
    #[cfg(target_os = "linux")]
    pub sni: Mutex<Option<sni_tray::SniTray>>,
}

pub fn current() -> PanelState {
    PanelState::current()
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
    crate::tray_icon::update(app, state.used_fraction(), &state.tooltip(), &state.alerts);
    let _ = app.emit("state", &state);
}
