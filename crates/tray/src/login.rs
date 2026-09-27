//! SSO sign-in from the panel: start the device login, open the browser, poll in the background,
//! and report progress to the panel with `login` events.

use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use usage_core::client::Client;
use usage_core::sso::{self, Poll, Team};
use usage_core::{account, log, refresh};

use crate::state::{self, AppState};

const POLL_EVERY: Duration = Duration::from_secs(2);

/// Shared between the panel commands and the polling thread.
#[derive(Default)]
pub struct LoginState {
    /// Bumped on every new login or cancel, so a stale polling thread stops.
    generation: u64,
    team: Option<String>,
}

#[derive(Serialize)]
pub struct LoginStarted {
    proxy_url: String,
    link: String,
    code: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum LoginEvent {
    SelectTeam { teams: Vec<Team> },
    Done,
    Error { message: String },
}

#[tauri::command]
pub async fn start_login(app: AppHandle, url: Option<String>) -> Result<LoginStarted, String> {
    tauri::async_runtime::spawn_blocking(move || begin(app, url))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn begin(app: AppHandle, url: Option<String>) -> usage_core::Result<LoginStarted> {
    let proxy_url = account::resolve_proxy_url(url.as_deref())?;
    let client = Client::new(&proxy_url, None);
    let start = sso::start(&client)?;
    let link = start.verification_url(&proxy_url);
    let _ = open::that_detached(&link);

    let generation = {
        let state = app.state::<AppState>();
        let mut login = state.login.lock().expect("login state");
        login.generation += 1;
        login.team = None;
        login.generation
    };
    let started = LoginStarted { proxy_url: proxy_url.clone(), link, code: start.user_code.clone() };
    std::thread::spawn(move || {
        let event = match poll_until_done(&app, &client, &start, generation) {
            Ok(Some((token, user_id, team_id))) => match account::save_sign_in(&proxy_url, &token, &user_id, team_id) {
                Ok(()) => {
                    let _ = refresh::refresh_now();
                    LoginEvent::Done
                }
                Err(e) => LoginEvent::Error { message: e.to_string() },
            },
            Ok(None) => return, // cancelled or replaced by a newer login
            Err(e) => LoginEvent::Error { message: e.to_string() },
        };
        if let LoginEvent::Error { message } = &event {
            log::error("tray-login", message);
        }
        let _ = app.emit("login", event);
        state::publish(&app);
    });
    Ok(started)
}

type SignIn = (String, String, Option<String>);

fn poll_until_done(app: &AppHandle, client: &Client, start: &sso::Start, generation: u64) -> usage_core::Result<Option<SignIn>> {
    let deadline = Instant::now() + Duration::from_secs(start.expires_in);
    let mut asked_team = false;
    loop {
        let team = {
            let state = app.state::<AppState>();
            let login = state.login.lock().expect("login state");
            if login.generation != generation {
                return Ok(None);
            }
            login.team.clone()
        };
        if Instant::now() > deadline {
            return Err(usage_core::Error::Invalid("Sign-in timed out. Try again.".into()));
        }
        match sso::poll(client, start, team.as_deref())? {
            Poll::Ready { token, user_id, team_id } => return Ok(Some((token, user_id, team_id))),
            Poll::SelectTeam(teams) if !asked_team => {
                asked_team = true;
                let _ = app.emit("login", LoginEvent::SelectTeam { teams });
            }
            Poll::Pending | Poll::SelectTeam(_) => {}
        }
        std::thread::sleep(POLL_EVERY);
    }
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
