#!/usr/bin/env bash
# Collect a self-contained desktop profiling bundle for Ambition.
#
# `scripts/profile_desktop.sh` acts like `./run_game.sh`, and additionally dumps
# the diagnostics needed to explain a slowdown afterwards. Run it, play, quit;
# read `summary.md` in the directory it prints.
#
# Design goals:
# - one command, no interactive profiler session, no capture button;
# - profile an OPTIMIZED runtime by default, and say so in the report;
# - never model what `run_game.sh` would launch -- ask it (`--print-plan`);
# - degrade instead of failing: a missing perf, Tracy, or GPU costs its own
#   artifact and nothing else;
# - emit small text/CSV artifacts a human or an agent can read without a GUI.
set -euo pipefail

original_args=("$@")
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
mode="timeline-run"
# Empty means "run until the game exits" (timeline-run's default). The bounded
# modes below substitute default_bounded_duration when nothing was requested.
duration=""
default_bounded_duration="30"
freq="99"
interval_ms="1000"
pid=""
# ⭐ BUNDLES LAND BESIDE THE LEDGER THEY FEED, IN
# `dev/ambition_dev_measurements/profiles/` (untracked), SINCE 2026-08-29.
# Previously `target/profiles`, which put every bundle inside the target
# bindmount where `cargo clean` could take it and where an agent in this
# checkout could not read it without somebody moving it out by hand.
#
# ⭐ WHY BESIDE THE LEDGER: the measurement REPO is what a reader consults, and a
# row in `runtime_frame_cost.jsonl` that names a bundle nobody can find is half a
# record. The raw bundle stays UNTRACKED (gigabytes); `summary.md` is copied into
# the repo's tracked `summaries/` so the readable half travels with the row.
# ⚠ It also means bundles ACCUMULATE rather than being swept with the build dir;
# `perf.data` dominates and a long run is gigabytes, so prune this directory.
# Override with `--out DIR` or `AMBITION_PROFILE_BASE`.
# ⇒ To adopt a bundle recorded elsewhere: move the directory here and re-run
#   `python3 scripts/lib/profile_bundle_to_history.py <dir>`, which appends the
#   ledger row with the new path.
out_base="${AMBITION_PROFILE_BASE:-$repo_root/dev/ambition_dev_measurements/profiles}"
profile_name=""
# Empty means "pick per mode": timeline-run records without call graphs so an
# open-ended session stays small on disk; bounded modes keep DWARF stacks.
perf_call_graph=""
perf_events="task-clock,cycles,instructions,context-switches,cpu-migrations,page-faults,cache-misses"
run_args=()
warm_build="auto"
report_preset="fast"
report_timeout="45"
include_raw_data="no"
include_perf_script="no"
timeline_chunks="12"

# ── What is being profiled ────────────────────────────────────────────────
# The optimized `profiling` cargo profile is the default because the question
# people bring here is "why is the RUNTIME slow", and the dev profile answers a
# different one (`--dev-build` asks that one deliberately). `profiling` is
# release optimization with symbols and line tables kept -- see
# [profile.profiling] in Cargo.toml.
build_profile="profiling"
# `--features profile` turns on bevy/trace + bevy/trace_tracy, which is what
# makes per-Bevy-system zones and per-render-pass diagnostics exist at all.
want_tracy="yes"
census="yes"
census_hz="1"
headless="no"
headless_ticks="1800"
# The launch target a bare `--headless` gets. `sandbox` is `run_game.sh`'s own
# direct-entry alias: normal bodies, normal movement, normal collision, no
# special save file and no special room.
default_headless_scenario="sandbox"
headless_scenario="n/a"
# ── Which WORKLOAD this bundle is about ───────────────────────────────────
# `default` means "whatever the caller launched", which is what every bundle
# before D-smash measured. A named workload is a claim the bundle makes about
# WHAT RAN, and it becomes the history's `scenario.id` -- so a Smash match can
# never be subtracted from a sandbox run, whatever else the two have in common.
workload="default"
scenario_id=""
# The run_game.sh launch target that reaches a LIVE ROUND. ⛔ NOT `smash`: that
# alias opens the standalone demo on character select, so it profiles a menu.
smash_launch_target="smash-match"
smash_fighters="2"
# Wall seconds of LIVE match before the game quits itself, or 0 for "play until
# you quit", which is timeline-run's own default shape.
smash_seconds="0"
# ⭐ APPEND THE BUNDLE'S FRAME COST TO THE MEASUREMENT LEDGER when the run ends.
# A bundle is hundreds of megabytes and gets deleted; the ledger row is what
# survives it, and it is the only thing that can answer "did this commit make
# the frame slower than last week's". Leaving the ingest as a SEPARATE command
# meant a run on real hardware -- the scarcest kind -- could produce numbers
# nobody ever recorded.
record_history="yes"
# Passed straight through to run_game.sh, so the warm build and the launch it
# wraps both honour it. Empty means "cargo's default", which on an agent
# worktree is the WHOLE machine -- see scripts/agent_worktree.sh jobs.
cargo_jobs=""

# Scenario markers AND the in-process censuses. A frame spike that does not
# appear next to the chunk it happened in is a spike nobody can attribute.
# `game-mode`/`sim-clock`/`world-event` are the world-state log: a frozen world
# is two globals, and a timeline that shows neither cannot say which one froze.
marker_regex='room|boss|encounter|title|session|menu|spawn|load|demo|frame-spike|frame-census|image-census|sprite-bind|sprite-size|game-mode|sim-clock|world-event|census'

usage() {
    cat <<'USAGE'
Usage:
  scripts/profile_desktop.sh [MODE] [OPTIONS] [-- RUN_GAME_ARGS ...]

With no arguments: builds the game with the optimized `profiling` cargo profile
and `--features profile`, launches it under perf with a Tracy capture and the
in-process workload censuses running, records until you quit the game, and
writes one bundle. Read the `summary.md` whose path it prints at the end.

On a machine with no usable GPU (the dev VM):

  scripts/profile_desktop.sh --headless

runs the game's own supported headless path in the sandbox scenario and
collects everything that stays meaningful there, labelling the GPU measurements
it could not take. Name a different scenario the way run_game.sh would:

  scripts/profile_desktop.sh --headless -- smash
  scripts/profile_desktop.sh --headless -- -- --start-room goblin_encounter

Modes:
  timeline-run    Launch, record until the game exits (or --duration), slice
                  the capture into labelled time chunks. THE DEFAULT.
  perf-run        Launch under perf record, then emit bounded text reports.
  perf-attach     Attach perf record to an already-running game process.
  stat-run        Launch under perf stat with interval output.
  stat-attach     Attach perf stat to an already-running game process.
  asset-run       Launch under strace and summarize repeated asset opens.
  asset-attach    Attach strace to an already-running game process.
  all-run         Run perf-run, stat-run, then asset-run sequentially.

Build selection:
  --build-profile P       dev | release | profiling | ship. Default: profiling.
  --dev-build             Shorthand for --build-profile dev. Answers "why is my
                           edit/play build slow", which is a DIFFERENT question
                           from "why is the optimized runtime slow"; the report
                           names which one it measured.
  --no-tracy              Do not add --features profile. Drops per-Bevy-system
                           zones and Bevy's render-pass diagnostics.

Smash match (a real round, not the character-select menu):
  --smash                 Profile a LIVE Smash match instead of whatever
                           run_game.sh would otherwise launch. Windowed and
                           hardware-rendered by default -- this is the GPU
                           machine's command. Add --headless for the windowless
                           arm; the two are recorded as different experiments
                           and the history refuses to compare them.
  --smash-fighters N      Fighters on the roster, 2-4. Default: 2.
  --smash-seconds N       Quit after N seconds of LIVE match (the clock starts
                           at the opening bell, not at process start), so an
                           unattended run bounds itself. Default: 0, meaning
                           play until you quit the game.

Headless / no-GPU:
  --headless              Run the game's headless path (--headless) instead of
                           opening a window. Launches the `sandbox` scenario
                           unless the arguments after -- already name a launch
                           target or start room, because a bare headless host
                           sits on the launcher route and simulates no bodies.
                           GPU/render-pass diagnostics are reported as
                           not-applicable rather than missing.
  --headless-ticks N      Ticks to run headlessly. Default: 1800.

Census:
  --census-hz N           Workload census sample rate. Default: 1 (per second).
  --no-census             Do not set AMBITION_PROFILE_CENSUS. The bundle then
                           has no camera/view/entity/portal rows.
  --no-record             Do not append this run's frame cost to the measurement
                          ledger. The ledger row is what outlives the bundle.

Options:
  -h, --help              Show this help.
  -d, --duration SEC      Capture duration. timeline-run defaults to running
                           until the game exits; other modes default to 30.
  --chunks N              timeline-run: number of equal time slices. Default: 12.
  --marker-regex RE       timeline-run: case-insensitive grep -E pattern picking
                           scenario markers out of the game log.
  -F, --freq HZ           perf record sampling frequency. Default: 99.
  -I, --interval MS       perf stat interval in milliseconds. Default: 1000.
  -p, --pid PID           PID to attach to (attach modes).
  -j, --jobs N            Cargo parallel job count, passed to ./run_game.sh for
                           both the warm build and the launch. On an agent
                           worktree use `scripts/agent_worktree.sh jobs N`.
  -o, --out DIR           Output base directory. Default: profiles/ (untracked,
                          outside target/ so it survives `cargo clean`).
  --name NAME             Output directory name suffix. Default: MODE-TIMESTAMP.
  --events LIST           perf stat events.
  --call-graph SPEC       perf call graph spec, or 'none'.
  --report-preset PRESET  none, fast, or full. Default: fast.
  --report-timeout SEC    Max seconds per perf report command. Default: 45.
  --include-perf-script   Also run bounded perf script and gzip it.
  --include-raw-data      Include perf.data in the bundle tarball.
  --no-raw-data           Do not include perf.data in the tarball. Default.
  --warm-build            Build before launch-based captures.
  --no-warm-build         Skip the pre-profile build step.
  --                      Arguments after -- are passed to ./run_game.sh.

Examples:
  scripts/profile_desktop.sh
  scripts/profile_desktop.sh --smash
  scripts/profile_desktop.sh --smash --smash-seconds 90 --smash-fighters 4
  scripts/profile_desktop.sh --smash --headless
  scripts/profile_desktop.sh --headless
  scripts/profile_desktop.sh --dev-build
  scripts/profile_desktop.sh -- sandbox
  scripts/profile_desktop.sh --chunks 20 -- -- --start-room you_have_to_cut_the_rope
  scripts/profile_desktop.sh perf-run --duration 30 --report-preset full

Notes:
  - Ctrl-C during a capture ends the recording and still writes the reports.
  - Every optional profiler is optional: a missing perf, Tracy, GPU timestamp
    query, or strace records WHY it is missing and the run continues.
  - Raw perf.data is excluded from the tarball by default (DWARF stacks reach
    hundreds of MB). Re-run with --include-raw-data when it is needed.
USAGE
}

