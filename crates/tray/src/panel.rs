//! Showing and hiding the popup panel next to the tray icon.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::{self, AppState};

const PANEL: &str = "panel";
/// A tray click first blurs (hides) an open panel; ignore the click that follows so it stays shut.
const REOPEN_GUARD: Duration = Duration::from_millis(300);
/// Gap between the click point and the panel.
const GAP: i32 = 8;

/// Left click on the icon. `click` is the screen position when the tray reports one (Linux).
pub fn toggle(app: &AppHandle, click: Option<(i32, i32)>) {
    let Some(window) = app.get_webview_window(PANEL) else {
        return;
    };
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
        show(app, click);
    }
}

pub fn show(app: &AppHandle, click: Option<(i32, i32)>) {
    let Some(window) = app.get_webview_window(PANEL) else {
        return;
    };
    if let Ok(mut anchor) = app.state::<AppState>().anchor.lock() {
        *anchor = click;
    }
    place(&window);
    let _ = window.show();
    let _ = window.set_focus();
    state::publish(app);
}

/// "Change LiteLLM URL…": open the panel on its settings view.
pub fn show_settings(app: &AppHandle) {
    show(app, None);
    let _ = app.emit("show-settings", ());
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        if let Ok(mut t) = app.state::<AppState>().last_hidden.lock() {
            *t = Some(Instant::now());
        }
    }
}

/// Next to the click when we know where it was (Linux), else at the tray icon (macOS, Windows), else
/// the tray's corner of the screen. Always kept on screen.
pub fn place(window: &WebviewWindow) {
    let anchor = window
        .app_handle()
        .state::<AppState>()
        .anchor
        .lock()
        .ok()
        .and_then(|a| *a);
    if let Some((x, y)) = anchor
        && place_near(window, x, y)
    {
        return;
    }
    if cfg!(target_os = "linux")
        || window
            .move_window_constrained(Position::TrayCenter)
            .is_err()
    {
        // The icon's position isn't known until it's been clicked (e.g. "Open" before any click).
        if !place_in_tray_corner(window) {
            let _ = window.move_window(Position::TopRight);
        }
    }
}

/// Bottom right above the taskbar on Windows; top right under the menu bar / top panel elsewhere.
fn place_in_tray_corner(window: &WebviewWindow) -> bool {
    let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) else {
        return false;
    };
    let area = monitor.work_area();
    let (w, h) = (size.width as i32, size.height as i32);
    let x = area.position.x + area.size.width as i32 - w - GAP;
    let y = if cfg!(windows) {
        area.position.y + area.size.height as i32 - h - GAP
    } else {
        area.position.y + GAP
    };
    window.set_position(PhysicalPosition::new(x, y)).is_ok()
}

fn place_near(window: &WebviewWindow, x: i32, y: i32) -> bool {
    let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) else {
        return false;
    };
    let (mx, my) = (monitor.position().x, monitor.position().y);
    let (mw, mh) = (monitor.size().width as i32, monitor.size().height as i32);
    let (w, h) = (size.width as i32, size.height as i32);
    let left = (x - w / 2).clamp(mx, mx + mw - w);
    // Top bar → open below the click; bottom panel → open above it.
    let top = if y < my + mh / 2 {
        y + GAP
    } else {
        y - h - GAP
    };
    window
        .set_position(PhysicalPosition::new(left, top.clamp(my, my + mh - h)))
        .is_ok()
}
