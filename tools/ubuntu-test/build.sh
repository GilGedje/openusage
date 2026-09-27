#!/bin/sh
# Runs inside ubuntu:22.04: builds ccline + tray .deb + bundle, like the release workflow.
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev file >/dev/null
if ! command -v cargo >/dev/null; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null
fi
. "$HOME/.cargo/env"
if ! command -v node >/dev/null; then
  curl -fsSL https://deb.nodesource.com/setup_20.x | bash - >/dev/null 2>&1
  apt-get install -y -qq nodejs >/dev/null
fi
export CARGO_TARGET_DIR=/cache/target
cd /src
cargo build --release -q -p ccline
cd crates/tray && npx --yes @tauri-apps/cli@2 build --bundles deb 2>&1 | tail -5 && cd /src
DEB=$(ls $CARGO_TARGET_DIR/release/bundle/deb/*.deb | head -n 1)
dpkg-deb -I "$DEB" | grep -E 'Package|Version|Depends|Architecture'
sh installer/linux/make-bundle.sh $CARGO_TARGET_DIR/release/ccline "$DEB" /src/.ubuntu-test/dist
echo BUILD_DONE
