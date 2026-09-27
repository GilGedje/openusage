//! `ccline login`, `logout`, and `status`.

use std::io::{BufRead, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};

use usage_core::client::Client;
use usage_core::config::{Config, normalize_url};
use usage_core::secret;
use usage_core::sso::{self, Poll, Team};
use usage_core::{Error, Result, cache};

use crate::{refresh, render};

const POLL_EVERY: Duration = Duration::from_secs(2);

pub fn login(url_flag: Option<String>) -> Result<()> {
    let raw = url_flag
        .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok().filter(|s| !s.trim().is_empty()))
        .or(Config::load()?.proxy_url)
        .ok_or(Error::NotConfigured)?;
    let proxy_url = normalize_url(&raw)?;
    let client = Client::new(&proxy_url, None);

    let start = sso::start(&client)?;
    let link = start.verification_url(&proxy_url);
    println!("Signing in to {proxy_url}\n");
    println!("  1. Open:  {link}");
    println!("  2. Enter this code: {}\n", start.user_code);
    if open::that_detached(&link).is_err() {
        println!("(Couldn't open the browser automatically — open the link above.)");
    }
    println!("Waiting for you to finish in the browser…");

    let deadline = Instant::now() + Duration::from_secs(start.expires_in);
    let mut team: Option<String> = None;
    let (token, user_id, team_id) = loop {
        if Instant::now() > deadline {
            return Err(Error::Invalid("Sign-in timed out. Run `ccline login` again.".into()));
        }
        match sso::poll(&client, &start, team.as_deref())? {
            Poll::Pending => sleep(POLL_EVERY),
            Poll::SelectTeam(teams) => team = Some(pick_team(&teams)?),
            Poll::Ready { token, user_id, team_id } => break (token, user_id, team_id),
        }
    };

    secret::save(&proxy_url, &token)?;
    Config {
        proxy_url: Some(proxy_url),
        user_id: Some(user_id.clone()),
        team_id,
        signed_in_at: Some(usage_core::now()),
    }
    .save()?;
    cache::clear()?;
    println!("\nSigned in as {user_id}.\n");
    status()
}

fn pick_team(teams: &[Team]) -> Result<String> {
    println!("\nYou're in several teams. Pick the one Claude Code uses:");
    for (i, t) in teams.iter().enumerate() {
        println!("  {}. {}", i + 1, t.team_alias.as_deref().unwrap_or(&t.team_id));
    }
    loop {
        print!("Team number: ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line)? == 0 {
            return Err(Error::Invalid("No team selected.".into()));
        }
        if let Some(t) = line.trim().parse::<usize>().ok().and_then(|n| teams.get(n.wrapping_sub(1))) {
            return Ok(t.team_id.clone());
        }
    }
}

pub fn logout() -> Result<()> {
    let config = Config::load()?;
    if let Some(url) = &config.proxy_url {
        secret::delete(url)?;
    }
    // Keep the proxy URL so `ccline login` can sign in again without `--url`.
    Config { proxy_url: config.proxy_url, ..Config::default() }.save()?;
    cache::clear()?;
    println!("Signed out.");
    Ok(())
}

pub fn status() -> Result<()> {
    let snap = refresh::refresh_now()?;
    print!("{}", render::report(&snap, usage_core::now()));
    Ok(())
}
