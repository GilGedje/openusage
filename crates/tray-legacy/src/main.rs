//! LiteLLM Usage tray for Ubuntu 20.04 — the same app as crates/tray, on Tauri 1 (WebKitGTK 4.0,
//! which is all 20.04 has). Same panel UI (crates/tray/ui), same commands and events; the logic
//! lives in `usage-core`, so this is only the Tauri 1 wiring.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, CustomMenuItem, Icon, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use usage_core::gauge::{SIZE, ring_rgba};
use usage_core::panel::PanelState;
use usage_core::{log, refresh};

const PANEL: &str = "panel";
/// How often the background loop checks whether the shared cache is due for a refresh.
const TICK: Duration = Duration::from_secs(15);
/// A tray click first blurs (hides) an open panel; ignore the click that follows so it stays shut.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct AppState {
    /// Bumped on every new sign-in or cancel, so a stale polling thread stops.
    pub login_generation: Mutex<(u64, Option<String>)>,
    pub last_hidden: Mutex<Option<Instant>>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--configure") {
        std::process::exit(commands::configure_from_args(&args[1..]));
    }

    let menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("refresh", "Refresh"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "Quit"));

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .system_tray(SystemTray::new().with_menu(menu))
        .on_system_tray_event(|app, event| {
            tauri_plugin_positioner::on_tray_event(app, &event);
            match event {
                SystemTrayEvent::LeftClick { .. } => toggle(app),
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "open" => show(app),
                    "refresh" => commands::refresh_in_background(app.clone()),
                    "quit" => app.exit(0),
                    _ => {}
                },
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::refresh,
            commands::sign_out,
            commands::open_url,
            commands::quit,
            commands::fit_height,
            commands::start_login,
            commands::choose_team,
            commands::cancel_login,
        ])
        .setup(|app| {
            if std::env::var_os("LITELLM_USAGE_OPEN_PANEL").is_some() {
                show(&app.handle());
            }
            let handle = app.handle();
            std::thread::spawn(move || {
                loop {
                    tick(&handle);
                    std::thread::sleep(TICK);
                }
            });
            Ok(())
        })
        .on_window_event(|event| {
            if let WindowEvent::Focused(false) = event.event() {
                hide(&event.window().app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start LiteLLM Usage");
}

/// Refreshes when due, then pushes the latest state to the panel and tray.
fn tick(app: &AppHandle) {
    let state = PanelState::current();
    if state.signed_in && refresh::is_due(state.cache.as_ref(), state.now) {
        if let Err(e) = refresh::refresh_if_free() {
            log::error("tray-refresh", &e.to_string());
        }
    }
    publish(app);
}

/// Sends the current state to the panel and updates the tray icon and tooltip.
pub fn publish(app: &AppHandle) {
    let state = PanelState::current();
    let tray = app.tray_handle();
    let _ = tray.set_icon(Icon::Rgba { rgba: ring_rgba(state.used_fraction(), false), width: SIZE, height: SIZE });
    let _ = tray.set_tooltip(&state.tooltip());
    let _ = app.emit_all("state", &state);
}

fn toggle(app: &AppHandle) {
    let Some(window) = app.get_window(PANEL) else { return };
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
    let Some(window) = app.get_window(PANEL) else { return };
    place(&window);
    let _ = window.show();
    let _ = window.set_focus();
    publish(app);
}

fn hide(app: &AppHandle) {
    let Some(window) = app.get_window(PANEL) else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        if let Ok(mut t) = app.state::<AppState>().last_hidden.lock() {
            *t = Some(Instant::now());
        }
    }
}

/// Linux trays don't report their position, so the panel opens at the top right.
pub fn place(window: &tauri::Window) {
    let _ = window.move_window(Position::TopRight);
}
