//! Shared core for the LiteLLM usage tools (the `ccline` status line and, later, the tray app):
//! SSO sign-in, the LiteLLM API client, secure token storage, and the on-disk usage cache.

pub mod account;
pub mod api;
pub mod cache;
pub mod client;
pub mod config;
pub mod error;
pub mod gauge;
pub mod log;
pub mod login_flow;
pub mod panel;
pub mod refresh;
pub mod secret;
pub mod snapshot;
pub mod sso;

pub use error::{Error, Result};

/// Current time in Unix seconds.
pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
