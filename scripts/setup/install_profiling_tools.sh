#!/usr/bin/env bash
# Install ONLY the profiling tools. No submodules, no Python venvs, no assets.
#
# ⭐ WHY THIS EXISTS SEPARATELY. `run_developer_setup.sh --profile` is a FULL
# developer setup that happens to also install Tracy: it syncs submodules,
# builds tool venvs and regenerates assets. Someone whose only problem is
# "tracy-capture is the wrong version" had to run all of that and kill it at the
# sprite regeneration — which is what happened on 2026-09-01.
#
# ⛔⛔ AND THE VERSION IS THE WHOLE POINT. A mismatched Tracy does not error: the
# client connects and is REFUSED with "incompatible protocol version", the run
# silently loses its per-system zones, and the bundle used to blame the game.
# The version is read from the header `tracy-client-sys` vendors, because a Bevy
# upgrade bumps it and a constant here would go stale.
#
#   install_profiling_tools.sh                 install/repair the tools
#   install_profiling_tools.sh --print-version the version the game speaks
#   install_profiling_tools.sh --check         report, install nothing
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bin_dir="$HOME/.local/bin"
cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/ambition"

log() { printf '  %s\n' "$*"; }
warn() { printf '  ⚠ %s\n' "$*" >&2; }
have() { command -v "$1" >/dev/null 2>&1; }

# The Tracy the game's `tracy-client-sys` vendors, read out of its header.
#
# ⛔⛔ THE PARSE HAS BEEN WRONG TWICE, IN TWO FILES, THE SAME WAY. Splitting on
# `[ =}]+` and taking `$(NF-1)` reads the WORD:
#
#   constexpr int Major = 0;   ->  "Major"
#
# Both copies then produced the version string "Major.Minor.Patch" — which is
# non-empty, so every emptiness guard passed. `run_developer_setup.sh` handed it
# to `git clone --branch vMajor.Minor.Patch` (which cannot resolve, so Tracy was
# never installed at the right version) and `profile_deps.sh` compared it against
# a real version (which never matched, so it reported MISMATCH on every machine
# and taught readers to ignore it). Take the LAST field and strip non-digits.
tracy_required_version() {
    local header major minor patch
    header="$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -maxdepth 6 \
        -path '*tracy-client-sys-*/tracy/common/TracyVersion.hpp' -print -quit 2>/dev/null || true)"
    if [ -z "$header" ] || [ ! -f "$header" ]; then
        return 1
    fi
    read -r major minor patch < <(awk '
        /Major *=/ { gsub(/[^0-9]/, "", $NF); maj = $NF }
        /Minor *=/ { gsub(/[^0-9]/, "", $NF); min = $NF }
        /Patch *=/ { gsub(/[^0-9]/, "", $NF); pat = $NF }
        END { print maj, min, pat }' "$header")
    case "$major.$minor.$patch" in
        [0-9]*.[0-9]*.[0-9]*) printf '%s.%s.%s\n' "$major" "$minor" "$patch" ;;
        *) return 1 ;;
    esac
}

build_tracy_tool() {
    local src_dir="$1" subproject="$2" binary="$3"
    local build_dir="$src_dir/build-$subproject"
    log "building $binary"
    cmake -B "$build_dir" -S "$src_dir/$subproject" \
        -DCMAKE_BUILD_TYPE=Release -DNO_FILESELECTOR=ON >/dev/null 2>&1 || return 1
    cmake --build "$build_dir" --parallel >/dev/null 2>&1 || return 1
    install -Dm755 "$build_dir/$binary" "$bin_dir/$binary" || return 1
}

mode="install"
case "${1:-}" in
    --print-version) mode="print" ;;
    --check) mode="check" ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    "") ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
esac

if ! version="$(tracy_required_version)"; then
    warn "could not read a version from the vendored TracyVersion.hpp."
    warn "It appears once a profiling build has fetched tracy-client-sys:"
    warn "  cargo build --profile profiling -p ambition_app --features profile"
    exit 1
fi

[ "$mode" = "print" ] && { printf '%s\n' "$version"; exit 0; }

installed="(none)"
if [ -x "$bin_dir/tracy-capture" ]; then
    installed="$(ls -d "$cache_dir"/tracy-* 2>/dev/null | sed 's#.*/tracy-##' | sort -V | tail -1)"
    [ -n "$installed" ] || installed="(unknown)"
fi
log "the game speaks Tracy $version; installed server: $installed"

if [ "$mode" = "check" ]; then
    [ "$installed" = "$version" ] && { log "versions agree"; exit 0; }
    warn "MISMATCH — tracy-capture will connect and be REFUSED, and the run"
    warn "will lose its per-system zones. Fix: $0"
    exit 1
fi

if [ "$installed" = "$version" ] && [ -x "$bin_dir/tracy-csvexport" ]; then
    log "already installed"
    exit 0
fi

have cmake || { warn "cmake is required (apt install cmake build-essential)"; exit 1; }
have git || { warn "git is required"; exit 1; }

src_dir="$cache_dir/tracy-$version"
mkdir -p "$cache_dir" "$bin_dir"
if [ ! -d "$src_dir/.git" ]; then
    rm -rf "$src_dir"
    log "cloning Tracy v$version"
    git clone --depth 1 --branch "v$version" \
        https://github.com/wolfpld/tracy.git "$src_dir" >/dev/null 2>&1 \
        || { warn "could not clone Tracy v$version"; exit 1; }
fi

ok=1
build_tracy_tool "$src_dir" capture tracy-capture || ok=0
build_tracy_tool "$src_dir" csvexport tracy-csvexport || ok=0
[ "$ok" -eq 1 ] || { warn "the Tracy CLI tools did not build"; exit 1; }

log "installed tracy-capture and tracy-csvexport ($version) to $bin_dir"
case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) warn "$bin_dir is not on PATH; add it to use tracy-capture" ;;
esac
