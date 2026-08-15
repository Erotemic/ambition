#!/usr/bin/env bash
#
# Put THIS worktree's `target/` on guest-local disk without hardcoding a path.
#
# The repo is checked out on virtiofs, shared between the VM and the host.
# Cargo's default target dir — `<worktree>/target` — therefore lands on the
# slow, shared filesystem, and VM and host would co-mingle artifacts in it.
#
# The old fix was `target-dir = /home/joncrall/ambition-target` in
# `.cargo/config.toml`: one absolute path that resolved to a different local
# disk on each machine. It worked, and it had three costs. Every worktree
# shared ONE target dir, so parallel agents fought over its lock and thrashed
# each other's fingerprints. The path was a guess about the machine, baked into
# a committed file. And an agent that overrode `CARGO_TARGET_DIR` — which is the
# obvious thing to do when you want your own — silently stopped sharing a build
# with anything that did not, including the goal guard, so "green here, red
# there" became possible and did happen (2026-08-15).
#
# A bind mount fixes all three: `<worktree>/target` stays cargo's default, so
# nothing is hardcoded and nothing needs an env override, while the bytes land
# on ext4. Each worktree gets its own backing store keyed by its path, so two
# agents never share a lock.
#
# ⭐ OPT-IN ON PURPOSE. Not running it is a working configuration — you just get
# a slower target dir on the shared mount. This script never edits the repo.
#
# Usage:
#     scripts/setup_target_bindmount.sh              # mount (idempotent)
#     scripts/setup_target_bindmount.sh --status
#     scripts/setup_target_bindmount.sh --unmount
#
set -euo pipefail

STORE_ROOT="${AMBITION_TARGET_STORE:-$HOME/.cache/ambition-targets}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

worktree_root() {
    git rev-parse --show-toplevel 2>/dev/null || die "not inside a git worktree"
}

# ⚠ the KEY is the worktree path, not its basename: `.claude/worktrees/agent-*`
# basenames are long and similar, and two checkouts of one branch must not
# collide. The readable half is kept only so `du -sh` output is legible.
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
        printf '             Run: scripts/setup_target_bindmount.sh\n'
    else
        printf 'state        not bound (and not needed: %s is already local)\n' "$fs"
    fi
    printf 'backing      %s\n' "$store"
}

cmd_mount() {
    local root target fs store
    root="$(worktree_root)"
    target="$root/target"
    fs="$(fstype_of "$root")"
    store="$(store_for "$root")"

    # ⭐ THE VM-VERSUS-REAL-SYSTEM BRANCH, and it is the whole point of the
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

    # ⛔ REFUSE rather than silently orphan bytes. A populated `target/` here is
    # a real build sitting on the shared mount; mounting over it hides it while
    # it keeps consuming virtiofs space, and the owner would never find it.
    if [ -d "$target" ] && [ -n "$(ls -A "$target" 2>/dev/null)" ]; then
        die "$target already has build output on the shared mount.
  Mounting over it would HIDE those bytes without freeing them.
  Delete it first (it is a cache):  rm -rf '$target'
  then re-run this script."
    fi

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
    --unmount|umount|--umount) cmd_unmount ;;
    -h|--help|help)    sed -n '3,30p' "$0" ;;
    *) die "unknown argument: $1 (try --status)" ;;
esac
