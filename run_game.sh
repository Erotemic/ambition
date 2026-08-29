#!/usr/bin/env bash
# Desktop-only run script. Builds/runs for the host platform (Linux x86-64
# in this dev VM), NOT for Android. Use --help for release, shipping, hot-reload,
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

# ── Machine-local launcher config ────────────────────────────────────────────
# `ambition.local.toml` describes THIS MACHINE — what it can afford to draw and
# what it should boot into. Git-ignored, the way a `.env` is;
# `ambition.local.toml.example` is the committed template.
#
# ⭐ WHY A FILE RATHER THAN A FLAG. The setting that matters here is a property
# of the hardware, not of the invocation: a laptop on integrated graphics wants
# Medium for every run, including the ones a profiling script launches without
# ever passing an argument. A flag has to be remembered every time and is absent
# exactly when an automated run needs it most.
#
# ⛔ AN EXPLICIT ENVIRONMENT VARIABLE ALWAYS WINS. The file only fills in what
# the caller did not already set, so `AMBITION_QUALITY_PROFILE=high ./run_game.sh`
# overrides the file rather than being silently overridden by it. A config that
# could not be beaten from the command line would be untestable.
ambition_load_local_config() {
    local config="$repo_root/ambition.local.toml"
    [[ -f "$config" ]] || return 0
    command -v python3 >/dev/null 2>&1 || {
        echo "run_game.sh: ambition.local.toml found but python3 is not available to read it" >&2
        return 0
    }
    local exports
    # Emits `NAME=value` lines. Errors go to stderr and produce no exports, so a
    # malformed config degrades to "no config" and says why -- it never takes
    # the launch down with it.
    exports="$(python3 - "$config" <<'PYEOF'
import sys

try:
    import tomllib
except ModuleNotFoundError:  # python < 3.11
    sys.stderr.write("run_game.sh: python3 has no tomllib; ambition.local.toml ignored\n")
    raise SystemExit(0)

path = sys.argv[1]
try:
    with open(path, "rb") as handle:
        config = tomllib.load(handle)
except (OSError, tomllib.TOMLDecodeError) as exc:
    sys.stderr.write(f"run_game.sh: {path} could not be read ({exc}); ignoring it\n")
    raise SystemExit(0)

out = {}

profile = config.get("quality", {}).get("profile")
if profile is not None:
    # Validated HERE as well as in the engine, because the engine's complaint
    # arrives after a build and a window; this one arrives before either.
    known = ("potato", "low", "medium", "high", "ultra")
    if str(profile).strip().lower() in known:
        out["AMBITION_QUALITY_PROFILE"] = str(profile).strip().lower()
    else:
        sys.stderr.write(
            f"run_game.sh: quality.profile={profile!r} is not one of {', '.join(known)}; ignoring it\n"
        )

for key, value in (config.get("env") or {}).items():
    if not isinstance(value, (str, int, float, bool)):
        sys.stderr.write(f"run_game.sh: env.{key} is not a scalar; ignoring it\n")
        continue
    if "\n" in str(value) or "=" in str(key):
        sys.stderr.write(f"run_game.sh: env.{key} is not a usable name/value; ignoring it\n")
        continue
    out[str(key)] = "1" if value is True else "0" if value is False else str(value)

for key, value in out.items():
    print(f"{key}={value}")
PYEOF
)" || return 0
    local line name value
    while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        name="${line%%=*}"
        value="${line#*=}"
        # Only fill in what the caller left unset.
        if [[ -z "${!name:-}" ]]; then
            export "$name=$value"
            launcher_config_applied+=("$name=$value")
        fi
    done <<< "$exports"
}

