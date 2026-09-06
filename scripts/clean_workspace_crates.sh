#!/usr/bin/env bash
#
# Drop OUR crates' build artifacts from target/, keeping every dependency.
#
# This is the "recompile ambition, not bevy" cut. `cargo clean --workspace` is
# the whole mechanism -- it deletes only units belonging to workspace members,
# verified: zero non-ambition crates appear in its delete list. This wrapper
# exists for the two things that flag cannot express:
#
#   1. It cleans ONE profile per invocation, and this repo carries several
#      (debug from run_game.sh, profiling from the Tracy captures).
#   2. --incremental-only, which reclaims the largest share of the directory
#      while deleting no artifact at all. See below.
#
# ⭐ INCREMENTAL IS MOST OF THE BLOAT AND COSTS ALMOST NOTHING TO DROP.
# Measured 2026-08-30: 70G of a 112G target was target/*/incremental, 915 dirs
# for ~70 crates -- roughly thirteen stale generations each, because a new hash
# is minted per feature/flag shape and the old one is never reaped. Deleting it
# invalidates NO fingerprint: a fresh crate stays fresh and is skipped on the
# next build. The only cost is that the next EDIT to a given crate recompiles it
# whole instead of incrementally.
#
# Contrast with the other two sweepers, which cut along a different axis:
#   scripts/sweep_target.py          mark-and-sweep: keep named live graphs,
#   scripts/sweep_cargo_target.sh    delete what is unreachable (deps included).
# Use those to reclaim without disturbing the graph you are building. Use this
# one when you want your own crates rebuilt from source and the dependency wall
# left standing.
#
# Examples:
#   ./scripts/clean_workspace_crates.sh                     # dry run, all profiles
#   ./scripts/clean_workspace_crates.sh --apply
#   ./scripts/clean_workspace_crates.sh --incremental-only --apply
#   ./scripts/clean_workspace_crates.sh --profile debug --apply
#
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

apply=0
incremental_only=0
only_profile=""

usage() {
    sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed 's/^#\{0,1\} \{0,1\}//; $d'
    cat <<'USAGE'
Options:
  --apply             Actually delete. Without it, report only.
  --incremental-only  Drop only target/*/incremental. Deletes no artifact and
                      invalidates no fingerprint; costs the incremental speedup
                      on the next edit of each crate.
  --profile NAME      Restrict to one profile directory (debug, profiling, ...).
                      Default: every profile directory present.
  -h, --help          Show this help.
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --apply) apply=1 ;;
        --incremental-only) incremental_only=1 ;;
        --profile) only_profile="${2:?--profile needs a name}"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
    shift
done

target_dir() {
    if [ -n "${CARGO_TARGET_DIR:-}" ]; then printf '%s\n' "$CARGO_TARGET_DIR"; return; fi
    printf '%s\n' "$repo_root/target"
}
target="$(target_dir)"

# ⛔ AGENTS.md: an enormous target/ is usually the bindmount being absent, and
# the duplicate underneath is not ours to reclaim by deleting the live one.
# Refuse to apply until the mount is what it should be.
if [ "$apply" = 1 ] && [ -x scripts/setup/target_bindmount.sh ]; then
    status="$(bash scripts/setup/target_bindmount.sh --status 2>&1 || true)"
    if grep -q 'virtiofs' <<<"$status" && ! grep -q 'state *BOUND' <<<"$status"; then
        printf '%s\n' "$status" >&2
        cat >&2 <<'REFUSE'

REFUSING: this worktree is on virtiofs with no bound target/. A target that has
grown enormous is a SYMPTOM -- fix the mount first and the duplicate copy goes
away on its own. Run: scripts/setup/target_bindmount.sh
REFUSE
        exit 1
    fi
fi

# A profile directory is one Cargo built into; `doc/`, `profiles/` and other
# people's output under target/ have no .fingerprint and are never touched.
profiles=()
for dir in "$target"/*/; do
    name="$(basename "$dir")"
    [ -d "$dir/.fingerprint" ] || continue
    [ -z "$only_profile" ] || [ "$name" = "$only_profile" ] || continue
    profiles+=("$name")
done
if [ "${#profiles[@]}" -eq 0 ]; then
    echo "no cargo profile directories under $target" >&2
    exit 1
fi

# The profile directory is named `debug`; the profile is named `dev`.
cargo_profile() { [ "$1" = debug ] && echo dev || echo "$1"; }

if [ "$apply" = 1 ]; then verb="removing"; else verb="would remove"; fi
echo "target: $target"
echo "profiles: ${profiles[*]}"
echo

for name in "${profiles[@]}"; do
    if [ "$incremental_only" = 1 ]; then
        inc="$target/$name/incremental"
        [ -d "$inc" ] || continue
        size="$(du -sh "$inc" | cut -f1)"
        dirs="$(find "$inc" -mindepth 1 -maxdepth 1 -type d | wc -l)"
        echo "$name/incremental: $verb $size across $dirs crate sessions"
        if [ "$apply" = 1 ]; then
            # Move-then-delete: the rename is atomic, so a build racing this
            # never sees a half-emptied session directory.
            doomed="$target/$name/.incremental-doomed-$$"
            mv "$inc" "$doomed"
            rm -rf "$doomed"
        fi
    else
        flags=(clean --workspace --profile "$(cargo_profile "$name")")
        [ "$apply" = 1 ] || flags+=(--dry-run)
        echo "$name: $verb workspace-member artifacts (dependencies kept)"
        cargo "${flags[@]}" 2>&1 | grep -E 'Summary|Removed' || true
    fi
done

echo
if [ "$apply" = 1 ]; then
    echo "free now: $(df -h "$target" | tail -1 | awk '{print $4}')"
else
    echo "dry run -- pass --apply to delete"
fi
