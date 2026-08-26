#!/usr/bin/env bash
#
# Remove unreachable Cargo artifacts while preserving the build graphs used by
# the normal run_game.sh and run_tests.sh workflows.
#
# Safe defaults:
#   - dry-run only
#   - keep incremental compilation state
#   - preserve the host, common windowed demos, and backbone Rust test graph
#
# Examples:
#   ./scripts/sweep_cargo_target.sh
#   ./scripts/sweep_cargo_target.sh --apply
#   ./scripts/sweep_cargo_target.sh --apply --deep
#   ./scripts/sweep_cargo_target.sh --full
#   ./scripts/sweep_cargo_target.sh --keep-cmd \
#       'build -p ambition_app --bin ambition_game_bin --features causal'
#
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

apply=0
deep=0
full=0
with_tests=1
extra_mark_commands=()

usage() {
    cat <<'USAGE'
Usage:
  ./scripts/sweep_cargo_target.sh [OPTIONS]

Options:
  --apply         Actually delete unreachable artifacts.
                  Without this option, only report what would be deleted.

  --deep          Also discard incremental compilation state.
                  Without this option, incremental/ is retained.

  --full          Preserve all standard first-party run_game.sh demo shapes:
                  dev/release/ship, both windowed and headless. The host's
                  normal, release, ship, and hot-reload shapes are always kept.

  --no-tests      Do not preserve the default run_tests.sh Rust build graph.
                  Tests are preserved by default.

  --keep-cmd CMD  Preserve one additional Cargo build graph. CMD is the part
                  after `cargo`, for example:
                    --keep-cmd 'build -p ambition_app --features causal'
                  Repeat this option to keep multiple custom feature graphs.

  -h, --help      Show this help.

Typical use:
  ./scripts/sweep_cargo_target.sh
  ./scripts/sweep_cargo_target.sh --apply

Maximum reclamation while keeping the normal game/test graphs:
  ./scripts/sweep_cargo_target.sh --apply --deep

Keep every standard demo shape as well:
  ./scripts/sweep_cargo_target.sh --apply --full
USAGE
}

fail() {
    echo "sweep_cargo_target.sh: $*" >&2
    exit 2
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --apply)
            apply=1
            ;;
        --deep)
            deep=1
            ;;
        --full)
            full=1
            ;;
        --no-tests)
            with_tests=0
            ;;
        --keep-cmd)
            shift
            [[ $# -gt 0 ]] || fail "--keep-cmd requires a Cargo command"
            [[ -n "$1" ]] || fail "--keep-cmd requires a non-empty Cargo command"
            extra_mark_commands+=("$1")
            ;;
        --keep-cmd=*)
            mark_command="${1#--keep-cmd=}"
            [[ -n "$mark_command" ]] || fail "--keep-cmd requires a non-empty Cargo command"
            extra_mark_commands+=("$mark_command")
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "unknown option: $1"
            ;;
    esac
    shift
done

if ! command -v cargo-mark-sweep >/dev/null 2>&1; then
    cat >&2 <<'MSG'
cargo-mark-sweep is not installed.

Install it with:

    cargo install cargo-mark-sweep
MSG
    exit 127
fi

cd "$repo_root"

target_dir="$(
    cargo metadata --format-version=1 --no-deps |
        python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
)"

[[ -n "$target_dir" ]] || fail "Cargo reported an empty target directory"
[[ "$target_dir" != "/" ]] || fail "refusing to operate on /"

mark_commands=(
    # Multi-game host. Runtime arguments do not change the Cargo graph.
    "build -p ambition_app --bin ambition_game_bin"
    "build -p ambition_app --bin ambition_game_bin --release"
    "build -p ambition_app --bin ambition_game_bin --profile ship"
    "build -p ambition_app --bin ambition_game_bin --features dev_hot_reload"
    "build -p ambition_app --bin ambition_game_bin --features dev_hot_reload --release"
)

# Windowed dev is the normal standalone-demo shape selected by run_game.sh.
demo_specs=(
    "ambition_demo_sanic_app:sanic_demo"
    "ambition_demo_mary_o_app:mary_o_demo"
    "ambition_demo_smash_app:smash_demo"
    "ambition_demo_twintrack_app:twintrack_demo"
)

