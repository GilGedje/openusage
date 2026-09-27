//! The tray icon. Left click toggles the panel; right click shows Open / Refresh / Change LiteLLM
//! URL… / Quit. On Linux the icon speaks StatusNotifierItem directly (`sni-tray`), because
//! AppIndicator — what Tauri uses there — can only show a menu on click; if the desktop has no
//! StatusNotifierItem host, it falls back to Tauri's icon (menu only).

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle};

use crate::{commands, gauge, panel, state};

pub fn install(app: &App) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    if install_sni(app.handle()) {
        return Ok(());
    }
    install_tauri(app)
}

fn install_tauri(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Change LiteLLM URL…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu =
        Menu::with_items(app, &[&open, &refresh, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id(state::TRAY_ID)
        .icon(gauge::icon(None, &Default::default()))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Quota by Exodus.Ai")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => panel::show(app, None),
            "refresh" => commands::refresh_in_background(app.clone()),
            "settings" => panel::show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                panel::toggle(tray.app_handle(), None);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_sni(app: &AppHandle) -> bool {
    use tauri::Manager;
    use usage_core::gauge::{SIZE, ring_rgba};

    let (a, b, c, d) = (app.clone(), app.clone(), app.clone(), app.clone());
    let actions = sni_tray::Actions {
        open: Box::new(move |pos| panel::toggle(&a, pos)),
        refresh: Box::new(move || commands::refresh_in_background(b.clone())),
        settings: Box::new(move || panel::show_settings(&c)),
        quit: Box::new(move || d.exit(0)),
    };
    match sni_tray::SniTray::spawn(actions, &ring_rgba(None, false, &Default::default()), SIZE, "Quota by Exodus.Ai") {
        Ok(tray) => {
            if let Ok(mut slot) = app.state::<state::AppState>().sni.lock() {
                *slot = Some(tray);
            }
            true
        }
        Err(e) => {
            usage_core::log::error("tray", &format!("no StatusNotifierItem host, using the menu-only icon: {e}"));
            false
        }
    }
}

/// Updates whichever icon is in use.
pub fn update(app: &AppHandle, fraction: Option<f64>, tooltip: &str, alerts: &usage_core::alerts::Alerts) {
    if let Some(tray) = app.tray_by_id(state::TRAY_ID) {
        let _ = tray.set_icon(Some(gauge::icon(fraction, alerts)));
        let _ = tray.set_icon_as_template(cfg!(target_os = "macos"));
        #[cfg(not(target_os = "windows"))]
        let _ = tray.set_title(fraction.map(|f| format!("{:.0}%", f * 100.0)).as_deref());
        let _ = tray.set_tooltip(Some(tooltip));
    }
    #[cfg(target_os = "linux")]
    {
        use tauri::Manager;
        use usage_core::gauge::{SIZE, ring_rgba};
        if let Ok(slot) = app.state::<state::AppState>().sni.lock()
            && let Some(tray) = slot.as_ref()
        {
            tray.update(&ring_rgba(fraction, false, alerts), SIZE, tooltip);
        }
    }
}
