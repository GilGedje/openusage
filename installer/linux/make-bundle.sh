#!/usr/bin/env sh
# Assembles the offline Ubuntu bundle. Run after building ccline (release) and the tray .deb:
#   installer/linux/make-bundle.sh <ccline binary> <tray .deb> <output dir>
# Produces <output dir>/litellm-usage-ubuntu-<arch>/ and a matching .tar.gz.
set -eu

CCLINE=$1
DEB=$2
OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)
ARCH=$(uname -m)
NAME="litellm-usage-ubuntu-$ARCH"
DIR="$OUT/$NAME"

rm -rf "$DIR"
mkdir -p "$DIR"
cp "$CCLINE" "$DIR/ccline"
# Stable file name without spaces: litellm-usage_<version>_<arch>.deb
cp "$DEB" "$DIR/litellm-usage_$(dpkg-deb -f "$DEB" Version)_$(dpkg-deb -f "$DEB" Architecture).deb"
cp "$HERE/install.sh" "$HERE/install.conf" "$HERE/README.txt" "$DIR/"
chmod 755 "$DIR/install.sh" "$DIR/ccline"
echo "$ARCH" > "$DIR/ARCH"
(cd "$DIR" && sha256sum ccline ./*.deb ARCH > SHA256SUMS)
tar -C "$OUT" -czf "$OUT/$NAME.tar.gz" "$NAME"
echo "$OUT/$NAME.tar.gz"
