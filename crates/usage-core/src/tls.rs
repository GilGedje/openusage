//! Which certificate authorities the LiteLLM connection trusts. Air-gapped networks use their own
//! CA, so we trust the machine's certificate store (where IT installs it) plus an optional custom CA
//! file (PEM, one or more certificates). Verification is never switched off.
//!
//! The custom CA file is, first match wins: `LITELLM_USAGE_CA_CERT`, the `ca_cert` path in the
//! settings (set by the installer's `CA_CERT`), or `ca.pem` next to the settings.

use std::fs;
use std::path::{Path, PathBuf};

use ureq::tls::{Certificate, PemItem, RootCerts, TlsConfig, parse_pem};

use crate::config::{Config, config_dir};
use crate::{Error, Result, log};

/// The custom CA file to trust, if any (see the module docs for the order).
pub fn ca_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("LITELLM_USAGE_CA_CERT").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    if let Some(p) = Config::load().ok().and_then(|c| c.ca_cert) {
        return Some(PathBuf::from(p));
    }
    Some(config_dir().join("ca.pem")).filter(|p| p.exists())
}

/// Roots for the LiteLLM connection: the OS store plus the extra CA file (if any).
pub fn tls_config() -> TlsConfig {
    let mut roots: Vec<Certificate<'static>> = Vec::new();

    let native = rustls_native_certs::load_native_certs();
    if !native.errors.is_empty() {
        log::error("tls", &format!("some system certificates couldn't be read: {:?}", native.errors));
    }
    roots.extend(native.certs.iter().map(|c| Certificate::from_der(c.as_ref()).to_owned()));

    if let Some(extra) = ca_path() {
        match fs::read(&extra).map_err(Error::from).and_then(|pem| certs_from_pem(&pem)) {
            Ok(certs) => roots.extend(certs),
            Err(e) => log::error("tls", &format!("can't use CA file {}: {e}", extra.display())),
        }
    }

    if roots.is_empty() {
        // No system store at all: fall back to the bundled public roots rather than trusting nothing.
        return TlsConfig::default();
    }
    TlsConfig::builder().root_certs(RootCerts::new_with_certs(&roots)).build()
}

/// Checks a CA file and returns the path to save in the settings. By default the file is used where
/// it is (so IT can update it in place); with `copy`, it's copied next to the settings first (for a
/// CA shipped inside the installer folder, which users may delete afterwards).
pub fn prepare_ca(source: &Path, copy: bool) -> Result<PathBuf> {
    let pem = fs::read(source).map_err(|e| Error::Invalid(format!("Can't read CA file {}: {e}", source.display())))?;
    certs_from_pem(&pem)?;
    if !copy {
        return Ok(std::path::absolute(source)?);
    }
    fs::create_dir_all(config_dir())?;
    let target = config_dir().join("ca.pem");
    fs::write(&target, &pem)?;
    Ok(target)
}

fn certs_from_pem(pem: &[u8]) -> Result<Vec<Certificate<'static>>> {
    let certs: Vec<Certificate<'static>> = parse_pem(pem)
        .filter_map(|item| match item {
            Ok(PemItem::Certificate(c)) => Some(c),
            _ => None,
        })
        .collect();
    if certs.is_empty() {
        return Err(Error::Invalid("The CA file has no certificates (expected PEM: -----BEGIN CERTIFICATE-----).".into()));
    }
    Ok(certs)
}

/// Whether a connection error means the server's certificate isn't trusted.
pub fn is_untrusted(message: &str) -> bool {
    ["UnknownIssuer", "invalid peer certificate", "certificate verify failed", "self-signed", "self signed"]
        .iter()
        .any(|m| message.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_files_without_certificates() {
        assert!(certs_from_pem(b"not a certificate").is_err());
        assert!(certs_from_pem(b"-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n").is_err());
    }

    #[test]
    fn recognizes_untrusted_certificate_errors() {
        assert!(is_untrusted("io: invalid peer certificate: UnknownIssuer"));
        assert!(!is_untrusted("io: Connection refused (os error 61)"));
    }
}