fail() { echo "profile_desktop.sh: $*" >&2; exit 2; }
log() { printf '[profile-desktop] %s\n' "$*" >&2; }
require_tool() { command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' not found"; }
have_tool() { command -v "$1" >/dev/null 2>&1; }
is_positive_int() { [[ "$1" =~ ^[1-9][0-9]*$ ]]; }
quote_cmd() { printf '%q ' "$@"; }

parse_mode_or_option() {
    case "$1" in
        perf-run|perf-attach|stat-run|stat-attach|asset-run|asset-attach|timeline-run|all-run)
            mode="$1"; return 0 ;;
        *) return 1 ;;
    esac
}

while [[ $# -gt 0 ]]; do
    if parse_mode_or_option "$1"; then shift; continue; fi
    case "$1" in
        -h|--help) usage; exit 0 ;;
        -d|--duration) shift; [[ $# -gt 0 ]] || fail "--duration requires a value"; is_positive_int "$1" || fail "--duration must be positive"; duration="$1" ;;
        --duration=*) duration="${1#--duration=}"; is_positive_int "$duration" || fail "--duration must be positive" ;;
        -F|--freq) shift; [[ $# -gt 0 ]] || fail "--freq requires a value"; is_positive_int "$1" || fail "--freq must be positive"; freq="$1" ;;
        --freq=*) freq="${1#--freq=}"; is_positive_int "$freq" || fail "--freq must be positive" ;;
        -I|--interval) shift; [[ $# -gt 0 ]] || fail "--interval requires a value"; is_positive_int "$1" || fail "--interval must be positive"; interval_ms="$1" ;;
        --interval=*) interval_ms="${1#--interval=}"; is_positive_int "$interval_ms" || fail "--interval must be positive" ;;
        -p|--pid) shift; [[ $# -gt 0 ]] || fail "--pid requires a value"; pid="$1" ;;
        --pid=*) pid="${1#--pid=}" ;;
        -o|--out) shift; [[ $# -gt 0 ]] || fail "--out requires a directory"; out_base="$1" ;;
        --out=*) out_base="${1#--out=}" ;;
        --name) shift; [[ $# -gt 0 ]] || fail "--name requires a value"; profile_name="$1" ;;
        --name=*) profile_name="${1#--name=}" ;;
        --events) shift; [[ $# -gt 0 ]] || fail "--events requires a value"; perf_events="$1" ;;
        --events=*) perf_events="${1#--events=}" ;;
        --call-graph) shift; [[ $# -gt 0 ]] || fail "--call-graph requires a value"; perf_call_graph="$1" ;;
        --call-graph=*) perf_call_graph="${1#--call-graph=}" ;;
        --report-preset) shift; [[ $# -gt 0 ]] || fail "--report-preset requires a value"; report_preset="$1" ;;
        --report-preset=*) report_preset="${1#--report-preset=}" ;;
        --report-timeout) shift; [[ $# -gt 0 ]] || fail "--report-timeout requires a value"; is_positive_int "$1" || fail "--report-timeout must be positive"; report_timeout="$1" ;;
        --report-timeout=*) report_timeout="${1#--report-timeout=}"; is_positive_int "$report_timeout" || fail "--report-timeout must be positive" ;;
        --chunks) shift; [[ $# -gt 0 ]] || fail "--chunks requires a value"; is_positive_int "$1" || fail "--chunks must be positive"; timeline_chunks="$1" ;;
        --chunks=*) timeline_chunks="${1#--chunks=}"; is_positive_int "$timeline_chunks" || fail "--chunks must be positive" ;;
        --marker-regex) shift; [[ $# -gt 0 ]] || fail "--marker-regex requires a value"; marker_regex="$1" ;;
        --marker-regex=*) marker_regex="${1#--marker-regex=}" ;;
        --build-profile) shift; [[ $# -gt 0 ]] || fail "--build-profile requires a value"; build_profile="$1" ;;
        --build-profile=*) build_profile="${1#--build-profile=}" ;;
        --dev-build|--dev-profile) build_profile="dev" ;;
        --no-tracy) want_tracy="no" ;;
        --headless) headless="yes" ;;
        --smash|--smash-match) workload="smash-match" ;;
        --smash-fighters) shift; [[ $# -gt 0 ]] || fail "--smash-fighters requires a value"; is_positive_int "$1" || fail "--smash-fighters must be positive"; workload="smash-match"; smash_fighters="$1" ;;
        --smash-fighters=*) smash_fighters="${1#--smash-fighters=}"; is_positive_int "$smash_fighters" || fail "--smash-fighters must be positive"; workload="smash-match" ;;
        --smash-seconds) shift; [[ $# -gt 0 ]] || fail "--smash-seconds requires a value"; [[ "$1" =~ ^[0-9]+$ ]] || fail "--smash-seconds must be a non-negative integer"; workload="smash-match"; smash_seconds="$1" ;;
        --smash-seconds=*) smash_seconds="${1#--smash-seconds=}"; [[ "$smash_seconds" =~ ^[0-9]+$ ]] || fail "--smash-seconds must be a non-negative integer"; workload="smash-match" ;;
        --headless-ticks) shift; [[ $# -gt 0 ]] || fail "--headless-ticks requires a value"; is_positive_int "$1" || fail "--headless-ticks must be positive"; headless="yes"; headless_ticks="$1" ;;
        --headless-ticks=*) headless_ticks="${1#--headless-ticks=}"; is_positive_int "$headless_ticks" || fail "--headless-ticks must be positive"; headless="yes" ;;
        -j|--jobs) shift; [[ $# -gt 0 ]] || fail "--jobs requires a job count"; is_positive_int "$1" || fail "--jobs must be a positive integer"; cargo_jobs="$1" ;;
        -j[0-9]*) cargo_jobs="${1#-j}"; is_positive_int "$cargo_jobs" || fail "-j must be a positive integer" ;;
        --jobs=*) cargo_jobs="${1#--jobs=}"; is_positive_int "$cargo_jobs" || fail "--jobs must be a positive integer" ;;
        --census-hz) shift; [[ $# -gt 0 ]] || fail "--census-hz requires a value"; census_hz="$1" ;;
        --census-hz=*) census_hz="${1#--census-hz=}" ;;
        --no-census) census="no" ;;
        --no-record) record_history="no" ;;
        --include-perf-script) include_perf_script="yes" ;;
        --include-raw-data) include_raw_data="yes" ;;
        --no-raw-data) include_raw_data="no" ;;
        --warm-build) warm_build="yes" ;;
        --no-warm-build) warm_build="no" ;;
        --) shift; run_args+=("$@"); break ;;
        --*) fail "unknown option '$1'" ;;
        *) fail "unknown mode or option '$1'" ;;
    esac
    shift
