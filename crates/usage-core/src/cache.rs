//! The last snapshot and last refresh error, on disk, so the status line can print instantly
//! without touching the network or secure storage. Holds no secrets.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::config::cache_dir;
use crate::snapshot::Snapshot;
use crate::{Error, Result};

/// A refresh lock older than this is treated as abandoned (the refresher crashed or was killed).
const STALE_LOCK: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachedError {
    pub kind: String,
    pub message: String,
    /// Unix seconds.
    pub at: i64,
}

impl CachedError {
    pub fn from_error(e: &Error, at: i64) -> CachedError {
        CachedError { kind: e.kind().to_string(), message: e.to_string(), at }
    }

    pub fn is_signed_out(&self) -> bool {
        self.kind == "signed_out"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CacheFile {
    /// Proxy the snapshot came from; a cache for another proxy is ignored.
    pub proxy_url: String,
    pub snapshot: Option<Snapshot>,
    /// Set when the latest refresh failed; the older snapshot (if any) is kept.
    pub error: Option<CachedError>,
    /// Unix seconds of the latest refresh attempt, successful or not.
    pub last_attempt: i64,
}

fn path() -> PathBuf {
    cache_dir().join("usage.json")
}

/// `None` when there's no cache yet or it can't be read (it's rebuilt on the next refresh).
pub fn read() -> Option<CacheFile> {
    fs::read(path()).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

/// Atomic write: temp file, then rename over the old one.
pub fn write(cache: &CacheFile) -> Result<()> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!("usage.json.{}.tmp", std::process::id()));
    fs::write(&tmp, serde_json::to_vec(cache)?)?;
    fs::rename(&tmp, path())?;
    Ok(())
}

pub fn clear() -> Result<()> {
    match fs::remove_file(path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Held while a refresh runs so concurrent Claude Code sessions don't all hit the proxy at once.
pub struct RefreshLock {
    path: PathBuf,
}

impl RefreshLock {
    /// `None` when another refresh is already running.
    pub fn try_acquire() -> Result<Option<RefreshLock>> {
        let dir = cache_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join("refresh.lock");
        for _ in 0..2 {
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Some(RefreshLock { path })),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let age = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok());
                    if age.is_some_and(|a| a > STALE_LOCK) {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(None)
    }
}

impl Drop for RefreshLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
