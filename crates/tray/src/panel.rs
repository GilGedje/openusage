//! Showing and hiding the popup panel under the tray icon.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::{self, AppState};

const PANEL: &str = "panel";
/// A tray click first blurs (hides) an open panel; ignore the click that follows so it stays shut.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL) else { return };
    if window.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    let recently_hidden = app
        .state::<AppState>()
        .last_hidden
        .lock()
        .ok()
        .and_then(|t| *t)
        .is_some_and(|t| t.elapsed() < REOPEN_GUARD);
    if !recently_hidden {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL) else { return };
    place(&window);
    let _ = window.show();
    let _ = window.set_focus();
    state::publish(app);
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL) else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        if let Ok(mut t) = app.state::<AppState>().last_hidden.lock() {
            *t = Some(Instant::now());
        }
    }
}

/// Under the tray icon, kept on screen. The tray position is unknown until the icon is clicked, and
/// always on Linux (no tray click events there), so fall back to the top right of the screen.
pub fn place(window: &WebviewWindow) {
    if cfg!(target_os = "linux") || window.move_window_constrained(Position::TrayCenter).is_err() {
        let _ = window.move_window(Position::TopRight);
    }
}
