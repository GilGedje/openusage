#!/usr/bin/env python3
"""Browser preview of the tray panel, fed from the real usage cache.

Copies crates/tray/ui + design/theme.css into .playwright-mcp/preview/ and stubs the Tauri bridge
with the cached state, so the panel renders in any browser (useful where screenshots can't see app
windows). Then serve it and open http://127.0.0.1:8765/index.html?period=today|yesterday|30d

    python3 tools/preview/make_preview.py
    cd .playwright-mcp/preview && python3 -m http.server 8765 --bind 127.0.0.1
"""

import json
import pathlib
import shutil
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / ".playwright-mcp" / "preview"
CACHE_CANDIDATES = [
    pathlib.Path.home() / "Library/Caches/litellm-usage/usage.json",  # macOS
    pathlib.Path.home() / ".cache/litellm-usage/usage.json",  # Linux
]


def main() -> None:
    cache_path = next((p for p in CACHE_CANDIDATES if p.exists()), None)
    if cache_path is None:
        sys.exit("No usage cache yet: sign in with `ccline login` and run `ccline status` first.")
    cache = json.loads(cache_path.read_text())
    proxy = cache["proxy_url"]
    state = {
        "signed_in": True,
        "user_id": (cache.get("snapshot") or {}).get("user_id"),
        "proxy_url": proxy,
        "suggested_url": proxy,
        "usage_url": proxy + "/ui/?page=new_usage",
        "status_url": "https://status.example.com",
        "cache": cache,
        "now": int(time.time()),
    }

    shutil.rmtree(OUT, ignore_errors=True)
    shutil.copytree(ROOT / "crates/tray/ui", OUT)
    shutil.copy(ROOT / "design/theme.css", OUT / "theme.css")

    stub = """<script>
const STATE = __STATE__;
const Q = new URLSearchParams(location.search);
STATE.cache.error = null;                     // clean shots: hide this machine's stale-data banner
STATE.cache.last_attempt = Math.floor(Date.now() / 1000) - 83;
if (Q.get("view") === "signin") STATE.signed_in = false;
const listeners = {};
window.__TAURI__ = {
  core: { invoke: async (cmd, args) => {
    switch (cmd) {
      case "get_state": case "refresh": case "save_settings": return STATE;
      case "sign_out": return { ...STATE, signed_in: false };
      case "start_login": return { code: "KS5Y-YGXQ", link: "#", proxy_url: STATE.proxy_url };
      case "save_image": return "/home/you/Downloads/" + args.name + ".png";
      case "save_alerts":
        STATE.alerts = { warning_pct: args.warningPct, warning_color: args.warningColor, critical_pct: args.criticalPct, critical_color: args.criticalColor };
        setTimeout(() => window.__emit("state", STATE)); // like the app's publish
        return STATE;
      default: return null;
    }
  } },
  event: { listen: async (name, fn) => { listeners[name] = fn; } },
};
window.__emit = (name, payload) => listeners[name] && listeners[name]({ payload });
try { localStorage.setItem("period", Q.get("period") || "today"); localStorage.setItem("theme", Q.get("theme") || "system"); } catch {}
</script>""".replace("__STATE__", json.dumps(state))
    html = (OUT / "index.html").read_text()
    html = html.replace('<script src="format.js">', stub + '\n    <script src="format.js">')
    html = html.replace("</head>", "<style>body{width:320px;margin:0}</style></head>")
    # Cache-bust so a browser never mixes old and new scripts.
    stamp = str(int(time.time()))
    for f in ("format.js", "views.js", "alerts.js", "share.js", "app.js", "theme.css", "panel.css", "alerts.css"):
        html = html.replace(f'"{f}"', f'"{f}?v={stamp}"')
    (OUT / "index.html").write_text(html)
    print(f"Preview ready in {OUT}")


if __name__ == "__main__":
    main()
