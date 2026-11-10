#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

target="$(rustc -vV | awk '$1 == "host:" { print $2 }')"
if [[ -z "$target" ]]; then
    echo "could not determine the Rust host target" >&2
    exit 1
fi
native_root="$ROOT/target/libghostty-vt/$target"
library_dir="$native_root/stage/usr/lib"
if [[ "${LIBGHOSTTY_VT_LIB_DIR:-}" == "" ]]; then
    if [[ ! -f "$library_dir/libghostty-vt.a" || ! -f "$native_root/install-root.txt" ]]; then
        bash "$ROOT/scripts/build-libghostty-vt.sh" "$target"
    fi
    export LIBGHOSTTY_VT_LIB_DIR="$library_dir"
fi

case "$(uname -s)" in
    Linux*)
        if [[ -n "${WSL_DISTRO_NAME:-}" ]] || grep -qi microsoft /proc/version 2>/dev/null; then
            exec bash "$ROOT/scripts/run-linux.sh" "$@"
        fi
        exec cargo run -p oxide-ssh-desktop --locked "$@"
        ;;
    Darwin*)
        exec cargo run -p oxide-ssh-desktop --locked "$@"
        ;;
    *)
        echo "unsupported preview host: $(uname -s)" >&2
        exit 2
        ;;
esac
