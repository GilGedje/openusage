#!/bin/sh
# Inside ubuntu:20.04: builds ccline against the oldest supported glibc so it runs on 20.04+.
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config binutils >/dev/null
command -v cargo >/dev/null || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal >/dev/null
. "$HOME/.cargo/env"
export CARGO_TARGET_DIR=/cache/target
cd /src && cargo build --release -q -p ccline
mkdir -p /src/.ubuntu-test/build && cp "$CARGO_TARGET_DIR/release/ccline" /src/.ubuntu-test/build/ccline
echo "ccline needs glibc $(objdump -T /src/.ubuntu-test/build/ccline | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1)"
