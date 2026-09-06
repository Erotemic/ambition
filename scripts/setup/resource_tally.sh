#!/usr/bin/env bash
# Arm the LLM resource-accounting git hook.
#
# Usage:
#   scripts/setup/resource_tally.sh
#   scripts/setup/resource_tally.sh --help
#
# Offline, idempotent, and never allowed to block a checkout from becoming
# runnable: accounting is bookkeeping, not a runtime dependency.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=resource-tally
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

skip_tally=0
case "${1:-}" in
    -h|--help) setup_usage "$0"; exit 0 ;;
    '') ;;
    *) fatal "unknown option: $1" ;;
esac

ensure_resource_tally() {
    if [ "$skip_tally" -eq 1 ]; then
        log "skipping LLM resource-accounting hook install"
        return 0
    fi
    [ -e "$repo_root/.llm_resource_tally/tool" ] || return 0
    have python3 || { warn "python3 not found; skipping resource-accounting hook install"; return 0; }

    # Offline and idempotent (it only points core.hooksPath at the committed
    # hook dir and refreshes the managed AGENTS.md block). Accounting is
    # bookkeeping, not a runtime dependency, so a failure here must never
    # block a checkout from becoming runnable.
    log "arming the LLM resource-accounting git hook"
    if ! python3 "$repo_root/.llm_resource_tally/tool" install; then
        warn "resource-accounting hook install failed; continuing (run it by hand later)"
    fi
}

ensure_resource_tally
