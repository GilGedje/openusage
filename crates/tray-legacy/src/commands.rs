//! Commands the panel calls — the same names and payloads as crates/tray, so the UI is shared.

use tauri::{AppHandle, Manager, Window};
use usage_core::login_flow::{self, Control, LoginStarted};
use usage_core::panel::PanelState;
use usage_core::{account, log, refresh};

use crate::{AppState, publish};

#[tauri::command]
pub fn get_state() -> PanelState {
    PanelState::current()
}

/// Fetches now (ignores the refresh interval).
#[tauri::command]
pub async fn refresh(app: AppHandle) -> Result<PanelState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = refresh::refresh_now() {
            log::error("tray-refresh", &e.to_string());
        }
        publish(&app);
        PanelState::current()
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
    publish(&app);
    Ok(PanelState::current())
}

/// Opens a web page in the default browser. Only http(s) links.
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
pub fn fit_height(window: Window, height: f64) -> Result<(), String> {
    let height = height.clamp(120.0, 720.0);
    window.set_size(tauri::LogicalSize::new(320.0, height)).map_err(|e| e.to_string())?;
    if window.is_visible().unwrap_or(false) {
        crate::place(&window);
    }
    Ok(())
}

#[tauri::command]
pub async fn start_login(app: AppHandle, url: Option<String>) -> Result<LoginStarted, String> {
    tauri::async_runtime::spawn_blocking(move || begin(app, url))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn begin(app: AppHandle, url: Option<String>) -> usage_core::Result<LoginStarted> {
    let pending = login_flow::begin(url.as_deref())?;
    let generation = {
        let state = app.state::<AppState>();
        let mut login = state.login_generation.lock().expect("login state");
        login.0 += 1;
        login.1 = None;
        login.0
    };
    let started = pending.started.clone();
    std::thread::spawn(move || {
        let control = || {
            let state = app.state::<AppState>();
            let login = state.login_generation.lock().expect("login state");
            if login.0 == generation { Control::Continue(login.1.clone()) } else { Control::Stop }
        };
        let emitter = app.clone();
        if let Some(event) = pending.finish(control, |event| {
            let _ = emitter.emit_all("login", event);
        }) {
            let _ = app.emit_all("login", event);
            publish(&app);
        }
    });
    Ok(started)
}

#[tauri::command]
pub fn choose_team(app: AppHandle, team_id: String) {
    if let Ok(mut login) = app.state::<AppState>().login_generation.lock() {
        login.1 = Some(team_id);
    }
}

#[tauri::command]
pub fn cancel_login(app: AppHandle) {
    if let Ok(mut login) = app.state::<AppState>().login_generation.lock() {
        login.0 += 1;
        login.1 = None;
    }
}

/// `litellm-usage --configure --url <LiteLLM> [--status-url <page>]` (used by the installer).
pub fn configure_from_args(args: &[String]) -> i32 {
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str);
    let setup = account::Setup {
        url: flag("--url"),
        status_url: flag("--status-url"),
        ca_cert: flag("--ca-cert").or(flag("--ca-cert-copy")),
        copy_ca: flag("--ca-cert-copy").is_some(),
    };
    match account::configure(setup) {
        Ok(config) => {
            println!("LiteLLM: {}", config.proxy_url.as_deref().unwrap_or("(not set)"));
            println!("Status page: {}", config.status_url.as_deref().unwrap_or("(not set)"));
            println!("Extra CA: {}", config.ca_cert.as_deref().unwrap_or("(none)"));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// Settings view: a new LiteLLM address (the old sign-in belongs to the old proxy, so it ends).
#[tauri::command]
pub fn save_settings(app: AppHandle, url: String) -> Result<PanelState, String> {
    account::configure(account::Setup { url: Some(&url), ..Default::default() }).map_err(|e| e.to_string())?;
    publish(&app);
    refresh_in_background(app);
    Ok(PanelState::current())
}

/// The Cost card exported as a PNG (share button): saved to Downloads; returns the file's path.
#[tauri::command]
pub fn save_image(bytes: Vec<u8>, name: String) -> Result<String, String> {
    usage_core::export::save_png(&bytes, &name).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}
