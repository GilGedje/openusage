#!/usr/bin/env sh
# LiteLLM Usage tray app — offline installer for Ubuntu. Run from this folder as the user (not sudo):
#   ./install.sh
# The panel opens when it's done; sign in there with your company SSO.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)

case "${1:-}" in
  -h | --help) sed -n '2,4p' "$0"; exit 0 ;;
  "") ;;
  *) echo "Unknown option: $1" >&2; exit 2 ;;
esac

say() { printf '\n==> %s\n' "$*"; }
fail() { printf '\nError: %s\n' "$*" >&2; exit 1; }

# --- Settings ----------------------------------------------------------------
[ -f "$HERE/install.conf" ] || fail "install.conf is missing next to install.sh."
# shellcheck disable=SC1091
. "$HERE/install.conf"
: "${LITELLM_URL:=}" "${STATUS_URL:=}" "${START_AT_LOGIN:=yes}"
case "$LITELLM_URL$STATUS_URL" in
  *example.internal*) fail "Edit install.conf first: replace the example addresses with your LiteLLM and status page." ;;
esac
[ -n "$LITELLM_URL" ] || fail "LITELLM_URL is empty in install.conf."

# --- Bundle checks -----------------------------------------------------------
[ "$(uname -s)" = "Linux" ] || fail "This installer is for Ubuntu/Linux."
BUNDLE_ARCH=$(cat "$HERE/ARCH" 2>/dev/null || echo unknown)
[ "$(uname -m)" = "$BUNDLE_ARCH" ] || fail "This folder is for $BUNDLE_ARCH, but this machine is $(uname -m)."
say "Checking files"
(cd "$HERE" && sha256sum --quiet -c SHA256SUMS) || fail "Checksum mismatch: the folder is damaged or was changed."

# --- Install -------------------------------------------------------------------
DEB=$(ls "$HERE"/*.deb 2>/dev/null | head -n 1 || true)
[ -n "$DEB" ] || fail "No .deb package in this folder."
say "Installing the tray app (asks for your password)"
if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi
# apt takes the app's libraries (WebKitGTK, AppIndicator) from your configured, internal mirror.
$SUDO apt-get install -y "$DEB" || fail "Couldn't install the tray app. Its libraries (WebKitGTK 4.1, AppIndicator) must be available from your apt mirror."

say "Saving your LiteLLM and status page addresses"
litellm-usage --configure --url "$LITELLM_URL" --status-url "$STATUS_URL"

if [ "$START_AT_LOGIN" = "yes" ]; then
  mkdir -p "$HOME/.config/autostart"
  cat > "$HOME/.config/autostart/litellm-usage.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=LiteLLM Usage
Exec=litellm-usage
Icon=litellm-usage
X-GNOME-Autostart-enabled=true
NoDisplay=true
DESKTOP
fi

# --- Start, with the panel open so the user can sign in ------------------------
if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
  pkill -x litellm-usage 2>/dev/null || true
  LITELLM_USAGE_OPEN_PANEL=1 nohup litellm-usage >/dev/null 2>&1 &
  say "Done. The panel is open at the top right: click Sign In and finish in your browser."
else
  say "Done. Log in to your desktop and start \"LiteLLM Usage\" to sign in."
fi
