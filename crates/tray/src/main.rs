//! Quota by Exodus.Ai — tray app: a tray icon showing budget used, and a popup panel with the details.
//! Data, sign-in and the cache come from `usage-core`, shared with `ccline`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod gauge;
mod login;
mod panel;
mod state;
mod tray_icon;

use std::time::Duration;

use tauri::Manager;

use crate::state::AppState;

/// How often the background loop checks whether the shared cache is due for a refresh.
const TICK: Duration = Duration::from_secs(15);

fn main() {
    // Installer hook: `litellm-usage --configure --url <LiteLLM> [--status-url <page>]` saves the
    // addresses and exits without starting the app.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--configure") {
        std::process::exit(configure(&args[1..]));
    }

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
            commands::save_settings,
            commands::save_image,
            login::start_login,
            login::choose_team,
            login::cancel_login,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray_icon::install(app)?;

            // For testing: open the panel right away (`pin` also keeps it open without focus).
            if let Some(mode) = std::env::var_os("LITELLM_USAGE_OPEN_PANEL") {
                if mode == "pin" {
                    app.state::<AppState>().pinned.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                panel::show(app.handle(), None);
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
            let pinned = window.app_handle().state::<AppState>().pinned.load(std::sync::atomic::Ordering::Relaxed);
            if let tauri::WindowEvent::Focused(false) = event
                && !pinned
            {
                panel::hide(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start Quota by Exodus.Ai");
}

fn configure(args: &[String]) -> i32 {
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str);
    let setup = usage_core::account::Setup {
        url: flag("--url"),
        status_url: flag("--status-url"),
        ca_cert: flag("--ca-cert").or(flag("--ca-cert-copy")),
        copy_ca: flag("--ca-cert-copy").is_some(),
    };
    match usage_core::account::configure(setup) {
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