done

case "$report_preset" in none|fast|full) ;; *) fail "--report-preset must be none, fast, or full" ;; esac
case "$build_profile" in dev|release|profiling|ship) ;; *) fail "--build-profile must be dev, release, profiling, or ship" ;; esac
[[ "$census_hz" =~ ^[0-9]+(\.[0-9]+)?$ ]] || fail "--census-hz must be a number"

# ⛔ A WINDOWED RUN NEEDS A DISPLAY, AND FAILING HERE IS THE POINT. Without one
# the game's own fallback quietly reroutes to the windowless shared host, which
# does not seat a match at all -- so the bundle would carry a Smash label over a
# measurement of the launcher. Say which command to run instead rather than
# producing a mislabelled bundle.
if [[ "$workload" == "smash-match" && "$headless" == "no" ]]; then
    if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
        fail "--smash opens a real window and this session has no DISPLAY or WAYLAND_DISPLAY.
  On a GPU desktop, run it from the desktop session.
  On this VM, either wrap it (xvfb-run -a -s '-screen 0 1280x720x24' ...), which
  measures a CPU emulating a GPU, or take the windowless arm instead:
      scripts/profile_desktop.sh --smash --headless
  ⛔ The two are NOT comparable and the history keys them apart."
    fi
    if [[ ! -d /dev/dri ]]; then
        log "WARNING: no /dev/dri render node on this host, so this windowed run will almost"
        log "         certainly fall back to a software rasterizer. summary.md will say"
        log "         SOFTWARE RENDERING, and that is NOT a GPU measurement."
    fi
fi

# ── The launch this run is about ──────────────────────────────────────────
# The script contributes the build profile, the tracing feature, and (headless)
# the game's own headless flags; everything the caller passed after `--` comes
# after them, so a caller can override any of it exactly as they would on
# run_game.sh's own command line.
#
# `run_args` may itself contain a `--` separating run_game.sh options from game
# arguments. Split on the first one so appended game arguments land on the game
# side rather than being read as launcher options.
split_run_args() {
    launcher_args=()
    game_args=()
    local seen_separator=0 arg
    for arg in "$@"; do
        if [[ "$seen_separator" -eq 0 && "$arg" == "--" ]]; then seen_separator=1; continue; fi
        if [[ "$seen_separator" -eq 1 ]]; then game_args+=("$arg"); else launcher_args+=("$arg"); fi
    done
}

# Did the caller name WHICH game to run? Any launch target, scenario alias, or
# room/entry game argument counts. `run_game.sh` owns this vocabulary; the list
# is its mode aliases, and a word added there without being added here only
# costs the profiler its default, never a wrong launch.
caller_chose_a_scenario() {
    local arg
    for arg in "$@"; do
        case "$arg" in
            sandbox|ambition-sandbox) return 0 ;;
            cut-rope|cut-rope-boss|smirking-behemoth|you-have-to-cut-the-rope) return 0 ;;
            sanic|sanic-demo) return 0 ;;
            mary-o|mary_o|maryo|mary-o-demo) return 0 ;;
            smash|smash-demo) return 0 ;;
            smash-match|smash-match-profile) return 0 ;;
            twintrack|twin-track|twintrack-demo) return 0 ;;
            --direct|--start-room|--start-room=*) return 0 ;;
        esac
    done
    return 1
}

build_effective_run_args() {
    split_run_args "${run_args[@]+"${run_args[@]}"}"
    effective_run_args=("$build_profile")
    if [[ "$want_tracy" == "yes" ]]; then effective_run_args+=(--features profile); fi
    if [[ -n "$cargo_jobs" ]]; then effective_run_args+=(--jobs "$cargo_jobs"); fi
    local extra_game_args=()
    if [[ "$workload" == "smash-match" ]]; then
        # ⛔⛔ THE MATCH IS A DIFFERENT LAUNCH TARGET, NOT A FLAG ON THE OLD ONE.
        # `run_game.sh smash` builds the standalone demo and opens it on
        # CHARACTER SELECT; profiling that profiles a menu. `smash-match` is the
        # instrument that installs a roster, enters the gameplay route, and
        # waits for the opening ceremony to release the cast before it reports
        # that it is measuring anything.
        effective_run_args+=("$smash_launch_target")
        # ⭐ THE ROSTER SIZE IS PART OF THE WORKLOAD, so it is part of the id. A
        # four-fighter round is not a two-fighter round with noise on it, and
        # one shared id would let a roster change read as a regression.
        scenario_id="smash-match-${smash_fighters}p"
        if [[ "$headless" == "yes" ]]; then
            effective_run_args+=(--headless)
            headless_scenario="$scenario_id"
        fi
        effective_run_args+=("${launcher_args[@]+"${launcher_args[@]}"}")
        extra_game_args+=(--fighters "$smash_fighters")
        if [[ "$headless" == "yes" ]]; then
            extra_game_args+=(--ticks "$headless_ticks")
        elif [[ "$smash_seconds" != "0" ]]; then
            # ⭐ THE BINARY'S OWN BUDGET, NOT `timeout`. `--duration` would start
            # its clock at process start, and a cold launch spends ten-plus
            # seconds on cargo, assets and the shell before a round exists -- so
            # the same number would measure a different window on every machine.
            # `--seconds` starts at the opening bell.
            extra_game_args+=(--seconds "$smash_seconds")
        fi
        if [[ "${#game_args[@]}" -gt 0 || "${#extra_game_args[@]}" -gt 0 ]]; then
            effective_run_args+=(--)
            effective_run_args+=("${game_args[@]+"${game_args[@]}"}")
            effective_run_args+=("${extra_game_args[@]+"${extra_game_args[@]}"}")
        fi
        return 0
    fi
    # ⛔ A HEADLESS RUN WITH NO SCENARIO PROFILES NOTHING WORTH PROFILING.
    # Bare `--headless` steps the production shared host, which sits on the
    # startup/launcher route: zero bodies, no movement, no collision. It
    # succeeds, and the bundle describes host composition. Pick the sandbox --
    # the ordinary supported entry, not a profiling-only setup -- so the
    # default command produces a representative simulation workload. A caller
    # who named a scenario keeps it.
    if [[ "$headless" == "yes" ]]; then
        if caller_chose_a_scenario \
            "${launcher_args[@]+"${launcher_args[@]}"}" \
            "${game_args[@]+"${game_args[@]}"}"; then
            headless_scenario="caller-specified"
        else
            effective_run_args+=("$default_headless_scenario")
            headless_scenario="$default_headless_scenario (profiler default)"
        fi
    fi
    effective_run_args+=("${launcher_args[@]+"${launcher_args[@]}"}")
    if [[ "$headless" == "yes" ]]; then
        extra_game_args+=(--headless --headless-ticks "$headless_ticks")
    fi
    if [[ "${#game_args[@]}" -gt 0 || "${#extra_game_args[@]}" -gt 0 ]]; then
        effective_run_args+=(--)
        effective_run_args+=("${game_args[@]+"${game_args[@]}"}")
        effective_run_args+=("${extra_game_args[@]+"${extra_game_args[@]}"}")
    fi
}

build_effective_run_args
run_cmd=("$repo_root/run_game.sh" "${effective_run_args[@]}")

# ASK the launcher which executable this is, do not model it. The old code
# scanned target/debug first and reported "no Tracy client" about a binary the
# command was not launching.
plan_binary=""
plan_package=""
plan_bin=""
plan_profile_dir=""
plan_features=""
plan_build_cmd=()
plan_status="unresolved"
resolve_launch_plan() {
    local line key value
    plan_build_cmd=()
    if ! plan_text="$("$repo_root/run_game.sh" --print-plan "${effective_run_args[@]}" 2>/dev/null)"; then
        plan_status="run_game.sh --print-plan failed"
        log "could not resolve the launch plan; falling back to a bare launch"
        return 0
    fi
    while IFS= read -r line; do
        key="${line%%=*}"; value="${line#*=}"
        case "$key" in
            binary_path) plan_binary="$value" ;;
            package) plan_package="$value" ;;
            binary) plan_bin="$value" ;;
            profile_dir) plan_profile_dir="$value" ;;
            features) plan_features="$value" ;;
            build_arg) plan_build_cmd+=("$value") ;;
        esac
    done <<< "$plan_text"
    plan_status="ok"
}

