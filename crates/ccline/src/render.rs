//! Text for the status line and `ccline status`. Pure functions of the session and cache, so it's
//! all testable. Colors follow design/theme.css (dark values, which read well on both backgrounds).

use usage_core::cache::CacheFile;
use usage_core::snapshot::Snapshot;

use usage_core::alerts::Alerts;

use crate::session::Session;

const BLUE: &str = "\x1b[38;2;10;132;255m";
const YELLOW: &str = "\x1b[38;2;255;214;10m";
const RED: &str = "\x1b[38;2;255;69;58m";
const ORANGE: &str = "\x1b[38;2;255;159;10m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

const BAR_CELLS: usize = 5;
/// Budget or context used at or above these fractions turns yellow / red.
const WARNING_AT: f64 = 0.75;
const CRITICAL_AT: f64 = 0.90;

/// Where the LiteLLM side of the line stands.
pub enum Usage<'a> {
    NotConfigured,
    Failed(&'a str),
    Cached(Option<&'a CacheFile>),
}

/// The status line: `Model · ctx 42% 84k/200k · ▰▰▰▱▱ $31.26/$50.00 63% · today $0.00 · resets 3d · $15.83/30d`.
/// When the terminal is narrow, the highest priority numbers are dropped first.
pub fn line(session: &Session, usage: Usage, now: i64, columns: usize, alerts: &Alerts) -> String {
    let mut segs: Vec<(u8, String)> = Vec::new();
    if let Some(model) = session.display_model() {
        segs.push((1, format!("{BOLD}{model}{RESET}")));
    }
    if let Some(ctx) = context_segment(session) {
        segs.push((1, ctx));
    }
    match usage {
        Usage::NotConfigured => segs.push((0, orange("LiteLLM: run `ccline login` to sign in"))),
        Usage::Failed(msg) => segs.push((0, orange(&format!("LiteLLM: {msg}")))),
        Usage::Cached(None) => segs.push((0, format!("{DIM}LiteLLM · loading…{RESET}"))),
        Usage::Cached(Some(cache)) => usage_segments(&mut segs, session, cache, now, alerts),
    }
    fit(segs, columns)
}

fn usage_segments(segs: &mut Vec<(u8, String)>, session: &Session, cache: &CacheFile, now: i64, alerts: &Alerts) {
    if cache.error.as_ref().is_some_and(|e| e.is_signed_out()) {
        return segs.push((0, orange("LiteLLM: signed out · run `ccline login`")));
    }
    let Some(snap) = &cache.snapshot else {
        let kind = cache.error.as_ref().map(|e| e.kind.as_str()).unwrap_or("");
        return segs.push((0, orange(&format!("LiteLLM: {}", short_error(kind)))));
    };
    let usage_page = usage_core::config::usage_page_url(&cache.proxy_url);
    segs.push((0, link(&usage_page, &budget_segment(snap, alerts))));
    segs.push((2, format!("today {}", money(snap.today.spend))));
    if let Some(reset) = snap.budget.reset_at.filter(|r| *r > now) {
        segs.push((3, format!("{DIM}resets {}{RESET}", duration(reset - now))));
    }
    let model_usage = session
        .model_id
        .as_deref()
        .and_then(|id| snap.models.iter().find(|m| m.name.eq_ignore_ascii_case(id)));
    if let Some(m) = model_usage {
        segs.push((4, format!("{} {DIM}this model/30d{RESET}", money(m.totals.spend))));
    }
    if let Some(err) = &cache.error {
        segs.push((1, orange(&format!("⚠ {} · {} old", short_error(&err.kind), duration(now - snap.fetched_at)))));
    }
}

fn context_segment(session: &Session) -> Option<String> {
    let pct = session.context_pct?;
    let color = severity_color(pct / 100.0);
    let size = match (session.context_tokens, session.context_size) {
        (Some(used), Some(size)) => format!(" {DIM}{}/{}{RESET}", tokens(used), tokens(size)),
        _ => String::new(),
    };
    Some(format!("ctx {color}{pct:.0}%{RESET}{size}"))
}

fn budget_segment(snap: &Snapshot, alerts: &Alerts) -> String {
    let spend = money(snap.budget.spend);
    match (snap.budget.max_budget, snap.budget.used_fraction()) {
        (Some(max), Some(frac)) => {
            let [r, g, b] = alerts.rgb(frac);
            let color = &format!("\x1b[38;2;{r};{g};{b}m");
            let filled = ((frac * BAR_CELLS as f64).round() as usize).min(BAR_CELLS);
            let bar = format!("{}{}", "▰".repeat(filled), "▱".repeat(BAR_CELLS - filled));
            format!("{color}{bar}{RESET} {spend}/{} {color}{:.0}%{RESET}", money(max), frac * 100.0)
        }
        _ => format!("{spend} {DIM}spent · no limit{RESET}"),
    }
}

fn severity_color(fraction: f64) -> &'static str {
    if fraction >= CRITICAL_AT {
        RED
    } else if fraction >= WARNING_AT {
        YELLOW
    } else {
        BLUE
    }
}

