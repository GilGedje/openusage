#!/usr/bin/env sh
# LiteLLM Usage tray app — offline installer for Ubuntu. Run from this folder as the user (not sudo):
#   ./install.sh
# Everything it needs is in this folder: the app and all its libraries. Nothing is downloaded.
# The panel opens when it's done; sign in there with your company SSO.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)

case "${1:-}" in
  -h | --help) sed -n '2,5p' "$0"; exit 0 ;;
  "") ;;
  *) echo "Unknown option: $1" >&2; exit 2 ;;
esac

say() { printf '\n==> %s\n' "$*"; }
fail() { printf '\nError: %s\n' "$*" >&2; exit 1; }

# --- Settings ----------------------------------------------------------------
[ -f "$HERE/install.conf" ] || fail "install.conf is missing next to install.sh."
# shellcheck disable=SC1091
. "$HERE/install.conf"
: "${LITELLM_URL:=}" "${STATUS_URL:=}" "${START_AT_LOGIN:=yes}" "${CA_CERT:=}"
case "$LITELLM_URL$STATUS_URL" in
  *example.internal*) fail "Edit install.conf first: replace the example addresses with your LiteLLM and status page." ;;
esac
[ -n "$LITELLM_URL" ] || fail "LITELLM_URL is empty in install.conf."

# CA certificate: absolute path = use in place; otherwise a file in this folder, copied.
CA_ARGS=""
if [ -n "${CA_CERT:-}" ]; then
  case "$CA_CERT" in
    /*) [ -f "$CA_CERT" ] || fail "CA_CERT file not found: $CA_CERT"; CA_ARGS="--ca-cert $CA_CERT" ;;
    *) [ -f "$HERE/$CA_CERT" ] || fail "CA_CERT file not found in this folder: $CA_CERT"; CA_ARGS="--ca-cert-copy $HERE/$CA_CERT" ;;
  esac
fi

# --- This folder must match this machine ---------------------------------------
[ "$(uname -s)" = "Linux" ] || fail "This installer is for Ubuntu."
FOLDER_ARCH=$(cat "$HERE/ARCH" 2>/dev/null || echo unknown)
[ "$(uname -m)" = "$FOLDER_ARCH" ] || fail "This folder is for $FOLDER_ARCH, but this machine is $(uname -m)."
FOLDER_UBUNTU=$(cat "$HERE/UBUNTU" 2>/dev/null || echo unknown)
# shellcheck disable=SC1091
. /etc/os-release
[ "${VERSION_ID:-}" = "$FOLDER_UBUNTU" ] ||
  fail "This folder is for Ubuntu $FOLDER_UBUNTU, but this machine runs ${PRETTY_NAME:-something else}. Use the folder for your Ubuntu version."

say "Checking files"
(cd "$HERE" && sha256sum --quiet -c SHA256SUMS) || fail "Checksum mismatch: the folder is damaged or was changed."

# --- Install from the packages in this folder only -----------------------------
# A throwaway apt setup that sees only ./packages: the machine's own apt sources and state are left
# untouched, nothing is downloaded, and apt installs only what's missing or too old.
if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/lists/partial" "$TMP/cache/archives/partial"
echo "deb [trusted=yes] file:$HERE/packages ./" > "$TMP/sources.list"
APT_OPTS="-o Dir::Etc::SourceList=$TMP/sources.list -o Dir::Etc::SourceParts=/dev/null \
  -o Dir::State::Lists=$TMP/lists -o Dir::Cache=$TMP/cache -o APT::Sandbox::User=root \
  -o Acquire::Languages=none"
APP_PKG=$(cat "$HERE/APP_PACKAGE")

say "Installing the tray app and its libraries from this folder (asks for your password)"
# shellcheck disable=SC2086
$SUDO apt-get $APT_OPTS -qq update || fail "Couldn't read the packages in this folder."
# shellcheck disable=SC2086
$SUDO env DEBIAN_FRONTEND=noninteractive apt-get $APT_OPTS install -y --no-remove "$APP_PKG" ||
  fail "Couldn't install the tray app from this folder. Nothing was removed; see the messages above."

say "Saving your LiteLLM and status page addresses (and CA certificate, if set)"
# shellcheck disable=SC2086
litellm-usage --configure --url "$LITELLM_URL" --status-url "$STATUS_URL" $CA_ARGS

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
