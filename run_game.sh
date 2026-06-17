#!/usr/bin/env bash
# Desktop-only run script. Builds/runs for the host platform (Linux x86-64
# in this dev VM), NOT for Android. Use --help for release, hot-reload,
# validation, and game-argument examples.
#
# An actual Android APK build is NOT produced by this script and would require
# a separate `cargo apk` / `cargo ndk` toolchain plus an Android NDK install.
# Nothing here invokes either of those.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
python_bin="${PYTHON:-python}"

release=0
hot_reload=0
validate_before_run=0
validate_only=0
no_default_features=0
cargo_jobs=""
cargo_timings=0
extra_features=()
game_args=()

usage() {
    cat <<'USAGE'
Usage:
  ./run_game.sh [OPTIONS] [MODE ...] [-- GAME_ARGS ...]

Common commands:
  ./run_game.sh
      Run the desktop sandbox in dev mode.

  ./run_game.sh release
  ./run_game.sh --release
      Run the desktop sandbox with cargo --release.

  ./run_game.sh hot
  ./run_game.sh --hot-reload
      Run with the dev_hot_reload feature enabled.

  ./run_game.sh hot release -- --start-room goblin_encounter
      Combine hot reload + release and pass arguments to the game binary.

  ./run_game.sh cut-rope
  ./run_game.sh smirking-behemoth
      Run directly in the You Have To Cut The Rope boss arena.

  ./run_game.sh validate
  ./run_game.sh ldtk
      Validate the sandbox LDtk world and exit.

  ./run_game.sh --validate hot release
      Validate LDtk first, then launch with hot reload + release.

  ./run_game.sh -j 8
  ./run_game.sh --jobs=8 release
      Limit the number of parallel cargo jobs.

  ./run_game.sh --timings
      Run with cargo build timing output enabled.

Options and mode aliases:
  -h, --help              Show this help.
  -r, --release, release  Use cargo --release.
  --debug, debug, dev     Force dev/debug cargo profile.
  --hot-reload, --hot,
  hot, hot-reload         Enable the dev_hot_reload feature.
  --no-hot-reload         Disable hot reload if an earlier alias enabled it.
  -v, --validate          Validate LDtk before launching.
  validate, ldtk,
  ldtk-validate,
  validate-only,
  --validate-only         Validate LDtk and exit.
  --features LIST         Add extra comma-separated cargo features.
  --no-default-features   Pass --no-default-features to cargo.
  -j, --jobs N            Pass cargo --jobs N.
  --jobs=N                Pass cargo --jobs N.
  --timings               Pass cargo --timings.
  --                      Everything after this is passed to the game binary.

Environment:
  PYTHON=/path/to/python   Python executable for ambition_ldtk_tools.
  RUST_BACKTRACE=full      Backtrace mode for cargo run; defaults to full.
USAGE
}

fail() {
    echo "run_game.sh: $*" >&2
    echo "Try './run_game.sh --help'." >&2
    exit 2
}

print_cmd() {
    printf '+ '
    printf '%q ' "$@"
    printf '\n'
}

require_positive_integer() {
    local opt="$1"
    local value="$2"
    [[ "$value" =~ ^[1-9][0-9]*$ ]] || fail "$opt requires a positive integer"
}

run_ldtk_validation() {
    local sandbox_world="$repo_root/crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk"
    local intro_world="$repo_root/crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk"
    local cut_rope_world="$repo_root/crates/ambition_gameplay_core/assets/ambition/worlds/you_have_to_cut_the_rope.ldtk"

    local cmd=(
        "$python_bin" -m ambition_ldtk_tools validate
        "$sandbox_world"
        --secondary-world "$intro_world"
    )
    if [[ -f "$cut_rope_world" ]]; then
        cmd+=(--secondary-world "$cut_rope_world")
    fi

    echo "Validating LDtk worlds..."
    print_cmd env "PYTHONPATH=$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            usage
            exit 0
            ;;
        -r|--release|release)
            release=1
            ;;
        --debug|debug|dev)
            release=0
            ;;
        --hot|--hot-reload|--dev-hot-reload|hot|hot-reload|dev-hot-reload)
            hot_reload=1
            ;;
        --no-hot-reload)
            hot_reload=0
            ;;
        -v|--validate)
            validate_before_run=1
            ;;
        validate|ldtk|ldtk-validate|validate-only|--validate-only)
            validate_before_run=1
            validate_only=1
            ;;
        cut-rope|cut-rope-boss|smirking-behemoth|you-have-to-cut-the-rope)
            game_args+=(--start-room you_have_to_cut_the_rope)
            ;;
        --features)
            shift
            [[ $# -gt 0 ]] || fail "--features requires a comma-separated feature list"
            extra_features+=("$1")
            ;;
        --features=*)
            extra_features+=("${1#--features=}")
            ;;
        --no-default-features)
            no_default_features=1
            ;;
        -j|--jobs)
            opt="$1"
            shift
            [[ $# -gt 0 ]] || fail "$opt requires a job count"
            require_positive_integer "$opt" "$1"
            cargo_jobs="$1"
            ;;
        -j[0-9]*)
            cargo_jobs="${1#-j}"
            require_positive_integer "-j" "$cargo_jobs"
            ;;
        --jobs=*)
            cargo_jobs="${1#--jobs=}"
            require_positive_integer "--jobs" "$cargo_jobs"
            ;;
        --timings)
            cargo_timings=1
            ;;
        --)
            shift
            game_args+=("$@")
            break
            ;;
        --*)
            fail "unknown option '$1'"
            ;;
        *)
            game_args+=("$1")
            ;;
    esac
    shift
done

if [[ "$validate_before_run" -eq 1 ]]; then
    run_ldtk_validation
fi

if [[ "$validate_only" -eq 1 ]]; then
    exit 0
fi

cargo_args=(run -p ambition_app --bin ambition_gameplay_core)

if [[ "$no_default_features" -eq 1 ]]; then
    cargo_args+=(--no-default-features)
fi

if [[ -n "$cargo_jobs" ]]; then
    cargo_args+=(--jobs "$cargo_jobs")
fi

if [[ "$cargo_timings" -eq 1 ]]; then
    cargo_args+=(--timings)
fi

features=()
if [[ "$hot_reload" -eq 1 ]]; then
    features+=(dev_hot_reload)
fi
for feature_list in "${extra_features[@]}"; do
    if [[ -n "$feature_list" ]]; then
        features+=("$feature_list")
    fi
done

if [[ "${#features[@]}" -gt 0 ]]; then
    IFS=,
    cargo_args+=(--features "${features[*]}")
    unset IFS
fi

if [[ "$release" -eq 1 ]]; then
    cargo_args+=(--release)
fi

if [[ "${#game_args[@]}" -gt 0 ]]; then
    cargo_args+=(-- "${game_args[@]}")
fi

cd "$repo_root"
export RUST_BACKTRACE="${RUST_BACKTRACE:-full}"
print_cmd cargo "${cargo_args[@]}"
exec cargo "${cargo_args[@]}"
