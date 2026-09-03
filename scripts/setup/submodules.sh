#!/usr/bin/env bash
# Initialize the authoring submodules recursively.
#
# Usage:
#   scripts/setup/submodules.sh
#   scripts/setup/submodules.sh --help
#
# An empty authoring directory does NOT mean the capability is absent; see the
# canonical repositories listed in AGENTS.md.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=submodules
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

skip_submodules=0
case "${1:-}" in
    -h|--help) setup_usage "$0"; exit 0 ;;
    '') ;;
    *) fatal "unknown option: $1" ;;
esac

ensure_submodules() {
    if [ "$skip_submodules" -eq 1 ]; then
        log "skipping git submodule setup"
        return 0
    fi
    have git || fatal "git is required for submodule setup"
    [ -f "$repo_root/.gitmodules" ] || return 0

    # ⛔⛔ INITIALIZE WHAT IS MISSING; NEVER MOVE WHAT IS THERE. A bare
    # `git submodule update --init --recursive` moves EVERY submodule to the
    # recorded gitlink, detaching whatever branch was checked out. The commits
    # survive as a branch ref; the working tree does not. Measured 2026-09-02: a
    # setup run silently reverted an in-progress fix in
    # `ambition_music_renderer`, the next asset regen failed with the exact
    # error that fix removes, and a commit made afterwards landed on the
    # detached HEAD where no branch could reach it.
    #
    # ⭐ REPORTING THE DAMAGE WAS THE WRONG FIX, and this file carried it for a
    # few hours. Setup's job is to make an UNPREPARED checkout ready, not to
    # decide that someone's in-progress branch should be somewhere else. A
    # gitlink mismatch is INFORMATION for the person who made it, never a
    # licence to relocate their working tree.
    #
    # `sync` is kept: it rewrites URLs from `.gitmodules` and moves no checkout.
    log "syncing submodule URLs"
    git -C "$repo_root" submodule sync --recursive >/dev/null

    local path initialized=0 created=0 ahead=0
    while read -r path; do
        [ -n "$path" ] || continue
        if [ -e "$repo_root/$path/.git" ]; then
            initialized=$((initialized + 1))
            report_gitlink_drift "$path" && ahead=$((ahead + 1))
            continue
        fi
        log "initializing $path"
        git -C "$repo_root" submodule update --init --recursive -- "$path"
        created=$((created + 1))
    done < <(git -C "$repo_root" ls-files --stage \
        | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    log "submodules: $created initialized, $initialized left as they were"

    # Verify against real gitlinks (index mode 160000), not `.gitmodules`
    # entries. A `.gitmodules` block whose gitlink was dropped is a stale
    # declaration that git correctly ignores; treating it as fatal bricks setup
    # for every fresh clone.
    while read -r path; do
        [ -n "$path" ] || continue
        [ -d "$repo_root/$path" ] || fatal "submodule path was not initialized: $path"
        if [ -z "$(find "$repo_root/$path" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
            fatal "submodule path is empty after update: $path"
        fi
    done < <(git -C "$repo_root" ls-files --stage \
        | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    local declared
    while read -r _ declared; do
        [ -n "$declared" ] || continue
        if ! git -C "$repo_root" ls-files --stage -- "$declared" | grep -q '^160000'; then
            warn "stale .gitmodules entry with no gitlink (ignored): $declared"
        fi
    done < <(git config -f "$repo_root/.gitmodules" --get-regexp '^submodule\..*\.path$' || true)
}

# Say that a checked-out submodule is not at its recorded gitlink, and leave it
# exactly where it is. Returns 0 when it reported drift.
report_gitlink_drift() {
    local path="$1" recorded checked branch
    recorded="$(git -C "$repo_root" ls-files --stage -- "$path" | awk '{print $2}')"
    checked="$(git -C "$repo_root/$path" rev-parse HEAD 2>/dev/null || true)"
    [ -n "$recorded" ] && [ -n "$checked" ] || return 1
    [ "$recorded" != "$checked" ] || return 1
    branch="$(git -C "$repo_root/$path" branch --show-current 2>/dev/null || true)"
    warn "$path is not at its recorded gitlink and was LEFT ALONE"
    log "   checked out ${branch:-detached} at ${checked:0:9}, superproject records ${recorded:0:9}"
    log "   commit the submodule and record the pointer, or move it yourself:"
    log "     git -C $path checkout ${recorded:0:9}"
    return 0
}

ensure_submodules
