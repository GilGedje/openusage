//! App state, the background refresh tick, and keeping the tray icon current. What the panel shows
//! comes from `usage_core::panel`.

use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use usage_core::panel::PanelState;
use usage_core::{log, refresh};

use crate::gauge;
use crate::login::LoginState;

pub const TRAY_ID: &str = "main";

#[derive(Default)]
pub struct AppState {
    pub login: Mutex<LoginState>,
    /// When the panel was last hidden by losing focus (a tray click also blurs it first).
    pub last_hidden: Mutex<Option<Instant>>,
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
    let fraction = state.used_fraction();
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(gauge::icon(fraction)));
        let _ = tray.set_icon_as_template(cfg!(target_os = "macos"));
        #[cfg(not(target_os = "windows"))]
        let _ = tray.set_title(fraction.map(|f| format!("{:.0}%", f * 100.0)).as_deref());
        let _ = tray.set_tooltip(Some(state.tooltip()));
    }
    let _ = app.emit("state", &state);
}
