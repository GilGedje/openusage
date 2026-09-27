# Roadmap

Everything left to do, with how to build it. Tick items off here as they land.

## Done

- LiteLLM API mapped and tested with SSO (`docs/litellm-api.md`)
- `usage-core`: SSO device login, LiteLLM client, token in OS secure storage (split into parts to fit
  Windows' 2,560-byte limit), usage cache, error log
- `ccline`: status line with model, context, budget, today, reset, this model's 30-day spend
  (`docs/ccline.md`)
- CI workflow for Windows / Ubuntu / macOS (`.github/workflows/ci.yml`, not yet pushed)
- Verified: macOS end to end; Ubuntu compiles and passes tests (Docker)

## Rolling out ccline (install script, air-gapped network)

Users install with one script. No step may reach the internet: everything comes from a bundle you
carry into the air-gapped network, and ccline only talks to the LiteLLM proxy.

### 1. Find the proxy URL automatically

Users' Claude Code already points at LiteLLM (`ANTHROPIC_BASE_URL`), so ccline should reuse it and
`ccline login` should need no `--url`.

- Look in this order, first hit wins:
  1. `--url`
  2. `ANTHROPIC_BASE_URL` in the environment
  3. Managed settings `env` — `managed-settings.json` then `managed-settings.d/*.json` (alphabetical)
     in `/Library/Application Support/ClaudeCode/` (macOS), `/etc/claude-code/` (Linux),
     `C:\Program Files\ClaudeCode\` (Windows)
  4. User settings `env` — `$CLAUDE_CONFIG_DIR/settings.json`, else `~/.claude/settings.json`
  5. The proxy saved at the last sign-in
- MDM-delivered settings (macOS profile, Windows registry) aren't read; if you deploy that way, add
  them later.
- Normalize as today (drop `/v1`, `/anthropic`, trailing slash).
- Tests: fixture settings files for each source and the precedence order.
- **Check in the air-gapped network:** the URL Claude Code uses must also reach LiteLLM's `/sso/*` and
  `/v2/user/info` routes. If a gateway in front only forwards model routes, ccline needs the proxy's
  own address instead.

### 2. `ccline install` / `ccline uninstall`

Settings edits live in the binary (not in shell scripts) so they behave the same on every OS.

- `install`:
  1. Copy itself to `~/.local/bin/ccline` (macOS/Linux) or `%LOCALAPPDATA%\Programs\ccline\ccline.exe`
     (Windows).
  2. Back up `~/.claude/settings.json` to `settings.json.bak-ccline`, then add `statusLine`
     (`{"type":"command","command":"<absolute path, forward slashes>","refreshInterval":30}`),
     keeping every other key. Write atomically.
  3. If a different `statusLine` is already set, stop and ask (`--force` replaces it).
  4. If managed settings set `statusLine`, warn that theirs wins.
  5. Run `ccline login`.
- `uninstall`: remove `statusLine` only if it points at ccline, delete the saved sign-in and cache,
  remove the binary.
- Tests: settings merge with an existing file, missing file, existing foreign status line.

### 3. Install scripts (offline)

- `install.sh` (macOS, Linux) and `install.ps1` (Windows), shipped inside the bundle next to the
  binaries and a `SHA256SUMS` file.
- Each script: detect OS and CPU → pick the binary → verify its checksum → run `<binary> install`.
- Accept a bundle folder (default: the script's own folder) or an internal mirror URL (file share,
  Artifactory/Nexus). Never download from the internet.
- macOS: strip the quarantine flag (`xattr -d com.apple.quarantine`). Windows: `Unblock-File`.
- Test on a clean VM per OS.

### 4. Release bundle

- CI job on a version tag builds:
  - `ccline-macos-arm64`, `ccline-macos-x86_64`
  - `ccline-linux-x86_64`, `ccline-linux-arm64` — static (musl) so they run on any Ubuntu version
  - `ccline-windows-x86_64.exe`
- Plus `SHA256SUMS`, both install scripts and a short README, zipped as one bundle to carry into the
  air-gapped network.
- Linux static builds: `cargo zigbuild` or `cross` for the musl targets.
- Version numbers need owner approval (see AGENTS.md).

### 5. Code signing (later)

- Windows: Authenticode certificate, so SmartScreen and antivirus trust the `.exe`.
- macOS: Developer ID signing + notarization. Notarization needs Apple's servers, so do it at build
  time, outside the air-gapped network.
- Inside an air-gapped network with files copied from a share, unsigned binaries mostly work; signing
  matters if IT policy requires it.

### 6. Headless Linux (decide)

- On servers or SSH sessions without a desktop there's no Secret Service, so ccline shows `secure
  storage unavailable`. Option: fall back to the Linux kernel keyring (keyutils) — secure, but the
  token is lost at reboot/logout. Needs a decision on whether headless users matter.

## Tray app (after ccline rollout)

- Tauri v2 app reusing `usage-core`; UI styled with `design/theme.css`.
- Tray icon with budget %; popup with budget bar, today, per-model spend, daily chart, sign-in and
  sign-out, refresh.
- Start at login; background refresh shares ccline's cache so the two never fetch twice.
- Ubuntu tray needs AppIndicator (built into Ubuntu's desktop).
- Offline installers per OS (`.msi`, `.deb`, `.dmg`) in the same bundle.

## Still to verify

- ccline inside a real Claude Code session on each OS (colors, width, refresh while idle)
- Token expiry shows "signed out" (needs a 24-hour-old token)
- Windows end to end (only compiled in CI so far)
- Ubuntu desktop sign-in with GNOME Keyring

## Open questions

- Which limit actually stops users: user, key, or team budget? ccline shows the user budget; key and
  team budgets are available if needed.
- Proxy settings to add: `LITELLM_CLI_JWT_EXPIRATION_HOURS` (longer sign-ins) and
  `allow_cli_sso_verification_uri_complete` (one-click re-login).
- Are Ubuntu users on desktops, or also over SSH / headless? (Decides item 6.)
