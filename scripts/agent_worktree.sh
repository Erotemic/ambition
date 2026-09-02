#!/usr/bin/env bash
#
# The three fixed worktrees agents and subagents work in.
#
# Slots are NAMED BY NUMBER, never by feature. A coordinator assigns a slot; the
# agent creates whatever branch it likes inside it. Feature-named worktrees are
# what this replaces: they accumulate, nobody knows which are live, and each one
# costs a cold target directory.
#
# Usage:
#   scripts/agent_worktree.sh list
#   scripts/agent_worktree.sh setup 1|2|3|all
#   scripts/agent_worktree.sh seed 2 [--from PATH]
#   scripts/agent_worktree.sh clear 2 [--incremental|--all]
#   scripts/agent_worktree.sh jobs 2
#
# Policy, and what a subagent needs to read: docs/tools/agent-worktrees.md
set -euo pipefail

MAIN="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SLOTS=(1 2 3)

die() { printf 'agent_worktree: %s\n' "$*" >&2; exit 2; }

slot_path() { printf '%s/.worktrees/agent-worktree%s' "$MAIN" "$1"; }

# ── CPU budget ────────────────────────────────────────────────────────────────
# main gets the machine; each slot gets half of the one above it. A coordinator
# overrules by passing -j explicitly. The point is not precision, it is that
# three agents building at once must not each believe they own 12 cores.
slot_jobs() {
    local n total; total="$(nproc)"
    case "$1" in
        main|0) printf '%s' "$total" ;;
        1) printf '%s' "$(( total / 2 > 0 ? total / 2 : 1 ))" ;;
        2) printf '%s' "$(( total / 4 > 0 ? total / 4 : 1 ))" ;;
        3) printf '%s' "$(( total / 8 > 0 ? total / 8 : 1 ))" ;;
        *) die "unknown slot: $1" ;;
    esac
}

require_slot() {
    case "${1:-}" in
        1|2|3) : ;;
        *) die "slot must be 1, 2 or 3 (got '${1:-}')" ;;
    esac
}

# Held build lock == somebody is compiling here right now. The same probe the
# guard uses; a lock nobody holds returns instantly.
busy() {
    local lock="$1/target/debug/.cargo-lock"
    [ -e "$lock" ] || return 1
    flock -n "$lock" true 2>/dev/null && return 1 || return 0
}

human() { du -sh "$1" 2>/dev/null | cut -f1 || printf '?'; }

# ⛔ THE PATH HARDLINKS ACTUALLY WORK THROUGH. Every slot's target/ is its own
# BIND MOUNT, and `link()` returns EXDEV across mount points even when both live
# on one filesystem — so linking `<wt>/target` to `<other>/target` always fails.
# The backing stores are all plain directories under one mount, so linking works
# there. Falls back to the target path itself when the tree is not bound.
#
# ⛔⛔ IT MUST NEVER RETURN AN EMPTY STRING. `findmnt` reports SOURCE as
# `<dev>[<path>//deleted]` once the backing directory is removed under a live
# mount, and `readlink -f` fails on that path because a non-final component is
# gone — so this printed NOTHING, and `cmd_seed` went on to build ROOT-RELATIVE
# paths out of it: `mkdir -p "/debug"`, `rm -rf "/debug/deps"`. Verified
# 2026-09-02. Fail loudly instead; `target_state` is what callers should ask.
store_of() {
    local root="$1" src inner out
    src="$(findmnt -no SOURCE --target "$root/target" 2>/dev/null || true)"
    case "$src" in
        *\[*\])
            inner="${src#*[}"
            inner="${inner%]}"
            # ⛔ STRIP THE KERNEL'S `//deleted` MARKER FIRST. findmnt reports a
            # mount whose backing directory was removed as `<dev>[<path>//deleted]`,
            # and `readlink -f` FAILS on that because a non-final component no
            # longer exists — which is how this used to return an empty string.
            # The store still has a name; it is simply gone. Name it.
            inner="${inner%//deleted}"
            out="$(readlink -f "$inner" 2>/dev/null || printf '%s' "$inner")"
            ;;
        *) out="$root/target" ;;
    esac
    [ -n "$out" ] || die "cannot resolve the backing store for $root/target (findmnt SOURCE: ${src:-none}).
  A mount over a DELETED store reports exactly this. Repair it:
      ( cd $root && scripts/setup/target_bindmount.sh )"
    printf '%s' "$out"
}

