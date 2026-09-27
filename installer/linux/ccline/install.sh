#!/usr/bin/env sh
# ccline — offline installer for Ubuntu. Run from this folder as the user (no sudo needed):
#   ./install.sh              install, set up Claude Code's status line, then sign in
#   ./install.sh --no-login   install without signing in (later: ccline login)
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
    -h | --help) sed -n '2,5p' "$0"; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

say() { printf '\n==> %s\n' "$*"; }
fail() { printf '\nError: %s\n' "$*" >&2; exit 1; }

# --- Settings ----------------------------------------------------------------
[ -f "$HERE/install.conf" ] || fail "install.conf is missing next to install.sh."
# shellcheck disable=SC1091
. "$HERE/install.conf"
: "${LITELLM_URL:=}" "${SETUP_CLAUDE_STATUSLINE:=yes}"
case "$LITELLM_URL" in
  *example.internal*) fail "Edit install.conf first: replace the example address with your LiteLLM proxy." ;;
esac
[ -n "$LITELLM_URL" ] || fail "LITELLM_URL is empty in install.conf."

# --- Bundle checks -----------------------------------------------------------
[ "$(uname -s)" = "Linux" ] || fail "This installer is for Ubuntu/Linux."
BUNDLE_ARCH=$(cat "$HERE/ARCH" 2>/dev/null || echo unknown)
[ "$(uname -m)" = "$BUNDLE_ARCH" ] || fail "This folder is for $BUNDLE_ARCH, but this machine is $(uname -m)."
say "Checking files"
(cd "$HERE" && sha256sum --quiet -c SHA256SUMS) || fail "Checksum mismatch: the folder is damaged or was changed."

# --- Install -------------------------------------------------------------------
say "Installing ccline to $BIN_DIR"
mkdir -p "$BIN_DIR"
rm -f "$BIN_DIR/ccline"   # replace, don't overwrite in place
cp "$HERE/ccline" "$BIN_DIR/ccline"
chmod 755 "$BIN_DIR/ccline"

if [ "$SETUP_CLAUDE_STATUSLINE" = "yes" ]; then
  # shellcheck disable=SC2086
  "$BIN_DIR/ccline" setup --url "$LITELLM_URL" $FORCE
else
  "$BIN_DIR/ccline" setup --url "$LITELLM_URL" --no-statusline
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "Note: $BIN_DIR isn't on your PATH yet. Log out and back in (Ubuntu adds it automatically), or run: export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

# --- Sign in -------------------------------------------------------------------
if [ "$LOGIN" = "yes" ]; then
  if "$BIN_DIR/ccline" status >/dev/null 2>&1; then
    say "Already signed in (shared with the tray app)."
  else
    say "Signing in to LiteLLM"
    "$BIN_DIR/ccline" login || echo "Sign-in didn't finish. Sign in later with: ccline login"
  fi
fi

say "Done. Claude Code shows your LiteLLM budget in its status line (new sessions pick it up)."
