#!/bin/sh
# Inside ubuntu:20.04: builds the Tauri 1 tray .deb for Ubuntu 20.04 (WebKitGTK 4.0).
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev file >/dev/null
command -v cargo >/dev/null || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null
. "$HOME/.cargo/env"
if ! command -v node >/dev/null; then
  curl -fsSL https://deb.nodesource.com/setup_20.x | bash - >/dev/null 2>&1
  apt-get install -y -qq nodejs >/dev/null
fi
export CARGO_TARGET_DIR=/cache/legacy-target
cd /src/crates/tray-legacy && npx --yes @tauri-apps/cli@1 build --bundles deb 2>&1 | grep -vE '^\s*(Compiling|Downloaded|Downloading)' | tail -25
DEB=$(ls "$CARGO_TARGET_DIR"/release/bundle/deb/*.deb | head -n 1)
mkdir -p /src/.ubuntu-test/build && cp "$DEB" /src/.ubuntu-test/build/tray-legacy.deb
dpkg-deb -f /src/.ubuntu-test/build/tray-legacy.deb Package Version Depends
dpkg-deb -c /src/.ubuntu-test/build/tray-legacy.deb | grep -E 'usr/bin|applications'