fn orange(text: &str) -> String {
    format!("{ORANGE}{text}{RESET}")
}

fn short_error(kind: &str) -> &'static str {
    match kind {
        "network" => "can't reach proxy",
        "certificate" => "certificate not trusted (see CA_CERT)",
        "keyring" => "secure storage unavailable",
        "needs_approval" => "run `ccline status` to allow Keychain access",
        "http" => "proxy error",
        "parse" => "unexpected response",
        "not_configured" => "run `ccline login`",
        _ => "refresh failed",
    }
}

/// Joins segments with a dim dot, dropping the highest-priority-number segments until it fits.
/// Ties drop the rightmost segment first.
fn fit(mut segs: Vec<(u8, String)>, columns: usize) -> String {
    let sep = format!(" {DIM}·{RESET} ");
    loop {
        let joined = segs.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join(&sep);
        if visible_len(&joined) <= columns || segs.len() <= 1 {
            return joined;
        }
        let drop = segs.iter().enumerate().max_by_key(|(i, (p, _))| (*p, *i)).map(|(i, _)| i).unwrap();
        segs.remove(drop);
    }
}

/// Clickable text (OSC 8 hyperlink); terminals without link support show the plain text.
fn link(url: &str, text: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

fn visible_len(s: &str) -> usize {
    strip_escapes(s).chars().count()
}

/// Removes color codes (`ESC [ … m`) and links (`ESC ] … ESC \`).
fn strip_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' || (c == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

pub fn money(v: f64) -> String {
    if v > 0.0 && v < 0.01 {
        "<$0.01".into()
    } else if v >= 1000.0 {
        format!("${v:.0}")
    } else {
        format!("${v:.2}")
    }
}

fn tokens(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{}k", (n as f64 / 1e3).round()),
        _ => {
            let m = n as f64 / 1e6;
            if m.fract() == 0.0 { format!("{m:.0}M") } else { format!("{m:.1}M") }
        }
    }
}

/// Compact duration: 45s, 12m, 5h, 3d.
fn duration(secs: i64) -> String {
    let s = secs.max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3_600 => format!("{}m", s / 60),
        3_600..172_800 => format!("{}h", s / 3_600),
        _ => format!("{}d", s / 86_400),
    }
}

/// Multi-line plain-text report for `ccline status`.
pub fn report(snap: &Snapshot, now: i64) -> String {
    let b = &snap.budget;
    let mut out = format!("Quota — Claude usage for {}\n\n", snap.user_id);
    let budget = match (b.max_budget, b.used_fraction()) {
        (Some(max), Some(frac)) => format!("{} of {} ({:.0}%)", money(b.spend), money(max), frac * 100.0),
        _ => format!("{} (no limit)", money(b.spend)),
    };
    let reset = b.reset_at.filter(|r| *r > now).map(|r| format!(" · resets in {}", duration(r - now))).unwrap_or_default();
    let window = b.duration.as_deref().map(|d| format!(" · {d} window")).unwrap_or_default();
    out += &format!("  Budget   {budget}{reset}{window}\n");
    for (label, t) in [("Today", &snap.today), ("30 days", &snap.last_30d)] {
        out += &format!("  {label:<8} {} · {} tokens · {} requests\n", money(t.spend), tokens(t.tokens), t.requests);
    }
    if !snap.models.is_empty() {
        out += "\n  By model, last 30 days\n";
        let width = snap.models.iter().map(|m| m.name.len()).max().unwrap_or(0);
        for m in &snap.models {
            out += &format!(
                "    {:<width$}  {:>8}  {:>7} tokens  {} requests\n",
                m.name,
                money(m.totals.spend),
                tokens(m.totals.tokens),
                m.totals.requests
            );
        }
    }
    out
}

#[cfg(test)]
mod tests;
