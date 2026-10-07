#!/usr/bin/env bash
# Initialize the authoring submodules and put each one on its `main`.
#
# Usage:
#   scripts/setup/submodules.sh
#   scripts/setup/submodules.sh --bump-pins   # also stage gitlinks that lag main
#   scripts/setup/submodules.sh --help
#
# THE RULE: every submodule is on its `main` branch (tracking `origin/main`),
# and a gitlink that disagrees with a submodule's `main` means the PIN is stale:
# update the pin, do not hold the checkout back. So, per initialized submodule
# that has no uncommitted changes:
#   - at `origin/main`, detached or on another branch name: attach `main` (no
#     content moves);
#   - strictly behind `origin/main`: fast-forward `main` to it;
#   - on the PRE-SPLIT history (no common commit with `origin/main`; the split
#     was intentional): keep the old position under `refs/backup/pre-split/*`
#     and move to `origin/main`;
#   - ahead of `origin/main` (unpushed commits) or diverged from it: LEFT ALONE
#     and reported. That is somebody's work, and AGENTS.md says to keep the
#     semantic superset of a divergent line, not to pick one;
#   - dirty: LEFT ALONE.
# Then, for each gitlink that is not `origin/main`, it says the pin needs
# updating, and `--bump-pins` stages it (it never commits).
#
# An empty authoring directory does NOT mean the capability is absent; see the
# canonical repositories listed in AGENTS.md.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=submodules
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

skip_submodules=0
bump_pins=0
for arg in "$@"; do
    case "$arg" in
        -h|--help) setup_usage "$0"; exit 0 ;;
        --bump-pins) bump_pins=1 ;;
        *) fatal "unknown option: $arg" ;;
    esac
done

