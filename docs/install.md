# Installing

Everything installs from offline folders that already contain every dependency. Nothing is
downloaded — not during install, not at runtime. The apps load no fonts, icons or scripts from the
internet and only talk to the LiteLLM and status page addresses you configure.

## Supported systems

| | Ubuntu | Windows | macOS |
|---|---|---|---|
| Versions | 20.04, 22.04, 24.04, 26.04 LTS | 10 (1809+) and 11 | 13+ |
| CPU | x86_64, arm64 | x64 | Apple silicon, Intel |
| `ccline` (status line) | ✅ one build for all four | ✅ | ✅ |
| Tray app | ✅ one folder per release | ✅ | ✅ |
| Offline installer | ✅ | planned (`install.ps1` + `.msi`) | planned |
| Tested for real | offline install on 20.04–26.04: desktops (arm64) and bare systems (x86_64, emulated); SSO + panel on 24.04 | CI build + test; `.exe` dependencies checked | yes |

The tray app runs on two engines: Ubuntu 22.04 and newer use Tauri 2 (WebKitGTK 4.1); Ubuntu 20.04,
which only has WebKitGTK 4.0, gets a Tauri 1 build of the same app (`crates/tray-legacy`). Same panel,
same features.

## What each system needs

**Ubuntu — nothing to prepare.** Each tray folder carries the app and every library it needs for its
release (WebKitGTK, GTK 3, AppIndicator and everything below them, down to base libraries). The
installer installs only what the machine is missing or has too old, never removes anything, and
doesn't touch the machine's apt sources — no apt mirror or internet needed.

- A desktop with a tray: Ubuntu's default desktop shows tray icons through its built-in AppIndicator
  extension. The installer also adds Quota's own small GNOME Shell extension (per user, no sudo) so a
  single left click on the icon opens the panel; it takes effect after the user logs out and back in.
- Secure storage for the sign-in token: GNOME Keyring (standard on Ubuntu desktop). Over SSH or on
  servers without a desktop there's no keyring, so sign-in can't be saved there (roadmap item 6).
- `ccline` needs nothing beyond the base system (built against glibc 2.30, so it runs on 20.04+).
- On 20.04 the standard desktop already includes WebKitGTK 4.0, so usually only a few packages are
  added.

**Windows**
- Tray app: Microsoft WebView2. Windows 11 and updated Windows 10 have it; the installer carries the
  offline WebView2 installer for machines that don't.
- `ccline`: nothing. Claude Code runs it through Git Bash or PowerShell.
- Sign-in token: Windows Credential Manager (built in).

**macOS**
- Nothing extra. Sign-in token: Keychain. Each new unsigned build asks once for Keychain access.

## Ubuntu: offline installers

Built by the **Bundles** GitHub workflow (one download each), or locally with
`tools/ubuntu-test/build-all.sh`. Users install either or both; they share one sign-in and one set of
addresses.

| Folder | For | Contents | Size | Needs sudo |
|---|---|---|---|---|
| `ccline-ubuntu-<arch>/` | 20.04–26.04 | `ccline`, `install.sh`, `install.conf`, `README.txt`, `SHA256SUMS`, `ARCH` | ~2 MB | no |
| `quota-tray-ubuntu-<release>-<arch>/` | that release only | `packages/` (app + all libraries, a local apt repo), `install.sh`, `install.conf`, `README.txt`, `SHA256SUMS`, `ARCH`, `UBUNTU`, `APP_PACKAGE` | 210–350 MB | yes |

The tray folders are large because they carry the whole dependency chain, so an old, never-updated
machine still installs cleanly. apt installs only what's missing (on the test desktops: 3–65
packages).

1. **Admin, once:** edit each folder's `install.conf`:
   - tray: `LITELLM_URL`, `STATUS_URL` (or `""` to hide the Status link), `START_AT_LOGIN`, `CA_CERT`
   - ccline: `LITELLM_URL`, `SETUP_CLAUDE_STATUSLINE`, `CA_CERT`

   `CA_CERT` is for an internal certificate: an absolute path to the CA file on the machines (used
   in place, so IT can update it), or the name of a CA file you put into the folder (copied during
   install). Leave it empty if the machines already trust the certificate.

   Each installer refuses to run while the example addresses are still there. `SHA256SUMS` covers
   every file except `install.conf`, so editing it doesn't break the check. Give each user the tray
   folder for **their** Ubuntu version (`cat /etc/os-release`); the installer refuses a mismatch.
2. **Each user:** unpack a folder and run `./install.sh` (not with sudo).
   - **Tray:** checks the files, Ubuntu version and CPU; installs the app and missing libraries from
     `packages/` with a throwaway apt setup (asks for the password); saves the addresses
     (`litellm-usage --configure`); adds autostart; opens the panel — the user clicks **Sign In** and
     finishes in the browser.
   - **ccline:** checks the files, installs `ccline` to `~/.local/bin`, saves the address, adds the
     status line to `~/.claude/settings.json` (backup kept as `settings.json.bak-ccline`; `--force`
     replaces someone else's status line), then signs in with `ccline login` — skipped when already
     signed in through the tray. `--no-login` skips it.

## How it was tested

On fresh Ubuntu 20.04, 22.04, 24.04 and 26.04 desktops in Docker with the **network disconnected**
and **no apt package lists** (no mirror): WebKit absent before install (on 20.04 removed first),
install from the folder, then `ldd` on the app and WebKit's helper processes shows zero missing
libraries and the tray runs. On 24.04 also: SSO sign-in from the panel, tray menu → panel, light and
dark, and the ccline folder reusing the tray's sign-in (`docs/screenshots/ubuntu-*.png`). `ccline`
was run on all four releases offline.

The x86_64 folders built by GitHub were also installed on **bare** 20.04, 22.04, 24.04 and 26.04
systems (no desktop at all, emulated x86_64, no network, no apt lists): 178–228 packages installed
from each folder, zero missing libraries, and the app started.

## Windows: early build

`windows-x64/` (from the CI run) has `ccline\ccline.exe` and `tray\litellm-usage.exe` with manual
steps in `installer/windows/README.txt`. Their DLL imports were checked: both use only DLLs built
into Windows 10/11 (the VC++ runtime is linked in), so the only outside need is WebView2 Runtime for
the tray. Not yet run on a real Windows machine.
