#!/usr/bin/env python3
"""Writes a signed-in demo state for screenshots on machines without LiteLLM (the Windows CI runner):
the settings file and a fresh usage cache with sample numbers. The cache is marked as just refreshed,
so the app shows it without fetching (and without touching secure storage).

    python3 tools/ci/demo_state.py <config dir> <cache dir>
"""

import datetime
import json
import pathlib
import random
import sys
import time

PROXY = "https://litellm.example.internal"
MODELS = ["claude-sonnet-4-6", "claude-opus-4-8", "claude-haiku-4-5", "gemini-2.5-pro", "gpt-4o", "gpt-4o-mini"]
WEIGHTS = [0.46, 0.22, 0.12, 0.10, 0.07, 0.03]


def totals(spend: float) -> dict:
    return {"spend": round(spend, 4), "tokens": int(spend * 180_000), "requests": int(spend * 30)}


def main() -> None:
    config_dir, cache_dir = (pathlib.Path(p) / "litellm-usage" for p in sys.argv[1:3])
    random.seed(7)
    now = int(time.time())
    today = datetime.date.today()

    days = []
    for back in range(29, -1, -1):
        if back > 1 and random.random() < 0.2:
            continue
        day_spend = random.uniform(0.8, 4.4)
        models = [{"name": n, "totals": totals(day_spend * w * random.uniform(0.7, 1.3))} for n, w in zip(MODELS, WEIGHTS)]
        models.sort(key=lambda m: -m["totals"]["spend"])
        days.append({
            "date": (today - datetime.timedelta(days=back)).isoformat(),
            "totals": totals(sum(m["totals"]["spend"] for m in models)),
            "models": models,
        })

    by_model: dict = {}
    for d in days:
        for m in d["models"]:
            t = by_model.setdefault(m["name"], {"spend": 0.0, "tokens": 0, "requests": 0})
            for k in t:
                t[k] += m["totals"][k]
    last_30d = {k: sum(d["totals"][k] for d in days) for k in ("spend", "tokens", "requests")}

    snapshot = {
        "fetched_at": now,
        "user_id": "demo",
        "budget": {"spend": round(last_30d["spend"] * 0.8, 2), "max_budget": 100.0, "duration": "30d",
                   "reset_at": now + 9 * 86400},
        "today": days[-1]["totals"],
        "yesterday": days[-2]["totals"],
        "last_30d": last_30d,
        "models": sorted(({"name": n, "totals": t} for n, t in by_model.items()), key=lambda m: -m["totals"]["spend"]),
        "days": days,
    }
    config_dir.mkdir(parents=True, exist_ok=True)
    cache_dir.mkdir(parents=True, exist_ok=True)
    (config_dir / "config.json").write_text(json.dumps(
        {"proxy_url": PROXY, "user_id": "demo", "team_id": None, "signed_in_at": now, "status_url": "https://status.example.internal"},
        indent=2))
    (cache_dir / "usage.json").write_text(json.dumps({"proxy_url": PROXY, "snapshot": snapshot, "error": None, "last_attempt": now}))
    print(f"demo state in {config_dir} and {cache_dir}")


if __name__ == "__main__":
    main()