ensure_submodules() {
    if [ "$skip_submodules" -eq 1 ]; then
        log "skipping git submodule setup"
        return 0
    fi
    have git || fatal "git is required for submodule setup"
    [ -f "$repo_root/.gitmodules" ] || return 0

    # ⛔ A SETUP RUN MUST NOT LOSE ANYBODY'S WORK. History: a bare
    # `git submodule update --init --recursive` moved EVERY submodule to the
    # recorded gitlink and detached the branch (2026-09-02: it reverted an
    # in-progress fix, and a later commit landed on a detached HEAD). The rule is
    # now "every submodule is on main", and `sync_submodule` keeps that safe:
    # it moves a checkout only when it is clean AND the move cannot orphan a
    # commit (fast-forward, same commit, or the pre-split lineage, whose old
    # position is kept as a ref). Unpushed or divergent work is left in place.
    #
    # `sync` is kept: it rewrites URLs from `.gitmodules` and moves no checkout.
    log "syncing submodule URLs"
    git -C "$repo_root" submodule sync --recursive >/dev/null

    local path initialized=0 created=0 stale_pins=0 staged=0
    while read -r path; do
        [ -n "$path" ] || continue
        if [ -e "$repo_root/$path/.git" ]; then
            initialized=$((initialized + 1))
        else
            log "initializing $path"
            git -C "$repo_root" submodule update --init --recursive -- "$path"
            created=$((created + 1))
        fi
        sync_submodule "$path"
        if pin_lags_main "$path"; then
            stale_pins=$((stale_pins + 1))
            if [ "$bump_pins" -eq 1 ]; then
                git -C "$repo_root" add -- "$path"
                staged=$((staged + 1))
                log "   pin staged: $path -> $(git -C "$repo_root/$path" rev-parse --short HEAD)"
            fi
        fi
    done < <(git -C "$repo_root" ls-files --stage \
        | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    log "submodules: $created initialized, $initialized already present"
    if [ "$stale_pins" -gt 0 ] && [ "$bump_pins" -eq 0 ]; then
        warn "$stale_pins pin(s) are not at their submodule's origin/main: the PIN needs updating."
        log "   run: scripts/setup/submodules.sh --bump-pins   (stages the gitlinks), then commit them"
    elif [ "$staged" -gt 0 ]; then
        log "staged $staged pin(s); commit with: git commit -m 'Bump submodule pins to main'"
    fi

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

# Put one submodule on `main` as far as that is safe; see the header. Never
# discards a commit: every move is a fast-forward, a same-commit attach, or is
# preceded by a ref at the old position.
sync_submodule() {
    local path="$1" abs="$repo_root/$1" head main branch relation dirty ahead
    git -C "$abs" fetch -q origin 2>/dev/null \
        || warn "$path: could not fetch origin (offline?); judging against what is already fetched"
    main="$(git -C "$abs" rev-parse --verify -q origin/main || true)"
    if [ -z "$main" ]; then
        warn "$path has no origin/main; LEFT ALONE"
        return 0
    fi
    head="$(git -C "$abs" rev-parse HEAD)"
    branch="$(git -C "$abs" branch --show-current 2>/dev/null || true)"
    dirty="$(git -C "$abs" status --porcelain --untracked-files=no | head -1)"

    if [ "$head" = "$main" ]; then
        relation=same
    elif git -C "$abs" merge-base --is-ancestor "$head" "$main"; then
        relation=behind
    elif git -C "$abs" merge-base --is-ancestor "$main" "$head"; then
        relation=ahead
    elif [ -z "$(git -C "$abs" merge-base "$head" "$main" 2>/dev/null)" ]; then
        relation=unrelated
    else
        relation=diverged
    fi

    case "$relation" in
        same)
            if [ "$branch" != main ]; then
                git -C "$abs" checkout -q -B main origin/main
                log "$path: attached main at ${main:0:9} (was ${branch:-detached})"
            fi
            ;;
        behind)
            if [ -n "$dirty" ]; then
                warn "$path is behind origin/main but has uncommitted changes; LEFT ALONE"
                return 0
            fi
            git -C "$abs" checkout -q -B main origin/main
            log "$path: fast-forwarded to origin/main ${main:0:9} (from ${head:0:9})"
            ;;
        unrelated)
            # The history was split on purpose, so a checkout on the old lineage
            # is expected on an older machine. Keep where it was, then move.
            if [ -n "$dirty" ]; then
                warn "$path is on the pre-split history and has uncommitted changes; LEFT ALONE"
                return 0
            fi
            local backup="refs/backup/pre-split/${head:0:12}"
            git -C "$abs" update-ref "$backup" "$head"
            git -C "$abs" checkout -q -B main origin/main
            log "$path: was on the pre-split history (${head:0:9}); kept as $backup, now on origin/main ${main:0:9}"
            ;;
        ahead)
            ahead="$(git -C "$abs" rev-list --count "$main..$head")"
            warn "$path has $ahead commit(s) origin/main lacks and was LEFT ALONE"
            log "   push them (git -C $path push origin HEAD:main) or keep working; then bump the pin"
            ;;
        diverged)
            warn "$path has diverged from origin/main and was LEFT ALONE"
            log "   ours:   $(git -C "$abs" rev-list --count "$main..$head") commit(s) origin/main lacks"
            log "   theirs: $(git -C "$abs" rev-list --count "$head..$main") commit(s) this checkout lacks"
            log "   keep the semantic superset of both lines (AGENTS.md); do not pick by recency"
            ;;
    esac
}

# True when the recorded gitlink is not the submodule's `origin/main` and the
# checkout IS at `origin/main`, so staging it would be the right pin. A checkout
# that was left alone (ahead, diverged, dirty) is not a pin to bump.
pin_lags_main() {
    local path="$1" abs="$repo_root/$1" recorded main head
    recorded="$(git -C "$repo_root" ls-files --stage -- "$path" | awk '{print $2}')"
    main="$(git -C "$abs" rev-parse --verify -q origin/main || true)"
    head="$(git -C "$abs" rev-parse HEAD 2>/dev/null || true)"
    [ -n "$recorded" ] && [ -n "$main" ] || return 1
    [ "$recorded" != "$main" ] || return 1
    if [ "$head" = "$main" ]; then
        local why="not on main"
        git -C "$abs" merge-base --is-ancestor "$recorded" "$main" 2>/dev/null \
            && why="$(git -C "$abs" rev-list --count "$recorded..$main") commit(s) behind main"
        warn "$path: pin ${recorded:0:9} is $why (main is ${main:0:9})"
        return 0
    fi
    return 1
}

ensure_submodules
