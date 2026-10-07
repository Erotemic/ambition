#!/usr/bin/env bash
# Initialize the authoring submodules and put each one on its `main`.
#
# Usage:
#   scripts/setup/submodules.sh
#   scripts/setup/submodules.sh --bump-pins   # also stage gitlinks that lag main
#   scripts/setup/submodules.sh --follow-main # development behaviour on a non-main checkout
#   scripts/setup/submodules.sh --help
#
# TWO MODES, chosen by what the SUPERPROJECT is, because "develop on main" and
# "look at an old commit" want opposite things from a general setup command:
#
#   DEVELOPMENT  the superproject is on its `main` branch (the normal case).
#                Submodules follow their own `origin/main`, as below.
#   REVIEW       the superproject is on a detached HEAD (`git checkout <sha>`,
#                `git bisect`, a rebase) or on any branch other than `main`.
#                An ALREADY-PRESENT submodule is NOT moved: running setup on a
#                historical Ambition must not quietly turn it into historical
#                code plus today's submodule tips. A missing submodule is
#                initialized at the commit this checkout records; a difference
#                from the recorded pin is REPORTED (it is not a "stale pin" here:
#                the recorded pin is the historical truth); `--bump-pins` is
#                refused. `--follow-main` selects development behaviour anyway,
#                for a deliberate feature branch or worktree that wants it.
#
# In EITHER mode a dirty, ahead or diverged submodule is never touched.
#
# THE DEVELOPMENT RULE: every submodule is on its `main` branch at `origin/main`,
# and a gitlink that disagrees with a submodule's `main` means the PIN is stale:
# update the pin, do not hold the checkout back. Setup moves a checkout there only
# when that DESTROYS NOTHING, and judges that by what the move would overwrite, not
# by whether `git status` is empty. Four facts are kept apart: where HEAD is, where
# LOCAL `main` is, where `origin/main` is, and what the move would write.
#
#   - local `main` has commits origin/main lacks (ahead), or has diverged: it is
#     somebody's unpushed work and is NEVER reset or moved, whichever branch is
#     checked out; the checkout is left as it is and the state is reported;
#   - HEAD is on commits origin/main lacks (a branch or a detached HEAD): left;
#   - HEAD is on a NAMED non-main branch with tracked edits: left on that branch
#     (switching would take work in progress off the branch it belongs to);
#   - the move would overwrite a locally modified TRACKED path, or write over an
#     UNTRACKED or IGNORED path that exists on disk: nothing is changed and the
#     path is named. An untracked file the move does not need is NOT dirty and
#     blocks nothing;
#   - otherwise: attach `main` (creating it, or fast-forwarding it while it is an
#     ancestor of origin/main). Tracked edits the move does not touch are carried
#     through. Operations are non-forcing: a fast-forward merge, a ref update
#     guarded by the old value, a checkout git refuses when it would lose work;
#   - on the PRE-SPLIT history (no common commit with `origin/main`; the split was
#     intentional): keep the old HEAD and local main under
#     `refs/backup/pre-split/*`, then move. This is the ONLY place a branch is
#     reset (`checkout -B`), and nothing is orphaned by it.
# Then, for each gitlink that is not `origin/main`, it says the pin needs
# updating, and `--bump-pins` stages it (it never commits).
#
# Setup never commits, merges, rebases or discards on anyone's behalf; when work
# blocks convergence it says what the work is and what to do.
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
follow_main=0
for arg in "$@"; do
    case "$arg" in
        -h|--help) setup_usage "$0"; exit 0 ;;
        --bump-pins) bump_pins=1 ;;
        --follow-main) follow_main=1 ;;
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

    local path initialized=0 created=0 stale_pins=0 staged=0 mode super_ref
    mode=development
    super_ref="$(git -C "$repo_root" symbolic-ref -q --short HEAD 2>/dev/null || true)"
    if [ "$super_ref" != main ] && [ "$follow_main" -eq 0 ]; then
        mode=review
        if [ -n "$super_ref" ]; then
            super_ref="on branch '$super_ref'"
        else
            super_ref="on a detached HEAD at $(git -C "$repo_root" rev-parse --short HEAD)"
        fi
        warn "the superproject is $super_ref, not main: REVIEW mode. Present submodules are NOT moved to origin/main."
        log "   (a missing one is initialized at the commit this checkout records; --follow-main overrides)"
        if [ "$bump_pins" -eq 1 ]; then
            fatal "--bump-pins stages gitlinks from a development checkout; this one is in review mode (use --follow-main if that is deliberate)"
        fi
    fi
    while read -r path; do
        [ -n "$path" ] || continue
        if [ -e "$repo_root/$path/.git" ]; then
            initialized=$((initialized + 1))
        else
            log "initializing $path"
            git -C "$repo_root" submodule update --init --recursive -- "$path"
            created=$((created + 1))
        fi
        if [ "$mode" = review ]; then
            report_submodule "$path"
            continue
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

