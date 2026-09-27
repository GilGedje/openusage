//! The SSO sign-in as a UI drives it (tray panels): start, poll in the background, report progress.
//! Framework-free; the Tauri 2 tray and the Tauri 1 tray (Ubuntu 20.04) wrap it.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::client::Client;
use crate::sso::{self, Poll, Team};
use crate::{Error, Result, account, refresh};

const POLL_EVERY: Duration = Duration::from_secs(2);

/// Returned to the panel when a sign-in starts.
#[derive(Debug, Clone, Serialize)]
pub struct LoginStarted {
    pub proxy_url: String,
    pub link: String,
    pub code: String,
}

/// Progress events for the panel (`login` event).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum LoginEvent {
    SelectTeam { teams: Vec<Team> },
    Done,
    Error { message: String },
}

/// What the polling thread asks the UI each round.
pub enum Control {
    /// Keep going, with the team the user picked (if any).
    Continue(Option<String>),
    /// The user cancelled or started a newer sign-in.
    Stop,
}

/// A started sign-in, ready to be polled on a background thread.
pub struct Pending {
    client: Client,
    start: sso::Start,
    pub started: LoginStarted,
}

/// Starts the device login and opens the browser.
pub fn begin(url: Option<&str>) -> Result<Pending> {
    let proxy_url = account::resolve_proxy_url(url)?;
    let client = Client::new(&proxy_url, None);
    let start = sso::start(&client)?;
    let link = start.verification_url(&proxy_url);
    let _ = open::that_detached(&link);
    let started = LoginStarted { proxy_url, link, code: start.user_code.clone() };
    Ok(Pending { client, start, started })
}

impl Pending {
    /// Polls until done (blocking — run on a background thread), saves the sign-in, and fetches
    /// fresh numbers. Returns `None` when stopped by `control`.
    pub fn finish(self, mut control: impl FnMut() -> Control, mut on_event: impl FnMut(LoginEvent)) -> Option<LoginEvent> {
        let event = match self.poll(&mut control, &mut on_event) {
            Ok(Some((token, user_id, team_id))) => {
                match account::save_sign_in(&self.started.proxy_url, &token, &user_id, team_id) {
                    Ok(()) => {
                        let _ = refresh::refresh_now();
                        LoginEvent::Done
                    }
                    Err(e) => LoginEvent::Error { message: e.to_string() },
                }
            }
            Ok(None) => return None,
            Err(e) => LoginEvent::Error { message: e.to_string() },
        };
        if let LoginEvent::Error { message } = &event {
            crate::log::error("login", message);
        }
        Some(event)
    }

    fn poll(
        &self,
        control: &mut impl FnMut() -> Control,
        on_event: &mut impl FnMut(LoginEvent),
    ) -> Result<Option<(String, String, Option<String>)>> {
        let deadline = Instant::now() + Duration::from_secs(self.start.expires_in);
        let mut asked_team = false;
        loop {
            let team = match control() {
                Control::Stop => return Ok(None),
                Control::Continue(team) => team,
            };
            if Instant::now() > deadline {
                return Err(Error::Invalid("Sign-in timed out. Try again.".into()));
            }
            match sso::poll(&self.client, &self.start, team.as_deref())? {
                Poll::Ready { token, user_id, team_id } => return Ok(Some((token, user_id, team_id))),
                Poll::SelectTeam(teams) if !asked_team => {
                    asked_team = true;
                    on_event(LoginEvent::SelectTeam { teams });
                }
                Poll::Pending | Poll::SelectTeam(_) => {}
            }
            std::thread::sleep(POLL_EVERY);
        }
    }
}
