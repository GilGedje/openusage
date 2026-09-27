//! Commands the panel calls.

use tauri::{AppHandle, WebviewWindow};
use usage_core::{account, log, refresh};

use usage_core::panel::PanelState;

use crate::state;

#[tauri::command]
pub fn get_state() -> PanelState {
    state::current()
}

/// Fetches now (ignores the refresh interval).
#[tauri::command]
pub async fn refresh(app: AppHandle) -> Result<PanelState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = refresh::refresh_now() {
            log::error("tray-refresh", &e.to_string());
        }
        state::publish(&app);
        state::current()
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn refresh_in_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = refresh(app).await;
    });
}

#[tauri::command]
pub fn sign_out(app: AppHandle) -> Result<PanelState, String> {
    account::sign_out().map_err(|e| e.to_string())?;
    state::publish(&app);
    Ok(state::current())
}

/// Opens a web page in the default browser. Only http(s) links: the OS opener would also launch
/// local files and apps.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("Not a web link: {url}"));
    }
    open::that_detached(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// The panel sizes itself to its content.
#[tauri::command]
pub fn fit_height(window: WebviewWindow, height: f64) -> Result<(), String> {
    let height = height.clamp(120.0, 720.0);
    window.set_size(tauri::LogicalSize::new(320.0, height)).map_err(|e| e.to_string())?;
    if window.is_visible().unwrap_or(false) {
        crate::panel::place(&window);
    }
    Ok(())
}

/// Settings view: a new LiteLLM address (the old sign-in belongs to the old proxy, so it ends).
#[tauri::command]
pub fn save_settings(app: AppHandle, url: String) -> Result<PanelState, String> {
    account::configure(account::Setup { url: Some(&url), ..Default::default() }).map_err(|e| e.to_string())?;
    state::publish(&app);
    refresh_in_background(app);
    Ok(state::current())
}

/// The Cost card exported as a PNG (share button): saved to Downloads; returns the file's path.
#[tauri::command]
pub fn save_image(bytes: Vec<u8>, name: String) -> Result<String, String> {
    usage_core::export::save_png(&bytes, &name).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}

/// Settings → Alerts: warning / critical levels (percent of budget used) and their colors.
#[tauri::command]
pub fn save_alerts(
    app: AppHandle,
    warning_pct: u8,
    warning_color: String,
    critical_pct: u8,
    critical_color: String,
) -> Result<PanelState, String> {
    let alerts = usage_core::alerts::Alerts { warning_pct, warning_color, critical_pct, critical_color };
    account::save_alerts(alerts).map_err(|e| e.to_string())?;
    state::publish(&app);
    Ok(state::current())
}
