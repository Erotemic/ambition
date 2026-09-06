#!/usr/bin/env bash
# Fetch locked dependencies and check the desktop game target compiles.
#
# Usage:
#   scripts/setup/desktop_check.sh
#   scripts/setup/desktop_check.sh --help
#
# ⛔ `cargo check -p <one_crate>` is NOT the compile gate; a crate-local check
# can be green while the assembled app fails. This checks `ambition_app`.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
AMBITION_SETUP_LABEL=desktop-check
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
setup_load_cargo_env

skip_cargo_check=0
case "${1:-}" in
    -h|--help) setup_usage "$0"; exit 0 ;;
    '') ;;
    *) fatal "unknown option: $1" ;;
esac

# **The committed `target-dir` is one user's home, and a bare `cargo build` by
# anyone else dies with `Permission denied` at a path they never chose.**
#
# `.cargo/config.toml` sets `target-dir = "/home/joncrall/ambition-target"`, and
# the REASON is sound and documented there: the repo sits on a virtiofs share, so
# an identical path string resolving to different local disks keeps VM and host
# fingerprints from co-mingling. It is only the username that does not travel.
#
# **and it cannot be overridden by a per-user config.** Cargo merges config
# files with the one nearest the working directory winning, so the repo's
# `.cargo/config.toml` beats `~/.cargo/config.toml`. `CARGO_TARGET_DIR` in the
# ENVIRONMENT is the only override — which is why `run_game.sh` and the
# rust-analyzer bridge both export it and work fine.
#
# So this does not change the config, the build, or anyone's cache. It turns an
# unexplained linker-adjacent permission error at the end of a long first build
# into a sentence at the start of setup.
check_cargo_target_dir_is_reachable() {
    if [ -n "${CARGO_TARGET_DIR:-}" ]; then
        return 0
    fi
    configured=$(sed -n 's/^target-dir *= *"\(.*\)"/\1/p' .cargo/config.toml 2>/dev/null | head -1)
    [ -n "$configured" ] || return 0
    parent=$(dirname "$configured")
    if [ -w "$configured" ] 2>/dev/null || [ -w "$parent" ] 2>/dev/null; then
        return 0
    fi
    log "⛔ the committed cargo target-dir is not writable by $(whoami): $configured"
    log "   a bare 'cargo build' will fail with Permission denied at that path."
    log "   export CARGO_TARGET_DIR to somewhere you own, for example:"
    log "     export CARGO_TARGET_DIR=\"\$HOME/ambition-target\""
    log "   ⚠ a per-user ~/.cargo/config.toml will NOT help: cargo lets the"
    log "   config nearest the working directory win, so the repo's beats yours."
    log "   see dev/journals/code_smells.md, 2026-07-26 entry, for why the path"
    log "   is absolute in the first place (virtiofs cache separation)."
}

check_desktop_target() {
    if [ "$skip_cargo_check" -eq 1 ]; then
        log "skipping Cargo fetch/check"
        return 0
    fi
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    have cargo || fatal "cargo is required for the desktop target check"

    log "fetching locked Cargo dependencies"
    cargo fetch --locked
    log "checking the desktop game target"
    cargo check --locked -p ambition_app --bin ambition_game_bin
}

check_cargo_target_dir_is_reachable
check_desktop_target
