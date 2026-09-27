#!/usr/bin/env python3
"""Builds a single self-contained HTML page with the real tray panel (crates/tray/ui) running on
sample data, for showing and trying the design in a browser (e.g. published as an artifact).

    python3 tools/preview/make_web_demo.py <output.html>

Everything is inlined (styles, scripts, the Claude icon), so the page needs no network.
"""

import base64
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
UI = ROOT / "crates/tray/ui"

STUB = r"""
// ---- Demo bridge: stands in for the app (Tauri) with sample data ------------------------------
(function () {
  const MODELS = ["claude-sonnet-4-6", "claude-opus-4-8", "claude-haiku-4-5", "gemini-2.5-pro", "gpt-4o", "gpt-4o-mini"];
  const WEIGHTS = [0.46, 0.22, 0.12, 0.1, 0.07, 0.03];
  let seed = 11;
  const rand = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const iso = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const now = () => Math.floor(Date.now() / 1000);
  const t = (spend) => ({ spend, tokens: Math.round(spend * 180000), requests: Math.round(spend * 30) });

  const days = [];
  const today = new Date();
  for (let back = 29; back >= 0; back--) {
    if (back > 1 && rand() < 0.2) continue;
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - back);
    const daySpend = 0.8 + rand() * 3.6;
    const models = MODELS.map((name, i) => ({ name, totals: t(daySpend * WEIGHTS[i] * (0.7 + rand() * 0.6)) }))
      .sort((a, b) => b.totals.spend - a.totals.spend);
    const spend = models.reduce((s, m) => s + m.totals.spend, 0);
    days.push({ date: iso(d), totals: t(spend), models });
  }
  const byName = {};
  for (const d of days) for (const m of d.models) {
    const x = (byName[m.name] ||= { spend: 0, tokens: 0, requests: 0 });
    x.spend += m.totals.spend; x.tokens += m.totals.tokens; x.requests += m.totals.requests;
  }
  const models = Object.entries(byName).map(([name, totals]) => ({ name, totals })).sort((a, b) => b.totals.spend - a.totals.spend);
  const last30 = days.reduce((a, d) => ({ spend: a.spend + d.totals.spend, tokens: a.tokens + d.totals.tokens, requests: a.requests + d.totals.requests }), { spend: 0, tokens: 0, requests: 0 });

  const SIGNED_IN = {
    signed_in: true, user_id: "demo", proxy_url: "https://litellm.example.internal",
    suggested_url: "https://litellm.example.internal", usage_url: "#", status_url: "#", ca_cert: null,
    refresh_secs: 300, version: "0.1.0", now: now(),
    cache: {
      proxy_url: "https://litellm.example.internal", error: null, last_attempt: now() - 83,
      snapshot: {
        fetched_at: now() - 83, user_id: "demo",
        budget: { spend: Math.round(last30.spend * 0.8 * 100) / 100, max_budget: 100, duration: "30d", reset_at: now() + 9 * 86400 },
        today: days[days.length - 1].totals, yesterday: days[days.length - 2].totals, last_30d: last30, models, days,
      },
    },
  };
  let state = JSON.parse(JSON.stringify(SIGNED_IN));
  const listeners = {};
  const emit = (name, payload) => listeners[name] && listeners[name]({ payload });

  window.__TAURI__ = {
    core: {
      invoke: async (cmd, args) => {
        state.now = now();
        switch (cmd) {
          case "get_state": case "save_settings": return state;
          case "refresh":
            await new Promise((r) => setTimeout(r, 700));
            state.cache.last_attempt = now(); state.cache.snapshot.fetched_at = now();
            return state;
          case "sign_out": state = { ...state, signed_in: false }; return state;
          case "start_login":
            setTimeout(() => { state = JSON.parse(JSON.stringify(SIGNED_IN)); state.now = now(); emit("login", { status: "done" }); emit("state", state); }, 3500);
            return { code: "QX7M-4KD2", link: "#", proxy_url: state.proxy_url };
          case "save_image": {
            const blob = new Blob([new Uint8Array(args.bytes)], { type: "image/png" });
            window.dispatchEvent(new CustomEvent("demo-export", { detail: URL.createObjectURL(blob) }));
            return "Downloads/" + args.name + ".png";
          }
          default: return null;
        }
      },
    },
    event: { listen: async (name, fn) => { listeners[name] = fn; } },
  };

  // The real app refreshes every 5 minutes; keep the demo's countdown going.
  setInterval(() => {
    if (state.signed_in && now() - state.cache.last_attempt >= state.refresh_secs) {
      state.cache.last_attempt = now(); state.now = now(); emit("state", state);
    }
  }, 1000);
})();
"""