# timeline-run is the "just record my session" mode: it runs until the game
# exits and skips call-graph capture, so an open-ended session cannot balloon
# perf.data. Every other mode keeps the old bounded-window defaults.
if [[ "$mode" != "timeline-run" && -z "$duration" ]]; then duration="$default_bounded_duration"; fi
if [[ -z "$perf_call_graph" ]]; then
    case "$mode" in
        timeline-run) perf_call_graph="none" ;;
        *) perf_call_graph="dwarf,8192" ;;
    esac
fi

# ⛔ Tracy REFUSES TO START, AND TAKES THE GAME WITH IT, on a CPU that does not
# advertise an invariant TSC -- which is every VM whose hypervisor hides
# `nonstop_tsc`. The client aborts during initialization and the process exits
# 1 before a frame runs, so `--features profile` would make the DEFAULT command
# of this script kill the game it was asked to profile. Set the documented
# escape hatch when the flags are absent, and record that the zone timestamps
# came off a TSC the kernel does not vouch for.
tracy_tsc_note=""
if [[ "$want_tracy" == "yes" ]]; then
    cpu_flags="$(grep -m1 '^flags' /proc/cpuinfo 2>/dev/null || true)"
    if [[ -n "$cpu_flags" ]] && ! { [[ "$cpu_flags" == *constant_tsc* ]] && [[ "$cpu_flags" == *nonstop_tsc* ]]; }; then
        export TRACY_NO_INVARIANT_CHECK=1
        tracy_tsc_note="this CPU does not advertise an invariant TSC (constant_tsc + nonstop_tsc); TRACY_NO_INVARIANT_CHECK=1 was set so the game could run. Tracy zone durations here come from a timer the kernel does not vouch for: treat their RATIOS as sound and their absolute microseconds as approximate."
    fi
fi

# The in-process censuses read this. Exported here, once, so every launch mode
# and the attach modes' target inherit the same cadence.
if [[ "$census" == "yes" ]]; then
    export AMBITION_PROFILE_CENSUS=1
    export AMBITION_PROFILE_CENSUS_HZ="$census_hz"
else
    unset AMBITION_PROFILE_CENSUS || true
fi

# perf rejects `-g` alongside no call graph, so drop both flags for 'none'
# rather than passing a spec perf may or may not accept.
perf_record_argv() {
    local data_file="$1"
    local args=(perf record -F "$freq")
    if [[ "$perf_call_graph" != "none" ]]; then args+=(-g --call-graph "$perf_call_graph"); fi
    args+=(-o "$data_file")
    printf '%s\0' "${args[@]}"
}

# Empty duration means "until the process exits" -- emit the command unwrapped
# instead of under `timeout`.
bounded_argv() {
    if [[ -n "$duration" ]]; then printf '%s\0' timeout --signal=INT --kill-after=5s "${duration}s"; fi
    printf '%s\0' "$@"
}

# ── Tracy ─────────────────────────────────────────────────────────────────
# Tracy answers what perf structurally cannot: per-Bevy-system and per-render-
# pass timings. It is used when it is USABLE and skipped loudly otherwise --
# never installed here, and never a reason for the default run to fail.
tracy_tools_present() {
    have_tool tracy-capture && have_tool tracy-csvexport
}

binary_has_tracy() {
    local bin="${1:-}"
    [[ -n "$bin" && -f "$bin" ]] || return 1
    # The tracy client leaves its symbols in the binary; no need to run it.
    # `grep -a -m1` reads the file directly and stops at the first hit -- a
    # profiling build with DWARF is over a gigabyte, and piping it all through
    # `strings` first would cost tens of seconds before the game even launches.
    grep -aqm1 -e 'tracy_emit_zone_begin' -e 'TracyPlot' "$bin" 2>/dev/null
}

# Start a Tracy capture in the background if this run can produce one. Echoes
# the capture PID so the caller can wait on it; echoes nothing when skipped.
start_tracy_capture() {
    local out_dir="$1"
    if [[ "$want_tracy" == "no" ]]; then
        echo "--no-tracy was passed; no per-system or per-render-pass timings were collected" \
            > "$out_dir/tracy.skipped"
        return 0
    fi
    if ! tracy_tools_present; then
        echo "tracy-capture/tracy-csvexport not found; per-system timings unavailable (run ./run_developer_setup.sh)" \
            > "$out_dir/tracy.skipped"
        log "Tracy tools not installed; skipping per-system capture (run ./run_developer_setup.sh)"
        return 0
    fi
    # The binary this command LAUNCHES, resolved by run_game.sh itself.
    if ! binary_has_tracy "$plan_binary"; then
        {
            echo "the launched binary has no tracy client"
            echo "binary: ${plan_binary:-<unresolved>}"
            echo "features: ${plan_features:-<none>}"
            echo "rebuild with --features profile (the default unless --no-tracy was passed)"
        } > "$out_dir/tracy.skipped"
        log "binary has no Tracy client: ${plan_binary:-<unresolved>}"
        return 0
    fi
    log "Tracy client detected in $plan_binary; capturing per-system timings alongside perf"
    if [[ -n "$tracy_tsc_note" ]]; then
        printf '%s\n' "$tracy_tsc_note" > "$out_dir/tracy.caveat"
        log "$tracy_tsc_note"
    fi
    tracy-capture -o "$out_dir/tracy.trace" -f > "$out_dir/tracy-capture.log" 2>&1 &
    printf '%s\n' "$!"
}

# Turn the trace into the thing that is actually readable without a GUI: a
# ranked table of the costliest zones (Bevy systems and render passes), plus
# per-window tables when the exporter can unwrap individual zone instances.
finish_tracy_capture() {
    local out_dir="$1" capture_pid="${2:-}"
    [[ -n "$capture_pid" ]] || return 0
    # tracy-capture exits on its own when the game disconnects; give it a
    # moment, then stop waiting rather than hanging the whole profile run.
    local waited=0
    while kill -0 "$capture_pid" 2>/dev/null && (( waited < 30 )); do
        sleep 1
        waited=$((waited + 1))
    done
    kill "$capture_pid" 2>/dev/null || true
    wait "$capture_pid" 2>/dev/null || true
    if [[ ! -s "$out_dir/tracy.trace" ]]; then
        echo "tracy-capture produced no trace (the game never connected)" > "$out_dir/tracy.skipped"
        log "Tracy produced no trace"
        return 0
    fi
    if ! tracy-csvexport "$out_dir/tracy.trace" > "$out_dir/tracy_zones.csv" 2>"$out_dir/tracy-csvexport.stderr"; then
        log "tracy-csvexport failed; the raw tracy.trace is still in the bundle"
        echo "tracy-csvexport failed; see tracy-csvexport.stderr" > "$out_dir/tracy.skipped"
        return 0
    fi
    # `--unwrap` emits one row per zone INSTANCE with its start time, which is
    # the only way to say "this system got slow between 74s and 81s" rather
    # than "this system cost 4s over the whole session". It is not present in
    # every Tracy build, so its absence is recorded, not fatal.
    if tracy-csvexport --unwrap "$out_dir/tracy.trace" > "$out_dir/tracy_zone_instances.csv" 2>/dev/null \
        && [[ -s "$out_dir/tracy_zone_instances.csv" ]]; then
        log "tracy zone instances exported (windowed zone tables available)"
    else
        rm -f "$out_dir/tracy_zone_instances.csv"
        echo "this tracy-csvexport has no --unwrap; zone tables are whole-session totals only" \
            > "$out_dir/tracy_zone_instances.missing"
    fi
    python3 "$repo_root/scripts/lib/tracy_zone_report.py" "$out_dir" "$census_hz" \
        || log "tracy zone report failed; tracy_zones.csv is still in the bundle"
    # ⛔⛔ AND THEN DROP IT: THIS INTERMEDIATE REACHED **90.6 GB** IN ONE BUNDLE.
    # `--unwrap` emits a row per zone INSTANCE, and a Tracy-on run of this game
    # emits ~4,000 zones per frame — 113.9 million rows for a four-minute
    # session. Two bundles held 118GB between them and filled a 484GB disk to
    # 100%, which stopped every build in the repo.
    #
    # ⭐ Nothing reads it after this point: `tracy_zone_report.py` has already
    # reduced it to the per-window tables, and every other consumer (the summary,
    # this file's own analysis) reads the AGGREGATE `tracy_zones.csv`.
    # ⇒ it is a regenerable intermediate, and `tracy.trace` beside it is the
    # thing that regenerates it — at ~3% of the size.
    if [[ -f "$out_dir/tracy_zone_instances.csv" ]]; then
        local instances_bytes
        instances_bytes="$(stat -c %s "$out_dir/tracy_zone_instances.csv" 2>/dev/null || echo 0)"
        rm -f "$out_dir/tracy_zone_instances.csv"
        printf '%s\n' \
            "removed after the zone report was built: it was ${instances_bytes} bytes." \
            "Regenerate with:" \
            "  tracy-csvexport --unwrap tracy.trace > tracy_zone_instances.csv" \
            > "$out_dir/tracy_zone_instances.removed"
        log "tracy zone instances pruned (${instances_bytes} bytes; regenerate from tracy.trace)"
    fi
}