# bound / LOCAL / BROKEN / absent.
#
# BROKEN is a live mount whose backing store has been deleted: `mountpoint` still
# says yes and the path looks present, but the directory is unlinked and every
# create under it returns ENOENT. PROBE A WRITE — no cheaper check sees it, and
# on 2026-09-02 `list` reported `bound` for a slot in exactly this state while a
# seed died on a bare `mkdir: No such file or directory`.
target_state() {
    local t="$1/target"
    [ -d "$t" ] || { printf 'absent'; return; }
    mountpoint -q "$t" 2>/dev/null || { printf 'LOCAL'; return; }
    if ( : > "$t/.bindprobe" ) 2>/dev/null; then
        rm -f "$t/.bindprobe"
        printf 'bound'
    elif [ -d "$(store_of "$1")" ]; then
        # The store is THERE and still refuses the write — read-only, a full or
        # errored filesystem, wrong owner. Not the deleted-store case, and the
        # repair is not the same, so do not call it that.
        printf 'UNWRITABLE'
    else
        printf 'DELETED'
    fi
}

# ── list ──────────────────────────────────────────────────────────────────────
cmd_list() {
    printf '%-6s %-4s %-22s %-7s %-8s %-6s %s\n' \
        SLOT JOBS HEAD TARGET SIZE BUSY PATH
    local total; total="$(nproc)"
    printf '%-6s %-4s %-22s %-7s %-8s %-6s %s\n' \
        main "$total" "$(git -C "$MAIN" rev-parse --abbrev-ref HEAD)" \
        "$(target_state "$MAIN")" \
        "$(human "$MAIN/target")" \
        "$(busy "$MAIN" && echo yes || echo no)" "$MAIN"

    local n path head bound size
    for n in "${SLOTS[@]}"; do
        path="$(slot_path "$n")"
        if [ ! -d "$path" ]; then
            printf '%-6s %-4s %-22s %-7s %-8s %-6s %s\n' \
                "$n" "$(slot_jobs "$n")" '(not created)' '-' '-' '-' "$path"
            continue
        fi
        head="$(git -C "$path" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"
        [ "$head" = HEAD ] && head="detached $(git -C "$path" rev-parse --short HEAD 2>/dev/null)"
        bound="$(target_state "$path")"
        size="$(human "$path/target")"
        printf '%-6s %-4s %-22s %-7s %-8s %-6s %s\n' \
            "$n" "$(slot_jobs "$n")" "$head" "$bound" "$size" \
            "$(busy "$path" && echo yes || echo no)" "$path"
    done
    printf '\nTARGET=LOCAL on a virtiofs checkout means builds are on the SHARED mount.\n'
    printf 'TARGET=DELETED means the mount is live but its backing store was REMOVED:\n'
    printf '  the artifacts are gone. Repair with\n'
    printf '  ( cd <path> && scripts/setup/target_bindmount.sh ), then reseed.\n'
    printf 'TARGET=UNWRITABLE means the store is present but refuses writes (read-only,\n'
    printf '  full, or wrong owner). The artifacts may be fine — diagnose before repairing.\n'
    printf 'BUSY=yes means a cargo build holds that build lock right now.\n'
}

# ── setup ─────────────────────────────────────────────────────────────────────
cmd_setup() {
    local n="$1" path
    path="$(slot_path "$n")"

    if [ -d "$path" ]; then
        printf 'slot %s exists: %s\n' "$n" "$path"
    else
        mkdir -p "$MAIN/.worktrees"
        # DETACHED on purpose: the worktree's identity is its PATH, not a branch.
        # The agent runs `git switch -c <whatever>` as its first act, and no two
        # slots can ever contend for one branch name.
        git -C "$MAIN" worktree add --detach "$path" main
        printf 'created %s (detached at main)\n' "$path"
    fi

    # A fresh worktree has EMPTY submodules; game/ambition_map_assets holds every
    # .ldtk world, and the symlinks into it dangle without this.
    git -C "$path" submodule update --init --recursive \
        || printf '⚠ submodule init failed — an unpushed sha needs `git -C %s submodule update --init` from a warm clone\n' "$path"

    # Generated art is gitignored, so an unmirrored worktree compiles an EMPTY
    # sheet table and ~40 tests fail for reasons unrelated to the change.
    ( cd "$path" && python3 "$path/scripts/mirror_assets_for_worktree.py" ) \
        || printf '⚠ asset mirror failed in %s\n' "$path"

    # ⛔ GITIGNORED FILES THAT TRACKED DOCS LINK TO. `README.md:46` points at
    # `.agent/README.md`, which `.gitignore:109` excludes — so `check_doc_links.py`
    # FAILS in every fresh worktree and passes in the main checkout, which reads
    # as "this worktree is broken" rather than "that file does not travel".
    for stray in .agent/README.md; do
        if [ -f "$MAIN/$stray" ] && [ ! -e "$path/$stray" ]; then
            mkdir -p "$path/$(dirname "$stray")"
            ln -s "$MAIN/$stray" "$path/$stray" && printf 'linked %s from the main checkout\n' "$stray"
        fi
    done

    # Fast storage, and its own store: scripts/setup/target_bindmount.sh keys the store
    # by worktree path, so slots never share one.
    ( cd "$path" && "$path/scripts/setup/target_bindmount.sh" ) \
        || printf '⚠ bind mount failed in %s\n' "$path"

    printf 'slot %s ready. Build with -j %s.\n' "$n" "$(slot_jobs "$n")"
}

