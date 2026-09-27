LiteLLM Usage for Ubuntu — offline install bundle

What's inside
  ccline                  Claude Code status line + command-line tool
  litellm-usage_*.deb     Tray app
  install.sh              Installer (run as the user, not with sudo)
  install.conf            Settings: your LiteLLM and status page addresses
  SHA256SUMS, ARCH        Integrity check and CPU type

For the admin, once
  1. Edit install.conf: set LITELLM_URL and STATUS_URL (or leave STATUS_URL empty).
  2. Hand the folder (or a re-packed .tar.gz) to users.

For each user
  1. Unpack the folder and run:  ./install.sh
  2. Enter your password when asked (installs the tray app).
  3. Your browser opens: sign in with SSO and type the code shown in the terminal.
  The ring icon appears in the top bar; Claude Code shows usage in its status line.

Requirements
  Ubuntu 22.04 or 24.04 with the standard desktop.
  The tray app's libraries (WebKitGTK 4.1, AppIndicator) come from your apt mirror; on a standard
  Ubuntu desktop most are already installed.
  Nothing is downloaded from the internet. The apps only talk to the addresses in install.conf.

Useful commands
  ccline status        show budget and usage now
  ccline login         sign in again
  ccline logout        sign out