describe_window() {
    if [[ -n "$duration" ]]; then printf 'window: %ss' "$duration"; else printf 'until you quit the game'; fi
}

make_profile_dir() {
    local local_mode="$1" suffix
    if [[ -n "$profile_name" ]]; then suffix="$profile_name"; else suffix="$local_mode-$stamp"; fi
    local dir="$out_base/desktop-$suffix"
    mkdir -p "$dir"
    printf '%s\n' "$dir"
}

find_game_pid() {
    if [[ -n "$pid" ]]; then printf '%s\n' "$pid"; return 0; fi
    local found="" candidate
    local names=(ambition_game_bin ambition_platformer2d_actor_monolith)
    [[ -n "$plan_bin" ]] && names=("$plan_bin" "${names[@]}")
    for candidate in "${names[@]}"; do
        found="$(pgrep -n -x "$candidate" 2>/dev/null || true)"
        if [[ -n "$found" ]]; then printf '%s\n' "$found"; return 0; fi
    done
    for candidate in "${names[@]}"; do
        found="$(pgrep -n -f "$candidate" 2>/dev/null || true)"
        if [[ -n "$found" ]]; then printf '%s\n' "$found"; return 0; fi
    done
    fail "could not find a running game process; pass --pid or use a *-run mode"
}

# The pgrep -f fallback can latch onto anything whose command line mentions an
# ambition crate (e.g. a concurrent cargo build). Record and show what we
# actually attached to so a wrong-target capture is obvious immediately.
record_attach_target() {
    local out_dir="$1" target_pid="$2" cmdline
    echo "$target_pid" > "$out_dir/pid.txt"
    cmdline="$(ps -o args= -p "$target_pid" 2>/dev/null || true)"
    printf '%s\n' "$cmdline" > "$out_dir/pid-cmdline.txt"
    log "attach target PID $target_pid: ${cmdline:-<process not found>}"
    case "$cmdline" in
        *ambition_game_bin*|*mary_o_demo*|*sanic_demo*|*smash_demo*|*twintrack_demo*) ;;
        *smash_match_profile*) ;;
        *) log "WARNING: attach target does not look like a game binary; pass --pid to override" ;;
    esac
}

# The kernel gate for unprivileged perf. Debian/Ubuntu ship
# kernel.perf_event_paranoid=3 or 4, which blocks ALL unprivileged perf_event_open;
# upstream 2 allows user-space-only samples; 1 additionally allows kernel-side
# samples, so cycles/context-switch attribution and mixed user/kernel stacks
# resolve (the level docs/recipes/profiling.md prescribes). Request exactly 1,
# for this boot only (`sysctl -w` does not persist across reboots).
ensure_perf_kernel_level() {
    local target=1 current
    current="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || true)"
    if [[ ! "$current" =~ ^-?[0-9]+$ ]]; then
        # No knob visible (non-Linux, locked-down container): let perf itself
        # produce the authoritative error.
        return 0
    fi
    if (( current <= target )); then return 0; fi
    if [[ "$(id -u)" == "0" ]]; then return 0; fi
    local sudo_cmd=(sudo)
    # Without a terminal there is nobody to answer a password prompt; -n makes
    # sudo fail fast instead of hanging the capture.
    if [[ ! -t 0 ]]; then sudo_cmd=(sudo -n); fi
    local blocked="kernel-side samples"
    if (( current > 2 )); then blocked="all unprivileged perf profiling"; fi
    log "kernel.perf_event_paranoid=$current blocks $blocked; requesting level $target for this boot"
    if "${sudo_cmd[@]}" sysctl -w kernel.perf_event_paranoid="$target"; then
        log "kernel.perf_event_paranoid=$target until reboot (persist via: echo kernel.perf_event_paranoid=$target | sudo tee /etc/sysctl.d/local-perf.conf)"
    elif (( current > 2 )); then
        fail "perf is fully blocked at kernel.perf_event_paranoid=$current; run: sudo sysctl -w kernel.perf_event_paranoid=$target"
    else
        log "sudo declined/unavailable; continuing with user-space-only samples (kernel.perf_event_paranoid=$current)"
    fi
}

# Separate knob from perf_event_paranoid: kptr_restrict=1 zeroes /proc/kallsyms
# addresses for unprivileged readers, so kernel-side samples all show as
# [unknown] 0xffffffff... rows. User-space symbols are unaffected; this only
# recovers kernel attribution (futex vs ioctl vs page fault).
ensure_kernel_symbol_visibility() {
    local current
    current="$(cat /proc/sys/kernel/kptr_restrict 2>/dev/null || true)"
    if [[ ! "$current" =~ ^-?[0-9]+$ ]] || (( current == 0 )); then return 0; fi
    if [[ "$(id -u)" == "0" ]]; then return 0; fi
    local sudo_cmd=(sudo)
    if [[ ! -t 0 ]]; then sudo_cmd=(sudo -n); fi
    log "kernel.kptr_restrict=$current hides kernel symbols; requesting 0 for this boot"
    if "${sudo_cmd[@]}" sysctl -w kernel.kptr_restrict=0; then
        log "kernel.kptr_restrict=0 until reboot"
    else
        log "sudo declined/unavailable; kernel samples will stay [unknown] (user-space symbols unaffected)"
    fi
}

mode_uses_launch() { case "$1" in perf-run|stat-run|asset-run|timeline-run) return 0 ;; *) return 1 ;; esac; }
warm_build_is_enabled_for() {
    case "$warm_build" in
        yes) return 0 ;;
        no) return 1 ;;
        auto) mode_uses_launch "$1" ;;
        *) fail "invalid warm_build setting '$warm_build'" ;;
    esac
}

run_warm_build_if_needed() {
    local out_dir="$1" local_mode="$2"
    if ! warm_build_is_enabled_for "$local_mode"; then echo "skipped" > "$out_dir/warm-build.status"; return 0; fi
    if [[ "${#plan_build_cmd[@]}" -eq 0 ]]; then
        echo "no build plan (run_game.sh --print-plan failed); the launch will build instead" \
            > "$out_dir/warm-build.status"
        return 0
    fi
    require_tool cargo
    echo "$(quote_cmd "${plan_build_cmd[@]}")" > "$out_dir/warm-build-command.txt"
    log "warm-building: $(quote_cmd "${plan_build_cmd[@]}")"
    set +e
    (cd "$repo_root" && "${plan_build_cmd[@]}") > >(tee "$out_dir/warm-build.stdout") 2> >(tee "$out_dir/warm-build.stderr" >&2)
    local status=$?
    set -e
    echo "$status" > "$out_dir/warm-build.status"
    if [[ "$status" -ne 0 ]]; then log "warm build failed; see $out_dir/warm-build.stderr"; return "$status"; fi
}