# ── seed ──────────────────────────────────────────────────────────────────────
# deps/ artifacts are content-hashed and REPLACED on rebuild (new inode), so
# hardlinks are safe and cost no space. .fingerprint/*.json is rewritten IN
# PLACE (same inode) — hardlinking it would corrupt the donor, so it is copied.
# incremental/ is per-worktree edit-loop state and the largest thing on disk;
# seeding it buys nothing.
cmd_seed() {
    local n="$1" from="${2:-$MAIN}" path dst profile src_store dst_store
    path="$(slot_path "$n")"
    [ -d "$path" ] || die "slot $n is not set up — run: $0 setup $n"
    [ -d "$from/target" ] || die "donor has no target/: $from"
    # ⛔ A donor mid-build has half-written rlibs, and a hardlink to one is a
    # truncated archive that fails at LINK time in the new worktree, minutes
    # later, reading like a corrupt toolchain.
    busy "$from" && die "donor is BUILDING right now: $from
  Seeding would hardlink partially written artifacts. Wait for it to finish."
    busy "$path" && die "slot $n is BUILDING right now — refusing to seed under it"

    # ⛔ A MOUNT OVER A DELETED STORE LOOKS BOUND AND ACCEPTS NOTHING. Without this
    # the first `mkdir -p` inside the loop fails with a bare
    # `No such file or directory` naming a path that plainly exists, which reads as
    # a broken script rather than a broken mount. Check BOTH ends: a donor in this
    # state has no artifacts to give either.
    case "$(target_state "$path")" in DELETED|UNWRITABLE) true ;; *) false ;; esac && die "slot $n's target cannot be written ($(target_state "$path")).
  A DELETED store means the artifacts are gone and a rebind is the fix; an
  UNWRITABLE one means the store is there and something else is wrong. Rebind:
      ( cd $path && scripts/setup/target_bindmount.sh )"
    case "$(target_state "$from")" in DELETED|UNWRITABLE) true ;; *) false ;; esac && die "the donor's target cannot be read as a store ($(target_state "$from")): $from
  There is nothing to seed FROM. Diagnose, or rebind and rebuild it first:
      ( cd $from && scripts/setup/target_bindmount.sh )"

    # Link through the STORES, not the bind-mounted target paths (see store_of).
    src_store="$(store_of "$from")"
    dst_store="$(store_of "$path")"

    for profile in debug release; do
        [ -d "$from/target/$profile" ] || continue
        dst="$path/target/$profile"
        mkdir -p "$dst"
        if [ -d "$src_store/$profile/deps" ]; then
            printf 'hardlinking %s/deps ...\n' "$profile"
            mkdir -p "$dst_store/$profile"
            rm -rf "$dst_store/$profile/deps"
            cp -al "$src_store/$profile/deps" "$dst_store/$profile/deps" \
                || printf '⚠ deps hardlink failed for %s\n' "$profile"
        fi
        for small in .fingerprint build; do
            [ -d "$from/target/$profile/$small" ] || continue
            printf 'copying %s/%s ...\n' "$profile" "$small"
            cp -a "$from/target/$profile/$small" "$dst/" 2>/dev/null \
                || printf '⚠ copy of %s failed\n' "$small"
        done
    done
    printf 'seeded slot %s from %s (incremental/ deliberately not copied)\n' "$n" "$from"
    printf 'size now: %s\n' "$(human "$path/target")"
}

