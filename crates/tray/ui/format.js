// Number and time formatting shared by the views (mirrors ccline's formatting).

const Format = {
  // Whole amounts without cents: $100, $1,250 (for totals like the budget).
  wholeMoney(v) {
    return Number.isInteger(v) ? "$" + v.toLocaleString("en-US") : this.money(v);
  },

  money(v) {
    if (v > 0 && v < 0.01) return "<$0.01";
    if (v >= 1000) return "$" + Math.round(v).toLocaleString("en-US");
    return "$" + v.toFixed(2);
  },

  tokens(n) {
    if (n < 1000) return String(n);
    if (n < 1e6) return Math.round(n / 1e3) + "k";
    const m = n / 1e6;
    return (Number.isInteger(m) ? m : m.toFixed(1)) + "M";
  },

  count(n) {
    return n.toLocaleString("en-US");
  },

  // Compact duration: 45s, 12m, 5h, 3d.
  duration(secs) {
    const s = Math.max(0, Math.floor(secs));
    if (s < 60) return s + "s";
    if (s < 3600) return Math.floor(s / 60) + "m";
    if (s < 172800) return Math.floor(s / 3600) + "h";
    return Math.floor(s / 86400) + "d";
  },

  // Two-unit duration like OpenUsage: 3d 9h, 1h 53m, 12m.
  longDuration(secs) {
    const s = Math.max(0, Math.floor(secs));
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return h > 0 ? `${d}d ${h}h` : `${d}d`;
    if (h > 0) return m > 0 ? `${h}h ${m}m` : `${h}h`;
    return `${Math.max(m, 1)}m`;
  },

  // LiteLLM budget durations: "30d", "7d", "24h", "1mo", "2w", "90m".
  durationSecs(text) {
    const m = /^(\d+)\s*(mo|m|h|d|w)$/.exec(String(text || "").trim());
    if (!m) return null;
    const n = Number(m[1]);
    return n * { m: 60, h: 3600, d: 86400, w: 604800, mo: 2592000 }[m[2]];
  },

  // How far through the budget window we are, and when the budget runs out at the current rate
  // (`runOutIn` is null when it lasts until the reset). Null when the window is unknown or just began.
  pace(budget, now) {
    const window = this.durationSecs(budget.duration);
    if (!window || !budget.reset_at || !(budget.max_budget > 0)) return null;
    const start = budget.reset_at - window;
    const spentFor = now - start;
    if (spentFor <= 0 || spentFor / window < 0.02) return null;
    const elapsed = Math.min(1, spentFor / window);
    if (budget.spend >= budget.max_budget) return { elapsed, runOutIn: 0 };
    const rate = budget.spend / spentFor;
    const secsToLimit = rate > 0 ? (budget.max_budget - budget.spend) / rate : Infinity;
    return { elapsed, runOutIn: now + secsToLimit < budget.reset_at ? secsToLimit : null };
  },

  // "Refreshes in 4:32" until the next automatic refresh (every `refresh_secs`, shared with ccline).
  countdown(state) {
    const cache = state && state.cache;
    if (!state || !state.signed_in) return "";
    if (!cache) return "Refreshing…";
    const left = cache.last_attempt + (state.refresh_secs || 300) - Math.floor(Date.now() / 1000);
    if (left <= 0) return "Refreshing…";
    const m = Math.floor(left / 60);
    const s = String(left % 60).padStart(2, "0");
    return `Refreshes in ${m}:${s}`;
  },

  severity(fraction) {
    if (fraction >= 0.9) return "critical";
    if (fraction >= 0.75) return "warning";
    return "normal";
  },

  // Last `n` local dates as YYYY-MM-DD, oldest first.
  lastDays(n) {
    const out = [];
    const d = new Date();
    for (let i = n - 1; i >= 0; i--) {
      const x = new Date(d.getFullYear(), d.getMonth(), d.getDate() - i);
      const mm = String(x.getMonth() + 1).padStart(2, "0");
      const dd = String(x.getDate()).padStart(2, "0");
      out.push(`${x.getFullYear()}-${mm}-${dd}`);
    }
    return out;
  },

  escape(s) {
    return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  },
};
