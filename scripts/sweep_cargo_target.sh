#!/usr/bin/env bash
#
# Remove unreachable Cargo artifacts while preserving the configurations used
# by run_game.sh.
#
# Safe defaults:
#   - dry-run only
#   - keep incremental compilation state
#   - preserve normal host and windowed-demo builds
#
# Examples:
#   ./scripts/sweep_cargo_target.sh
#   ./scripts/sweep_cargo_target.sh --apply
#   ./scripts/sweep_cargo_target.sh --apply --deep
#   ./scripts/sweep_cargo_target.sh --full --with-tests
#
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

apply=0
deep=0
full=0
with_tests=0

usage() {
    cat <<'USAGE'
Usage:
  ./scripts/sweep_cargo_target.sh [OPTIONS]

Options:
  --apply         Actually delete unreachable artifacts.
                  Without this option, only report what would be deleted.

  --deep          Also discard incremental compilation state.
                  Without this option, incremental/ is retained.

  --full          Preserve every standard run_game.sh build shape, including
                  release and headless versions of both standalone demos.

  --with-tests    Preserve `cargo test --no-run --workspace` artifacts.

  -h, --help      Show this help.

Typical use:
  ./scripts/sweep_cargo_target.sh
  ./scripts/sweep_cargo_target.sh --apply

Maximum reclamation:
  ./scripts/sweep_cargo_target.sh --apply --deep
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
        --with-tests)
            with_tests=1
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

# These correspond to the normal build configurations reachable through
# run_game.sh. Runtime arguments such as sandbox and --start-room do not need
# separate entries because they do not alter the Cargo build graph.
mark_commands=(
    "build -p ambition_app --bin ambition_game_bin"
    "build -p ambition_app --bin ambition_game_bin --release"
    "build -p ambition_app --bin ambition_game_bin --features dev_hot_reload"
    "build -p ambition_app --bin ambition_game_bin --features dev_hot_reload --release"

    "build -p ambition_demo_sanic_app --bin sanic_demo --features visible"
    "build -p ambition_demo_mary_o_app --bin mary_o_demo --features visible"
)

if [[ "$full" -eq 1 ]]; then
    mark_commands+=(
        "build -p ambition_demo_sanic_app --bin sanic_demo --features visible --release"
        "build -p ambition_demo_sanic_app --bin sanic_demo"
        "build -p ambition_demo_sanic_app --bin sanic_demo --release"

        "build -p ambition_demo_mary_o_app --bin mary_o_demo --features visible --release"
        "build -p ambition_demo_mary_o_app --bin mary_o_demo"
        "build -p ambition_demo_mary_o_app --bin mary_o_demo --release"
    )
fi

if [[ "$with_tests" -eq 1 ]]; then
    mark_commands+=(
        "test --no-run --workspace"
    )
fi

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
