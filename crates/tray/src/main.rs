//! LiteLLM Usage tray app: a tray icon showing budget used, and a popup panel with the details.
//! Data, sign-in and the cache come from `usage-core`, shared with `ccline`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod gauge;
mod login;
mod panel;
mod state;

use std::time::Duration;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::state::AppState;

/// How often the background loop checks whether the shared cache is due for a refresh.
const TICK: Duration = Duration::from_secs(15);

fn main() {
    // WebKitGTK's DMA-BUF renderer shows a blank window in many VMs and with some GPU drivers.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: first thing in main, before any other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::refresh,
            commands::sign_out,
            commands::open_url,
            commands::quit,
            commands::fit_height,
            login::start_login,
            login::choose_team,
            login::cancel_login,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &refresh, &PredefinedMenuItem::separator(app)?, &quit])?;

            TrayIconBuilder::with_id(state::TRAY_ID)
                .icon(gauge::icon(None))
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("LiteLLM Usage")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => panel::show(app),
                    "refresh" => commands::refresh_in_background(app.clone()),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } =
                        event
                    {
                        panel::toggle(tray.app_handle());
                    }
                })
                .build(app)?;

            // For testing: open the panel right away.
            if std::env::var_os("LITELLM_USAGE_OPEN_PANEL").is_some() {
                panel::show(app.handle());
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    state::tick(&handle);
                    std::thread::sleep(TICK);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                panel::hide(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start LiteLLM Usage");
}
