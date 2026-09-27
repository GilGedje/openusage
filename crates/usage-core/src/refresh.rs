//! Fetch a new snapshot and write it to the shared cache. Used by the `ccline refresh` background
//! process and the tray app; both read the same cache, so they never fetch twice.

use crate::cache::{self, CacheFile, CachedError, RefreshLock};
use crate::client::Client;
use crate::config::Config;
use crate::snapshot::{self, Snapshot};
use crate::{Error, Result, log, secret};

/// Minimum seconds between refresh attempts (successful or failed).
pub const REFRESH_SECS: i64 = 60;

/// Whether the cache is due for a refresh.
pub fn is_due(cache: Option<&CacheFile>, now: i64) -> bool {
    cache.is_none_or(|c| now - c.last_attempt >= REFRESH_SECS)
}

/// One refresh, unless another process is already refreshing (then `Ok(false)`).
pub fn refresh_if_free() -> Result<bool> {
    let Some(lock) = RefreshLock::try_acquire()? else {
        return Ok(false);
    };
    let _ = refresh_now();
    drop(lock);
    Ok(true)
}

/// Fetches and caches a snapshot; on failure keeps the previous snapshot and records the error.
pub fn refresh_now() -> Result<Snapshot> {
    let now = crate::now();
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
