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

/// The panel's settings view: LiteLLM address, status page, CA certificate. A new LiteLLM address
/// signs the user out (the old sign-in belongs to the old proxy).
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    url: String,
    status_url: String,
    ca_cert: String,
) -> Result<PanelState, String> {
    account::configure(account::Setup {
        url: Some(&url),
        status_url: Some(&status_url),
        ca_cert: Some(&ca_cert),
        copy_ca: false,
    })
    .map_err(|e| e.to_string())?;
    state::publish(&app);
    refresh_in_background(app);
    Ok(state::current())
}
