//! Minimal blocking HTTP client for the LiteLLM proxy.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::{Error, Result};

pub struct Client {
    agent: ureq::Agent,
    base: String,
    token: Option<String>,
}

impl Client {
    pub fn new(base_url: &str, token: Option<&str>) -> Client {
        let config = ureq::Agent::config_builder()
            .tls_config(crate::tls::tls_config())
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(15)))
            .build();
        Client {
            agent: ureq::Agent::new_with_config(config),
            base: loopback_for_localhost(base_url),
            token: token.map(str::to_string),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let url = format!("{}{}{}", self.base, path, encode_query(query));
        let mut req = self.agent.get(&url);
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        read(req.call())
    }

    pub fn post_json<T: DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.agent.post(&url);
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        read(req.send_json(body))
    }

    /// GET with extra headers and no bearer token (used by the SSO poll).
    pub fn get_with_headers<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}{}{}", self.base, path, encode_query(query));
        let mut req = self.agent.get(&url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        read(req.call())
    }
}

fn read<T: DeserializeOwned>(result: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<T> {
    let mut resp = result.map_err(|e| {
        let message = e.to_string();
        if crate::tls::is_untrusted(&message) { Error::Certificate(message) } else { Error::Network(message) }
    })?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| Error::Network(e.to_string()))?;
    if status == 401 {
        return Err(Error::SignedOut);
    }
    if !(200..300).contains(&status) {
        return Err(Error::Http { status, message: error_message(&body) });
    }
    serde_json::from_str(&body).map_err(|e| Error::Parse(e.to_string()))
}

/// Pulls the human message out of LiteLLM's error shapes (`{"error":{"message":…}}` or
/// `{"detail":…}`), falling back to a trimmed body.
fn error_message(body: &str) -> String {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let msg = parsed.as_ref().and_then(|v| {
        v.pointer("/error/message")
            .or_else(|| v.get("detail"))
            .and_then(|m| m.as_str().map(str::to_string).or_else(|| Some(m.to_string())))
    });
    msg.unwrap_or_else(|| body.chars().take(200).collect())
}

/// `localhost` always means this machine (RFC 6761), and browsers and curl never look it up. Some
/// machines have no `localhost` line in their hosts file, so connect to 127.0.0.1 directly.
fn loopback_for_localhost(base_url: &str) -> String {
    for scheme in ["http://", "https://"] {
        if let Some(rest) = base_url.strip_prefix(scheme).and_then(|r| r.strip_prefix("localhost"))
            && (rest.is_empty() || rest.starts_with(':') || rest.starts_with('/'))
        {
            return format!("{scheme}127.0.0.1{rest}");
        }
    }
    base_url.to_string()
}

fn encode_query(query: &[(&str, &str)]) -> String {
    if query.is_empty() {
        return String::new();
    }
    let pairs: Vec<String> = query.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect();
    format!("?{}", pairs.join("&"))
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query() {
        assert_eq!(encode_query(&[]), "");
        assert_eq!(encode_query(&[("user_id", "a@b.c"), ("x", "1 2")]), "?user_id=a%40b.c&x=1%202");
    }

    #[test]
    fn connects_to_loopback_for_localhost() {
        assert_eq!(loopback_for_localhost("http://localhost:4000"), "http://127.0.0.1:4000");
        assert_eq!(loopback_for_localhost("http://localhost/litellm"), "http://127.0.0.1/litellm");
        assert_eq!(loopback_for_localhost("http://localhost"), "http://127.0.0.1");
        assert_eq!(loopback_for_localhost("http://localhost.example.com"), "http://localhost.example.com");
        assert_eq!(loopback_for_localhost("https://llm.example.com"), "https://llm.example.com");
    }

    #[test]
    fn extracts_error_messages() {
        assert_eq!(error_message(r#"{"error":{"message":"Key not found"}}"#), "Key not found");
        assert_eq!(error_message(r#"{"detail":"Invalid CLI login session id"}"#), "Invalid CLI login session id");
        assert_eq!(error_message("plain"), "plain");
    }
}
