#!/usr/bin/env sh
# Assembles the offline Ubuntu installer folders (each with its installer, settings, checksums, and
# a .tar.gz of the folder):
#
#   make-bundle.sh ccline <ccline binary> <output dir>
#       -> ccline-ubuntu-<arch>/                      (runs on Ubuntu 20.04 and newer)
#
#   make-bundle.sh tray <ubuntu version> <packages dir from fetch-deps.sh> <output dir>
#       -> quota-tray-ubuntu-<version>-<arch>/ (the app + every library it needs)
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ARCH=$(uname -m)

finish() {
  DIR=$1
  (cd "$DIR" && find . -type f ! -name SHA256SUMS ! -name install.conf | sed 's|^\./||' | sort |
    xargs sha256sum > SHA256SUMS)
  tar -C "$(dirname "$DIR")" -czf "$DIR.tar.gz" "$(basename "$DIR")"
  echo "$DIR.tar.gz ($(du -sh "$DIR.tar.gz" | cut -f1))"
}

start() {
  DIR=$1
  SRC=$2
  rm -rf "$DIR"
  mkdir -p "$DIR"
  cp "$SRC/install.sh" "$SRC/install.conf" "$SRC/README.txt" "$DIR/"
  chmod 755 "$DIR/install.sh"
  echo "$ARCH" > "$DIR/ARCH"
}

case "$1" in
  ccline)
    OUT=$3
    mkdir -p "$OUT"
    DIR="$OUT/ccline-ubuntu-$ARCH"
    start "$DIR" "$HERE/ccline"
    cp "$2" "$DIR/ccline"
    chmod 755 "$DIR/ccline"
    finish "$DIR"
    ;;
  tray)
    UBUNTU=$2
    PACKAGES=$3
    OUT=$4
    mkdir -p "$OUT"
    DIR="$OUT/quota-tray-ubuntu-$UBUNTU-$ARCH"
    start "$DIR" "$HERE/tray"
    echo "$UBUNTU" > "$DIR/UBUNTU"
    cp -R "$HERE/tray/gnome-extension" "$DIR/gnome-extension"
    cp -R "$PACKAGES" "$DIR/packages"
    # The app's own package name, from its .deb (the one with the litellm-usage binary).
    for deb in "$DIR"/packages/*.deb; do
      if dpkg-deb -c "$deb" | grep -q 'usr/bin/litellm-usage$'; then
        dpkg-deb -f "$deb" Package > "$DIR/APP_PACKAGE"
      fi
    done
    [ -s "$DIR/APP_PACKAGE" ] || { echo "No tray app .deb in $PACKAGES" >&2; exit 1; }
    finish "$DIR"
    ;;
  *)
    sed -n '2,10p' "$0"
    exit 2
    ;;
esac