# REVIEW mode: move nothing; say how the present checkout relates to the commit
# this superproject checkout records, so a mixture is visible and never silent.
report_submodule() {
    local path="$1" abs="$repo_root/$1" recorded head dirty
    recorded="$(git -C "$repo_root" ls-files --stage -- "$path" | awk '{print $2}')"
    head="$(git -C "$abs" rev-parse HEAD 2>/dev/null || true)"
    dirty="$(git -C "$abs" status --porcelain --untracked-files=no 2>/dev/null | head -1)"
    [ -n "$dirty" ] && warn "$path has uncommitted changes (left exactly as it is)"
    if [ -z "$head" ] || [ "$head" = "$recorded" ]; then
        return 0
    fi
    local rel="a different line of history"
    if git -C "$abs" cat-file -e "$recorded^{commit}" 2>/dev/null; then
        if git -C "$abs" merge-base --is-ancestor "$recorded" "$head"; then
            rel="$(git -C "$abs" rev-list --count "$recorded..$head") commit(s) AHEAD of the recorded pin"
        elif git -C "$abs" merge-base --is-ancestor "$head" "$recorded"; then
            rel="$(git -C "$abs" rev-list --count "$head..$recorded") commit(s) BEHIND the recorded pin"
        fi
    else
        rel="at a commit that is not the recorded pin (the pin is not fetched here)"
    fi
    warn "$path is at ${head:0:9} but this checkout records ${recorded:0:9}: $rel; left where it is"
    log "   to see the state this commit recorded: git submodule update -- $path   (only when it is clean)"
}

# How commit A relates to commit B: absent (A is empty), same, behind (A is an
# ancestor of B), ahead (B is an ancestor of A), diverged, or unrelated (no
# common commit; the intentional history split).
commit_relation() {
    local abs="$1" a="$2" b="$3"
    if [ -z "$a" ]; then echo absent
    elif [ "$a" = "$b" ]; then echo same
    elif git -C "$abs" merge-base --is-ancestor "$a" "$b"; then echo behind
    elif git -C "$abs" merge-base --is-ancestor "$b" "$a"; then echo ahead
    elif [ -z "$(git -C "$abs" merge-base "$a" "$b" 2>/dev/null)" ]; then echo unrelated
    else echo diverged
    fi
}

# Would moving the checkout from HEAD to `target` overwrite local work? Judged by
# what the move WRITES, not by whether `git status` is empty:
#
#   tracked   a path the move changes that is also modified/staged locally
#   untracked a path the move ADDS that already exists on disk (an untracked file,
#             a file in the way of a directory, OR AN IGNORED FILE: git itself
#             overwrites those silently, so this check does not leave them to it)
#
# An untracked file the move does not need is not a hazard and is not reported.
# Sets `hazard_tracked` / `hazard_untracked` (newline lists); returns 0 when there
# is a hazard.
transition_hazard() {
    local abs="$1" target="$2" incoming local_edits p d
    hazard_tracked=""
    hazard_untracked=""
    incoming="$(git -C "$abs" -c core.quotepath=off diff --name-only HEAD "$target" -- 2>/dev/null | sort -u)"
    local_edits="$(git -C "$abs" -c core.quotepath=off diff --name-only HEAD -- 2>/dev/null | sort -u)"
    if [ -n "$incoming" ] && [ -n "$local_edits" ]; then
        hazard_tracked="$(comm -12 <(printf '%s\n' "$incoming") <(printf '%s\n' "$local_edits"))"
    fi
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        if [ -e "$abs/$p" ] || [ -L "$abs/$p" ]; then
            hazard_untracked+="$p"$'\n'
            continue
        fi
        d="$p"
        while [ "$d" != "${d%/*}" ]; do
            d="${d%/*}"
            if [ -e "$abs/$d" ] && [ ! -d "$abs/$d" ]; then
                hazard_untracked+="$p (blocked by the file $d)"$'\n'
                break
            fi
        done
    done < <(git -C "$abs" -c core.quotepath=off diff --name-only --diff-filter=ACR HEAD "$target" -- 2>/dev/null)
    [ -n "$hazard_tracked" ] || [ -n "$hazard_untracked" ]
}

