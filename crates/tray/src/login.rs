//! Sign-in commands for the panel; the flow itself is `usage_core::login_flow`.

use tauri::{AppHandle, Emitter, Manager};
use usage_core::login_flow::{self, Control, LoginStarted};

use crate::state::{self, AppState};

/// Shared between the panel commands and the polling thread.
#[derive(Default)]
pub struct LoginState {
    /// Bumped on every new login or cancel, so a stale polling thread stops.
    generation: u64,
    team: Option<String>,
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
        let mut login = state.login.lock().expect("login state");
        login.generation += 1;
        login.team = None;
        login.generation
    };
    let started = pending.started.clone();
    std::thread::spawn(move || {
        let control = || {
            let state = app.state::<AppState>();
            let login = state.login.lock().expect("login state");
            if login.generation == generation { Control::Continue(login.team.clone()) } else { Control::Stop }
        };
        let emitter = app.clone();
        if let Some(event) = pending.finish(control, |event| {
            let _ = emitter.emit("login", event);
        }) {
            let _ = app.emit("login", event);
            state::publish(&app);
        }
    });
    Ok(started)
}

#[tauri::command]
pub fn choose_team(app: AppHandle, team_id: String) {
    if let Ok(mut login) = app.state::<AppState>().login.lock() {
        login.team = Some(team_id);
    }
}

#[tauri::command]
pub fn cancel_login(app: AppHandle) {
    if let Ok(mut login) = app.state::<AppState>().login.lock() {
        login.generation += 1;
        login.team = None;
    }
}