launcher_config_applied=()
ambition_load_local_config
# ⛔ SAY WHAT THE FILE DID. A config that changes how the game boots and leaves
# no trace is indistinguishable from a bug in the game. Printed to stderr so
# `--print-plan`'s stdout contract is untouched.
if ((${#launcher_config_applied[@]})); then
    printf 'run_game.sh: ambition.local.toml set %s\n' "${launcher_config_applied[@]}" >&2
fi

build_profile="dev"
clean_coverage=0
coverage=0
hot_reload=0
validate_before_run=0
validate_only=0
no_default_features=0
cargo_jobs=""
cargo_timings=0
sweep=0
sweep_apply=0
print_plan=0
extra_features=()
game_args=()

# Launch target. Defaults to the multi-game host (the Ambition title screen).
# The `sanic` / `mary-o` / `smash` / `twintrack` mode aliases retarget this to a demo's OWN standalone
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
      Run the multi-game host and open the title screen.

  ./run_game.sh sandbox
      Skip the title screen and run directly in the Ambition sandbox.

  ./run_game.sh sandbox release
  ./run_game.sh sandbox --release
      Run directly in the Ambition sandbox with cargo --release.

  ./run_game.sh --ship
  ./run_game.sh sandbox --ship
      Run the portable native shipping build: opt-level 3, fat LTO, one
      codegen unit, aborting panics, no debug info, and stripped symbols.

  ./run_game.sh profiling --features profile
      Run the optimized profiling build with Tracy instrumentation on. The
      same launch scripts/profile_desktop.sh wraps.

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
  ./run_game.sh smash
  ./run_game.sh twintrack
      Launch a demo's OWN standalone shell (windowed) instead of the host.

  ./run_game.sh sanic --headless -- --ticks 600
      Run a demo's sim-only shell headlessly and pass game args through.

  ./run_game.sh smash-match
  ./run_game.sh profiling --features profile smash-match -- --seconds 90
      Play a real Smash match in a window. The second form is what
      scripts/profile_desktop.sh --smash launches, and quits itself after 90
      seconds of LIVE match (the clock starts at the opening bell, not at
      process start).

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
  sandbox                 Direct Ambition sandbox entry; passes --direct.
  sanic, sanic-demo       Sanic's standalone shell (ambition_demo_sanic_app).
  mary-o, mary_o, maryo   Mary-O's standalone shell (ambition_demo_mary_o_app).
  smash, smash-demo       The stocks demo's standalone shell — opens on
                          CHARACTER SELECT (ambition_demo_smash_app). The same
                          experience is also listed on the host's title screen.
  smash-match             A LIVE ROUND of the shipped Smash composition, for
                          profiling: roster installed, gameplay route entered,
                          measured only once the cast is released
                          (ambition_app_tools/smash_match_profile). Windowed by
                          default; --headless steps it with no window at all.
  twintrack, twin-track,
  twintrack-demo          TwinTrack's standalone shell — the two-seat
                          split-screen round trip (ambition_demo_twintrack_app).
                          Demos default to windowed (--features visible + the
                          --window game arg).
  --headless, headless    Opt the selected demo into its sim-only shell (no
                          window). Ignored for the host.

Options and mode aliases:
  -h, --help              Show this help.
  -r, --release, release  Use cargo --release.
  --ship, ship            Use the native shipping profile intended for the
                          packaged Steam build.
  --profiling, profiling  Use the `profiling` cargo profile: release
                          optimization with DWARF and symbols left in, so perf
                          and Tracy can attribute a frame. This is what
                          scripts/profile_desktop.sh builds by default.
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
  --print-plan            Print the launch plan this invocation resolves to
                          (package, binary, cargo profile, target subdirectory,
                          features, and the cargo build argv) and exit without
                          building or running. Machine-readable `key=value`
                          lines; `build_arg=` repeats once per cargo argument.
                          scripts/profile_desktop.sh reads this rather than
                          re-deriving which executable it is about to launch.
  --sweep                 Report what a sweep would reclaim, keeping ONLY the
                          graph this invocation builds, then run.
  --sweep-apply           As --sweep, but delete. ⚠ the test graph and the other
                          demos are NOT protected and will be rebuilt cold.
  --                      Everything after this is passed to the game binary.

Machine-local config:
  ambition.local.toml       Git-ignored, like a .env; see
                            ambition.local.toml.example for the template.
                            Describes THIS machine, not this invocation --
                            [quality] profile = "medium" forces the visual
                            quality tier for every run, including the ones a
                            profiling script launches with no arguments, and
                            [env] passes any AMBITION_* through verbatim.
                            An explicit environment variable always wins.

Environment:
  AMBITION_QUALITY_PROFILE=potato|low|medium|high|ultra
                            Force the visual quality tier for this process.
                            medium and below cap the window's DPI scale at 1x
                            and turn MSAA off -- on a 2x display that is four
                            times fewer fragments for the same picture, which is
                            the first thing to reach for on a weak GPU. Nothing
                            is written back to the saved settings, and the
                            in-game menu cannot change quality while it is set.
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

# Where the authored worlds live, and which one the validator starts from.
ldtk_worlds_dir="$repo_root/game/ambition_content/assets/worlds"
ldtk_primary_world="sandbox.ldtk"

run_ldtk_validation() {
    [[ -d "$ldtk_worlds_dir" ]] ||
        fail "no LDtk world directory at ${ldtk_worlds_dir#"$repo_root"/} — content moved again?"

    local primary="$ldtk_worlds_dir/$ldtk_primary_world"
    [[ -f "$primary" ]] ||
        fail "the entry world ${ldtk_primary_world} is not in ${ldtk_worlds_dir#"$repo_root"/}"

    # DISCOVERED, not listed.
    local worlds=()
    while IFS= read -r -d '' world; do
        worlds+=("$world")
    done < <(find "$ldtk_worlds_dir" -maxdepth 1 -type f -name '*.ldtk' -print0 | sort -z)

    local cmd=("$python_bin" -m ambition_ldtk_tools validate "$primary")
    local secondaries=()
    local world
    for world in "${worlds[@]}"; do
        [[ "$world" == "$primary" ]] && continue
        cmd+=(--secondary-world "$world")
        secondaries+=("$(basename "$world")")
    done

    echo "Validating LDtk worlds in ${ldtk_worlds_dir#"$repo_root"/}:"
    echo "  entry:     $ldtk_primary_world"
    if ((${#secondaries[@]} == 0)); then
        echo "  secondary: (none found)"
    else
        echo "  secondary: ${secondaries[*]}"
    fi
    print_cmd env "PYTHONPATH=$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
    PYTHONPATH="$repo_root/tools/ambition_ldtk_tools" "${cmd[@]}"
}

run_dialogue_lint() {
    # Fast pre-flight: catch malformed Yarn markup (e.g. a `[STAGE DIRECTION]`
    # bracket the runtime parses as a tag and panics on at line delivery —
    # "Expected a = inside markup"). Mirrors the authoritative Rust guard
    # `ambition_platformer2d_actor_monolith::dialog_lint::no_malformed_yarn_markup_tags`, but
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
            build_profile="release"
            ;;
        --ship|ship)
            build_profile="ship"
            ;;
        --profiling|profiling|profile-build)
            build_profile="profiling"
            ;;
        --debug|debug|dev)
            build_profile="dev"
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
        sandbox|ambition-sandbox)
            game_args+=(--direct)
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
        smash|smash-demo)
            target_pkg="ambition_demo_smash_app"
            target_bin="smash_demo"
            target_kind="demo"
            ;;
        smash-match|smash-match-profile)
            # ⛔⛔ NOT `smash`. That alias opens the standalone demo on CHARACTER
            # SELECT, so profiling it profiles a MENU. This target is the
            # instrument that reaches a LIVE ROUND of the shipped composition:
            # the same app the game binary builds, plus a roster, the smash
            # gameplay route, and a wait for the opening ceremony to release the
            # cast before it claims to be measuring a match. See
            # `game/ambition_app_tools/src/bin/smash_match_profile.rs`.
            target_pkg="ambition_app_tools"
            target_bin="smash_match_profile"
            target_kind="match_profile"
            ;;
        twintrack|twin-track|twintrack-demo)
            target_pkg="ambition_demo_twintrack_app"
            target_bin="twintrack_demo"
            target_kind="demo"
            ;;
        --headless|headless)
            demo_headless=1
            ;;
        --print-plan)
            print_plan=1
            ;;
        --sweep)
            sweep=1
            ;;
        --sweep-apply)
            sweep=1
            sweep_apply=1
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

