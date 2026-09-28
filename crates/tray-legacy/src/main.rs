//! Quota by Exodus.Ai tray for Ubuntu 20.04 — the same app as crates/tray, on Tauri 1 (WebKitGTK 4.0,
//! which is all 20.04 has). Same panel UI (crates/tray/ui), same commands and events; the logic
//! lives in `usage-core`, so this is only the Tauri 1 wiring.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, CustomMenuItem, Icon, Manager, PhysicalPosition, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent,
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
    /// Where the tray was clicked (StatusNotifierItem reports it); the panel opens next to it.
    pub anchor: Mutex<Option<(i32, i32)>>,
}

/// Set once the app runs; the StatusNotifierItem icon (started before it) acts through it.
static APP: OnceLock<AppHandle> = OnceLock::new();
/// The StatusNotifierItem icon, when the desktop supports it (left click → panel, right click →
/// menu). Otherwise Tauri 1's AppIndicator icon, which can only show the menu.
static SNI: OnceLock<sni_tray::SniTray> = OnceLock::new();

fn start_sni() -> bool {
    let with_app = |f: fn(&AppHandle)| move || {
        if let Some(app) = APP.get() {
            f(app)
        }
    };
    let actions = sni_tray::Actions {
        open: Box::new(|pos| {
            if let Some(app) = APP.get() {
                toggle(app, pos)
            }
        }),
        refresh: Box::new(with_app(|app| commands::refresh_in_background(app.clone()))),
        settings: Box::new(with_app(show_settings)),
        quit: Box::new(with_app(|app| app.exit(0))),
    };
    match sni_tray::SniTray::spawn(actions, &ring_rgba(None, false, &Default::default()), SIZE, "Quota by Exodus.Ai") {
        Ok(tray) => SNI.set(tray).is_ok(),
        Err(e) => {
            log::error("tray", &format!("no StatusNotifierItem host, using the menu-only icon: {e}"));
            false
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--configure") {
        std::process::exit(commands::configure_from_args(&args[1..]));
    }

    // On a Wayland session, run through XWayland: Wayland doesn't let an app place its own window
    // (the panel goes next to the tray icon) or take focus without a token, which "click away to
    // close" relies on (see sni_tray::present_with_server_time).
    #[cfg(target_os = "linux")]
    if std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("DISPLAY").is_some()
    {
        // SAFETY: first thing in main, before any other thread exists.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }

    let menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open", "Open"))
        .add_item(CustomMenuItem::new("refresh", "Refresh"))
        .add_item(CustomMenuItem::new("settings", "Change LiteLLM URL…"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "Quit"));

    let mut builder = tauri::Builder::default().plugin(tauri_plugin_positioner::init()).manage(AppState::default());
    if !start_sni() {
        builder = builder.system_tray(SystemTray::new().with_menu(menu));
    }
    builder
        .on_system_tray_event(|app, event| {
            tauri_plugin_positioner::on_tray_event(app, &event);
            match event {
                SystemTrayEvent::LeftClick { .. } => toggle(app, None),
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "open" => show(app, None),
                    "refresh" => commands::refresh_in_background(app.clone()),
                    "settings" => show_settings(app),
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
            commands::save_settings,
            commands::save_image,
            commands::save_alerts,
            commands::start_login,
            commands::choose_team,
            commands::cancel_login,
        ])
        .setup(|app| {
            let _ = APP.set(app.handle());
            if std::env::var_os("LITELLM_USAGE_OPEN_PANEL").is_some() {
                show(&app.handle(), None);
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
        .expect("failed to start Quota by Exodus.Ai");
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
    let rgba = ring_rgba(state.used_fraction(), false, &state.alerts);
    if let Some(sni) = SNI.get() {
        sni.update(&rgba, SIZE, &state.tooltip());
    } else {
        let tray = app.tray_handle();
        let _ = tray.set_icon(Icon::Rgba { rgba, width: SIZE, height: SIZE });
        let _ = tray.set_tooltip(&state.tooltip());
    }
    let _ = app.emit_all("state", &state);
}

fn toggle(app: &AppHandle, click: Option<(i32, i32)>) {
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
        show(app, click);
    }
}

pub fn show(app: &AppHandle, click: Option<(i32, i32)>) {
    let Some(window) = app.get_window(PANEL) else { return };
    if let Ok(mut anchor) = app.state::<AppState>().anchor.lock() {
        *anchor = click;
    }
    place(&window);
    let _ = window.show();
    let _ = window.set_focus();
    #[cfg(target_os = "linux")]
    focus_on_x11(&window);
    publish(app);
}

/// GNOME only focuses a window shown from a tray click if it carries a fresh user timestamp; without
/// focus, clicking elsewhere couldn't close the panel. See `sni_tray::present_with_server_time`.
#[cfg(target_os = "linux")]
fn focus_on_x11(window: &tauri::Window) {
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        if let Ok(gtk_window) = w.gtk_window() {
            use gtk::glib::ObjectType;
            // SAFETY: a live GtkWindow, on the GTK main thread.
            unsafe { sni_tray::present_with_server_time(gtk_window.as_ptr().cast()) };
        }
    });
}

/// "Change LiteLLM URL…": open the panel on its settings view.
fn show_settings(app: &AppHandle) {
    show(app, None);
    let _ = app.emit_all("show-settings", ());
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

/// Next to the click when known, else the top right of the screen (Linux trays don't report their
/// position otherwise). Always kept on screen.
pub fn place(window: &tauri::Window) {
    let anchor = window.app_handle().state::<AppState>().anchor.lock().ok().and_then(|a| *a);
    if let Some((x, y)) = anchor {
        if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
            let (mx, my) = (monitor.position().x, monitor.position().y);
            let (mw, mh) = (monitor.size().width as i32, monitor.size().height as i32);
            let (w, h) = (size.width as i32, size.height as i32);
            let left = (x - w / 2).clamp(mx, mx + mw - w);
            let top = if y < my + mh / 2 { y + 8 } else { y - h - 8 };
            if window.set_position(PhysicalPosition::new(left, top.clamp(my, my + mh - h))).is_ok() {
                return;
            }
        }
    }
    let _ = window.move_window(Position::TopRight);
}
