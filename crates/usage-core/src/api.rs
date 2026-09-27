//! LiteLLM usage endpoints that work with the SSO token. Response types keep only the fields we
//! use; everything is defaulted so newer proxy versions adding or dropping fields don't break us.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Deserialize;

use crate::Result;
use crate::client::Client;

/// `GET /v2/user/info` — the signed-in user's budget.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    /// Spend in the current budget window (resets to 0 at `budget_reset_at`).
    #[serde(default)]
    pub spend: f64,
    /// `None` means no limit.
    pub max_budget: Option<f64>,
    pub budget_duration: Option<String>,
    /// When the current window ends (RFC 3339).
    pub budget_reset_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Metrics {
    pub spend: f64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub total_tokens: u64,
    pub api_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct BreakdownEntry {
    pub metrics: Metrics,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Breakdown {
    /// Keyed by the model name users pick in Claude Code (e.g. `sonnet`, `claude-sonnet-4-6`).
    pub model_groups: BTreeMap<String, BreakdownEntry>,
    /// Keyed by the real backend model (e.g. `gemini/gemini-2.5-pro`).
    pub models: BTreeMap<String, BreakdownEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Day {
    /// `YYYY-MM-DD`
    pub date: String,
    #[serde(default)]
    pub metrics: Metrics,
    #[serde(default)]
    pub breakdown: Breakdown,
}

/// `GET /user/daily/activity/aggregated`
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct DailyActivity {
    pub results: Vec<Day>,
}

pub fn user_info(client: &Client) -> Result<UserInfo> {
    client.get("/v2/user/info", &[])
}

/// Daily usage between two local dates, including today's spend. `utc_offset_minutes` is the
/// local offset east of UTC (Israel summer = 180); LiteLLM wants the JavaScript sign, so it's negated.
pub fn daily_activity(
    client: &Client,
    user_id: &str,
    start: NaiveDate,
    end: NaiveDate,
    utc_offset_minutes: i32,
) -> Result<DailyActivity> {
    let start = start.format("%Y-%m-%d").to_string();
    let end = end.format("%Y-%m-%d").to_string();
    let tz = (-utc_offset_minutes).to_string();
    client.get(
        "/user/daily/activity/aggregated",
        &[
            ("user_id", user_id),
            ("start_date", &start),
            ("end_date", &end),
            ("timezone", &tz),
            ("include_current_utc_day", "true"),
        ],
    )
}
