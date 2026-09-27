# Installing

Everything installs from an offline bundle. The apps load nothing from the internet (fonts, icons and
styles are built in) and only talk to the LiteLLM and status page addresses you configure.

## Supported systems

| | Ubuntu | Windows | macOS |
|---|---|---|---|
| Versions | 22.04 LTS, 24.04 LTS | 10 (1809+) and 11 | 13+ |
| CPU | x86_64, arm64 | x64 | Apple silicon, Intel |
| `ccline` (status line) | ✅ | ✅ | ✅ |
| Tray app | ✅ | ✅ | ✅ |
| Offline installer | ✅ `install.sh` + `.deb` | planned (`install.ps1` + `.msi`) | planned |
| Tested for real | Ubuntu 24.04 desktop (Docker, arm64), bundle built on 22.04 | CI build only | yes |

Older Ubuntu (20.04) isn't supported: it lacks WebKitGTK 4.1, which the tray app needs.

## What each system needs

**Ubuntu**
- Tray app: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`. The `.deb` declares
  them, so `apt` installs whatever is missing from your internal mirror. A standard Ubuntu desktop
  already has most of them; a minimal install pulls in ~70 packages (WebKitGTK and its media stack).
- A desktop with a tray: Ubuntu's default desktop shows tray icons through its built-in
  AppIndicator extension.
- Secure storage for the sign-in token: GNOME Keyring (standard on Ubuntu desktop). Over SSH or on
  servers without a desktop there's no keyring, so sign-in can't be saved there (roadmap item 6).
- `ccline` alone needs nothing beyond the standard C library (glibc 2.35+, i.e. Ubuntu 22.04+).

**Windows**
- Tray app: Microsoft WebView2. Windows 11 and updated Windows 10 have it; the installer carries the
  offline WebView2 installer for machines that don't.
- `ccline`: nothing. Claude Code runs it through Git Bash or PowerShell.
- Sign-in token: Windows Credential Manager (built in).

**macOS**
- Nothing extra. Sign-in token: Keychain. Each new unsigned build asks once for Keychain access.

## Ubuntu: offline bundle

Each bundle (`litellm-usage-ubuntu-<arch>.tar.gz`, built by the **Bundles** GitHub workflow) holds:
`ccline`, the tray `.deb`, `install.sh`, `install.conf`, `README.txt`, `SHA256SUMS`, `ARCH`.

1. **Admin, once:** edit `install.conf`:
   - `LITELLM_URL` — your LiteLLM proxy (the address Claude Code uses as `ANTHROPIC_BASE_URL`)
   - `STATUS_URL` — your status page, or `""` to hide the Status link
   - `INSTALL_TRAY`, `START_TRAY_AT_LOGIN`, `SETUP_CLAUDE_STATUSLINE` — `yes` / `no`

   The installer refuses to run while the example addresses are still there.
2. **Each user:** unpack and run `./install.sh` (not with sudo). It:
   1. checks the files against `SHA256SUMS` and the CPU type,
   2. installs `ccline` to `~/.local/bin` and saves the two addresses,
   3. adds the status line to `~/.claude/settings.json` (backup kept as `settings.json.bak-ccline`;
      it won't replace someone else's status line unless run with `--force`),
   4. installs the tray `.deb` with `sudo apt-get install` (libraries from your mirror) and adds it to
      autostart,
   5. starts the tray and runs `ccline login` — the user signs in with SSO in the browser.

   `--no-login` skips step 5; users can sign in later from the tray panel.