# Say what blocked the move and what to do. Never "dirty": the state is named.
explain_hazard() {
    local path="$1"
    if [ -n "$hazard_tracked" ]; then
        warn "$path has local tracked edits that would be overwritten by following origin/main: $(printf '%s' "$hazard_tracked" | head -5 | tr '\n' ' ')"
        log "   nothing was changed. Inspect: git -C $path status ; git -C $path diff"
        log "   If the edits are important, review them, commit them on the appropriate branch,"
        log "   integrate that into the submodule's main and push, then rerun setup. Setup will not discard them."
    fi
    if [ -n "$hazard_untracked" ]; then
        warn "$path has an untracked path that following origin/main needs to write: $(printf '%s' "$hazard_untracked" | head -5 | tr '\n' ' ')"
        log "   nothing was changed. Move or delete it if it is not needed; if it is meaningful, add and commit it"
        log "   appropriately and integrate it into main. Then rerun setup. Setup will not overwrite it."
    fi
}

log_local_main_work() {
    local path="$1" abs="$repo_root/$1" rel="$2" main="$3" local_main="$4" n
    case "$rel" in
        ahead)
            n="$(git -C "$abs" rev-list --count "$main..$local_main")"
            warn "$path: local main has $n commit(s) origin/main lacks (unpushed development); main was NOT moved and the checkout was left as it is"
            log "   inspect: git -C $path log --oneline origin/main..main"
            log "   If that work is important: review it, commit anything uncommitted as appropriate, integrate it into main,"
            log "   push, then rerun setup. Setup never resets or discards it."
            ;;
        diverged)
            warn "$path: local main has diverged from origin/main and needs reconciliation; main was NOT moved and the checkout was left as it is"
            log "   local main: $(git -C "$abs" rev-list --count "$main..$local_main") commit(s) origin/main lacks"
            log "   origin/main: $(git -C "$abs" rev-list --count "$local_main..$main") commit(s) local main lacks"
            log "   Reconcile: merge origin/main into main keeping the semantic superset of both lines (AGENTS.md), push, then rerun setup."
            ;;
    esac
}

