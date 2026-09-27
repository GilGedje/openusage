//! Text for the status line and `ccline status`. Pure functions of the cache, so it's all testable.
//! Colors follow design/theme.css (dark values, which read well on both terminal backgrounds).

use usage_core::cache::CacheFile;
use usage_core::snapshot::Snapshot;

const BLUE: &str = "\x1b[38;2;10;132;255m";
const YELLOW: &str = "\x1b[38;2;255;214;10m";
const RED: &str = "\x1b[38;2;255;69;58m";
const ORANGE: &str = "\x1b[38;2;255;159;10m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

const BAR_CELLS: usize = 5;
/// Budget used at or above these fractions turns the meter yellow / red.
const WARNING_AT: f64 = 0.75;
const CRITICAL_AT: f64 = 0.90;

/// The model Claude Code is using, from the session JSON on stdin.
pub fn current_model(input: &serde_json::Value) -> Option<String> {
    input.pointer("/model/id").and_then(|v| v.as_str()).map(str::to_string)
}

pub fn not_configured(columns: usize) -> String {
    message("LiteLLM: run `ccline login` to sign in", columns)
}

pub fn message(text: &str, columns: usize) -> String {
    fit(vec![(0, format!("{ORANGE}{text}{RESET}"))], columns)
}

pub fn line(cache: Option<&CacheFile>, model: Option<&str>, now: i64, columns: usize) -> String {
    let Some(cache) = cache else {
        return fit(vec![(0, format!("{DIM}LiteLLM · loading…{RESET}"))], columns);
    };
    if cache.error.as_ref().is_some_and(|e| e.is_signed_out()) {
        return message("LiteLLM: signed out · run `ccline login`", columns);
    }
    let Some(snap) = &cache.snapshot else {
        let kind = cache.error.as_ref().map(|e| e.kind.as_str()).unwrap_or("");
        return message(&format!("LiteLLM: {}", short_error(kind)), columns);
    };

    // (priority, text): lower priority numbers survive longer when the terminal is narrow.
    let mut segs: Vec<(u8, String)> = vec![(0, budget_segment(snap))];
    if let Some(reset) = snap.budget.reset_at.filter(|r| *r > now) {
        segs.push((3, format!("{DIM}resets {}{RESET}", duration(reset - now))));
    }
    segs.push((2, format!("today {}", money(snap.today.spend))));
    if let Some(m) = model.and_then(|id| snap.models.iter().find(|m| m.name.eq_ignore_ascii_case(id))) {
        segs.push((4, format!("{} {}{DIM}/30d{RESET}", m.name, money(m.totals.spend))));
    }
    if let Some(err) = &cache.error {
        segs.push((1, format!("{ORANGE}⚠ {} · {} old{RESET}", short_error(&err.kind), duration(now - snap.fetched_at))));
    }
    fit(segs, columns)
}

fn budget_segment(snap: &Snapshot) -> String {
    let spend = money(snap.budget.spend);
    match (snap.budget.max_budget, snap.budget.used_fraction()) {
        (Some(max), Some(frac)) => {
            let color = if frac >= CRITICAL_AT {
                RED
            } else if frac >= WARNING_AT {
                YELLOW
            } else {
                BLUE
            };
            let filled = ((frac * BAR_CELLS as f64).round() as usize).min(BAR_CELLS);
            let bar = format!("{}{}", "▰".repeat(filled), "▱".repeat(BAR_CELLS - filled));
            format!("{color}{bar}{RESET} {spend}/{} {color}{:.0}%{RESET}", money(max), frac * 100.0)
        }
        _ => format!("{spend} {DIM}spent · no limit{RESET}"),
    }
}

fn short_error(kind: &str) -> &'static str {
    match kind {
        "network" => "can't reach proxy",
        "keyring" => "secure storage unavailable",
        "http" => "proxy error",
        "parse" => "unexpected response",
        "not_configured" => "run `ccline login`",
        _ => "refresh failed",
    }
}

/// Joins segments with a dim dot, dropping the highest-priority-number segments until it fits.
fn fit(mut segs: Vec<(u8, String)>, columns: usize) -> String {
    let sep = format!(" {DIM}·{RESET} ");
    loop {
        let joined = segs.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join(&sep);
        if visible_len(&joined) <= columns || segs.len() <= 1 {
            return joined;
        }
        let drop = segs.iter().enumerate().max_by_key(|(_, (p, _))| *p).map(|(i, _)| i).unwrap();
        segs.remove(drop);
    }
}

fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        match (in_escape, c) {
            (false, '\x1b') => in_escape = true,
            (true, 'm') => in_escape = false,
            (true, _) => {}
            (false, _) => len += 1,
        }
    }
    len
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
        1_000..1_000_000 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
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
    let mut out = format!("LiteLLM usage for {}\n\n", snap.user_id);
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
