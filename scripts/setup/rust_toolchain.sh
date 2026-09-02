#!/usr/bin/env bash
# Install rustup and the stable toolchain the desktop build needs.
#
# Usage:
#   scripts/setup/rust_toolchain.sh
#   scripts/setup/rust_toolchain.sh --profile   # + the cargo analysis tools
#   scripts/setup/rust_toolchain.sh --help
#
# ⚠ rustup appends to a shell profile the CALLING shell has already read, so a
# script that needs cargo afterwards must source `~/.cargo/env` itself;
# `scripts/lib/cargo_env.sh` does that for the repository's entry points.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=rust
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

skip_rust=0
want_profiling=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --profile) want_profiling=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

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

ensure_rust() {
    if [ "$skip_rust" -eq 1 ]; then
        log "skipping Rust setup"
        return 0
    fi

    if ! have rustup; then
        have curl || fatal "curl is required to install rustup"
        log "installing rustup and the stable Rust toolchain"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --profile default --default-toolchain stable
    fi

    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi

    have rustup || fatal "rustup is not on PATH after installation"
    rustup toolchain install stable
    rustup default stable
    rustup component add rustfmt clippy llvm-tools-preview
    have cargo || fatal "cargo is not on PATH after Rust setup"

    # ⛔ NONE of these is on the path from a clone to a running game, and every
    # one is a source build. They were unconditional, and together they were the
    # single largest block of a default run — cargo-modules alone compiles the
    # rust-analyzer crate graph. Each is reachable without them:
    #   cargo-llvm-cov    only `run_game.sh --cov`
    #   cargo-mark-sweep  only `scripts/sweep_cargo_target.sh`, which already
    #                     prints its own install line when it is missing
    #   cargo-modules     no caller in the repo
    #   cargo-sweep       no caller in the repo
    #   cargo-nextest     `scripts/run_tests.py` says so itself, at its
    #                     definition: "OPTIONAL, NOT REQUIRED. A contributor
    #                     without nextest still gets the same" results, because
    #                     the runner falls back to plain `cargo test`.
    if [ "$want_profiling" -eq 1 ]; then
        ensure_cargo_tool cargo-llvm-cov cargo-llvm-cov
        ensure_cargo_tool cargo-modules cargo-modules
        ensure_cargo_tool cargo-sweep cargo-sweep
        ensure_cargo_tool cargo-mark-sweep cargo-mark-sweep
        # Per-test wall times, which stable libtest cannot report:
        # `--report-time` is nightly-only, so ranking the slowest tests in a
        # 1600s suite otherwise means timing each test binary by hand. nextest
        # also runs each test in its own process, so a test that only passes on
        # a sibling's leftover state shows up as a failure instead of hiding.
        ensure_cargo_tool cargo-nextest cargo-nextest
    else
        log "cargo analysis tools not requested (--profile adds them)"
    fi

    log "Rust ready: $(rustc --version)"
}

ensure_rust
