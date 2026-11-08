#!/usr/bin/env sh
set -eu
version=1.0.0
zig_version=0.17.0
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
case "$(zig version)" in "$zig_version"|"$zig_version".*) ;; *) echo "requires Zig $zig_version.x" >&2; exit 1;; esac
case "${1:-}" in
  x86_64-pc-windows-msvc) target=x86_64-windows-msvc ;;
  aarch64-pc-windows-msvc) target=aarch64-windows-msvc ;;
  x86_64-apple-darwin) target=x86_64-macos-none ;;
  aarch64-apple-darwin) target=aarch64-macos-none ;;
  x86_64-unknown-linux-gnu) target=x86_64-linux-gnu.2.17 ;;
  aarch64-unknown-linux-gnu) target=aarch64-linux-gnu.2.17 ;;
  *) echo "unsupported target: ${1:-<missing>}" >&2; exit 2 ;;
esac
out=${2:-"$root/target/libghostty-vt/$1"}
cache=$(mktemp -d)
trap 'rm -rf "$cache"' EXIT
base="https://release.files.ghostty.org/libghostty-vt/$version"
curl --fail --location "$base/libghostty-vt-$version.tar.gz" -o "$cache/src.tar.gz"
curl --fail --location "$base/libghostty-vt-$version-deps.tar.gz" -o "$cache/deps.tar.gz"
printf '%s  %s\n' 4a6ef758be721923e1969e7b81b443dcc92d8dd3fb5a39600c03a8817a753676 "$cache/src.tar.gz" | sha256sum -c -
printf '%s  %s\n' 4d5ea5e448e4c7ea8fb55ccbaa8668f2a97d44f438cee831e2b7ecd162d85c29 "$cache/deps.tar.gz" | sha256sum -c -
mkdir "$cache/source" "$cache/deps"
tar -xzf "$cache/src.tar.gz" -C "$cache/source"
tar -xzf "$cache/deps.tar.gz" -C "$cache/deps"
source_dir="$cache/source/libghostty-vt-$version"
deps_dir="$cache/deps/libghostty-vt-$version-deps"
mkdir -p "$out"
(cd "$source_dir" && DESTDIR="$out/stage" ZIG_GLOBAL_CACHE_DIR="$cache/zig-cache" zig build --prefix /usr --system "$deps_dir" -Doptimize=ReleaseFast -Dcpu=baseline -Dtarget="$target")
printf '%s\n' "$out/stage/usr" > "$out/install-root.txt"
printf '%s\n' "Built pinned libghostty-vt $version with Zig $zig_version for $1"
