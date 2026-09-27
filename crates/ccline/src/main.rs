//! `ccline` — Claude Code status line showing your LiteLLM budget and usage.
//!
//! With no arguments it's the status line: prints from the local cache instantly and, when the
//! cache is older than `REFRESH_SECS`, starts a detached `ccline refresh` for next time. It never
//! touches the network or secure storage while rendering (Claude Code cancels slow status lines).

mod account;
mod render;
mod session;
mod setup;
mod spawn;

use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use usage_core::config::Config;
use usage_core::{cache, log, refresh};

const REFRESH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

const HELP: &str = "\
ccline — Claude Code status line for your LiteLLM budget and usage

USAGE:
  ccline                    Print the status line (Claude Code runs this)
  ccline login [--url URL]  Sign in with SSO (URL defaults to ANTHROPIC_BASE_URL)
  ccline setup [--url URL] [--status-url URL] [--ca-cert FILE | --ca-cert-copy FILE]
               [--force] [--no-statusline]
                            Save the LiteLLM / status page addresses and set up
                            Claude Code's status line (used by the installer)
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
        Some("login") => account::login(flag_value(&args[1..], "--url")),
        Some("logout") => account::logout(),
        Some("setup") => setup::run(setup::Options {
            url: flag_value(&args[1..], "--url"),
            status_url: flag_value(&args[1..], "--status-url"),
            ca_cert: flag_value(&args[1..], "--ca-cert").or_else(|| flag_value(&args[1..], "--ca-cert-copy")),
            copy_ca: flag_value(&args[1..], "--ca-cert-copy").is_some(),
            force: args.iter().any(|a| a == "--force"),
            no_statusline: args.iter().any(|a| a == "--no-statusline"),
        }),
        Some("status") => account::status(),
        Some("refresh") => background_refresh(),
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

/// Background refresh started by the status line: never prompts, and never outlives
/// `REFRESH_TIMEOUT` (it's under the refresh lock's stale age, so a stuck process can't pile up).
fn background_refresh() -> usage_core::Result<()> {
    usage_core::secret::disallow_prompts();
    std::thread::spawn(|| {
        std::thread::sleep(REFRESH_TIMEOUT);
        log::error("refresh", "timed out; giving up");
        std::process::exit(1);
    });
    refresh::refresh_if_free().map(|_| ())
}

/// `--name value` or `--name=value`.
fn flag_value(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
        if let Some(v) = a.strip_prefix(&prefix) {
            return Some(v.to_string());
        }
    }
    None
}

fn status_line() {
    let session = read_stdin().map(|v| session::Session::from_input(&v)).unwrap_or_default();
    let columns = std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()).unwrap_or(120);
    let now = usage_core::now();
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            log::error("config", &e.to_string());
            println!("{}", render::line(&session, render::Usage::Failed("settings file unreadable"), now, columns));
            return;
        }
    };
    let Some(proxy_url) = config.proxy_url else {
        println!("{}", render::line(&session, render::Usage::NotConfigured, now, columns));
        return;
    };
    let cached = cache::read().filter(|c| c.proxy_url == proxy_url);
    if refresh::is_due(cached.as_ref(), now) {
        spawn::refresh_detached();
    }
    println!("{}", render::line(&session, render::Usage::Cached(cached.as_ref()), now, columns));
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
