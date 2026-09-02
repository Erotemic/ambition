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

    # ⛔⛔ `git submodule update` MOVES A SUBMODULE TO THE RECORDED GITLINK AND
    # DETACHES WHATEVER BRANCH WAS THERE. The commits survive as a branch ref;
    # the WORKING TREE does not. Measured 2026-09-02: a setup run silently
    # reverted an in-progress fix in `ambition_music_renderer`, the next asset
    # regen failed with the exact error that fix removes, and a commit made
    # afterwards landed on the detached HEAD where no branch could see it.
    #
    # Nothing here can stop git doing its job, so the job is to SAY SO — with
    # the branch name, so restoring is one command instead of an archaeology
    # session in the reflog.
    local path branch
    local -A prior_branch=()
    while read -r path; do
        [ -n "$path" ] || continue
        branch="$(git -C "$repo_root/$path" branch --show-current 2>/dev/null || true)"
        [ -n "$branch" ] && prior_branch["$path"]="$branch"
    done < <(git -C "$repo_root" ls-files --stage | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    log "syncing and initializing git submodules recursively"
    git submodule sync --recursive
    git submodule update --init --recursive

    local detached=0
    for path in "${!prior_branch[@]}"; do
        if [ -z "$(git -C "$repo_root/$path" branch --show-current 2>/dev/null || true)" ]; then
            if [ "$detached" -eq 0 ]; then
                warn "submodules were DETACHED from the branch they were on:"
                detached=1
            fi
            printf '    git -C %s checkout %s
' "$path" "${prior_branch[$path]}" >&2
        fi
    done
    if [ "$detached" -eq 1 ]; then
        log "   their commits are safe on those branches; the working trees moved"
    fi

    # Verify against real gitlinks (index mode 160000), not `.gitmodules` entries.
    # A `.gitmodules` block whose gitlink was dropped is a stale declaration that
    # `git submodule update` correctly ignores; treating it as fatal bricks setup
    # for every fresh clone. Only a gitlink git *should* have materialized is an error.
    local path
    while read -r path; do
        [ -n "$path" ] || continue
        [ -d "$repo_root/$path" ] || fatal "submodule path was not initialized: $path"
        if [ -z "$(find "$repo_root/$path" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
            fatal "submodule path is empty after update: $path"
        fi
    done < <(git -C "$repo_root" ls-files --stage | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    # A declared submodule with no gitlink can never be initialized. Warn so the
    # stale entry gets cleaned up, but do not block the rest of setup.
    local declared
    while read -r _ declared; do
        [ -n "$declared" ] || continue
        if ! git -C "$repo_root" ls-files --stage -- "$declared" | grep -q '^160000'; then
            warn "stale .gitmodules entry with no gitlink (ignored): $declared"
        fi
    done < <(git config -f "$repo_root/.gitmodules" --get-regexp '^submodule\..*\.path$' || true)
}

ensure_submodules
