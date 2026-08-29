#!/usr/bin/env bash
# Bind this worktree's default `target/` path to guest-local storage. The repo may
# live on a shared filesystem, but Cargo still sees its ordinary per-worktree
# target directory, so no `CARGO_TARGET_DIR` coordination is required and
# parallel worktrees do not share locks or artifacts.
#
# This is optional and does not edit the repository.
# Usage:
#   scripts/setup/target_bindmount.sh
#   scripts/setup/target_bindmount.sh --status
#   scripts/setup/target_bindmount.sh --check     # exit 2 if unbound on virtiofs
#   scripts/setup/target_bindmount.sh --unmount
set -euo pipefail

STORE_ROOT="${AMBITION_TARGET_STORE:-$HOME/.cache/ambition-targets}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

worktree_root() {
    git rev-parse --show-toplevel 2>/dev/null || die "not inside a git worktree"
}

# The readable half is kept only so `du -sh` output is legible.
store_for() {
    local root="$1" hash slug
    hash="$(printf '%s' "$root" | sha1sum | cut -c1-10)"
    slug="$(basename "$root" | tr -c 'A-Za-z0-9._-' '-')"
    printf '%s/%s-%s' "$STORE_ROOT" "$slug" "$hash"
}

fstype_of() { findmnt -no FSTYPE --target "$1" 2>/dev/null || echo unknown; }

cmd_status() {
    local root target fs store
    root="$(worktree_root)"
    target="$root/target"
    fs="$(fstype_of "$root")"
    store="$(store_for "$root")"

    printf 'worktree     %s\n' "$root"
    printf 'repo fs      %s\n' "$fs"
    printf 'target       %s\n' "$target"
    if [ ! -d "$target" ]; then
        printf 'state        ABSENT (cargo has not built here yet)\n'
    elif mountpoint -q "$target"; then
        printf 'state        BOUND -> %s\n' "$(findmnt -no SOURCE --target "$target" 2>/dev/null || echo '?')"
        printf 'size         %s\n' "$(du -sh "$target" 2>/dev/null | cut -f1)"
    elif [ "$fs" = virtiofs ]; then
        printf 'state        ⚠ NOT BOUND, and this worktree is on virtiofs — builds are\n'
        printf '             paying shared-mount overhead and are visible to the host.\n'
        printf '             Run: scripts/setup/target_bindmount.sh\n'
    else
        printf 'state        not bound (and not needed: %s is already local)\n' "$fs"
    fi
    printf 'backing      %s\n' "$store"
}

# The MACHINE-READABLE half of `--status`, for gates.
#
# ⛔ A SEPARATE VERB rather than an exit code on `--status`, because `--status`
# is something a human runs to look, and a looking command that exits non-zero
# breaks every `set -e` caller that only wanted to print it.
#
# Exits 2 when this worktree is on virtiofs and `target/` is NOT shadowed onto
# local disk — the one state that silently costs minutes per build and grows a
# duplicate of every artifact under the mount point. Everything else is 0,
# including "already local, nothing to do".
cmd_check() {
    local root target fs
    root="$(worktree_root)"
    target="$root/target"
    fs="$(fstype_of "$root")"

    [ "$fs" = virtiofs ] || return 0
    [ -d "$target" ] || return 0
    mountpoint -q "$target" && return 0

    printf '\n' >&2
    printf '  ⛔⛔ TARGET IS ON VIRTIOFS AND NOT SHADOWED.\n' >&2
    printf '\n' >&2
    printf '  %s\n' "$target" >&2
    printf '  is being written straight through the shared mount. Builds pay the\n' >&2
    printf '  overhead on every file, and a full second copy of every artifact is\n' >&2
    printf '  accumulating there instead of in the local store.\n' >&2
    printf '\n' >&2
    printf '  FIX IT — one command, and it is not optional:\n' >&2
    printf '\n' >&2
    printf '      scripts/setup/target_bindmount.sh\n' >&2
    printf '\n' >&2
    printf '  ⛔ DO NOT delete anything under target/ to reclaim the space. The\n' >&2
    printf '     duplicate is the SYMPTOM of this missing mount; binding it puts\n' >&2
    printf '     the real store back and the space with it. See AGENTS.md.\n' >&2
    printf '\n' >&2
    return 2
}

cmd_mount() {
    local root target fs store
    root="$(worktree_root)"
    target="$root/target"
    fs="$(fstype_of "$root")"
    store="$(store_for "$root")"

    # THE VM-VERSUS-REAL-SYSTEM BRANCH, and it is the whole point of the
    # script being conditional: on a machine where the checkout is already on
    # local disk, a bind mount buys nothing and only adds a thing to forget to
    # tear down. Say so and succeed, so this is safe to run unconditionally
    # from setup scripts and agent briefs.
    if [ "$fs" != virtiofs ]; then
        printf 'nothing to do: %s is on %s, not virtiofs — cargo already writes locally\n' \
            "$root" "$fs"
        return 0
    fi

    if [ -d "$target" ] && mountpoint -q "$target"; then
        printf 'already bound: %s -> %s\n' "$target" "$store"
        return 0
    fi

    # ⛔⛔ A POPULATED `target/` IS NOT A PROBLEM — SHADOWING IT IS THE POINT.
    # The bind mount covers whatever is there; the underlying bytes stay exactly
    # as they were and reappear on `--unmount`. On a virtiofs checkout that is
    # the PROTECTION: the host's own `target/` is left untouched while the guest
    # builds onto local disk, instead of both sides writing the same directory.
    #
    # This block used to REFUSE here and tell the caller `rm -rf '$target'`, and
    # that instruction has a body count: 2026-08-25 an agent followed it exactly
    # and destroyed `target/first_shots/` and `target/swing_shots/` — screenshot
    # captures from a live investigation — to clear a path the mount would have
    # preserved on its own. `target/` in this repo is not only a cache; it also
    # carries probe output, run logs and a font download cache.
    #
    # ⇒ mount over it. Delete nothing, move nothing, ask for nothing.

    mkdir -p "$store"
    mkdir -p "$target"
    sudo mount --bind "$store" "$target"
    # The mount is made by root; hand it back to whoever runs builds here.
    sudo chown "$(id -u):$(id -g)" "$target"

    printf 'bound %s -> %s\n' "$target" "$store"
    printf '⚠ a bind mount does NOT survive a reboot — re-run this script after one.\n'
}

cmd_unmount() {
    local root target
    root="$(worktree_root)"
    target="$root/target"
    if [ ! -d "$target" ] || ! mountpoint -q "$target"; then
        printf 'not bound: %s\n' "$target"
        return 0
    fi
    sudo umount "$target"
    printf 'unbound %s (backing store kept at %s)\n' "$target" "$(store_for "$root")"
}

case "${1:---mount}" in
    --mount|mount)     cmd_mount ;;
    --status|status)   cmd_status ;;
    --check|check)     cmd_check ;;
    --unmount|umount|--umount) cmd_unmount ;;
    -h|--help|help)    sed -n '3,30p' "$0" ;;
    *) die "unknown argument: $1 (try --status)" ;;
esac
