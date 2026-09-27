#!/usr/bin/env sh
# Assembles the two offline Ubuntu folders. Run after building ccline (release) and the tray .deb:
#   installer/linux/make-bundle.sh <ccline binary> <tray .deb> <output dir>
# Produces, each with its own installer, settings and checksums, plus a .tar.gz of each:
#   <output dir>/ccline-ubuntu-<arch>/
#   <output dir>/litellm-usage-tray-ubuntu-<arch>/
set -eu

CCLINE=$1
DEB=$2
OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)
ARCH=$(uname -m)
mkdir -p "$OUT"

# folder name, source dir, payload files...
pack() {
  NAME=$1
  SRC=$2
  shift 2
  DIR="$OUT/$NAME"
  rm -rf "$DIR"
  mkdir -p "$DIR"
  cp "$SRC/install.sh" "$SRC/install.conf" "$SRC/README.txt" "$DIR/"
  chmod 755 "$DIR/install.sh"
  echo "$ARCH" > "$DIR/ARCH"
  for f in "$@"; do cp "$f" "$DIR/"; done
  (cd "$DIR" && sha256sum $(ls | grep -v -e '^SHA256SUMS$' -e '^install.conf$') > SHA256SUMS)
  tar -C "$OUT" -czf "$OUT/$NAME.tar.gz" "$NAME"
  echo "$OUT/$NAME.tar.gz"
}

# Stable .deb file name without spaces: litellm-usage_<version>_<arch>.deb
DEB_NAME="litellm-usage_$(dpkg-deb -f "$DEB" Version)_$(dpkg-deb -f "$DEB" Architecture).deb"
TMP=$(mktemp -d)
cp "$DEB" "$TMP/$DEB_NAME"
cp "$CCLINE" "$TMP/ccline"
chmod 755 "$TMP/ccline"

pack "ccline-ubuntu-$ARCH" "$HERE/ccline" "$TMP/ccline"
pack "litellm-usage-tray-ubuntu-$ARCH" "$HERE/tray" "$TMP/$DEB_NAME"
rm -rf "$TMP"
