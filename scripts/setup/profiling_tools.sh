#!/usr/bin/env bash
# The profiling and analysis toolchain: Tracy, cargo-flamegraph, and the cargo
# analysis tools (llvm-cov, modules, sweep, mark-sweep, nextest).
#
# Usage:
#   scripts/setup/profiling_tools.sh
#   scripts/setup/profiling_tools.sh --help
#
# ⭐ THE ONLY THING `run_developer_setup.sh` LEAVES OUT. Everything else it
# installs is required to build and run the game; this is required only to
# MEASURE it, costs ~190 apt packages plus several source builds, and is
# therefore `--profile`.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=profiling-tools
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
setup_load_cargo_env

want_profiling=1
case "${1:-}" in
    -h|--help) setup_usage "$0"; exit 0 ;;
    '') ;;
    *) fatal "unknown option: $1" ;;
esac

ensure_cargo_tool() {
    local package="$1"
    local binary="$2"
    if have "$binary"; then
        log "$binary already installed"
    else
        log "installing $package"
        cargo install --locked "$package"
    fi
}

ensure_profiling_tools() {
    if [ "$want_profiling" -eq 0 ]; then
        return 0
    fi

    # A flame graph as a file, for the workflow that does not want a GUI.
    if have cargo; then
        ensure_cargo_tool flamegraph flamegraph
    else
        warn "cargo unavailable; skipping cargo-flamegraph"
    fi

    # ⭐ ONE IMPLEMENTATION, AND IT IS THE STANDALONE ONE. This used to carry its
    # own copy of the Tracy build and its own version parse — and that parse read
    # the WORD "Major" instead of the number, so it asked git for a branch called
    # `vMajor.Minor.Patch`, failed to clone, and left whatever Tracy was already
    # there. That is how a 0.13.1 server survived beside a 0.14.0 client and cost
    # a capture on real hardware its per-system zones.
    #
    # It is also the whole reason the standalone script exists: somebody whose
    # only problem is a mismatched Tracy should not have to run submodule sync,
    # tool venvs and asset regeneration to fix it.
    if ! "$repo_root/scripts/setup/install_profiling_tools.sh"; then
        warn "Tracy tools not installed; perf-based profiling is unaffected"
    fi
}

# **`-lstdc++` fails at the END of a cold profiling build, and having `g++`
# installed does not prevent it.** Tracy's client is C++, so `tracy-client-sys`
# emits `cargo:rustc-link-lib=stdc++` and every `--features profile` link ends
# in `-lstdc++`. clang resolves that against the ONE gcc version directory it
# selected — the newest COMPLETE installation under `/usr/lib/gcc/<triple>/` —
# so if that directory has no `libstdc++.so`, the link dies with
# `mold: library not found: stdc++` while `g++` sits installed and innocent,
# because the version that owns the symlink is not the version clang picked.
#
# ⚠ Met on `calculex` 2026-08-29 (Ubuntu 22.04, clang 14): gcc 9 and gcc 11
# both had `libstdc++.so`, `apt install g++` reported "already the newest
# version", and the link still failed after all 537 rlibs had compiled.
#
# ⛔ The message reads like a stale incremental cache or a full disk. It is
# neither, and nothing under `target/` should be deleted for it.
#
# Same bargain as `check_cargo_target_dir_is_reachable` below: turn a linker
# error at the end of a twenty-minute build into a sentence at the start of
# setup, naming the package to install.
check_profiling_cxx_stdlib() {
    [ "$want_profiling" -eq 1 ] || return 0
    have clang || return 0
    # clang echoes the bare name back when it cannot resolve it, so the test is
    # "did I get a path that exists", not "did the command succeed".
    resolved=$(clang -print-file-name=libstdc++.so 2>/dev/null || true)
    [ -e "$resolved" ] && return 0

    # `-v` prints the selection before it fails on the empty translation unit.
    selected=$(clang -v -x c++ /dev/null -o /dev/null 2>&1 \
        | sed -n 's/^Selected GCC installation: //p' | head -1 || true)
    log "⛔ clang cannot resolve libstdc++.so; a --features profile build will"
    log "   fail at the LINK with: mold: library not found: stdc++"
    if [ -n "$selected" ]; then
        log "   clang selected GCC installation: $selected"
        log "   install the C++ stdlib dev package for THAT version:"
        log "     sudo apt install libstdc++-$(basename "$selected")-dev"
    else
        log "   could not read clang's GCC selection; run it by hand:"
        log "     clang -v -x c++ /dev/null -o /dev/null 2>&1 | grep 'GCC installation'"
    fi
    log "   ⚠ \"g++ is already the newest version\" does NOT settle this: the"
    log "   version that owns the symlink need not be the one clang picked."
    for d in /usr/lib/gcc/*/*/; do
        [ -e "$d/libstdc++.so" ] && log "   present in: $d"
    done
    log "   ⛔ this is not a stale cache and not a full disk — do not delete"
    log "   anything under target/ for it."
}

"$repo_root/scripts/setup/system_packages.sh" --profile
ensure_cargo_tool cargo-llvm-cov cargo-llvm-cov
ensure_cargo_tool cargo-modules cargo-modules
ensure_cargo_tool cargo-sweep cargo-sweep
ensure_cargo_tool cargo-mark-sweep cargo-mark-sweep
ensure_cargo_tool cargo-nextest cargo-nextest
ensure_profiling_tools
check_profiling_cxx_stdlib
