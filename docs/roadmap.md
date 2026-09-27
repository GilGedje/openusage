# Roadmap

Everything left to do, with how to build it. Tick items off here as they land.

## Done

- LiteLLM API mapped and tested with SSO (`docs/litellm-api.md`)
- `usage-core`: SSO device login, LiteLLM client, token in OS secure storage (split into parts to fit
  Windows' 2,560-byte limit), usage cache, error log
- `ccline`: status line with model, context, budget, today, reset, this model's 30-day spend
  (`docs/ccline.md`)
- Tray app, first version (`docs/tray.md`)
- ccline budget links to LiteLLM's Usage page; background refresh never waits on a Keychain prompt
- Ubuntu offline bundle: `install.sh` + `install.conf` (placeholder addresses) + `.deb` + checksums,
  `ccline setup` (saves addresses, adds the Claude Code status line safely), Bundles workflow
  (`docs/install.md`). Tested end to end on an Ubuntu 24.04 desktop in Docker: install, SSO sign-in,
  tray menu → panel, light and dark (`docs/screenshots/ubuntu-*.png`)
- CI workflow for Windows / Ubuntu / macOS (`.github/workflows/ci.yml`, not yet pushed)
- Verified: macOS end to end; Ubuntu end to end (Docker desktop, arm64); Windows not yet built

## Next: tray app (priority)

First version is in (`docs/tray.md`). Remaining tray items are under "Tray app" below; the ccline
rollout items follow.

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

### 3. Install scripts (offline) — Ubuntu done, Windows next

- Windows: `install.ps1` + `.msi` (WebView2 offline installer is already enabled in the Tauri config).

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

## Tray app

Done: Tauri v2 app on `usage-core`, OpenUsage-style panel (Cost donut with Today / Yesterday /
30 Days, budget meter with pace, daily chart), ring tray icon with %, SSO sign-in in the panel,
Open LiteLLM link, shared cache with ccline.

Still to do:
- **Available Models** section (collapsed) and `ccline models` — `/v1/models` + `/model/info`; open
  question whether to list user- or key-level access.
- **Start at login** — `tauri-plugin-autostart`, a toggle in the panel footer.
- **Rounded, translucent panel on macOS** — needs `macOSPrivateApi` + transparent window.
- **Check on Windows and Ubuntu** — panel placement (Ubuntu has no tray click position, so the panel
  opens top right), tray icon colors, AppIndicator on Ubuntu.
- **Offline installers** per OS (`.msi`, `.deb`, `.dmg`) via `tauri build`, in the same bundle as
  ccline; needs the Tauri CLI at build time.
- **Stable Keychain approval on macOS** — every new unsigned build asks again; code signing (item 5
  above) fixes it.

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