# Put one submodule on `main` at `origin/main` as far as that is SAFE. The
# invariant: no local commit becomes unreachable and no local file or edit is
# overwritten because setup wanted `main` to follow `origin/main`.
#
# The facts are kept separate: where HEAD is, where LOCAL `main` is, where
# `origin/main` is, whether the move would overwrite tracked edits, and whether it
# would overwrite an untracked path. Operations are the non-forcing ones: a
# fast-forward merge, a ref update guarded by the old value, or a checkout git
# refuses when it would lose something. `checkout -B` (which resets a branch) is
# used ONLY on the intentional pre-split migration, after both old positions are
# kept under refs/backup/pre-split/*.
sync_submodule() {
    local path="$1" abs="$repo_root/$1" head main branch local_main main_rel head_rel old n
    git -C "$abs" fetch -q origin 2>/dev/null \
        || warn "$path: could not fetch origin (offline?); judging against what is already fetched"
    main="$(git -C "$abs" rev-parse --verify -q origin/main || true)"
    if [ -z "$main" ]; then
        warn "$path has no origin/main; LEFT ALONE"
        return 0
    fi
    head="$(git -C "$abs" rev-parse HEAD)"
    branch="$(git -C "$abs" branch --show-current 2>/dev/null || true)"
    local_main="$(git -C "$abs" rev-parse --verify -q refs/heads/main || true)"
    main_rel="$(commit_relation "$abs" "$local_main" "$main")"
    head_rel="$(commit_relation "$abs" "$head" "$main")"

    # 1. LOCAL main with work origin/main lacks is protected before anything else,
    #    whichever branch is checked out: its commits are only reachable from it.
    case "$main_rel" in
        ahead|diverged)
            log_local_main_work "$path" "$main_rel" "$main" "$local_main"
            return 0
            ;;
    esac

    # 2. The checkout itself: work on a branch/detached HEAD that origin/main lacks.
    if [ "$branch" != main ]; then
        case "$head_rel" in
            ahead|diverged)
                local where="a detached HEAD"
                [ -z "$branch" ] || where="branch '$branch'"
                if [ "$head_rel" = diverged ]; then
                    warn "$path: $where has diverged from origin/main and was LEFT ALONE"
                    log "   ours: $(git -C "$abs" rev-list --count "$main..$head") commit(s) origin/main lacks"
                    log "   theirs: $(git -C "$abs" rev-list --count "$head..$main") commit(s) this checkout lacks"
                    log "   Reconcile: merge origin/main in, keeping the semantic superset of both lines (AGENTS.md); do not pick by recency."
                else
                    n="$(git -C "$abs" rev-list --count "$main..$head")"
                    warn "$path: $where has $n commit(s) origin/main lacks and was LEFT ALONE"
                    log "   If that work is important, review it, commit anything uncommitted, integrate it into main and push."
                fi
                [ -n "$branch" ] || log "   (HEAD is detached: keep it with: git -C $path switch -c <name>)"
                log "   Then rerun setup. Setup never moves a checkout off unpushed work."
                return 0
                ;;
        esac
        # A named non-main branch with tracked edits: switching would take the work
        # in progress off the branch it belongs to, even when the commits match.
        if [ -n "$branch" ] && ! git -C "$abs" diff --quiet HEAD --; then
            warn "$path is on branch '$branch' with tracked edits; LEFT on '$branch' (switching to main would carry them off it)"
            log "   Commit the work on '$branch', integrate it into main, push, then rerun setup."
            return 0
        fi
    fi

    # 3. Already there.
    if [ "$branch" = main ] && [ "$head_rel" = same ]; then
        return 0
    fi

    # 4. The move would write the tree between HEAD and origin/main.
    if transition_hazard "$abs" "$main"; then
        explain_hazard "$path"
        return 0
    fi

    # 5. The intentional history split: keep BOTH old positions, then move. This
    #    is the only place a branch is reset, and nothing is orphaned by it.
    if [ "$head_rel" = unrelated ] || [ "$main_rel" = unrelated ]; then
        local pos
        for pos in "$head" "$local_main"; do
            [ -n "$pos" ] || continue
            [ "$(commit_relation "$abs" "$pos" "$main")" = unrelated ] || continue
            git -C "$abs" update-ref "refs/backup/pre-split/${pos:0:12}" "$pos"
            log "$path: pre-split position ${pos:0:9} kept as refs/backup/pre-split/${pos:0:12}"
        done
        git -C "$abs" checkout -q -B main origin/main \
            || { warn "$path: git refused the pre-split move (nothing was lost; the old positions are kept under refs/backup/pre-split/*)"; return 0; }
        log "$path: moved to origin/main ${main:0:9} (was on the pre-split history)"
        return 0
    fi

    # 6. The ordinary case: local main is absent, equal, or behind origin/main.
    if [ "$branch" = main ]; then
        git -C "$abs" merge -q --ff-only origin/main \
            || { warn "$path: git refused the fast-forward (nothing was changed)"; return 0; }
        log "$path: fast-forwarded main to origin/main ${main:0:9} (from ${head:0:9})"
        return 0
    fi
    if [ -z "$local_main" ]; then
        git -C "$abs" branch -q --track main origin/main
    elif [ "$local_main" != "$main" ]; then
        # local main is BEHIND origin/main (an ancestor), so this loses no commit;
        # the old value guards against it having moved since it was read.
        git -C "$abs" update-ref refs/heads/main "$main" "$local_main"
    fi
    git -C "$abs" checkout -q main \
        || { warn "$path: git refused to switch to main (nothing was lost)"; return 0; }
    log "$path: now on main at origin/main ${main:0:9} (was ${branch:-detached} at ${head:0:9})"
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