write_metadata() {
    local out_dir="$1" local_mode="$2"
    {
        echo "mode=$local_mode"
        echo "utc_stamp=$stamp"
        echo "repo_root=$repo_root"
        echo "output_dir=$out_dir"
        echo "duration_seconds=${duration:-until-game-exits}"
        echo "sampling_frequency_hz=$freq"
        echo "stat_interval_ms=$interval_ms"
        echo "perf_call_graph=$perf_call_graph"
        echo "perf_events=$perf_events"
        echo "report_preset=$report_preset"
        echo "timeline_chunks=$timeline_chunks"
        echo "marker_regex=$marker_regex"
        echo "report_timeout_seconds=$report_timeout"
        echo "include_raw_data=$include_raw_data"
        echo "include_perf_script=$include_perf_script"
        echo "cargo_profile=$build_profile"
        echo "cargo_jobs=${cargo_jobs:-<cargo default>}"
        echo "profile_dir=$plan_profile_dir"
        echo "cargo_features=$plan_features"
        echo "package=$plan_package"
        echo "binary=$plan_bin"
        echo "binary_path=$plan_binary"
        echo "launch_plan_status=$plan_status"
        echo "headless=$headless"
        echo "headless_ticks=$headless_ticks"
        echo "headless_scenario=$headless_scenario"
        # ⭐ WHAT RAN, SAID BY THE THING THAT LAUNCHED IT. The history used to
        # reverse-engineer a windowed run's workload from the command line,
        # which lands every windowed bundle in one `windowed:default` group --
        # so a Smash match and a title-screen session would be subtracted from
        # each other. Empty means "no claim", and the ingest falls back to the
        # old derivation for every bundle taken before this line existed.
        echo "workload=$workload"
        echo "scenario_id=$scenario_id"
        echo "smash_fighters=$smash_fighters"
        echo "smash_seconds=$smash_seconds"
        echo "tracy_requested=$want_tracy"
        echo "census_enabled=$census"
        echo "census_hz=$census_hz"
        echo "run_command=$(quote_cmd "${run_cmd[@]}")"
        echo "warm_build_setting=$warm_build"
        echo "script_command=$(quote_cmd "$0" "${original_args[@]}")"
        echo "hostname=$(hostname 2>/dev/null || true)"
        # ⛔⛔ THE HOSTNAME IS NOT THE MACHINE. These hosts reuse names -- two
        # different boxes have both answered to `aivm-2404`, with different CPUs
        # and core counts, and a baseline recorded on one was read as if it came
        # from the other. `/etc/machine-id` is per-installation and is what the
        # history's comparability key keys on; the hostname stays for humans.
        echo "machine_id=$(cat /etc/machine-id 2>/dev/null || true)"
        echo "uname=$(uname -a 2>/dev/null || true)"
        echo "rust_target=$(rustc -vV 2>/dev/null | awk '/^host:/ {print $2}' || true)"
        echo "rustc_version=$(rustc --version 2>/dev/null || true)"
        echo "git_head=$(cd "$repo_root" && git rev-parse HEAD 2>/dev/null || true)"
        echo "git_head_short=$(cd "$repo_root" && git rev-parse --short=12 HEAD 2>/dev/null || true)"
        echo "git_branch=$(cd "$repo_root" && git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
        echo "git_status_porcelain_begin"
        (cd "$repo_root" && git status --short 2>/dev/null || true)
        echo "git_status_porcelain_end"
        echo "perf_version=$(perf --version 2>/dev/null || true)"
        echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || true)"
        # -1/0 here is what lets kernel symbols resolve. Recorded because a
        # capture is usually read on a different machine, where re-running
        # `perf report` resolves kernel addresses against the WRONG kallsyms
        # and silently prints raw 0xffffffff... rows.
        echo "kptr_restrict=$(cat /proc/sys/kernel/kptr_restrict 2>/dev/null || true)"
        echo "strace_version=$(strace --version 2>/dev/null | head -1 || true)"
        echo "python3_version=$(python3 --version 2>/dev/null || true)"
        echo "tracy_invariant_tsc_override=${TRACY_NO_INVARIANT_CHECK:-0}"
        echo "tracy_capture=$(command -v tracy-capture 2>/dev/null || echo '<not installed>')"
        echo "tracy_csvexport=$(command -v tracy-csvexport 2>/dev/null || echo '<not installed>')"
    } > "$out_dir/metadata.txt"
    write_host_environment "$out_dir"
    python3 "$repo_root/scripts/lib/profile_metadata_json.py" "$out_dir" || true
}

# Samples alone cannot tell you which GPU -- if any -- the run actually used,
# and that single fact reorders every other number here: a software-rendered
# run puts ~90% of cycles in the rasterizer's JIT'd shader code, where no
# symbol will ever explain them. Captures are also routinely read on a
# different machine than they were taken on, so record the graphics
# environment next to the samples rather than leaving the reader to inspect
# their own box and reach confident, wrong conclusions.
write_host_environment() {
    local out_dir="$1"
    {
        echo "## CPU / memory"
        grep -m1 'model name' /proc/cpuinfo 2>/dev/null || true
        echo "logical_cpus=$(nproc 2>/dev/null || true)"
        grep -m1 MemTotal /proc/meminfo 2>/dev/null || true
        echo
        echo "## Session"
        echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>}"
        echo "DISPLAY=${DISPLAY:-<unset>}"
        echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
        echo
        echo "## Graphics env overrides (these can force software rendering)"
        env | grep -E '^(VK_|WGPU_|MESA_|LIBGL|GALLIUM_|__NV_|__GL|__VK|NVIDIA_|DRI_|AMD_)' | sort || true
        echo
        echo "## DRM render nodes"
        ls -l /dev/dri/ 2>&1 || true
        echo
        echo "## Vulkan ICDs installed"
        ls /usr/share/vulkan/icd.d/ /etc/vulkan/icd.d/ 2>&1 || true
        echo
        echo "## NVIDIA"
        if command -v nvidia-smi >/dev/null 2>&1; then
            timeout 10s nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv 2>&1 || true
        else
            echo "nvidia-smi not found"
        fi
        cat /proc/driver/nvidia/version 2>/dev/null || true
        echo
        echo "## Vulkan adapters"
        if command -v vulkaninfo >/dev/null 2>&1; then
            timeout 20s vulkaninfo --summary 2>&1 | sed -n '/Devices:/,$p' | head -40 || true
        else
            echo "vulkaninfo not found (apt install vulkan-tools to enumerate adapters)"
        fi
        echo
        echo "## GL"
        if command -v glxinfo >/dev/null 2>&1; then
            timeout 10s glxinfo -B 2>&1 | head -12 || true
        else
            echo "glxinfo not found (apt install mesa-utils)"
        fi
    } > "$out_dir/host-environment.txt" 2>&1
}

run_with_tee() {
    local stdout_file="$1" stderr_file="$2"; shift 2
    set +e
    "$@" > >(tee "$stdout_file") 2> >(tee "$stderr_file" >&2)
    local status=$?
    set -e
    return "$status"
}

run_capture_command() {
    local status_file="$1"; shift
    local stem="${status_file%.status}"
    echo "$(quote_cmd "$@")" > "$stem.command.txt"
    log "running $(basename "$stem"): $(quote_cmd "$@")"
    # Captures are silent while running; tick so a quiet terminal is not
    # mistaken for a hang (Ctrl-C mid-capture leaves a truncated perf.data).
    local heartbeat_pid=""
    (
        elapsed=0
        while sleep 5; do
            elapsed=$((elapsed + 5))
            log "$(basename "$stem") capturing... ${elapsed}s elapsed ($(describe_window))"
        done
    ) &
    heartbeat_pid=$!
    run_with_tee "$stem.stdout" "$stem.stderr" "$@"
    local status=$?
    kill "$heartbeat_pid" 2>/dev/null || true
    wait "$heartbeat_pid" 2>/dev/null || true
    echo "$status" > "$status_file"
    if [[ "$status" -ne 0 && "$status" -ne 124 && "$status" -ne 130 ]]; then
        log "command exited with status $status: $(quote_cmd "$@")"
    fi
    return 0
}

run_timed_report() {
    local out_dir="$1" name="$2"; shift 2
    local out_file="$out_dir/$name.txt" err_file="$out_dir/$name.stderr" status_file="$out_dir/$name.status"
    echo "$(quote_cmd timeout --kill-after=5s "${report_timeout}s" "$@")" > "$out_dir/$name.command.txt"
    log "generating $name with ${report_timeout}s timeout"
    set +e
    timeout --kill-after=5s "${report_timeout}s" "$@" > "$out_file" 2> >(tee "$err_file" >&2)
    local status=$?
    set -e
    echo "$status" > "$status_file"
    if [[ "$status" -eq 124 || "$status" -eq 137 ]]; then
        log "$name timed out after ${report_timeout}s; continuing"
    elif [[ "$status" -ne 0 ]]; then
        log "$name failed with status $status; continuing"
    fi
}

