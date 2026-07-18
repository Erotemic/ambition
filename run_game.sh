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
ldtk_tools_dir="$repo_root/tools/ambition_ldtk_tools"
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"
python_bin="$(ambition_select_tool_python "$ldtk_tools_dir" AMBITION_LDTK_PYTHON)"

release=0
clean_coverage=0
coverage=0
hot_reload=0
validate_before_run=0
validate_only=0
no_default_features=0
cargo_jobs=""
cargo_timings=0
extra_features=()
game_args=()

# Launch target. Defaults to the multi-game host (the Ambition title screen).
# The `sanic` / `mary-o` mode aliases retarget this to a demo's OWN standalone
# shell crate — the same binary `game/ambition_demo_*_app` ships, unrelated to
# the host. This script is a launcher: demos default to WINDOWED
# (`--features visible` + the `--window` game arg). `--headless` opts a demo
# into its sim-only shell instead (no window, no `visible`).
target_pkg="ambition_app"
target_bin="ambition_game_bin"
target_kind="host"
demo_headless=0

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

  ./run_game.sh sanic
  ./run_game.sh mary-o
      Launch a demo's OWN standalone shell (windowed) instead of the host.

  ./run_game.sh sanic --headless -- --ticks 600
      Run a demo's sim-only shell headlessly and pass game args through.

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

Launch targets (mode aliases):
  (default)               The multi-game host — the Ambition title screen.
  sanic, sanic-demo       Sanic's standalone shell (ambition_demo_sanic_app).
  mary-o, mary_o, maryo   Mary-O's standalone shell (ambition_demo_mary_o_app).
                          Demos default to windowed (--features visible + the
                          --window game arg).
  --headless, headless    Opt the selected demo into its sim-only shell (no
                          window). Ignored for the host.

Options and mode aliases:
  -h, --help              Show this help.
  -r, --release, release  Use cargo --release.
  --cov, coverage         Run through cargo llvm-cov run --no-report.
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
  AMBITION_LDTK_PYTHON=/path/to/python
                            Override the LDtk tool-local .venv.
  PYTHON=/path/to/python   Legacy override for ambition_ldtk_tools.
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
    local worlds_dir="$repo_root/crates/ambition_actors/assets/ambition/worlds"
    local sandbox_world="$worlds_dir/sandbox.ldtk"
    local intro_world="$worlds_dir/intro.ldtk"
    local cut_rope_world="$worlds_dir/you_have_to_cut_the_rope.ldtk"
    local hall_world="$worlds_dir/hall_of_characters.ldtk"

    local cmd=(
        "$python_bin" -m ambition_ldtk_tools validate
        "$sandbox_world"
        --secondary-world "$intro_world"
    )
    # Every secondary world must be passed so the validator resolves cross-file
    # LoadingZone targets (the hub door into the Hall, etc.).
    if [[ -f "$cut_rope_world" ]]; then
        cmd+=(--secondary-world "$cut_rope_world")
    fi
    if [[ -f "$hall_world" ]]; then
        cmd+=(--secondary-world "$hall_world")
    fi

    echo "Validating LDtk worlds..."
    print_cmd env "PYTHONPATH=$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
}

run_dialogue_lint() {
    # Fast pre-flight: catch malformed Yarn markup (e.g. a `[STAGE DIRECTION]`
    # bracket the runtime parses as a tag and panics on at line delivery —
    # "Expected a = inside markup"). Mirrors the authoritative Rust guard
    # `ambition_actors::dialog_lint::no_malformed_yarn_markup_tags`, but
    # runs in milliseconds without a cargo build.
    echo "Linting Yarn dialogue..."
    print_cmd env "PYTHONPATH=$repo_root/tools/ambition_ldtk_tools" \
        "$python_bin" -m ambition_ldtk_tools dialogue lint
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" \
        "$python_bin" -m ambition_ldtk_tools dialogue lint
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
        --cov|coverage)
            coverage=1
            ;;
        --clean-cov|clean-coverage)
            clean_coverage=1
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
        sanic|sanic-demo)
            target_pkg="ambition_demo_sanic_app"
            target_bin="sanic_demo"
            target_kind="demo"
            ;;
        mary-o|mary_o|maryo|mary-o-demo)
            target_pkg="ambition_demo_mary_o_app"
            target_bin="mary_o_demo"
            target_kind="demo"
            ;;
        --headless|headless)
            demo_headless=1
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
    run_dialogue_lint
fi

if [[ "$validate_only" -eq 1 ]]; then
    exit 0
fi

if [[ "$clean_coverage" -eq 1 ]]; then
    cd "$repo_root"
    cargo llvm-cov clean --workspace
fi

cargo_args=()

if [[ "$coverage" -eq 1 ]]; then
    cargo_args+=(llvm-cov run --no-report)
else
    cargo_args+=(run)
fi

cargo_args+=(-p "$target_pkg" --bin "$target_bin")

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
if [[ "$target_kind" == "demo" ]]; then
    if [[ "$demo_headless" -eq 0 ]]; then
        # The demo shells draw only under `visible`. The bin checks
        # `std::env::args().any(|a| a == "--window")` to pick its drawn path,
        # so prepend it — it survives even with no other game args.
        features+=(visible)
        game_args=(--window "${game_args[@]}")
    fi
elif [[ "$hot_reload" -eq 1 ]]; then
    # Hot reload is a host feature; the demo shells don't define it.
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

# Mian cargo run
cargo "${cargo_args[@]}"


# Update coverage files if we are doing that.
if [[ "$coverage" -eq 1 ]]; then

    cd "$repo_root"
    echo "repo_root = $repo_root"

    export COVERAGE_REPORT_DIR="$repo_root/coverage-reports/ambition-manual"

    # Programatic way to get the target dir if we need to
    target_dir="$(cargo metadata --format-version=1 --no-deps |
            "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])'
        )"
    echo "$target_dir"

    # Use the same target dir that contains llvm-cov-target.
    export CARGO_TARGET_DIR="$target_dir"

    mkdir -p "$COVERAGE_REPORT_DIR"
    echo "COVERAGE_REPORT_DIR = $COVERAGE_REPORT_DIR"

    # Compact machine-readable full coverage.
    cargo llvm-cov report \
        --lcov \
        --output-path "$COVERAGE_REPORT_DIR"/manual-game.lcov

    # Smaller per-file summary. Good for quick ranking.
    cargo llvm-cov report \
        --json \
        --summary-only \
        --output-path "$COVERAGE_REPORT_DIR"/manual-game-summary.json

    # Optional: full JSON. Can be large, but useful if upload size is OK.
    cargo llvm-cov report \
        --json \
        --output-path "$COVERAGE_REPORT_DIR"/manual-game-full.json

    # Optional: browsable local report. Useful for you; may be big to upload.
    cargo llvm-cov report \
        --html \
        --output-dir "$COVERAGE_REPORT_DIR"/html

    echo "Coverage reports updated in: $COVERAGE_REPORT_DIR"

fi
