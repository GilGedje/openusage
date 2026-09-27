//! The SSO token, kept in the OS secure storage: Keychain on macOS, Credential Manager on Windows,
//! Secret Service (GNOME Keyring / KWallet) on Ubuntu. Never written to a plain file.
//!
//! Windows caps one credential at 2,560 bytes and LiteLLM tokens grow with the user's team and
//! model lists, so the token is stored as UTF-8 bytes split across numbered entries:
//! `<proxy url>` holds `<part count>\n<first part>`, then `<proxy url>#2`, `<proxy url>#3`, …

use crate::{Error, Result};

const SERVICE: &str = "litellm-usage";
/// Comfortably under Windows' 2,560-byte credential limit (tokens are ASCII).
const PART_BYTES: usize = 2_000;
/// Refuse absurd sizes rather than writing dozens of entries.
const MAX_PARTS: usize = 16;

fn entry(proxy_url: &str, part: usize) -> Result<keyring::Entry> {
    let user = if part == 1 { proxy_url.to_string() } else { format!("{proxy_url}#{part}") };
    keyring::Entry::new(SERVICE, &user).map_err(keyring_error)
}

fn keyring_error(e: keyring::Error) -> Error {
    Error::Keyring(e.to_string())
}

pub fn save(proxy_url: &str, token: &str) -> Result<()> {
    let parts = split(token);
    if parts.len() > MAX_PARTS {
        return Err(Error::Keyring(format!("token is too large to store ({} bytes)", token.len())));
    }
    delete(proxy_url)?;
    for (i, part) in parts.iter().enumerate() {
        let payload = if i == 0 { format!("{}\n{part}", parts.len()) } else { part.to_string() };
        entry(proxy_url, i + 1)?.set_secret(payload.as_bytes()).map_err(keyring_error)?;
    }
    Ok(())
}

/// `Ok(None)` when there is no saved sign-in for this proxy.
pub fn load(proxy_url: &str) -> Result<Option<String>> {
    let first = match entry(proxy_url, 1)?.get_secret() {
        Ok(bytes) => bytes,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => return Err(keyring_error(e)),
    };
    let first = String::from_utf8(first).map_err(|_| Error::Keyring("saved sign-in is corrupted".into()))?;
    let (count, mut token) = parse_first(&first)?;
    for part in 2..=count {
        let bytes = entry(proxy_url, part)?.get_secret().map_err(keyring_error)?;
        token += &String::from_utf8(bytes).map_err(|_| Error::Keyring("saved sign-in is corrupted".into()))?;
    }
    Ok(Some(token))
}

pub fn delete(proxy_url: &str) -> Result<()> {
    for part in 1..=MAX_PARTS {
        match entry(proxy_url, part)?.delete_credential() {
            Ok(()) => {}
            Err(keyring::Error::NoEntry) => break,
            Err(e) => return Err(keyring_error(e)),
        }
    }
    Ok(())
}

fn split(token: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = token;
    while !rest.is_empty() {
        let mut end = rest.len().min(PART_BYTES);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (part, tail) = rest.split_at(end);
        parts.push(part);
        rest = tail;
    }
    if parts.is_empty() {
        parts.push("");
    }
    parts
}

fn parse_first(first: &str) -> Result<(usize, String)> {
    let (count, part) = first.split_once('\n').ok_or_else(|| Error::Keyring("saved sign-in is corrupted".into()))?;
    let count: usize = count.parse().map_err(|_| Error::Keyring("saved sign-in is corrupted".into()))?;
    if !(1..=MAX_PARTS).contains(&count) {
        return Err(Error::Keyring("saved sign-in is corrupted".into()));
    }
    Ok((count, part.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_rejoins() {
        let token = "x".repeat(PART_BYTES * 2 + 7);
        let parts = split(&token);
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.len() <= PART_BYTES));
        assert_eq!(parts.concat(), token);
        assert_eq!(split("abc"), ["abc"]);
    }

    #[test]
    fn parses_first_part() {
        assert_eq!(parse_first("2\nabc").unwrap(), (2, "abc".to_string()));
        assert!(parse_first("abc").is_err());
        assert!(parse_first("0\nabc").is_err());
        assert!(parse_first("99\nabc").is_err());
    }
}
