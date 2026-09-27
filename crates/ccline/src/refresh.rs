//! Background refresh: fetch a new snapshot and write it to the cache. Runs as a detached
//! `ccline refresh` process so the status line itself stays instant.

use std::process::{Command, Stdio};

use usage_core::cache::{self, CacheFile, CachedError, RefreshLock};
use usage_core::client::Client;
use usage_core::config::Config;
use usage_core::snapshot::{self, Snapshot};
use usage_core::{Error, Result, log, secret};

/// Minimum seconds between refresh attempts (successful or failed).
pub const REFRESH_SECS: i64 = 60;

pub fn spawn_detached() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return log::error("spawn", &e.to_string()),
    };
    let mut cmd = Command::new(exe);
    cmd.arg("refresh").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Own process group / no console, so Claude Code cancelling the status line doesn't kill it.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    if let Err(e) = cmd.spawn() {
        log::error("spawn", &e.to_string());
    }
}

/// `ccline refresh`: one refresh, unless another process is already refreshing.
pub fn run() {
    let lock = match RefreshLock::try_acquire() {
        Ok(Some(lock)) => lock,
        Ok(None) => return,
        Err(e) => return log::error("refresh", &e.to_string()),
    };
    let _ = refresh_now();
    drop(lock);
}

/// Fetches and caches a snapshot; on failure keeps the previous snapshot and records the error.
pub fn refresh_now() -> Result<Snapshot> {
    let now = usage_core::now();
    let proxy_url = Config::load()?.proxy_url.ok_or(Error::NotConfigured)?;
    let previous = cache::read().filter(|c| c.proxy_url == proxy_url);
    let result = fetch(&proxy_url);

    let mut file = CacheFile { proxy_url: proxy_url.clone(), last_attempt: now, ..Default::default() };
    match &result {
        Ok(snap) => file.snapshot = Some(snap.clone()),
        Err(e) => {
            log::error("refresh", &e.to_string());
            file.snapshot = previous.and_then(|p| p.snapshot);
            file.error = Some(CachedError::from_error(e, now));
        }
    }
    if let Err(e) = cache::write(&file) {
        log::error("cache", &e.to_string());
    }
    result
}

fn fetch(proxy_url: &str) -> Result<Snapshot> {
    let token = secret::load(proxy_url)?.ok_or(Error::SignedOut)?;
    let client = Client::new(proxy_url, Some(&token));
    snapshot::fetch(&client)
}