if [[ "$build_profile" == "ship" && "$hot_reload" -eq 1 ]]; then
    fail "--ship cannot be combined with hot reload; shipping builds exclude development-only reload machinery"
fi

if [[ "$build_profile" == "ship" && "$coverage" -eq 1 ]]; then
    fail "--ship cannot be combined with coverage instrumentation; use --release or dev for coverage runs"
fi

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
elif [[ "$target_kind" == "match_profile" ]]; then
    # The match profiler carries no `visible` feature of its own: it reaches the
    # renderer through `ambition_app`, whose DEFAULT features already include
    # it. What the launcher owes it is the same `--window` word the demo shells
    # read, because that is what selects a real window over the windowless
    # stepping loop.
    if [[ "$demo_headless" -eq 0 ]]; then
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

case "$build_profile" in
    release)
        cargo_args+=(--release)
        ;;
    ship)
        cargo_args+=(--profile ship)
        ;;
    profiling)
        cargo_args+=(--profile profiling)
        ;;
    dev)
        ;;
    *)
        fail "internal error: unknown build profile '$build_profile'"
        ;;
esac

if [[ "${#game_args[@]}" -gt 0 ]]; then
    cargo_args+=(-- "${game_args[@]}")
fi

cd "$repo_root"

# The launch plan, for tooling that must talk about the SAME executable this
# script is about to run. A profiler that re-derives the path itself gets it
# wrong the moment a flag here changes -- it inspects target/debug while the
# command launches target/profiling, and reports "no instrumentation" about a
# binary nobody ran.
if [[ "$print_plan" -eq 1 ]]; then
    case "$build_profile" in
        dev) profile_dir="debug" ;;
        release) profile_dir="release" ;;
        ship) profile_dir="ship" ;;
        profiling) profile_dir="profiling" ;;
        *) fail "internal error: unknown build profile '$build_profile'" ;;
    esac
    plan_build=(cargo build -p "$target_pkg" --bin "$target_bin")
    skip_next=0
    for arg in "${cargo_args[@]}"; do
        if [[ "$skip_next" -eq 1 ]]; then skip_next=0; continue; fi
        case "$arg" in
            # Everything the run-only wrapper adds; the rest (features,
            # profile, jobs, --no-default-features) is exactly what a build of
            # this plan needs.
            run|llvm-cov|--no-report) ;;
            -p|--bin) skip_next=1 ;;
            --) break ;;
            *) plan_build+=("$arg") ;;
        esac
    done
    echo "package=$target_pkg"
    echo "binary=$target_bin"
    echo "target_kind=$target_kind"
    echo "build_profile=$build_profile"
    echo "profile_dir=$profile_dir"
    echo "target_dir=${CARGO_TARGET_DIR:-$repo_root/target}"
    echo "binary_path=${CARGO_TARGET_DIR:-$repo_root/target}/$profile_dir/$target_bin"
    if [[ "${#features[@]}" -gt 0 ]]; then
        IFS=,
        echo "features=${features[*]}"
        unset IFS
    else
        echo "features="
    fi
    if [[ "${#game_args[@]}" -gt 0 ]]; then printf 'game_arg=%s\n' "${game_args[@]}"; fi
    printf 'build_arg=%s\n' "${plan_build[@]}"
    exit 0
