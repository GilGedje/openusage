//! Saving an image the panel exported (the Cost card's share button).

use std::fs;
use std::path::PathBuf;

use crate::{Error, Result};

/// Writes a PNG to the Downloads folder (home folder if there is none) without overwriting an
/// existing file, and returns where it went.
pub fn save_png(bytes: &[u8], name: &str) -> Result<PathBuf> {
    if !bytes.starts_with(b"\x89PNG") {
        return Err(Error::Invalid("Not a PNG image.".into()));
    }
    let dir = dirs::download_dir().or_else(dirs::home_dir).ok_or_else(|| Error::Io("no Downloads folder".into()))?;
    fs::create_dir_all(&dir)?;
    let stem: String = name.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
    let stem = stem.trim().trim_end_matches(".png");
    let mut path = dir.join(format!("{stem}.png"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).png"));
        n += 1;
    }
    fs::write(&path, bytes)?;
    Ok(path)
}
