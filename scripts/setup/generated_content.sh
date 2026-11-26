#!/usr/bin/env bash
# Update and check all generated runtime assets and the `.agent/` index.
#
# Usage:
#   scripts/setup/generated_content.sh
#   scripts/setup/generated_content.sh --skip-navigation
#   scripts/setup/generated_content.sh --help
#
# ⚠ Needs `scripts/setup/python_tools.sh` and `scripts/setup/audio_libraries.sh`
# first: a category whose toolchain is absent REFUSES rather than publishing
# degraded art, and music refuses rather than rendering General-MIDI stand-ins.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=generated-content
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
setup_load_cargo_env

skip_navigation=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-navigation) skip_navigation=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

verify_generated_content_is_current() {
    python3 "$repo_root/scripts/regen/generate_library_content.py" --check
    python3 "$repo_root/scripts/checks/check_quality_variants_are_fresh.py"
    python3 "$repo_root/scripts/checks/check_regeneration_coverage.py"
}

# ⛔ `scripts/agent_query.py` IS STEP 2 OF THE COLD START AGENTS.md MANDATES, AND
# ON A FRESH CLONE IT ANSWERED NOTHING. The `.agent/` packets it queries are
# generated, not committed — `git ls-files .agent` is two files — so a new
# checkout got `⚠ index has no generation stamp` and an empty packet from the
# first command the guide tells an agent to run. Every other kind of generated
# content in this repo is built by this script; this one was left to a person
# knowing to run it.
#
# Reports rather than fails: `source_navigation.sh` ends with the agent-KB
# audit, which is repository DOC hygiene (frontmatter keys, stray files, a
# dangling doc reference) and has nothing to do with whether this checkout is
# runnable. Setup must not refuse a working environment over it.
regenerate_source_navigation() {
    [ -x "$repo_root/scripts/regen/source_navigation.sh" ] || return 0
    # The ECS inventory resolves the dependency graph through cargo, and prints
    # `resolved dependency graph unavailable` and carries on when it cannot —
    # a quietly poorer index rather than an error.
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    log "regenerating .agent/ navigation"
    if "$repo_root/scripts/regen/source_navigation.sh" >/dev/null 2>&1; then
        log "generated navigation is current"
        return 0
    fi
    # The indexes are written before the audit runs, so a non-zero exit here
    # does not mean they are missing. Say which it was.
    if [ -d "$repo_root/.agent/index" ]; then
        warn "navigation regenerated, but the agent-KB audit reported doc-hygiene problems"
        log "   they do not affect the build:"
        log "     ./scripts/regen/source_navigation.sh   # what it objects to"
    else
        warn "navigation regeneration FAILED; scripts/agent_query.py will answer nothing"
        log "     ./scripts/regen/source_navigation.sh   # see why"
    fi
}

log "updating missing, damaged, or stale runtime assets"
setup_limit_jobs
"$repo_root/scripts/regen/assets.sh" --jobs "$AMBITION_SETUP_JOBS"
export AMBITION_MUSIC_JOBS="$AMBITION_SETUP_JOBS"
verify_generated_content_is_current
if [ "$skip_navigation" -eq 0 ]; then
    regenerate_source_navigation
fi
