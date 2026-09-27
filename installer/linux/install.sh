#!/usr/bin/env sh
# LiteLLM Usage installer for Ubuntu. Runs fully offline from this folder:
#   ./install.sh              install for the current user, then sign in
#   ./install.sh --no-login   install without signing in (sign in later from the tray or `ccline login`)
#   ./install.sh --force      replace an existing Claude Code status line that isn't ccline's
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
BIN_DIR="$HOME/.local/bin"
LOGIN=yes
FORCE=""

for arg in "$@"; do
  case "$arg" in
    --no-login) LOGIN=no ;;
    --force) FORCE="--force" ;;
    -h | --help) sed -n '2,6p' "$0"; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

say() { printf '\n==> %s\n' "$*"; }
fail() { printf '\nError: %s\n' "$*" >&2; exit 1; }

# --- Settings ----------------------------------------------------------------
[ -f "$HERE/install.conf" ] || fail "install.conf is missing next to install.sh."
# shellcheck disable=SC1091
. "$HERE/install.conf"
: "${LITELLM_URL:=}" "${STATUS_URL:=}" "${INSTALL_TRAY:=yes}" "${START_TRAY_AT_LOGIN:=yes}" "${SETUP_CLAUDE_STATUSLINE:=yes}"

case "$LITELLM_URL$STATUS_URL" in
  *example.internal*) fail "Edit install.conf first: replace the example addresses with your LiteLLM and status page." ;;
esac
[ -n "$LITELLM_URL" ] || fail "LITELLM_URL is empty in install.conf."

# --- Bundle checks -----------------------------------------------------------
[ "$(uname -s)" = "Linux" ] || fail "This installer is for Ubuntu/Linux."
ARCH=$(uname -m)
BUNDLE_ARCH=$(cat "$HERE/ARCH" 2>/dev/null || echo unknown)
[ "$ARCH" = "$BUNDLE_ARCH" ] || fail "This bundle is for $BUNDLE_ARCH, but this machine is $ARCH."

say "Checking files"
(cd "$HERE" && sha256sum --quiet -c SHA256SUMS) || fail "Checksum mismatch: the bundle is damaged or was changed."

# --- ccline (status line + CLI) ----------------------------------------------
say "Installing ccline to $BIN_DIR"
mkdir -p "$BIN_DIR"
rm -f "$BIN_DIR/ccline"   # replace, don't overwrite in place
cp "$HERE/ccline" "$BIN_DIR/ccline"
chmod 755 "$BIN_DIR/ccline"

SETUP_ARGS="--url $LITELLM_URL"
[ -n "$STATUS_URL" ] && SETUP_ARGS="$SETUP_ARGS --status-url $STATUS_URL"
[ "$SETUP_CLAUDE_STATUSLINE" = "yes" ] || SETUP_ARGS="$SETUP_ARGS --no-statusline"
# shellcheck disable=SC2086
"$BIN_DIR/ccline" setup $SETUP_ARGS $FORCE

# --- Tray app ------------------------------------------------------------------
if [ "$INSTALL_TRAY" = "yes" ]; then
  DEB=$(ls "$HERE"/*.deb 2>/dev/null | head -n 1 || true)
  [ -n "$DEB" ] || fail "No .deb package in the bundle."
  say "Installing the tray app (asks for your password)"
  if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi
  # apt resolves the package's libraries from your configured (internal) mirror.
  $SUDO apt-get install -y "$DEB" || fail "Couldn't install the tray app. Its libraries (WebKitGTK, AppIndicator) must be available from your apt mirror."

  if [ "$START_TRAY_AT_LOGIN" = "yes" ]; then
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

  if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ] && ! pgrep -x litellm-usage >/dev/null 2>&1; then
    nohup litellm-usage >/dev/null 2>&1 &
  fi
fi

# --- PATH hint -----------------------------------------------------------------
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "Note: $BIN_DIR isn't on your PATH yet. Log out and back in (Ubuntu adds it automatically), or run: export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

# --- Sign in -------------------------------------------------------------------
if [ "$LOGIN" = "yes" ]; then
  say "Signing in to LiteLLM"
  "$BIN_DIR/ccline" login || echo "Sign-in didn't finish. Sign in later from the tray icon or with: ccline login"
fi

say "Done. Look for the ring icon in the top bar; Claude Code shows usage in its status line."