PAGE_CSS = r"""
/* Page around the panel (the panel itself uses theme.css + panel.css unchanged). */
:root {
  --desk: #e6e9ee;
  --desk-ink: #1d2127;
  --desk-muted: #5d6673;
  --frame-shadow: 0 18px 50px rgba(20, 28, 40, 0.16), 0 2px 6px rgba(20, 28, 40, 0.08);
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --desk: #121417;
    --desk-ink: #e8eaee;
    --desk-muted: #9aa3ae;
    --frame-shadow: 0 18px 50px rgba(0, 0, 0, 0.55), 0 2px 6px rgba(0, 0, 0, 0.4);
  }
}
:root[data-theme="dark"] {
  --desk: #121417;
  --desk-ink: #e8eaee;
  --desk-muted: #9aa3ae;
  --frame-shadow: 0 18px 50px rgba(0, 0, 0, 0.55), 0 2px 6px rgba(0, 0, 0, 0.4);
}
html, body { background: var(--desk); }
body.demo-page { margin: 0; padding-inline: 16px; padding-block: 28px 40px; color: var(--desk-ink); }
.demo {
  display: grid;
  justify-items: center;
  gap: 18px;
  max-width: 720px;
  margin: 0 auto;
}
.demo-head { text-align: center; display: grid; gap: 6px; }
.demo-head h1 { margin: 0; font-size: 20px; font-weight: 650; letter-spacing: -0.01em; text-wrap: balance; }
.demo-head p { margin: 0; color: var(--desk-muted); font-size: 13px; max-width: 46ch; line-height: 1.45; }
.try { display: flex; flex-wrap: wrap; justify-content: center; gap: 6px; max-width: 520px; }
.try span {
  padding: 4px 9px; border-radius: 999px; font-size: 12px; color: var(--desk-muted);
  border: 1px solid color-mix(in srgb, var(--desk-muted) 30%, transparent);
}
.frame {
  width: 320px; max-width: 100%;
  border-radius: 12px; overflow: hidden;
  background: var(--tray);
  box-shadow: var(--frame-shadow);
}
.frame .panel { width: 100%; }
.frame { position: relative; transform: translateZ(0); } /* the panel's toast stays inside the frame */
.export { display: grid; justify-items: center; gap: 8px; width: 100%; }
.export[hidden] { display: none; }
.export h2 { margin: 0; font-size: 13px; font-weight: 600; color: var(--desk-muted); }
.export img { width: 360px; max-width: 100%; border-radius: 10px; box-shadow: var(--frame-shadow); }
.demo-note { color: var(--desk-muted); font-size: 12px; }
"""


def main() -> None:
    out = pathlib.Path(sys.argv[1])
    icon = "data:image/png;base64," + base64.b64encode((UI / "claude.png").read_bytes()).decode()
    css = (ROOT / "design/theme.css").read_text() + (UI / "panel.css").read_text() + PAGE_CSS
    views = (UI / "views.js").read_text().replace('src="claude.png"', f'src="{icon}"')
    scripts = "\n".join([STUB, (UI / "format.js").read_text(), views, (UI / "share.js").read_text(), (UI / "app.js").read_text()])
    scripts += r"""
window.addEventListener("demo-export", (e) => {
  const box = document.getElementById("export");
  box.querySelector("img").src = e.detail;
  box.hidden = false;
});
"""
    html = f"""<title>Quota Tray Panel</title>
<style>{css}</style>
<div class="demo">
  <header class="demo-head">
    <h1>Quota by Exodus.Ai</h1>
    <p>The tray panel as it ships, running here on sample numbers.</p>
  </header>
  <div class="try" aria-label="Things to try">
    <span>Switch Today / Yesterday / 30 Days</span>
    <span>Export the Cost card</span>
    <span>Open Settings (cogwheel)</span>
    <span>Change Appearance</span>
    <span>Sign out and back in</span>
  </div>
  <div class="frame"><main class="panel" id="app"></main></div>
  <section class="export" id="export" hidden>
    <h2>Exported image</h2>
    <img alt="The Cost card exported as an image" />
  </section>
  <p class="demo-note">Sample data. In the app, the numbers come from your LiteLLM proxy.</p>
</div>
<script>
document.body.classList.add("demo-page");
{scripts}
</script>
"""
    out.write_text(html)
    print(f"{out} ({len(html) // 1024} KB)")


if __name__ == "__main__":
    main()
