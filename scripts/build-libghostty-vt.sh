#!/usr/bin/env sh
set -eu
version=1.0.0
zig_version=0.17.1
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
case "$(zig version)" in "$zig_version") ;; *) echo "requires Zig $zig_version" >&2; exit 1;; esac
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
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s  %s\n' 4a6ef758be721923e1969e7b81b443dcc92d8dd3fb5a39600c03a8817a753676 "$cache/src.tar.gz" | sha256sum -c -
  printf '%s  %s\n' 4d5ea5e448e4c7ea8fb55ccbaa8668f2a97d44f438cee831e2b7ecd162d85c29 "$cache/deps.tar.gz" | sha256sum -c -
else
  printf '%s  %s\n' 4a6ef758be721923e1969e7b81b443dcc92d8dd3fb5a39600c03a8817a753676 "$cache/src.tar.gz" | shasum -a 256 -c -
  printf '%s  %s\n' 4d5ea5e448e4c7ea8fb55ccbaa8668f2a97d44f438cee831e2b7ecd162d85c29 "$cache/deps.tar.gz" | shasum -a 256 -c -
fi
mkdir "$cache/source" "$cache/deps"
tar -xzf "$cache/src.tar.gz" -C "$cache/source"
tar -xzf "$cache/deps.tar.gz" -C "$cache/deps"
source_dir="$cache/source/libghostty-vt-$version"
deps_dir="$cache/deps/libghostty-vt-$version-deps"
mkdir -p "$out/stage/usr"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) prefix=$(cygpath -m "$out/stage/usr"); deps_dir=$(cygpath -m "$deps_dir"); export MSYS_NO_PATHCONV=1 ;;
  *) prefix="$out/stage/usr" ;;
esac
(cd "$source_dir" && ZIG_GLOBAL_CACHE_DIR="$cache/zig-cache" zig build --prefix "$prefix" --system "$deps_dir" -Doptimize=ReleaseFast -Dcpu=baseline -Dtarget="$target")
printf '%s\n' "$prefix" > "$out/install-root.txt"
printf '%s\n' "Built pinned libghostty-vt $version with Zig $(zig version) for $1"
