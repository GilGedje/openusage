//! `ccline setup`: used by the installer. Saves the LiteLLM and status page addresses, and points
//! Claude Code's status line at this binary (backing up settings.json and keeping every other key).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use usage_core::config::{Config, normalize_url};
use usage_core::{Error, Result};

pub struct Options {
    pub url: Option<String>,
    pub status_url: Option<String>,
    /// Replace a status line that isn't ccline's.
    pub force: bool,
    /// Only save the addresses; leave Claude Code's settings alone.
    pub no_statusline: bool,
}

pub fn run(opts: Options) -> Result<()> {
    let mut config = Config::load()?;
    if let Some(url) = &opts.url {
        let url = normalize_url(url)?;
        if config.proxy_url.as_deref() != Some(url.as_str()) {
            // A different proxy: the old sign-in doesn't apply there.
            config.user_id = None;
            config.team_id = None;
            config.signed_in_at = None;
        }
        config.proxy_url = Some(url);
    }
    if let Some(status) = &opts.status_url {
        let status = status.trim().trim_end_matches('/').to_string();
        if !(status.starts_with("https://") || status.starts_with("http://")) {
            return Err(Error::Invalid(format!("Status page must start with http:// or https:// (got `{status}`)")));
        }
        config.status_url = Some(status);
    }
    config.save()?;
    println!("LiteLLM: {}", config.proxy_url.as_deref().unwrap_or("(not set)"));
    println!("Status page: {}", config.status_url.as_deref().unwrap_or("(not set)"));

    if !opts.no_statusline {
        let settings = claude_settings_path();
        let command = command_path(&std::env::current_exe()?);
        install_statusline(&settings, &command, opts.force)?;
        println!("Claude Code status line: {} → {command}", settings.display());
    }
    Ok(())
}

/// `$CLAUDE_CONFIG_DIR/settings.json`, else `~/.claude/settings.json`.
pub fn claude_settings_path() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .unwrap_or_else(|| PathBuf::from(".claude"))
        .join("settings.json")
}

/// Claude Code runs the status line through a shell (Git Bash or PowerShell on Windows), where
/// backslashes get eaten — so always use forward slashes.
fn command_path(exe: &Path) -> String {
    exe.to_string_lossy().replace('\\', "/")
}

fn install_statusline(settings: &Path, command: &str, force: bool) -> Result<()> {
    let existing = match fs::read_to_string(settings) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let updated = merge_statusline(existing.as_deref(), command, force)?;
    if let Some(dir) = settings.parent() {
        fs::create_dir_all(dir)?;
    }
    if existing.is_some() {
        fs::copy(settings, settings.with_extension("json.bak-ccline"))?;
    }
    let tmp = settings.with_extension("json.tmp-ccline");
    fs::write(&tmp, updated)?;
    fs::rename(&tmp, settings)?;
    Ok(())
}

/// The new settings.json text: `statusLine` set to ccline, everything else untouched.
pub fn merge_statusline(existing: Option<&str>, command: &str, force: bool) -> Result<String> {
    let mut root: Map<String, Value> = match existing.map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => match serde_json::from_str(text) {
            Ok(Value::Object(map)) => map,
            _ => return Err(Error::Invalid("Claude Code's settings.json isn't a JSON object; fix it first.".into())),
        },
        None => Map::new(),
    };
    if let Some(current) = root.get("statusLine") {
        let theirs = current.get("command").and_then(Value::as_str).unwrap_or("");
        if !theirs.contains("ccline") && !force {
            return Err(Error::Invalid(format!(
                "Claude Code already has a status line (`{theirs}`). Re-run with --force to replace it."
            )));
        }
    }
    root.insert("statusLine".into(), json!({ "type": "command", "command": command, "refreshInterval": 30 }));
    Ok(serde_json::to_string_pretty(&Value::Object(root))? + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_settings_when_missing() {
        let out = merge_statusline(None, "/home/u/.local/bin/ccline", false).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["statusLine"]["command"], "/home/u/.local/bin/ccline");
        assert_eq!(v["statusLine"]["refreshInterval"], 30);
    }

    #[test]
    fn keeps_other_keys_in_order() {
        let before = r#"{"model":"opus","env":{"A":"1"},"permissions":{"allow":[]}}"#;
        let out = merge_statusline(Some(before), "ccline", false).unwrap();
        let keys: Vec<String> = serde_json::from_str::<Map<String, Value>>(&out).unwrap().keys().cloned().collect();
        assert_eq!(keys, ["model", "env", "permissions", "statusLine"]);
    }

    #[test]
    fn refuses_to_replace_someone_elses_status_line() {
        let before = r#"{"statusLine":{"type":"command","command":"~/my-line.sh"}}"#;
        assert!(merge_statusline(Some(before), "ccline", false).is_err());
        assert!(merge_statusline(Some(before), "ccline", true).is_ok());
        let ours = r#"{"statusLine":{"type":"command","command":"/old/ccline"}}"#;
        assert!(merge_statusline(Some(ours), "/new/ccline", false).is_ok());
    }

    #[test]
    fn rejects_broken_settings() {
        assert!(merge_statusline(Some("[1,2]"), "ccline", false).is_err());
        assert!(merge_statusline(Some("{oops"), "ccline", false).is_err());
    }

    #[test]
    fn windows_paths_use_forward_slashes() {
        assert_eq!(command_path(Path::new(r"C:\Users\u\bin\ccline.exe")), "C:/Users/u/bin/ccline.exe");
    }
}
