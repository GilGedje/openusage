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

    stub = f"""<script>
const STATE = {json.dumps(state)};
window.__TAURI__ = {{ core: {{ invoke: async (cmd) => (cmd === "get_state" || cmd === "refresh" ? STATE : null) }},
                      event: {{ listen: async () => {{}} }} }};
try {{ localStorage.setItem("period", new URLSearchParams(location.search).get("period") || "today"); }} catch {{}}
</script>"""
    html = (OUT / "index.html").read_text()
    html = html.replace('<script src="format.js">', stub + '\n    <script src="format.js">')
    html = html.replace("</head>", "<style>body{width:320px;margin:0}</style></head>")
    (OUT / "index.html").write_text(html)
    print(f"Preview ready in {OUT}")


if __name__ == "__main__":
    main()