write_perf_reports() {
    local out_dir="$1"; local data_file="$out_dir/perf.data"
    if [[ ! -s "$data_file" ]]; then log "perf.data missing or empty; skipping perf reports"; return 0; fi
    stat -c '%s' "$data_file" > "$out_dir/perf.data.bytes" 2>/dev/null || true
    [[ "$report_preset" == "none" ]] && return 0

    # Fast, robust, no callgraph report first. This is the one most likely to finish.
    run_timed_report "$out_dir" perf_report \
        perf report -i "$data_file" --stdio --sort comm,dso,symbol --call-graph none --percent-limit 0.25 --no-inline --no-source

    # Whole-population rollups at no percent limit. These answer "which layer
    # owns the machine" (renderer vs game code vs kernel) in one read, and they
    # must be produced HERE, on the capture host: re-deriving them later on
    # another machine resolves symbols against the wrong binaries.
    run_timed_report "$out_dir" perf-report-by-dso \
        perf report -i "$data_file" --stdio --sort dso --call-graph none --percent-limit 0 --no-inline --no-source
    run_timed_report "$out_dir" perf-report-by-thread \
        perf report -i "$data_file" --stdio --sort comm --call-graph none --percent-limit 0 --no-inline --no-source

    if [[ "$report_preset" == "full" ]]; then
        run_timed_report "$out_dir" perf-report-self-symbols \
            perf report -i "$data_file" --stdio --no-children --sort comm,dso,symbol --percent-limit 0.25 --no-inline --no-source
        run_timed_report "$out_dir" perf-report-children-symbols \
            perf report -i "$data_file" --stdio --children --sort comm,dso,symbol --call-graph graph,0.5,caller,function --percent-limit 0.25 --no-inline --no-source
    fi

    if [[ "$include_perf_script" == "yes" ]]; then
        run_timed_report "$out_dir" perf-script perf script -i "$data_file"
        if [[ -s "$out_dir/perf-script.txt" ]]; then gzip -9 "$out_dir/perf-script.txt"; fi
    else
        echo "skipped" > "$out_dir/perf-script.status"
    fi
}

write_asset_summary() {
    local out_dir="$1"; local trace_file="$out_dir/strace-assets.txt"
    if [[ ! -s "$trace_file" ]]; then log "strace output missing or empty; skipping asset summary"; return 0; fi
    python3 - "$trace_file" "$out_dir" <<'PY'
import os, re, sys
from collections import Counter
trace_path, out_dir = sys.argv[1:3]
image_exts = ('.png','.jpg','.jpeg','.webp','.ktx2')
asset_exts = image_exts + ('.ron','.ldtk','.ttf','.otf','.wav','.ogg','.flac','.mp3')
open_pat = re.compile(r'openat(?:2)?\([^\"]*\"([^\"]+)\"')
image_counts, asset_counts, all_counts = Counter(), Counter(), Counter()
with open(trace_path, 'r', errors='replace') as f:
    for line in f:
        m = open_pat.search(line)
        if not m: continue
        path = m.group(1); lower = path.lower(); all_counts[path] += 1
        if lower.endswith(image_exts): image_counts[path] += 1
        if lower.endswith(asset_exts): asset_counts[path] += 1

def write_counts(filename, counts):
    with open(os.path.join(out_dir, filename), 'w') as f:
        f.write('count\tpath\n')
        for path, count in counts.most_common(): f.write(f'{count}\t{path}\n')
write_counts('image-open-counts.tsv', image_counts)
write_counts('asset-open-counts.tsv', asset_counts)
write_counts('all-open-counts.tsv', all_counts)
with open(os.path.join(out_dir, 'asset-trace-summary.md'), 'w') as f:
    f.write('# Asset trace summary\n\n')
    for title, counts, limit in [('Top image opens', image_counts, 40), ('Top asset opens', asset_counts, 60)]:
        f.write(f'## {title}\n\n')
        if counts:
            f.write('```text\n')
            for path, count in counts.most_common(limit): f.write(f'{count:6d} {path}\n')
            f.write('```\n\n')
        else:
            f.write('No opens found.\n\n')
PY
}

# The two report writers every mode ends with: the census log becomes CSVs, and
# everything in the directory becomes one summary.md.
write_bundle_reports() {
    local out_dir="$1"
    python3 "$repo_root/scripts/lib/profile_census_csv.py" "$out_dir" \
        || log "census CSV extraction failed; the stamped log is still in the bundle"
    python3 "$repo_root/scripts/lib/profile_bundle_summary.py" "$out_dir" \
        || log "summary generation failed; the raw artifacts are still in the bundle"
}

package_dir() {
    local out_dir="$1"; local tarball="$out_dir.tar.gz" base
    base="$(basename "$out_dir")"
    printf '%s\n' "$tarball" > "$out_dir/package-path.txt"
    if [[ "$include_raw_data" == "yes" ]]; then
        tar -czf "$tarball" -C "$(dirname "$out_dir")" "$base"
    else
        tar -czf "$tarball" \
            --exclude='*/perf.data' \
            --exclude='*/perf-script.txt' \
            --exclude='*/perf-script.txt.gz' \
            -C "$(dirname "$out_dir")" "$base"
    fi
    log "wrote $tarball"
    printf '%s\n' "$tarball"
}

# Bevy colorizes its log; the escapes survive into the file and break naive
# greps for module paths (`[2mambition::save[0m`). Strip them and prefix each
# line with seconds since launch, so the game's own census rows share one clock
# with the perf capture wrapped around them.
#
# NB: keep every quote in here a double quote -- this is a single-quoted bash
# string, so an escaped \" would reach Python verbatim and fail to parse,
# silently producing an EMPTY stamped log.
stamp_py='
import re, sys, time
ansi = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
t0 = time.monotonic()
for line in sys.stdin:
    clean = ansi.sub("", line)
    sys.stdout.write(f"[{time.monotonic()-t0:9.3f}s] {clean}")
    sys.stdout.flush()
'

assert_stamper_starts() {
    local out_dir="$1"
    # If the stamper cannot start, its pipe closes, the capture takes a SIGPIPE
    # mid-run, and perf.data is silently truncated to an unreadable stub -- the
    # whole session lost, discovered only at report time.
    if ! printf '' | python3 -c "$stamp_py" >/dev/null 2>"$out_dir/stamper-check.stderr"; then
        log "log stamper failed to start; see $out_dir/stamper-check.stderr"
        fail "refusing to record a capture that would be truncated by a dead stamper"
    fi
    rm -f "$out_dir/stamper-check.stderr"
}

# Launch the game under whatever capture wrapper the mode built, with the Tracy
# capture running alongside and both streams stamped. Always writes the status
# file, always finalizes Tracy -- an interrupted or crashed game must still
# leave usable artifacts behind.
run_instrumented_launch() {
    local out_dir="$1" status_file="$2"; shift 2
    assert_stamper_starts "$out_dir"
    echo "$(quote_cmd "$@")" > "${status_file%.status}.command.txt"

    local tracy_pid=""
    tracy_pid="$(start_tracy_capture "$out_dir")"

    local heartbeat_pid=""
    ( elapsed=0; while sleep 5; do elapsed=$((elapsed + 5)); log "capturing... ${elapsed}s elapsed ($(describe_window))"; done ) &
    heartbeat_pid=$!
    # A handler (not `trap '' INT`) is deliberate: ignoring INT would be
    # inherited by perf as SIG_IGN, so Ctrl-C would no longer stop the capture.
    # With a handler installed, perf still takes the interrupt and flushes
    # perf.data, and this script survives to write the reports.
    trap 'log "interrupt received; ending capture and writing reports"' INT
    set +e
    "$@" \
        > >(python3 -u -c "$stamp_py" > "$out_dir/game-stdout-stamped.txt") \
        2> >(python3 -u -c "$stamp_py" | tee "$out_dir/game-stderr-stamped.txt" >&2)
    local status=$?
    set -e
    trap - INT
    kill "$heartbeat_pid" 2>/dev/null || true
    wait "$heartbeat_pid" 2>/dev/null || true
    echo "$status" > "$status_file"
    if [[ "$status" -ne 0 && "$status" -ne 124 && "$status" -ne 130 ]]; then
        log "the game exited with status $status; finalizing the artifacts collected so far"
    fi
    finish_tracy_capture "$out_dir" "$tracy_pid"
}

prepare_launch_mode() {
    local out_dir="$1" local_mode="$2"
    write_metadata "$out_dir" "$local_mode"
    run_warm_build_if_needed "$out_dir" "$local_mode" || true
}

