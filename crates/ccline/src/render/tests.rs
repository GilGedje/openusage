use usage_core::cache::{CacheFile, CachedError};
use usage_core::snapshot::{Budget, ModelUsage, Snapshot, Totals};

use super::*;

const NOW: i64 = 1_000_000;

fn snap(spend: f64, max: Option<f64>) -> Snapshot {
    Snapshot {
        fetched_at: NOW - 30,
        user_id: "u".into(),
        budget: Budget { spend, max_budget: max, duration: Some("30d".into()), reset_at: Some(NOW + 4 * 86_400) },
        today: Totals { spend: 1.2, tokens: 12_300, requests: 5 },
        last_30d: Totals { spend: 12.4, tokens: 1_200_000, requests: 80 },
        models: vec![ModelUsage { name: "sonnet".into(), totals: Totals { spend: 8.1, tokens: 900_000, requests: 50 } }],
    }
}

fn cache(snapshot: Option<Snapshot>, error: Option<&str>) -> CacheFile {
    CacheFile {
        proxy_url: "http://x".into(),
        snapshot,
        error: error.map(|k| CachedError { kind: k.into(), message: String::new(), at: NOW }),
        last_attempt: NOW,
    }
}

fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        match (esc, c) {
            (false, '\x1b') => esc = true,
            (true, 'm') => esc = false,
            (true, _) => {}
            (false, c) => out.push(c),
        }
    }
    out
}

#[test]
fn full_line() {
    let c = cache(Some(snap(12.4, Some(50.0))), None);
    assert_eq!(
        plain(&line(Some(&c), Some("sonnet"), NOW, 200)),
        "▰▱▱▱▱ $12.40/$50.00 25% · resets 4d · today $1.20 · sonnet $8.10/30d"
    );
}

#[test]
fn colors_by_budget_used() {
    let blue = line(Some(&cache(Some(snap(10.0, Some(50.0))), None)), None, NOW, 200);
    let yellow = line(Some(&cache(Some(snap(40.0, Some(50.0))), None)), None, NOW, 200);
    let red = line(Some(&cache(Some(snap(46.0, Some(50.0))), None)), None, NOW, 200);
    assert!(blue.starts_with(BLUE));
    assert!(yellow.starts_with(YELLOW));
    assert!(red.starts_with(RED));
}

#[test]
fn no_limit_and_unknown_model() {
    let c = cache(Some(snap(3.0, None)), None);
    assert_eq!(plain(&line(Some(&c), Some("opus"), NOW, 200)), "$3.00 spent · no limit · resets 4d · today $1.20");
}

#[test]
fn narrow_terminal_drops_low_priority_segments() {
    let c = cache(Some(snap(12.4, Some(50.0))), None);
    assert_eq!(plain(&line(Some(&c), Some("sonnet"), NOW, 40)), "▰▱▱▱▱ $12.40/$50.00 25% · today $1.20");
}

#[test]
fn stale_data_shows_age() {
    let c = cache(Some(snap(12.4, Some(50.0))), Some("network"));
    let out = plain(&line(Some(&c), None, NOW, 200));
    assert!(out.ends_with("⚠ can't reach proxy · 30s old"), "{out}");
}

#[test]
fn signed_out_and_empty_states() {
    let c = cache(Some(snap(1.0, Some(50.0))), Some("signed_out"));
    assert_eq!(plain(&line(Some(&c), None, NOW, 200)), "LiteLLM: signed out · run `ccline login`");
    assert_eq!(plain(&line(None, None, NOW, 200)), "LiteLLM · loading…");
    assert_eq!(plain(&line(Some(&cache(None, Some("keyring"))), None, NOW, 200)), "LiteLLM: secure storage unavailable");
}

#[test]
fn formats_money_and_durations() {
    assert_eq!(money(0.0), "$0.00");
    assert_eq!(money(0.004), "<$0.01");
    assert_eq!(money(1234.5), "$1234");
    assert_eq!(duration(59), "59s");
    assert_eq!(duration(3 * 3600), "3h");
    assert_eq!(duration(5 * 86_400), "5d");
    assert_eq!(tokens(1_234), "1.2k");
}
