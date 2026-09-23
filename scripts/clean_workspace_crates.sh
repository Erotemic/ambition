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
# Contrast with the other sweepers, which cut along a different axis:
#   scripts/sweep_target_lru.py      delete units unused for N days; runs no
#                                    cargo, so it is safe on a full disk.
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
        # ⛔⛤ REFUSE WHILE A BUILD HOLDS THIS PROFILE'S LOCK. The comment
        # here used to claim the atomic rename made a racing build safe:
        # "a build racing this never sees a half-emptied session
        # directory". That is true and it is not the hazard. A rustc
        # already holding descriptors keeps writing through them while
        # every later path open lands on a directory that is simply gone —
        # split incremental state at best, a killed compile at worst.
        # Named by the architecture review of 2026-09-17, on guidance this
        # repository had just started advertising as the cheapest reclaim.
        #
        # `.cargo-lock` is cargo's own build lock for the profile
        # directory, advisory and released when the process dies, so there
        # is no stale-lock case to work around. MEASURED on this box: the
        # probe exits 1 during `cargo build` and 0 once it settles.
        # ⛔⛤ AND THE LOCK MUST BE HELD THROUGH THE DELETE, NOT PROBED BEFORE
        # IT. This read `flock -n "$lock" true`, which takes the lock for the
        # lifetime of `true` and drops it before the next line — so a cargo
        # starting in the window between the probe and the `rm` acquired it
        # freely and the refusal above protected nothing. Hold an open
        # descriptor across `du`, `find`, `mv` and `rm`, and a build that
        # starts meanwhile waits on the lock instead of racing the deletion.
        # `flock(2)` ignores the open mode, so a read descriptor takes the
        # exclusive lock cargo itself waits for.
        #
        # AND THE SECOND HALF OF THE SAME RACE WAS THE `-e` TEST, found by the
        # architecture review of 2026-09-17. A profile that has no `.cargo-lock`
        # yet took NO lock at all, so cargo could create the file, acquire it,
        # and enter the profile while this script deleted underneath it. A
        # populated profile normally has one, which is why the exposure was
        # small — but the safety property must not depend on that. In apply mode
        # the file is OPENED FOR WRITING, which creates it when absent, and
        # locked unconditionally: cargo's own `flock` on the same path then
        # waits, whichever of the two got there first.
        lock="$target/$name/.cargo-lock"
        lockfd=
        if [ "$apply" = 1 ]; then
            exec {lockfd}>>"$lock"
            if ! flock -n "$lockfd"; then
                exec {lockfd}<&-
                echo "REFUSING: a build holds $lock." >&2
                echo "  Deleting the incremental cache under a live rustc can" >&2
                echo "  split its state or kill the compile. Wait for the build" >&2
                echo "  to finish, or stop it, and run this again." >&2
                exit 3
            fi
        fi
        size="$(du -sh "$inc" | cut -f1)"
        dirs="$(find "$inc" -mindepth 1 -maxdepth 1 -type d | wc -l)"
        echo "$name/incremental: $verb $size across $dirs crate sessions"
        if [ "$apply" = 1 ]; then
            doomed="$target/$name/.incremental-doomed-$$"
            mv "$inc" "$doomed"
            rm -rf "$doomed"
        fi
        if [ -n "$lockfd" ]; then exec {lockfd}<&-; fi
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
