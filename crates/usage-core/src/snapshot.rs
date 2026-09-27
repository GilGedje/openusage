//! One fetched view of the user's usage — what the status line and tray render from.

use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::api::{self, DailyActivity, Metrics, UserInfo};
use crate::client::Client;

/// Days of history in `last_30d` and `models`.
pub const HISTORY_DAYS: i64 = 30;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub spend: f64,
    pub tokens: u64,
    pub requests: u64,
}

impl Totals {
    fn add(&mut self, m: &Metrics) {
        self.spend += m.spend;
        self.tokens += m.total_tokens;
        self.requests += m.api_requests;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelUsage {
    /// The model name users pick in Claude Code (LiteLLM's model group).
    pub name: String,
    pub totals: Totals,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    /// Spend in the current budget window.
    pub spend: f64,
    pub max_budget: Option<f64>,
    pub duration: Option<String>,
    /// Unix seconds when the window resets.
    pub reset_at: Option<i64>,
}

impl Budget {
    /// Fraction of the budget used (can exceed 1.0), or `None` without a limit.
    pub fn used_fraction(&self) -> Option<f64> {
        self.max_budget.filter(|m| *m > 0.0).map(|m| self.spend / m)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unix seconds.
    pub fetched_at: i64,
    pub user_id: String,
    pub budget: Budget,
    pub today: Totals,
    pub last_30d: Totals,
    /// Last 30 days by model, most spend first.
    pub models: Vec<ModelUsage>,
}

pub fn fetch(client: &Client) -> Result<Snapshot> {
    let user = api::user_info(client)?;
    let now = Local::now();
    let today = now.date_naive();
    let offset = now.offset().local_minus_utc() / 60;
    let activity =
        api::daily_activity(client, &user.user_id, today - Duration::days(HISTORY_DAYS - 1), today, offset)?;
    Ok(build(&user, &activity, today, now.timestamp()))
}

/// Pure assembly, split out for tests. "Today" is every bucket dated today or later: LiteLLM
/// buckets by UTC day, and `include_current_utc_day` may add a bucket dated tomorrow locally.
pub fn build(user: &UserInfo, activity: &DailyActivity, today: NaiveDate, now: i64) -> Snapshot {
    let today_str = today.format("%Y-%m-%d").to_string();
    let mut today_totals = Totals::default();
    let mut total = Totals::default();
    let mut models: Vec<ModelUsage> = Vec::new();

    for day in &activity.results {
        total.add(&day.metrics);
        if day.date >= today_str {
            today_totals.add(&day.metrics);
        }
        for (name, entry) in &day.breakdown.model_groups {
            match models.iter_mut().find(|m| &m.name == name) {
                Some(m) => m.totals.add(&entry.metrics),
                None => {
                    let mut totals = Totals::default();
                    totals.add(&entry.metrics);
                    models.push(ModelUsage { name: name.clone(), totals });
                }
            }
        }
    }
    models.retain(|m| m.totals.requests > 0 || m.totals.spend > 0.0);
    models.sort_by(|a, b| b.totals.spend.total_cmp(&a.totals.spend).then_with(|| a.name.cmp(&b.name)));

    Snapshot {
        fetched_at: now,
        user_id: user.user_id.clone(),
        budget: Budget {
            spend: user.spend,
            max_budget: user.max_budget,
            duration: user.budget_duration.clone(),
            reset_at: user
                .budget_reset_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp()),
        },
        today: today_totals,
        last_30d: total,
        models,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = include_str!("../tests/fixtures/user_info_v2.json");
    const DAILY: &str = include_str!("../tests/fixtures/user_daily_activity.json");

    #[test]
    fn builds_snapshot_from_fixtures() {
        let user: UserInfo = serde_json::from_str(USER).unwrap();
        let activity: DailyActivity = serde_json::from_str(DAILY).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let snap = build(&user, &activity, today, 1_000);

        assert_eq!(snap.user_id, "test-user");
        assert_eq!(snap.budget.max_budget, Some(50.0));
        assert_eq!(snap.budget.reset_at, Some(1_790_812_800)); // 2026-10-01T00:00:00Z
        assert!((snap.today.spend - 0.00256385).abs() < 1e-9);
        assert_eq!(snap.today.requests, 7);
        assert_eq!(snap.last_30d.requests, 34);

        let names: Vec<&str> = snap.models.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["gemini-2.5-pro", "claude-sonnet-4-6", "gemini-2.5-flash", "haiku", "sonnet"]);
    }

    #[test]
    fn used_fraction_handles_no_limit() {
        let b = Budget { spend: 5.0, max_budget: None, duration: None, reset_at: None };
        assert_eq!(b.used_fraction(), None);
        let b = Budget { max_budget: Some(20.0), ..b };
        assert_eq!(b.used_fraction(), Some(0.25));
    }
}
