#!/usr/bin/env bash
# Build and link the desktop game.
#
# Usage:
#   scripts/setup/desktop_check.sh
#   scripts/setup/desktop_check.sh --help
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
AMBITION_SETUP_LABEL=desktop-build
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
setup_load_cargo_env

case "${1:-}" in
    -h|--help) setup_usage "$0"; exit 0 ;;
    '') ;;
    *) fatal "unknown option: $1" ;;
esac

have cargo || fatal "cargo is required. Run scripts/setup/rust_toolchain.sh."
setup_limit_jobs
"$repo_root/scripts/setup/target_bindmount.sh"
log "building the desktop game"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$AMBITION_SETUP_JOBS}" \
    cargo build --locked -p ambition_app --bin ambition_game_bin
