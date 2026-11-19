#!/usr/bin/env bash
# Select the native Linux mold linker for Cargo.
#
# Usage:
#   scripts/setup/native_linker.sh --install
#   scripts/setup/native_linker.sh --check
# Cargo supplies compiler arguments when it uses this script as its linker.
set -euo pipefail

version=2.42.1
cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/ambition"
architecture="$(uname -m)"
case "$architecture" in
    x86_64) checksum=6ff270c9bf07d2bec5c98aa324eb7c4daf6a1a4d815c05ff1708049616047855 ;;
    aarch64) checksum=16b025652d3d7456689e6025a77e1903bb2a15e7630877c26cc133f5df95b9c6 ;;
    *)
        if [[ "${1:-}" == --install || "${1:-}" == --check ]]; then
            printf 'The managed mold linker is not used on this host architecture.\n'
            exit 0
        fi
        printf 'Unsupported mold host architecture: %s\n' "$architecture" >&2
        exit 1 ;;
esac
if [[ "$(uname -s)" != Linux ]]; then
    if [[ "${1:-}" == --install || "${1:-}" == --check ]]; then
        printf 'The managed mold linker is for Linux hosts.\n'
        exit 0
    fi
    printf 'The managed mold linker requires a Linux host.\n' >&2
    exit 1
fi
install_dir="$cache_root/mold-$version-$architecture-linux"
mold_binary="$install_dir/bin/mold"

case "${1:-}" in
    --install)
        mkdir -p "$cache_root"
        exec 9>"$cache_root/.mold-$version-$architecture.lock"
        flock 9
        if [[ -x "$mold_binary" ]]; then
            "$mold_binary" --version
            exit 0
        fi
        task_mold_temp="$(mktemp -d "$cache_root/.mold-install.XXXXXXXX")"
        trap 'rm -rf "$task_mold_temp"' EXIT
        archive="mold-$version-$architecture-linux.tar.gz"
        curl --fail --location --silent --show-error \
            "https://github.com/rui314/mold/releases/download/v$version/$archive" \
            --output "$task_mold_temp/$archive"
        printf '%s  %s\n' "$checksum" "$task_mold_temp/$archive" | sha256sum --check
        mkdir "$task_mold_temp/root"
        tar -xzf "$task_mold_temp/$archive" --strip-components=1 -C "$task_mold_temp/root"
        "$task_mold_temp/root/bin/mold" --version
        if [[ -e "$install_dir" ]]; then
            mv "$install_dir" "$task_mold_temp/previous"
        fi
        mv "$task_mold_temp/root" "$install_dir"
        exit 0 ;;
    --check) ;;
    -h|--help)
        sed -n '2,7p' "$0"
        exit 0 ;;
esac

if [[ ! -x "$mold_binary" ]]; then
    printf 'Run scripts/setup/native_linker.sh --install to install mold %s.\n' "$version" >&2
    exit 1
fi
if [[ "${1:-}" == --check ]]; then
    exec "$mold_binary" --version
fi
exec clang "$@" "-fuse-ld=$mold_binary"