fi

# ⭐ SWEEP FIRST, RUN SECOND. `sweep_target.py` marks by asking cargo which
# artifacts THIS graph resolves, which compiles nothing when the graph is warm —
# so the sweep leaves exactly what the run below needs and the game still starts
# immediately. Sweeping afterwards would protect the same set a beat too late.
#
# ⚠ IT KEEPS THE GRAPH YOU ASKED FOR AND NOTHING ELSE. The other demos, the
# release and ship profiles, and the test graph all go. `--sweep` alone reports;
# `--sweep-apply` deletes.
if [[ "$sweep" -eq 1 ]]; then
    sweep_cmd=(build)
    for arg in "${cargo_args[@]}"; do
        case "$arg" in
            run|llvm-cov|--no-report|--timings) ;;
            --) break ;;
            *) sweep_cmd+=("$arg") ;;
        esac
    done
    sweep_args=(--runs-only --cmd "${sweep_cmd[*]#build }")
    [[ "$sweep_apply" -eq 1 ]] && sweep_args+=(--apply)
    printf '⚠ --sweep keeps ONLY this graph; the test graph and other demos will be swept.\n' >&2
    python3 "$repo_root/scripts/sweep_target.py" "${sweep_args[@]}" || fail "the sweep failed; nothing was run"
fi

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
