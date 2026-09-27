//! Append-only error log (`<cache dir>/litellm-usage/error.log`), trimmed when it grows past 1 MB.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::config::cache_dir;

const MAX_BYTES: u64 = 1_000_000;

pub fn path() -> PathBuf {
    cache_dir().join("error.log")
}

/// Best effort: a failure to log must never take down the caller.
pub fn error(context: &str, message: &str) {
    let path = path();
    let _ = fs::create_dir_all(cache_dir());
    if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = fs::rename(&path, path.with_extension("log.old"));
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%:z");
        let _ = writeln!(f, "{now} [{context}] {message}");
    }
}
