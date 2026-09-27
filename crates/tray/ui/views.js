// HTML for each panel view. Pure functions of the state; app.js wires up the buttons and motion.
// Layout mirrors OpenUsage: a Cost card (period switch + donut), then the Claude budget card.

const esc = Format.escape;

const PERIODS = [
  { key: "today", label: "Today" },
  { key: "yesterday", label: "Yesterday" },
  { key: "30d", label: "30 Days" },
];
const LEGEND_MAX = 5;

// Inline icons (no external files: the app runs air-gapped). Stroke follows the text color.
const svg = (body, size = 15) =>
  `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
const Icons = {
  cog: svg(
    '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
  ),
  refresh: svg('<path d="M21 12a9 9 0 1 1-2.64-6.36"/><polyline points="21 3 21 9 15 9"/>', 14),
  share: svg('<path d="M4 12v7a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-7"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/>', 15),
  back: svg('<polyline points="15 18 9 12 15 6"/>', 16),
};

const Views = {
  usage(state, period) {
    const cache = state.cache;
    const snap = cache && cache.snapshot;
    const err = cache && cache.error;
    if (!snap) {
      const msg = err ? esc(err.message) : "Loading your usage…";
      return this.accountHeader() + `<div class="card"><div class="stack"><div class="muted">${msg}</div></div></div>` + this.footer(state);
    }
    return (
      (err ? this.staleBanner(err, snap, state.now) : "") +
      this.cost(snap, period) +
      this.accountHeader() +
      this.account(snap, state.now) +
      this.links(state) +
      this.footer(state)
    );
  },

  staleBanner(err, snap, now) {
    return `<div class="banner notice">⚠ <span>${esc(err.message)} Showing numbers from ${Format.duration(now - snap.fetched_at)} ago.</span></div>`;
  },

  // --- Cost card -----------------------------------------------------------

  cost(snap, period) {
    const { total, models } = this.periodData(snap, period);
    const index = Math.max(0, PERIODS.findIndex((p) => p.key === period));
    const segs = PERIODS.map(
      (p) => `<button class="segment ${p.key === period ? "selected" : ""}" data-action="period" data-period="${p.key}">${p.label}</button>`,
    ).join("");
    const legend = this.legendItems(models);
    const rows = legend.length
      ? legend
          .map(
            (m) => `
          <div class="legend-row">
            <span class="dot" style="background:${m.color}"></span>
            <span class="legend-name">${esc(m.name)}</span>
            <span class="legend-value num">${Format.money(m.spend)}</span>
          </div>`,
          )
          .join("")
      : `<div class="muted">No spend</div>`;
    return `
      <section class="section">
        <div class="section-header-row">
          <div class="section-title">Cost</div>
          <button class="icon-button" data-action="share" aria-label="Export as image">${Icons.share}</button>
        </div>
        <div class="card cost-card" id="cost-card">
          <div class="segmented" style="--segment-index:${index}">
            <span class="segment-thumb" aria-hidden="true"></span>${segs}
          </div>
          <div class="cost-body">
            ${this.donut(legend, total.spend)}
            <div class="legend">${rows}</div>
          </div>
          ${period === "30d" ? this.chart(snap) : ""}
        </div>
      </section>`;
  },

  // Totals and per-model spend for the chosen period.
  periodData(snap, period) {
    if (period === "30d") return { total: snap.last_30d, models: snap.models };
    const [yesterday, today] = Format.lastDays(2);
    const days = (snap.days || []).filter((d) => (period === "today" ? d.date >= today : d.date === yesterday));
    const byName = new Map();
    for (const d of days) {
      for (const m of d.models || []) byName.set(m.name, (byName.get(m.name) || 0) + m.totals.spend);
    }
    const models = [...byName].map(([name, spend]) => ({ name, totals: { spend } })).sort((a, b) => b.totals.spend - a.totals.spend);
    return { total: period === "today" ? snap.today : snap.yesterday || { spend: 0, tokens: 0, requests: 0 }, models };
  },

  // Top models get their own color; the rest fold into "Other".
  legendItems(models) {
    const spent = models.filter((m) => m.totals.spend > 0);
    const top = spent.slice(0, LEGEND_MAX).map((m, i) => ({ name: m.name, spend: m.totals.spend, color: `var(--series-${i + 1})` }));
    const rest = spent.slice(LEGEND_MAX).reduce((s, m) => s + m.totals.spend, 0);
    if (rest > 0) top.push({ name: "Other", spend: rest, color: "var(--series-other)" });
    return top;
  },

  donut(items, total) {
    const r = 42;
    const c = 2 * Math.PI * r;
    const gap = items.length > 1 ? 3 : 0;
    let offset = 0;
    const arcs = items
      .map((m) => {
        const len = total > 0 ? (m.spend / total) * c : 0;
        const dash = Math.max(len - gap, 0.5);
        const arc = `<circle class="arc" r="${r}" cx="55" cy="55" fill="none" stroke="${m.color}" stroke-width="16"
          stroke-dasharray="${dash} ${c - dash}" stroke-dashoffset="${-offset}" data-dash="${dash}" data-circ="${c}" transform="rotate(-90 55 55)" />`;
        offset += len;
        return arc;
      })
      .join("");
    const track = items.length ? "" : `<circle r="${r}" cx="55" cy="55" fill="none" stroke="var(--meter-track)" stroke-width="16" />`;
    return `
      <div class="donut">
        <svg viewBox="0 0 110 110" width="110" height="110" aria-hidden="true">${track}${arcs}</svg>
        <div class="donut-center"><div class="donut-amount num">${Format.money(total)}</div><div class="donut-unit">dollars</div></div>
      </div>`;
  },

  chart(snap) {
    const byDate = new Map((snap.days || []).map((d) => [d.date, d.totals.spend]));
    const dates = Format.lastDays(30);
    const max = Math.max(0, ...dates.map((d) => byDate.get(d) || 0));
    const bars = dates
      .map((d, i) => {
        const v = byDate.get(d) || 0;
        const h = max > 0 && v > 0 ? Math.max(6, (v / max) * 100) : 6;
        const cls = ["bar", v > 0 ? "" : "empty", i === dates.length - 1 ? "today" : ""].join(" ");
        return `<div class="${cls}" style="height:${h}%;--i:${i}"></div>`;
      })
      .join("");
    return `<div class="chart" aria-label="Daily spend, last 30 days">${bars}</div>
      <div class="chart-axis"><span>30 days ago</span><span>Today</span></div>`;
  },

  // --- Claude budget card --------------------------------------------------

  accountHeader() {
    return `<div class="section-header-row"><div class="section-title">Claude</div></div>`;
  },

  account(snap, now) {
    const line = (t) => `${Format.money(t.spend)} · ${Format.tokens(t.tokens)} tokens`;
    const yesterday = snap.yesterday || { spend: 0, tokens: 0 };
    return `
      <div class="card account-card">
        ${this.budget(snap, now)}
        <div class="text-rows">
          <div class="text-row"><span>Today</span><span class="num">${line(snap.today)}</span></div>
          <div class="text-row"><span>Yesterday</span><span class="num">${line(yesterday)}</span></div>
          <div class="text-row"><span>Last 30 Days</span><span class="num">${line(snap.last_30d)}</span></div>
        </div>
      </div>`;
  },

  budget(snap, now) {
    const b = snap.budget;
    const reset = b.reset_at && b.reset_at > now ? `Resets in ${Format.longDuration(b.reset_at - now)}` : "";
    if (!(b.max_budget > 0)) {
      return `
        <div class="metric">
          <div class="metric-top"><span class="metric-label">Budget</span><span class="metric-side">No limit</span></div>
          <div class="metric-bottom"><span class="metric-value">${Format.money(b.spend)} spent</span><span class="metric-side">${reset}</span></div>
        </div>`;
    }
    const used = b.spend / b.max_budget;
    const left = Math.max(0, 1 - used);
    const pace = Format.pace(b, now);
    const tick = pace ? `<div class="meter-tick" style="left:${(1 - pace.elapsed) * 100}%"></div>` : "";
    const warning =
      pace && pace.runOutIn !== null
        ? `<span class="metric-side notice">🔥 Limit in ${Format.longDuration(pace.runOutIn)}</span>`
        : `<span class="metric-side">${Format.money(b.spend)} of ${Format.money(b.max_budget)}</span>`;
    return `
      <div class="metric">
        <div class="metric-top"><span class="metric-label">Budget</span>${warning}</div>
        <div class="meter" data-severity="${Format.severity(used)}">
          <div class="meter-fill" style="width:${left * 100}%"></div>${tick}
        </div>
        <div class="metric-bottom"><span class="metric-value">${Math.round(left * 100)}% left</span><span class="metric-side">${reset}</span></div>
      </div>`;
  },

  // Dashboard (LiteLLM's Usage page) and Status (the org's status page, when configured).
  links(state) {
    const items = [];
    if (state.usage_url) items.push(`<button class="provider-link" data-action="open-usage">Dashboard <span aria-hidden="true">↗</span></button>`);
    if (state.status_url) items.push(`<button class="provider-link" data-action="open-status">Status <span aria-hidden="true">↗</span></button>`);
    return items.length ? `<div class="provider-links">${items.join("")}</div>` : "";
  },

  // Bottom left: auto-refresh countdown + refresh button. Bottom right: settings.
  footer(state) {
    return `
      <div class="footer">
        <span class="footer-status">
          <span id="countdown" class="num">${Format.countdown(state)}</span>
          <button class="icon-button small" data-action="refresh" aria-label="Refresh now">${Icons.refresh}</button>
        </span>
        <button class="icon-button" data-action="settings" aria-label="Settings">${Icons.cog}</button>
      </div>`;
  },

  // --- Settings (cogwheel, or "Change LiteLLM URL…" in the tray menu) -------

  settings(state, { error, draftUrl, theme }) {
    const url = draftUrl ?? (state.proxy_url || state.suggested_url || "");
    const themes = [
      { key: "system", label: "System" },
      { key: "light", label: "Light" },
      { key: "dark", label: "Dark" },
    ];
    const index = Math.max(0, themes.findIndex((t) => t.key === theme));
    const segs = themes
      .map((t) => `<button class="segment ${t.key === theme ? "selected" : ""}" data-action="theme" data-theme="${t.key}">${t.label}</button>`)
      .join("");
    const signOut = state.signed_in
      ? `<div class="card"><button class="row-button destructive" data-action="sign-out">Sign Out</button></div>`
      : "";
    return `
      <div class="section-header-row settings-header">
        <button class="back-button" data-action="close-settings" aria-label="Back">${Icons.back}<span>Back</span></button>
        <div class="section-title">Settings</div>
        <span class="header-spacer"></span>
      </div>
      <div class="card"><div class="stack">
        <label class="field"><span class="metric-label">LiteLLM Address</span>
          <input class="text-input" id="set-url" value="${esc(url)}" placeholder="https://your-litellm-proxy" spellcheck="false" /></label>
        <div class="muted">Changing it signs you out of the old one.</div>
        ${error ? `<div class="muted notice">${esc(error)}</div>` : ""}
        <button class="primary-button" data-action="save-url">Save</button>
      </div></div>
      <div class="card"><div class="stack">
        <span class="metric-label">Appearance</span>
        <div class="segmented" style="--segment-index:${index}"><span class="segment-thumb" aria-hidden="true"></span>${segs}</div>
      </div></div>
      ${signOut}`;
  },

  // --- Sign-in flow --------------------------------------------------------

  signIn(state, error, draftUrl) {
    const url = draftUrl ?? (state.suggested_url || state.proxy_url || "");
    return `
      <div class="section-header-row"><div class="section-title">Claude</div></div>
      <div class="card"><div class="stack">
        <div class="metric-label">Sign In</div>
        <div class="muted">Sign in with your company SSO to see your Claude budget and usage.</div>
        <input class="text-input" id="proxy-url" placeholder="https://your-litellm-proxy" value="${esc(url)}" spellcheck="false" />
        ${error ? `<div class="muted notice">${esc(error)}</div>` : ""}
        <button class="primary-button" data-action="sign-in">Sign In</button>
      </div></div>
      <div class="footer"><span></span><button class="icon-button" data-action="settings" aria-label="Settings">${Icons.cog}</button></div>`;
  },

  waiting(login) {
    return `
      <div class="section-header-row"><div class="section-title">Claude</div></div>
      <div class="card"><div class="stack">
        <div class="metric-label">Finish In Your Browser</div>
        <div class="muted">Sign in with SSO, then enter this code:</div>
        <div class="code">${esc(login.code)}</div>
        <div class="muted waiting-dots">Waiting for you to finish</div>
        <button class="link-button" data-action="open-link">Open Browser Again</button>
      </div></div>
      <div class="footer"><span></span><span class="actions"><button class="link-button" data-action="cancel-login">Cancel</button></span></div>`;
  },

  pickTeam(teams) {
    const buttons = teams
      .map((t) => `<button data-action="team" data-team="${esc(t.team_id)}">${esc(t.team_alias || t.team_id)}</button>`)
      .join("");
    return `
      <div class="section-header-row"><div class="section-title">Claude</div></div>
      <div class="card"><div class="stack">
        <div class="metric-label">Choose Your Team</div>
        <div class="muted">You're in several teams. Pick the one Claude Code uses.</div>
        <div class="team-list">${buttons}</div>
      </div></div>
      <div class="footer"><span></span><span class="actions"><button class="link-button" data-action="cancel-login">Cancel</button></span></div>`;
  },
};
