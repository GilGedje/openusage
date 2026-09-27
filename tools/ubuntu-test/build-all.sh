#!/bin/sh
# On the host (needs Docker). Builds every Ubuntu installer folder into .ubuntu-test/dist:
#   ccline (built on 20.04); Tauri 2 tray .deb (built on 22.04, for 22.04/24.04/26.04); Tauri 1 tray
#   .deb (built on 20.04, for 20.04); offline packages collected inside each target release.
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
B=.ubuntu-test/build
rm -rf "$B" .ubuntu-test/dist && mkdir -p "$B"

run() { # image, cache-name, command...
  IMG=$1; CACHE=$2; shift 2
  docker run --rm -v "$ROOT":/src -v "ubuntu-$CACHE-cache":/cache -v "ubuntu-$CACHE-cargo":/root/.cargo \
    -v "ubuntu-$CACHE-rustup":/root/.rustup "$IMG" "$@"
}

echo "== ccline (Ubuntu 20.04)";   run ubuntu:20.04 focal sh /src/tools/ubuntu-test/build-ccline.sh
echo "== tray .deb (Ubuntu 22.04)"; run ubuntu:22.04 build sh /src/tools/ubuntu-test/build-tray.sh
echo "== tray .deb for 20.04 (Tauri 1)"
docker run --rm -e CARGO_BUILD_JOBS=2 -v "$ROOT":/src -v ubuntu-focal-cache:/cache -v ubuntu-focal-cargo:/root/.cargo \
  -v ubuntu-focal-rustup:/root/.rustup ubuntu:20.04 sh /src/tools/ubuntu-test/build-tray-legacy.sh
echo "== offline packages for 20.04"
run ubuntu:20.04 pkgs sh /src/installer/linux/fetch-deps.sh "/src/$B/tray-legacy.deb" "/src/$B/packages-20.04"
for REL in 22.04 24.04 26.04; do
  echo "== offline packages for $REL"
  run "ubuntu:$REL" "pkgs" sh /src/installer/linux/fetch-deps.sh "/src/$B/tray.deb" "/src/$B/packages-$REL"
done
echo "== folders"
run ubuntu:22.04 pkgs sh -c "cd /src && sh installer/linux/make-bundle.sh ccline $B/ccline .ubuntu-test/dist &&
  sh installer/linux/make-bundle.sh tray 20.04 $B/packages-20.04 .ubuntu-test/dist &&
  sh installer/linux/make-bundle.sh tray 22.04 $B/packages-22.04 .ubuntu-test/dist &&
  sh installer/linux/make-bundle.sh tray 24.04 $B/packages-24.04 .ubuntu-test/dist &&
  sh installer/linux/make-bundle.sh tray 26.04 $B/packages-26.04 .ubuntu-test/dist"