run_perf_record() {
    local local_mode="$1" out_dir="$2"
    require_tool perf
    ensure_perf_kernel_level
    ensure_kernel_symbol_visibility
    prepare_launch_mode "$out_dir" "$local_mode"
    local record_argv=() item
    while IFS= read -r -d '' item; do record_argv+=("$item"); done < <(perf_record_argv "$out_dir/perf.data")
    if [[ "$local_mode" == "perf-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; record_attach_target "$out_dir" "$target_pid"
        log "recording perf on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/perf-record.status" \
            "${record_argv[@]}" -p "$target_pid" -- sleep "$duration"
    else
        local capture_argv=()
        while IFS= read -r -d '' item; do capture_argv+=("$item"); done \
            < <(bounded_argv "${record_argv[@]}" -- "${run_cmd[@]}")
        log "launching under perf ($(describe_window)): $(quote_cmd "${run_cmd[@]}")"
        run_instrumented_launch "$out_dir" "$out_dir/perf-record.status" "${capture_argv[@]}"
    fi
    write_perf_reports "$out_dir"
}

run_perf_stat() {
    local local_mode="$1" out_dir="$2"
    require_tool perf
    ensure_perf_kernel_level
    ensure_kernel_symbol_visibility
    prepare_launch_mode "$out_dir" "$local_mode"
    # `-o` keeps perf's interval table off stderr, which belongs to the game's
    # census rows.
    if [[ "$local_mode" == "stat-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; record_attach_target "$out_dir" "$target_pid"
        log "recording perf stat on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/perf-stat.status" \
            perf stat -o "$out_dir/perf-stat-interval.txt" -p "$target_pid" -I "$interval_ms" -e "$perf_events" -- sleep "$duration"
    else
        log "launching under perf stat for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_instrumented_launch "$out_dir" "$out_dir/perf-stat.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            perf stat -o "$out_dir/perf-stat-interval.txt" -I "$interval_ms" -e "$perf_events" -- "${run_cmd[@]}"
    fi
}

run_asset_trace() {
    local local_mode="$1" out_dir="$2"
    require_tool strace; require_tool python3
    prepare_launch_mode "$out_dir" "$local_mode"
    if [[ "$local_mode" == "asset-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; record_attach_target "$out_dir" "$target_pid"
        log "recording strace asset opens on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/strace.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -p "$target_pid" -o "$out_dir/strace-assets.txt"
    else
        log "launching under strace for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_instrumented_launch "$out_dir" "$out_dir/strace.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -o "$out_dir/strace-assets.txt" -- "${run_cmd[@]}"
    fi
    write_asset_summary "$out_dir"
}

write_timeline_reports() {
    # NB: split locals — in `local a="$1" b="$a/x"`, $a expands from the OUTER
    # scope, not the local being declared on the same line.
    local out_dir="$1"; local data_file="$out_dir/perf.data"
    if [[ ! -s "$data_file" ]]; then log "perf.data missing or empty; skipping timeline reports"; return 0; fi
    # Whole-run reports first, then one flat report per equal time slice of the
    # capture (perf's a%-b% --time filter operates on trace-relative time).
    write_perf_reports "$out_dir"
    mkdir -p "$out_dir/perf_windows"
    local i lo hi label
    for (( i = 0; i < timeline_chunks; i++ )); do
        lo=$(( i * 100 / timeline_chunks ))
        hi=$(( (i + 1) * 100 / timeline_chunks ))
        label="$(printf 'perf_windows/chunk-%02d' "$i")"
        run_timed_report "$out_dir" "$label" \
            perf report -i "$data_file" --stdio --sort comm,dso,symbol --call-graph none \
            --percent-limit 0.5 --no-inline --no-source --time "${lo}%-${hi}%"
    done
}

run_timeline() {
    local local_mode="$1" out_dir="$2"
    require_tool perf; require_tool python3
    ensure_perf_kernel_level
    ensure_kernel_symbol_visibility
    prepare_launch_mode "$out_dir" "$local_mode"
    local capture_argv=() item
    while IFS= read -r -d '' item; do capture_argv+=("$item"); done < <(
        record_argv=()
        while IFS= read -r -d '' item; do record_argv+=("$item"); done < <(perf_record_argv "$out_dir/perf.data")
        bounded_argv "${record_argv[@]}" -- "${run_cmd[@]}"
    )
    log "timeline capture: $(describe_window), ${timeline_chunks} chunks -- play through the phases you want profiled"
    if [[ -z "$duration" && "$headless" == "no" ]]; then
        log "quit the game (or Ctrl-C here) when you are done; reports are written after it exits"
    fi
    run_instrumented_launch "$out_dir" "$out_dir/perf-record.status" "${capture_argv[@]}"
    write_timeline_reports "$out_dir"
    python3 "$repo_root/scripts/lib/profile_timeline.py" "$out_dir" "$timeline_chunks" "$marker_regex" \
        || log "timeline summary failed; the per-window perf reports are still in the bundle"
}

run_one_mode() {
    local local_mode="$1" out_dir="$2"
    mkdir -p "$out_dir"
    case "$local_mode" in
        perf-run|perf-attach) run_perf_record "$local_mode" "$out_dir" ;;
        stat-run|stat-attach) run_perf_stat "$local_mode" "$out_dir" ;;
        asset-run|asset-attach) run_asset_trace "$local_mode" "$out_dir" ;;
        timeline-run) run_timeline "$local_mode" "$out_dir" ;;
        *) fail "unsupported mode '$local_mode'" ;;
    esac
    write_bundle_reports "$out_dir"
}

main() {
    mkdir -p "$out_base"; cd "$repo_root"
    resolve_launch_plan
    log "profiling the $build_profile build: ${plan_binary:-<unresolved>}"
    if [[ -n "$plan_features" ]]; then log "cargo features: $plan_features"; fi
    if [[ "$workload" == "smash-match" ]]; then
        log "workload: a LIVE Smash match ($smash_fighters fighters), not the character-select menu"
        if [[ "$headless" == "no" ]]; then
            if [[ "$smash_seconds" == "0" ]]; then
                log "  windowed: play the match, then quit the game to end the capture"
            else
                log "  windowed: quits itself after ${smash_seconds}s of LIVE match"
            fi
        fi
    fi
    if [[ "$headless" == "yes" ]]; then
        log "headless run: scenario $headless_scenario, $headless_ticks ticks"
        log "  GPU and render-pass measurements are not applicable to a headless run"
    fi
    local out_dir tarball
    if [[ "$mode" == "all-run" ]]; then
        out_dir="$(make_profile_dir "$mode")"; mkdir -p "$out_dir"; write_metadata "$out_dir" "$mode"
        run_one_mode perf-run "$out_dir/perf-run"
        run_one_mode stat-run "$out_dir/stat-run"
        run_one_mode asset-run "$out_dir/asset-run"
        write_bundle_reports "$out_dir"
    else
        out_dir="$(make_profile_dir "$mode")"
        run_one_mode "$mode" "$out_dir"
    fi
    tarball="$(package_dir "$out_dir")"
    if grep -q 'SOFTWARE RENDERING' "$out_dir/summary.md" 2>/dev/null; then
        log "WARNING: this run had NO GPU (software rasterizer). Most samples are the"
        log "         CPU rasterizer's JIT'd shaders, not game code. See the Renderer"
        log "         section of $out_dir/summary.md"
    fi
    printf '\n' >&2
    log "profiling bundle: file://$out_dir"
    log "read first:       file://$out_dir/summary.md"
    log "archive:          file://$tarball"
    if [[ "$record_history" == "yes" ]]; then
        # ⛔ NON-FATAL ON PURPOSE. A profiling run that succeeded must not report
        # failure because the bookkeeping step did -- the expensive artifact is
        # already on disk, and losing it to a `set -e` would be the worst
        # possible trade.
        # ⛔⛔ EXIT STATUS IS NOT ENOUGH, AND TRUSTING IT PRINTED A LIE ONCE.
        # The ingest REFUSES a bundle that never reached a frame -- correctly,
        # since a row of zeroes would read as an improvement -- and it refuses
        # by returning 0. So "the command succeeded" and "a row was recorded"
        # are different questions, and only the second one is worth announcing.
        # Count the ledger instead.
        local ledger before after
        ledger="$repo_root/dev/ambition_dev_measurements/runtime_frame_cost.jsonl"
        before="$(wc -l < "$ledger" 2>/dev/null || echo 0)"
        python3 "$repo_root/scripts/lib/profile_bundle_to_history.py" "$out_dir" >&2 || true
        after="$(wc -l < "$ledger" 2>/dev/null || echo 0)"
        if [[ "$after" -gt "$before" ]]; then
            log "ledger:           appended to dev/ambition_dev_measurements/runtime_frame_cost.jsonl"
            # ⭐ THE READABLE HALF TRAVELS WITH THE ROW. The bundle itself is
            # gigabytes and stays untracked; `summary.md` is a few KB and is the
            # thing a human or an agent actually reads, so it is copied into the
            # measurement repo's TRACKED `summaries/` under the same record id
            # the ledger row carries. ⇒ a row can always be read back, even on a
            # machine that never had the bundle.
            local summaries="$repo_root/dev/ambition_dev_measurements/summaries"
            if [[ -f "$out_dir/summary.md" ]]; then
                mkdir -p "$summaries"
                cp "$out_dir/summary.md" "$summaries/$(basename "$out_dir").md"
                log "summary:          dev/ambition_dev_measurements/summaries/$(basename "$out_dir").md (commit this)"
            fi
        else
            log "WARNING: nothing was appended to the measurement ledger (see above)."
            log "         The bundle is intact; ingest it by hand with:"
            log "         python3 scripts/lib/profile_bundle_to_history.py $out_dir"
        fi
    fi
}
main