# ── clear ─────────────────────────────────────────────────────────────────────
cmd_clear() {
    local n="$1" mode="${2:---incremental}" path
    path="$(slot_path "$n")"
    [ -d "$path" ] || die "slot $n is not set up"
    busy "$path" && die "slot $n is BUILDING right now — refusing to delete under it"

    case "$mode" in
        --incremental)
            printf 'before: %s\n' "$(human "$path/target")"
            rm -rf "$path/target/debug/incremental" "$path/target/release/incremental"
            printf 'after:  %s  (incremental only; artifacts kept)\n' "$(human "$path/target")"
            ;;
        --all)
            printf 'before: %s\n' "$(human "$path/target")"
            # ⛔ NOT `rm -rf target` — that unmounts nothing and, on a bound
            # worktree, would delete the mountpoint's contents wholesale
            # including anything non-cargo somebody parked there.
            rm -rf "$path/target/debug" "$path/target/release"
            printf 'after:  %s  (cold — reseed with: %s seed %s)\n' \
                "$(human "$path/target")" "$0" "$n"
            ;;
        *) die "clear takes --incremental (default) or --all" ;;
    esac
}

# ── dedupe ────────────────────────────────────────────────────────────────────
# Cargo names artifacts `<crate>-<16 hex>` where the hash covers package,
# version, features, profile, rustc and deps — so an identical NAME means
# identical inputs, and the bytes match unless something path-dependent leaked
# in. Sampling this repo: 0 of 25 third-party rlibs embed the checkout path, so
# foundation crates dedupe across worktrees. First-party `ambition_*` crates and
# crates whose build script `include!`s generated code (serde embeds its OUT_DIR)
# simply will not match, which needs no allowlist — they fail the compare.
#
# ⛔ deps/ ONLY. `.fingerprint/*.json` is rewritten IN PLACE (same inode), so a
# hardlink there corrupts every worktree sharing it. incremental/ is mutable
# per-worktree state. build/ holds script output that reruns rewrite.
cmd_dedupe() {
    local apply="${1:-}" n path dirs=()
    command -v hardlink >/dev/null || die "hardlink(1) is not installed (util-linux)"

    # deps/ dirs EXPLICITLY, never the target root. A whole-target scan also
    # sweeps up capture PNGs, probe output and logs that agents park under
    # target/ — linking two unrelated files that merely happen to match today,
    # so a later in-place edit of one silently rewrites the other.
    # Store paths, not mounted target paths — hardlink(1) hits the same EXDEV
    # wall across bind mounts that cp -al does (see store_of).
    local profile store
    for path in "$MAIN" $(for n in "${SLOTS[@]}"; do slot_path "$n"; done); do
        [ -d "$path/target" ] || continue
        busy "$path" && die "$path is BUILDING right now — refusing to dedupe under it"
        store="$(store_of "$path")"
        for profile in debug release; do
            [ -d "$store/$profile/deps" ] && dirs+=("$store/$profile/deps")
        done
    done
    [ "${#dirs[@]}" -gt 0 ] || die "no deps/ directories to scan"

    local args=(--content --respect-name --minimum-size 4k --verbose)
    [ "$apply" = --apply ] || args+=(--dry-run)

    printf 'scanning: %s\n' "${dirs[*]}"
    [ "$apply" = --apply ] || printf '(dry run — pass --apply to link)\n'
    hardlink "${args[@]}" "${dirs[@]}"
}

case "${1:-list}" in
    list) cmd_list ;;
    dedupe) shift; cmd_dedupe "${1:-}" ;;
    setup)
        shift; [ $# -ge 1 ] || die "setup needs a slot (1|2|3|all)"
        if [ "$1" = all ]; then for n in "${SLOTS[@]}"; do cmd_setup "$n"; done
        else require_slot "$1"; cmd_setup "$1"; fi ;;
    seed)
        shift; require_slot "${1:-}"; n="$1"; shift
        from="$MAIN"
        [ "${1:-}" = --from ] && { from="${2:?--from needs a path}"; }
        cmd_seed "$n" "$from" ;;
    clear)
        shift; require_slot "${1:-}"; n="$1"; shift
        cmd_clear "$n" "${1:---incremental}" ;;
    jobs) shift; slot_jobs "${1:?jobs needs a slot}"; printf '\n' ;;
    -h|--help|help) sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//' ;;
    *) die "unknown command: $1 (try: list, setup, seed, clear, dedupe, jobs)" ;;
esac
