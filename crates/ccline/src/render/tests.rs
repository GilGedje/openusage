use usage_core::cache::{CacheFile, CachedError};
use usage_core::snapshot::{Budget, ModelUsage, Snapshot, Totals};

use usage_core::alerts::Alerts;

use super::*;

const NOW: i64 = 1_000_000;

fn snap(spend: f64, max: Option<f64>) -> Snapshot {
    Snapshot {
        fetched_at: NOW - 30,
        user_id: "u".into(),
        budget: Budget { spend, max_budget: max, duration: Some("30d".into()), reset_at: Some(NOW + 4 * 86_400) },
        today: Totals { spend: 1.2, tokens: 12_300, requests: 5 },
        yesterday: Totals::default(),
        last_30d: Totals { spend: 12.4, tokens: 1_200_000, requests: 80 },
        models: vec![ModelUsage { name: "sonnet".into(), totals: Totals { spend: 8.1, tokens: 900_000, requests: 50 } }],
        days: vec![],
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

fn session(pct: Option<f64>) -> Session {
    Session {
        model_id: Some("sonnet".into()),
        model_name: Some("Sonnet 4.6".into()),
        context_pct: pct,
        context_tokens: pct.map(|p| (p * 2_000.0) as u64),
        context_size: pct.map(|_| 200_000),
    }
}

fn plain(s: &str) -> String {
    strip_escapes(s)
}

fn render(s: &Session, c: &CacheFile, columns: usize) -> String {
    plain(&line(s, Usage::Cached(Some(c)), NOW, columns, &Alerts::default()))
}

#[test]
fn full_line() {
    let c = cache(Some(snap(12.4, Some(50.0))), None);
    assert_eq!(
        render(&session(Some(42.0)), &c, 200),
        "Sonnet 4.6 · ctx 42% 84k/200k · ▰▱▱▱▱ $12.40/$50.00 25% · today $1.20 · resets 4d · $8.10 this model/30d"
    );
}

#[test]
fn colors_by_fraction_used() {
    let at = |spend: f64| line(&Session::default(), Usage::Cached(Some(&cache(Some(snap(spend, Some(50.0))), None))), NOW, 200, &Alerts::default());
    assert!(at(10.0).contains(&format!("{BLUE}▰")));
    assert!(at(40.0).contains(&format!("{YELLOW}▰")));
    assert!(at(46.0).contains(&format!("{RED}▰")));
    let ctx = line(&session(Some(92.0)), Usage::NotConfigured, NOW, 200, &Alerts::default());
    assert!(ctx.contains(&format!("ctx {RED}92%")), "{ctx}");
}

#[test]
fn no_limit_and_unknown_model() {
    let s = Session { model_id: Some("opus".into()), ..Session::default() };
    assert_eq!(render(&s, &cache(Some(snap(3.0, None)), None), 200), "opus · $3.00 spent · no limit · today $1.20 · resets 4d");
}

#[test]
fn narrow_terminal_drops_low_priority_segments() {
    let c = cache(Some(snap(12.4, Some(50.0))), None);
    let s = session(Some(42.0));
    assert_eq!(render(&s, &c, 70), "Sonnet 4.6 · ctx 42% 84k/200k · ▰▱▱▱▱ $12.40/$50.00 25% · today $1.20");
    assert_eq!(render(&s, &c, 60), "Sonnet 4.6 · ctx 42% 84k/200k · ▰▱▱▱▱ $12.40/$50.00 25%");
    assert_eq!(render(&s, &c, 40), "Sonnet 4.6 · ▰▱▱▱▱ $12.40/$50.00 25%");
    assert_eq!(render(&s, &c, 30), "▰▱▱▱▱ $12.40/$50.00 25%");
}

#[test]
fn context_before_first_response_is_hidden() {
    let c = cache(Some(snap(12.4, Some(50.0))), None);
    assert!(render(&session(None), &c, 200).starts_with("Sonnet 4.6 · ▰"));
}

#[test]
fn stale_data_shows_age() {
    let out = render(&Session::default(), &cache(Some(snap(12.4, Some(50.0))), Some("network")), 200);
    assert!(out.ends_with("⚠ can't reach proxy · 30s old"), "{out}");
}

#[test]
fn session_info_shows_in_every_state() {
    let s = session(Some(42.0));
    assert_eq!(render(&s, &cache(Some(snap(1.0, Some(50.0))), Some("signed_out")), 200), "Sonnet 4.6 · ctx 42% 84k/200k · LiteLLM: signed out · run `ccline login`");
    assert_eq!(plain(&line(&s, Usage::Cached(None), NOW, 200, &Alerts::default())), "Sonnet 4.6 · ctx 42% 84k/200k · LiteLLM · loading…");
    assert_eq!(plain(&line(&s, Usage::NotConfigured, NOW, 200, &Alerts::default())), "Sonnet 4.6 · ctx 42% 84k/200k · LiteLLM: run `ccline login` to sign in");
    assert_eq!(render(&Session::default(), &cache(None, Some("keyring")), 200), "LiteLLM: secure storage unavailable");
    assert_eq!(
        render(&Session::default(), &cache(None, Some("needs_approval")), 200),
        "LiteLLM: run `ccline status` to allow Keychain access"
    );
}

#[test]
fn budget_links_to_usage_page() {
    let out = line(&Session::default(), Usage::Cached(Some(&cache(Some(snap(12.4, Some(50.0))), None))), NOW, 200, &Alerts::default());
    assert!(out.starts_with("\x1b]8;;http://x/ui/?page=new_usage\x1b\\"), "{out:?}");
    assert_eq!(strip_escapes("a\x1b]8;;http://u\x1b\\b\x1b]8;;\x1b\\c\x1b[1md"), "abcd");
}

#[test]
fn budget_colors_follow_the_users_alerts() {
    let alerts = Alerts { warning_pct: 20, warning_color: "#bf5af2".into(), ..Alerts::default() };
    let out = line(&Session::default(), Usage::Cached(Some(&cache(Some(snap(12.4, Some(50.0))), None))), NOW, 200, &alerts);
    assert!(out.contains("\x1b[38;2;191;90;242m▰"), "{out:?}"); // 25% used ≥ 20% warning → purple
}

#[test]
fn formats_money_tokens_and_durations() {
    assert_eq!(money(0.0), "$0.00");
    assert_eq!(money(0.004), "<$0.01");
    assert_eq!(money(1234.5), "$1234");
    assert_eq!(duration(59), "59s");
    assert_eq!(duration(3 * 3600), "3h");
    assert_eq!(duration(5 * 86_400), "5d");
    assert_eq!(tokens(84_000), "84k");
    assert_eq!(tokens(1_000_000), "1M");
    assert_eq!(tokens(14_300_000), "14.3M");
}
