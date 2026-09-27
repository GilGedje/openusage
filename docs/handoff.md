# Handoff

Everything needed to continue this project — what it is, what the owner decided, how it's built,
how to test and ship it, what bit us, and what's next. Read this first, then `AGENTS.md`.

Last updated: 2026-09-27 · branch `litellm-usage` (GitHub `GilGedje/openusage`, public fork).

## 1. What this is

The owner's users run **Claude Code as a harness** against a **LiteLLM proxy** on an
**air-gapped network**. LiteLLM holds budgets, spend and per-model usage; the model names users pick
in Claude Code (`sonnet`, `opus`, …) are LiteLLM aliases that may point at other models. This project
shows each user their LiteLLM budget and usage:

- **`ccline`** — a Claude Code status line (`docs/ccline.md`)
- **Tray app** — a tray icon + popup panel in OpenUsage's look (`docs/tray.md`)

It started as a fork of [OpenUsage](https://github.com/robinebers/openusage) (MIT, a macOS Swift app).
All original code was removed; only its look (ported to `design/theme.css`) and some conventions
remain. It is **not** OpenUsage and must not use its name or logo.

## 2. Owner decisions (don't re-litigate)

| Topic | Decision |
|---|---|
| Platforms | **Ubuntu 20.04, 22.04, 24.04, 26.04 and Windows** are the targets (Ubuntu first); macOS works too |
| Offline | Installers must contain **every dependency** — users can't fetch packages (no apt mirror) |
| Data source | **LiteLLM only** — never Anthropic's API |
| Sign-in | **SSO only**, LiteLLM's device flow. **No API keys**, and never silently create one |
| Token storage | OS secure storage only (Keychain / Credential Manager / GNOME Keyring). Never a plain file |
| Token lifetime | Proxy sets `LITELLM_CLI_JWT_EXPIRATION_HOURS=336` (env) and `allow_cli_sso_verification_uri_complete: true` (config.yaml) — verified working with no code change; set on the owner's local proxy (backups `*.bak-20260927-191038` in `~/Exodus-Ai Project/litellm-local`) |
| Network | **Everything local/offline**: no fonts, icons or scripts from the internet; apps only talk to the configured LiteLLM + status page |
| Rollout | **Install scripts** with placeholder addresses (not MDM), shipped as offline folders |
| Packaging | **Two separate folders**: tray app and `ccline`, each with its own installer and settings |
| Look | **Match OpenUsage's panel**: Cost card (Today / Yesterday / 30 Days switch + donut + legend), then the account card, Dashboard / Status links, footer with Refresh Now |
| Links | **Dashboard** → LiteLLM's Usage page; **Status** → the org's status page (like status.claude.com) |
| Language | **Rust** (shared core with the Tauri tray app) |
| Certificates | Trust the org CA (system store + custom CA path); **never** disable verification |
| Tray clicks | Left click opens the panel; right click = menu (Open, Refresh, Change LiteLLM URL…, Quit) |
| Name | Product is **Quota by Exodus.Ai** (visible names). Internal ids stay `litellm-usage` (binary, settings folder, keychain service, env vars) so existing installs keep their sign-in |
| Versions | Never bump or tag a version without explicit owner approval |

## 3. Layout

```
crates/usage-core/   Shared library: LiteLLM client, SSO, secure storage, cache, refresh
crates/ccline/       Status line binary (+ login/logout/status/setup commands)
crates/tray/         Tauri 2 tray app for 22.04+ (Rust side in src/, panel UI in ui/ — shared)
crates/tray-legacy/  Tauri 1 tray for Ubuntu 20.04 (WebKitGTK 4.0); own workspace, same ui/ and core
crates/sni-tray/     Linux tray icon over StatusNotifierItem (left click → panel), used by both trays
design/theme.css     Colors, light/dark, sizes — copied into the tray UI at build time
installer/linux/     tray/ and ccline/ installers, fetch-deps.sh (offline package set), make-bundle.sh
tools/ubuntu-test/   Docker build + Ubuntu desktop test harness
tools/preview/       Browser preview of the panel from real cached data
.github/workflows/   ci.yml (Win/Ubuntu/macOS build+test), release.yml ("Bundles": Ubuntu folders)
docs/                User docs, LiteLLM notes, roadmap, this handoff
```

### How data flows

1. **Sign-in** (`usage-core/src/sso.rs`, `account.rs`): `POST /sso/cli/start` → browser opens
   `/sso/key/generate?source=litellm-cli&key=<login_id>` → user signs in with SSO and types the code →
   app polls `GET /sso/cli/poll/<login_id>` (header `x-litellm-cli-poll-secret`) → gets a token.
   Multiple teams → poll again with `?team_id=`.
2. **Storage**: token → OS secure storage via the `keyring` crate, **split into ≤2,000-byte parts**
   (`<proxy>`, `<proxy>#2`, …) because Windows caps a credential at 2,560 bytes. Non-secret fields
   (proxy URL, user, team, sign-in time, status URL) → `litellm-usage/config.json` in the OS config
   folder.
3. **Fetch** (`api.rs`, `snapshot.rs`): `GET /v2/user/info` (budget) + `GET
   /user/daily/activity/aggregated` (30 days, per day, per model) → one `Snapshot` (budget, today,
   yesterday, 30 days, per-model, per-day-per-model).
4. **Cache** (`cache.rs`, `refresh.rs`): `litellm-usage/usage.json` in the OS cache folder, shared by
   `ccline` and the tray. Refresh at most once a minute; a lock file stops concurrent refreshes; on
   failure the old snapshot stays with an error + age. Errors also go to `error.log` next to it.
5. **Render**: `ccline` prints from the cache instantly and starts a detached `ccline refresh` when
   it's stale (never waits on the network). The tray polls every 15s and pushes a `state` event to
   the panel.

### LiteLLM facts that matter (verified live on 1.99.1; see `docs/litellm-api.md`)

- The SSO token works on `/v2/user/info`, `/user/daily/activity*`, `/user/spend/report`,
  `/v1/models`, `/model/info`, `/spend/logs/v2` — **not** `/key/info` (404: it isn't a stored key).
- Non-admins must pass their own `user_id` to daily activity. `timezone` is minutes **behind** UTC
  (JS sign: Israel summer = `-180`); add `include_current_utc_day=true` to include today.
- `breakdown.model_groups` = names users pick in Claude Code (show these); `breakdown.models` = real
  backend models.
- Budget `spend` is the current window only; `budget_reset_at` is when it ends.
- LiteLLM's own Usage page: `<proxy>/ui/?page=new_usage`.
- The token is encrypted (can't read its expiry); a 401 means signed out.

## 4. Build and test

Toolchain: Rust stable (`brew install rustup && rustup default stable` on macOS). Everything below runs
from the repo root.

| Task | Command |
|---|---|
| Tests (26, incl. fixtures in `crates/usage-core/tests/fixtures/`) | `cargo test` |
| Lint (CI uses `-D warnings`) | `cargo clippy --all-targets -- -D warnings` |
| ccline | `cargo build --release -p ccline` → `target/release/ccline` |
| Tray (dev) | `cargo run -p litellm-usage-tray` (`LITELLM_USAGE_OPEN_PANEL=1` opens the panel at start) |
| Tray installers (.deb/.msi/.dmg) | from `crates/tray`: `npx --yes @tauri-apps/cli@2 build --bundles deb` (needs Node); 20.04: from `crates/tray-legacy` with `@tauri-apps/cli@1` inside ubuntu:20.04 |
| All Ubuntu folders (Docker) | `tools/ubuntu-test/build-all.sh` → `.ubuntu-test/dist/` |
| Real desktop / offline test | `tools/ubuntu-test/README.md` |
| Panel in a browser (real data) | `tools/preview/make_preview.py` |

Linux build needs: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
libssl-dev build-essential`.

## 5. Ship

- **CI** (`ci.yml`): every push to `litellm-usage`/`main` — clippy, tests, release build on
  Windows, Ubuntu, macOS. All green as of `efc08a0`.
- **Bundles** (`release.yml`): every push to `litellm-usage`, tags `v*`, or manual. For x86_64 and
  arm64: `ccline` built in ubuntu:20.04 (runs on 20.04+); the Tauri 2 tray `.deb` built on 22.04; the
  Tauri 1 tray `.deb` built in ubuntu:20.04; then, inside a container of **each** release (20.04,
  22.04, 24.04, 26.04), `fetch-deps.sh` collects the full dependency closure into a local apt repo and
  `make-bundle.sh` assembles `quota-tray-ubuntu-<release>-<arch>`. Tags attach everything to
  a GitHub Release.
- **How offline install works:** `packages/` has the app, all libraries and a `Packages` index;
  `install.sh` runs apt with a throwaway config (`Dir::Etc::SourceList` = only that folder, temp
  lists/cache), `install --no-remove`, so apt installs only what's missing/older, never downloads,
  never removes, and leaves the machine's apt state alone.
- Admin edits each folder's `install.conf` (placeholders `https://litellm.example.internal`,
  `https://status.example.internal` — the installers refuse to run until changed), then hands folders
  to users. Full steps: `docs/install.md`.

## 6. Configuration reference

| What | Where |
|---|---|
| LiteLLM + status page addresses, who's signed in | `litellm-usage/config.json` in the OS config folder (`~/.config` on Ubuntu, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows) — written by the installers (`ccline setup` / `litellm-usage --configure`) |
| Status page override | env `LITELLM_USAGE_STATUS_URL` (wins over the file) |
| Default LiteLLM address for sign-in | `--url`, else env `ANTHROPIC_BASE_URL`, else the last one used |
| Claude Code status line | `statusLine` in `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR`), set by `ccline setup`, backup `settings.json.bak-ccline`, `refreshInterval: 30` |
| Cache, error log | `litellm-usage/usage.json`, `error.log` in the OS cache folder |
| Token | OS secure storage, service `litellm-usage`, account = proxy URL (+ `#2`… parts) |

## 7. Gotchas we hit (read before debugging)

- **macOS Keychain asks again after every rebuild** (unsigned binary changes identity). Background
  refreshes now **never prompt** (`secret::disallow_prompts`) and stop after 45s; the status line then
  says "run `ccline status` to allow Keychain access" — run it in a terminal and choose Always Allow.
  Before this fix, stuck refreshes piled up one per minute.
- **Replacing a signed binary on macOS with `cp` over the old file gets it SIGKILLed** (exit 137).
  Always `rm` then `cp` (the installers do).
- **This dev Mac's `/etc/hosts` has no `localhost` line**, so non-browser tools can't resolve it. The
  client maps `localhost` → `127.0.0.1` itself (as browsers do).
- **Windows Credential Manager** caps a secret at 2,560 bytes, halved for UTF-16 passwords → token is
  stored as split UTF-8 parts.
- **WebKitGTK** shows blank windows in VMs/some GPUs → the tray sets `WEBKIT_DISABLE_DMABUF_RENDERER=1`
  on Linux unless already set.
- **Tauri 1 and 2 can't share one Cargo workspace** (GTK/WebKit `-sys` crates link the same native
  libs) → `crates/tray-legacy` is excluded from the root workspace. Tauri 1's tooling can't parse
  `edition = "2024"` (legacy crate uses 2021), needs a `custom-protocol` feature, and expects the
  binary to be named after the crate (crate is named `litellm-usage`).