for demo_spec in "${demo_specs[@]}"; do
    IFS=: read -r demo_pkg demo_bin <<<"$demo_spec"
    mark_commands+=("build -p $demo_pkg --bin $demo_bin --features visible")

    if [[ "$full" -eq 1 ]]; then
        mark_commands+=(
            "build -p $demo_pkg --bin $demo_bin --features visible --release"
            "build -p $demo_pkg --bin $demo_bin --features visible --profile ship"
            "build -p $demo_pkg --bin $demo_bin"
            "build -p $demo_pkg --bin $demo_bin --release"
            "build -p $demo_pkg --bin $demo_bin --profile ship"
        )
    fi
done

# The default run_tests.sh Rust lane is `cargo test --workspace`. `--no-run`
# builds the same test graph without spending time executing it during marking.
#
# ⛔⛔ AND IT MARKS THE WRONG POPULATION, WHICH IS WHY THIS IS A WARNING RATHER
# THAN A FIX. `cargo mark-sweep` runs every `--cmd` in ONE environment, and this
# script sets no `CARGO_INCREMENTAL`, so all of them inherit `incremental = true`
# from `.cargo/config.toml`. But `scripts/run_tests.py:717` sets
# `CARGO_INCREMENTAL=0`, and the two modes produce DIFFERENT ARTIFACT HASHES that
# coexist — measured 2026-08-25: the same crate built both ways yields
# `6eeced958ebf3728` and `93ca37900671d6c1` side by side, and switching back
# rebuilds nothing.
#
# ⇒ marking here preserves incremental test artifacts NOTHING USES, while the
# non-incremental ones `run_tests.sh` actually depends on go unmarked and are
# swept. "Backbone tests: preserve" prints and the opposite happens.
#
# It cannot be fixed through `--cmd`, which shares one environment. Until the
# marker can take a per-command environment, say so out loud rather than let the
# banner below be believed.
if [[ "$with_tests" -eq 1 ]]; then
    mark_commands+=("test --no-run --workspace")
    printf '⚠ the test graph is marked under CARGO_INCREMENTAL=1 (this repo defaults
'
    printf '  incremental=true) but run_tests.sh builds it with CARGO_INCREMENTAL=0.
'
    printf '  Those are DIFFERENT artifacts, so the ones run_tests.sh uses are NOT
'
    printf '  protected by this run. Expect a cold test rebuild after --apply.

' >&2
fi

# run_game.sh accepts arbitrary --features / --no-default-features combinations,
# so there is no finite built-in list for those graphs. Callers can preserve the
# exact variants they care about without teaching this script every feature.
mark_commands+=("${extra_mark_commands[@]}")

cargo_args=(mark-sweep)

if [[ "$apply" -eq 0 ]]; then
    cargo_args+=(--dry-run)
fi

if [[ "$deep" -eq 0 ]]; then
    cargo_args+=(--keep-incremental)
fi

for mark_command in "${mark_commands[@]}"; do
    cargo_args+=(--cmd "$mark_command")
done

echo "Workspace:  $repo_root"
echo "Target dir: $target_dir"

if [[ -d "$target_dir" ]]; then
    echo -n "Current size: "
    du -sh "$target_dir" | cut -f1
fi

if [[ "$apply" -eq 1 ]]; then
    echo "Operation:  DELETE unreachable artifacts"
else
    echo "Operation:  dry run only"
fi

if [[ "$deep" -eq 1 ]]; then
    echo "Incremental state: discard"
else
    echo "Incremental state: preserve"
fi

if [[ "$with_tests" -eq 1 ]]; then
    echo "Backbone tests: preserve"
else
    echo "Backbone tests: discard if otherwise unreachable"
fi

echo
echo "Marked Cargo configurations:"
for mark_command in "${mark_commands[@]}"; do
    printf '  cargo %s\n' "$mark_command"
done
echo

cargo "${cargo_args[@]}"

if [[ "$apply" -eq 1 && -d "$target_dir" ]]; then
    echo
    echo -n "Resulting size: "
    du -sh "$target_dir" | cut -f1
fi

