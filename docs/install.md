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

## Ubuntu: offline installers

There are two independent folders (built by the **Bundles** GitHub workflow, one download each). Users
can install either or both; they share one sign-in and one set of addresses.

| Folder | Contents | Installs | Needs sudo |
|---|---|---|---|
| `litellm-usage-tray-ubuntu-<arch>/` | tray `.deb`, `install.sh`, `install.conf`, `README.txt`, `SHA256SUMS`, `ARCH` | the tray app, autostart at login | yes (apt) |
| `ccline-ubuntu-<arch>/` | `ccline`, `install.sh`, `install.conf`, `README.txt`, `SHA256SUMS`, `ARCH` | `ccline` in `~/.local/bin`, Claude Code status line | no |

1. **Admin, once:** edit each folder's `install.conf`:
   - tray: `LITELLM_URL`, `STATUS_URL` (or `""` to hide the Status link), `START_AT_LOGIN`
   - ccline: `LITELLM_URL`, `SETUP_CLAUDE_STATUSLINE`

   Each installer refuses to run while the example addresses are still there. `SHA256SUMS` covers
   every file except `install.conf`, so editing it doesn't break the check.
2. **Each user:** unpack a folder and run `./install.sh` (not with sudo).
   - **Tray:** checks the files and CPU type, installs the `.deb` with `sudo apt-get install`
     (libraries from your mirror), saves the addresses (`litellm-usage --configure`), adds autostart,
     and opens the panel — the user clicks **Sign In** and finishes in the browser.
   - **ccline:** checks the files, installs `ccline`, saves the address, adds the status line to
     `~/.claude/settings.json` (backup kept as `settings.json.bak-ccline`; `--force` replaces someone
     else's status line), then signs in with `ccline login` — skipped when already signed in through
     the tray. `--no-login` skips it.

Tested end to end on an Ubuntu 24.04 desktop (screenshots in `docs/screenshots/ubuntu-*.png`):
placeholder guard, install, sign-in from the panel (including a timed-out code and retry), tray
menu → panel, light and dark, then the ccline folder reusing the tray's sign-in.
