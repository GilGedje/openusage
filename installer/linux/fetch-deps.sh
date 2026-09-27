#!/usr/bin/env sh
# Builds the offline package folder for the tray app: the app .deb plus every package it needs,
# with an apt index, so install.sh can install it on a machine with no apt mirror.
# Run INSIDE a clean container of the target release and CPU (e.g. docker run ubuntu:24.04):
#   installer/linux/fetch-deps.sh <tray .deb> <output packages dir>
# It downloads the full recursive dependency set (including base libraries), so machines installed
# from an older point release still find versions new enough. apt, at install time, only installs
# what a machine is actually missing or needs newer.
set -eu

DEB=$1
OUT=$2
export DEBIAN_FRONTEND=noninteractive

apt-get update -qq
apt-get install -y -qq dpkg-dev >/dev/null
mkdir -p "$OUT"

# Top-level dependencies declared by the .deb, e.g. "libwebkit2gtk-4.1-0, libgtk-3-0, …"
TOP=$(dpkg-deb -f "$DEB" Depends | tr ',' '\n' | sed 's/(.*)//; s/|.*//; s/^ *//; s/ *$//' | grep -v '^$')

# Recursive closure of real packages (virtual ones show as <name> and are skipped).
# shellcheck disable=SC2086
PKGS=$(apt-cache depends --recurse --no-recommends --no-suggests --no-conflicts --no-breaks \
  --no-replaces --no-enhances $TOP | grep -E '^[a-z0-9]' | sort -u)

cp "$DEB" "$OUT/"
cd "$OUT"
# shellcheck disable=SC2086
apt-get download -qq $PKGS
dpkg-scanpackages --multiversion . /dev/null 2>/dev/null > Packages
gzip -9kf Packages
echo "$(ls ./*.deb | wc -l) packages, $(du -sh . | cut -f1) in $OUT"