- **Tauri's `.deb` lists its binary as `usr/bin/…`** (no leading slash) — match accordingly.
- **Docker builds OOM** compiling GTK with LTO when two builds run at once — use `CARGO_BUILD_JOBS=2`.
- **Ubuntu 20.04's standard desktop already ships WebKitGTK 4.0** (yelp, GNOME online accounts);
  24.04 dropped 4.0; 26.04 has 4.1 (`libgtk-3-0t64` provides `libgtk-3-0`).
- **AppIndicator (Tauri's Linux tray) sends no click events**, so both trays use `crates/sni-tray`
  (StatusNotifierItem via `ksni`): left click → `activate(x, y)` → panel next to the click; right
  click → menu. If the desktop has no StatusNotifierItem host, they fall back to AppIndicator (menu
  only, panel at the top right).
- **TLS**: roots = system store (`rustls-native-certs`) + optional CA file (`usage-core/src/tls.rs`);
  untrusted/name-mismatch errors map to `Error::Certificate` with fix-it text.
- **Tauri's CLI rewrites `crates/tray/Cargo.toml`** formatting on build — harmless.
- Tooling on the dev Mac: screenshots can't capture app windows (no Screen Recording permission) — use
  the browser preview or the Docker desktop; the Chrome extension wasn't connected (Playwright works);
  zsh treats `?` in URLs as a glob — quote them.
- `WebFetch`/sandboxed tools can't reach `localhost`; Docker containers reach the host's LiteLLM at
  `host.docker.internal:4000`.

## 8. Verified so far

| | macOS | Ubuntu | Windows |
|---|---|---|---|
| Build + tests (CI) | ✅ | ✅ | ✅ |
| ccline end to end | ✅ | ✅ (Docker desktop) | ❌ not run |
| Tray end to end | ✅ (preview + running) | ✅ install, panel sign-in, menu → panel, light/dark | ❌ not run |
| Installer | — | ✅ offline on 20.04, 22.04, 24.04, 26.04 (no network, no apt lists) | ❌ not written |

Screenshots: `docs/screenshots/ubuntu-*.png`.

## 9. Open questions for the owner

1. **Status page address** — to make it the default in `install.conf`.
2. **"Used" vs "left"** — status line and macOS tray title show % used (63%); the panel shows % left
   (37%, like OpenUsage). Pick one for all.
3. **User, key, or team?** Which budget/limit actually stops users, and whose model access to list.
   (The owner's key allows only two Gemini models while the user allows all — they differ.)
4. **Folder size** — tray folders are 210–350 MB because they carry the full dependency chain. Could
   shrink by leaving out packages every Ubuntu desktop has, at some risk on very old machines.
5. **Headless Ubuntu** (SSH/servers) — no keyring there; support it (e.g. kernel keyring) or not?
6. **Repo name** — OpenUsage's trademark policy says forks must not be called "OpenUsage"; rename the
   GitHub repo and product before wide sharing.

## 10. Next steps (priority order)

1. **Windows installer**: `installer/windows/{tray,ccline}` with `install.ps1` + `install.conf`
   mirroring Linux; tray via `tauri build --bundles msi` (WebView2 offline installer already enabled);
   ccline to `%LOCALAPPDATA%\Programs\ccline\` with `ccline setup` (forward-slash path). Add a Windows
   job to `release.yml`. Note: the tray's `--configure` prints nothing on Windows release builds
   (GUI subsystem) — rely on the exit code.
2. **Proxy address from Claude Code's settings** (`docs/roadmap.md` item 1) so `ccline login` needs no
   `--url` when `ANTHROPIC_BASE_URL` is only in `~/.claude/settings.json` or managed settings.
3. **Available Models** section + `ccline models` (after question 3).
4. **Start at login** toggle in the panel; **uninstall** scripts.
5. **Code signing** (Windows Authenticode, macOS Developer ID) — also stops the Keychain re-prompts.

Full list with implementation notes: `docs/roadmap.md`.

## 11. State of the owner's dev Mac (as left)

- `~/.local/bin/ccline` installed; `statusLine` in `~/.claude/settings.json` (backup
  `settings.json.bak-ccline`).
- Signed in to `http://localhost:4000` (user `gilgedje`, $50 / 30-day budget).
- A dev build of the tray may be running (`target/debug/litellm-usage`).
- Docker: container `ubuntu-desktop` (test desktop, still running), volumes `ubuntu-build-cache`,
  `ubuntu-build-cargo`, `ubuntu-build-rustup`, images `litellm-usage-desktop`, `ubuntu:22.04`.
- Local LiteLLM: `http://localhost:4000` (SSO via Authentik at `auth.localhost:59090`).
