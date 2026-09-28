#!/bin/sh
# Inside ubuntu:22.04: builds the Tauri 2 tray .deb (runs on 22.04 and 24.04).
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev file >/dev/null
command -v cargo >/dev/null || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null
. "$HOME/.cargo/env"
if ! command -v node >/dev/null; then
  curl -fsSL https://deb.nodesource.com/setup_20.x | bash - >/dev/null 2>&1
  apt-get install -y -qq nodejs >/dev/null
fi
export CARGO_TARGET_DIR=/cache/target
cd /src/crates/tray && npx --yes @tauri-apps/cli@2 build --bundles deb >/dev/null 2>&1 || npx --yes @tauri-apps/cli@2 build --bundles deb
DEB=$(ls -t "$CARGO_TARGET_DIR"/release/bundle/deb/*.deb | head -n 1)  # newest: the folder keeps old builds
mkdir -p /src/.ubuntu-test/build && cp "$DEB" /src/.ubuntu-test/build/tray.deb
dpkg-deb -f /src/.ubuntu-test/build/tray.deb Package Version Depends
