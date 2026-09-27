//! `ccline` — Claude Code status line showing your LiteLLM budget and usage.
//!
//! With no arguments it's the status line: prints from the local cache instantly and, when the
//! cache is older than `REFRESH_SECS`, starts a detached `ccline refresh` for next time. It never
//! touches the network or secure storage while rendering (Claude Code cancels slow status lines).

mod account;
mod refresh;
mod render;

use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use usage_core::config::Config;
use usage_core::{cache, log};

const HELP: &str = "\
ccline — Claude Code status line for your LiteLLM budget and usage

USAGE:
  ccline                    Print the status line (Claude Code runs this)
  ccline login [--url URL]  Sign in with SSO (URL defaults to ANTHROPIC_BASE_URL)
  ccline logout             Sign out and forget the saved sign-in
  ccline status             Fetch now and show everything
  ccline refresh            Refresh the cache (used internally)
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => {
            status_line();
            Ok(())
        }
        Some("login") => account::login(url_flag(&args[1..])),
        Some("logout") => account::logout(),
        Some("status") => account::status(),
        Some("refresh") => {
            refresh::run();
            Ok(())
        }
        Some("-h" | "--help" | "help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("-V" | "--version") => {
            println!("ccline {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(other) => Err(usage_core::Error::Invalid(format!("Unknown command `{other}`.\n\n{HELP}"))),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn url_flag(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--url" {
            return it.next().cloned();
        }
        if let Some(v) = a.strip_prefix("--url=") {
            return Some(v.to_string());
        }
    }
    None
}

fn status_line() {
    let input = read_stdin();
    let columns = std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()).unwrap_or(120);
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            log::error("config", &e.to_string());
            println!("{}", render::message(&e.to_string(), columns));
            return;
        }
    };
    let Some(proxy_url) = config.proxy_url else {
        println!("{}", render::not_configured(columns));
        return;
    };
    let cached = cache::read().filter(|c| c.proxy_url == proxy_url);
    let now = usage_core::now();
    if cached.as_ref().is_none_or(|c| now - c.last_attempt >= refresh::REFRESH_SECS) {
        refresh::spawn_detached();
    }
    let model = input.as_ref().and_then(render::current_model);
    println!("{}", render::line(cached.as_ref(), model.as_deref(), now, columns));
}

/// Claude Code pipes session JSON on stdin; skip reading when run by hand in a terminal.
fn read_stdin() -> Option<serde_json::Value> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return None;
    }
    let mut buf = String::new();
    stdin.lock().read_to_string(&mut buf).ok()?;
    serde_json::from_str(&buf).ok()
}
