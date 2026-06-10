#!/usr/bin/env bash
# Collect parseable desktop profiling artifacts for Ambition.
#
# This script intentionally wraps the existing ./run_game.sh entrypoint rather
# than changing game code. It writes text reports under target/profiles/ and
# packages the result as a .tar.gz suitable for upload / LLM analysis.
set -euo pipefail

original_args=("$@")
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
mode="perf-attach"
duration="30"
freq="99"
interval_ms="1000"
pid=""
out_base="${AMBITION_PROFILE_BASE:-$repo_root/target/profiles}"
profile_name=""
perf_call_graph="dwarf,65528"
perf_events="task-clock,cycles,instructions,context-switches,cpu-migrations,page-faults,cache-misses"
run_args=()
warm_build="auto"

usage() {
    cat <<'USAGE'
Usage:
  scripts/profile_desktop.sh [MODE] [OPTIONS] [-- RUN_GAME_ARGS ...]

Modes:
  perf-run        Launch ./run_game.sh under perf record, then emit text reports.
  perf-attach     Attach perf record to an already-running ambition_sandbox process.
  stat-run        Launch ./run_game.sh under perf stat with interval output.
  stat-attach     Attach perf stat to an already-running ambition_sandbox process.
  asset-run       Launch ./run_game.sh under strace and summarize repeated asset opens.
  asset-attach    Attach strace to an already-running ambition_sandbox process.
  all-run         Run perf-run, stat-run, then asset-run sequentially.

Default mode: perf-attach

Options:
  -h, --help            Show this help.
  -d, --duration SEC    Capture duration in seconds. Default: 30.
  -F, --freq HZ         Sampling frequency for perf record. Default: 99.
  -I, --interval MS     perf stat interval in milliseconds. Default: 1000.
  -p, --pid PID         PID to attach to. If omitted, the newest ambition_sandbox PID is used.
  -o, --out DIR         Output base directory. Default: target/profiles.
  --name NAME           Output directory name suffix. Default: MODE-UTC_TIMESTAMP.
  --events LIST         perf stat events. Default: task-clock,cycles,instructions,...
  --call-graph SPEC     perf call graph spec. Default: dwarf,65528.
  --warm-build          Build the game before launch-based captures. Default for *-run modes.
  --no-warm-build       Skip the pre-profile build step for launch-based captures.
  --                    Arguments after -- are passed to ./run_game.sh for run modes.

Examples:
  # Attach to the currently running game and collect symbol-level perf reports.
  scripts/profile_desktop.sh perf-attach --duration 30

  # Launch the normal dev game under perf for 30 seconds.
  scripts/profile_desktop.sh perf-run --duration 30

  # Launch with release mode using the normal run_game.sh interface.
  scripts/profile_desktop.sh perf-run --duration 30 -- release

  # Trace repeated PNG/JPEG/WebP/KTX2 opens and summarize them.
  scripts/profile_desktop.sh asset-run --duration 30

  # Run three separate launch-based captures and package them together.
  scripts/profile_desktop.sh all-run --duration 30

Notes:
  - attach modes may require Linux ptrace permissions. If strace attach fails,
    either use asset-run or temporarily run: sudo sysctl kernel.yama.ptrace_scope=0
  - launch-based modes warm-build first so compilation does not pollute timed captures.
  - perf report generation is symbol/function oriented to avoid addr2line stalls.
  - This script emits parseable .txt/.tsv/.json-ish metadata, not SVG flamegraphs.
USAGE
}

fail() {
    echo "profile_desktop.sh: $*" >&2
    exit 2
}

log() {
    printf '[profile-desktop] %s\n' "$*" >&2
}

require_tool() {
    command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' not found"
}

is_positive_int() {
    [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

quote_cmd() {
    printf '%q ' "$@"
}

parse_mode_or_option() {
    case "$1" in
        perf-run|perf-attach|stat-run|stat-attach|asset-run|asset-attach|all-run)
            mode="$1"
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

while [[ $# -gt 0 ]]; do
    if parse_mode_or_option "$1"; then
        shift
        continue
    fi
    case "$1" in
        -h|--help)
            usage
            exit 0
            ;;
        -d|--duration)
            shift
            [[ $# -gt 0 ]] || fail "--duration requires a value"
            is_positive_int "$1" || fail "--duration must be a positive integer"
            duration="$1"
            ;;
        --duration=*)
            duration="${1#--duration=}"
            is_positive_int "$duration" || fail "--duration must be a positive integer"
            ;;
        -F|--freq)
            shift
            [[ $# -gt 0 ]] || fail "--freq requires a value"
            is_positive_int "$1" || fail "--freq must be a positive integer"
            freq="$1"
            ;;
        --freq=*)
            freq="${1#--freq=}"
            is_positive_int "$freq" || fail "--freq must be a positive integer"
            ;;
        -I|--interval)
            shift
            [[ $# -gt 0 ]] || fail "--interval requires a value"
            is_positive_int "$1" || fail "--interval must be a positive integer"
            interval_ms="$1"
            ;;
        --interval=*)
            interval_ms="${1#--interval=}"
            is_positive_int "$interval_ms" || fail "--interval must be a positive integer"
            ;;
        -p|--pid)
            shift
            [[ $# -gt 0 ]] || fail "--pid requires a value"
            pid="$1"
            ;;
        --pid=*)
            pid="${1#--pid=}"
            ;;
        -o|--out)
            shift
            [[ $# -gt 0 ]] || fail "--out requires a directory"
            out_base="$1"
            ;;
        --out=*)
            out_base="${1#--out=}"
            ;;
        --name)
            shift
            [[ $# -gt 0 ]] || fail "--name requires a value"
            profile_name="$1"
            ;;
        --name=*)
            profile_name="${1#--name=}"
            ;;
        --events)
            shift
            [[ $# -gt 0 ]] || fail "--events requires a comma-separated list"
            perf_events="$1"
            ;;
        --events=*)
            perf_events="${1#--events=}"
            ;;
        --call-graph)
            shift
            [[ $# -gt 0 ]] || fail "--call-graph requires a value"
            perf_call_graph="$1"
            ;;
        --call-graph=*)
            perf_call_graph="${1#--call-graph=}"
            ;;
        --warm-build)
            warm_build="yes"
            ;;
        --no-warm-build)
            warm_build="no"
            ;;
        --)
            shift
            run_args+=("$@")
            break
            ;;
        --*)
            fail "unknown option '$1'"
            ;;
        *)
            fail "unknown mode or option '$1'"
            ;;
    esac
    shift
done

run_cmd=("$repo_root/run_game.sh" "${run_args[@]}")

make_profile_dir() {
    local local_mode="$1"
    local suffix
    if [[ -n "$profile_name" ]]; then
        suffix="$profile_name"
    else
        suffix="$local_mode-$stamp"
    fi
    local dir="$out_base/desktop-$suffix"
    mkdir -p "$dir"
    printf '%s\n' "$dir"
}

find_game_pid() {
    if [[ -n "$pid" ]]; then
        printf '%s\n' "$pid"
        return 0
    fi
    local found=""
    found="$(pgrep -n -x ambition_sandbox 2>/dev/null || true)"
    if [[ -z "$found" ]]; then
        found="$(pgrep -n -f 'ambition_sandbox' 2>/dev/null || true)"
    fi
    [[ -n "$found" ]] || fail "could not find an ambition_sandbox process; pass --pid or use a *-run mode"
    printf '%s\n' "$found"
}


mode_uses_launch() {
    case "$1" in
        perf-run|stat-run|asset-run)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

warm_build_is_enabled_for() {
    local local_mode="$1"
    case "$warm_build" in
        yes)
            return 0
            ;;
        no)
            return 1
            ;;
        auto)
            mode_uses_launch "$local_mode"
            return $?
            ;;
        *)
            fail "invalid warm_build setting '$warm_build'"
            ;;
    esac
}

derive_cargo_build_cmd() {
    local release=0
    local hot_reload=0
    local no_default_features=0
    local cargo_jobs=""
    local cargo_timings=0
    local extra_features=()
    local arg

    # Mirror the build-affecting subset of run_game.sh's argument parser.
    # Unknown tokens are treated as game arguments and ignored for the build.
    # Everything after -- is also game-only and ignored.
    while [[ $# -gt 0 ]]; do
        arg="$1"
        case "$arg" in
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
            --features)
                shift
                [[ $# -gt 0 ]] || fail "--features requires a comma-separated feature list"
                extra_features+=("$1")
                ;;
            --features=*)
                extra_features+=("${arg#--features=}")
                ;;
            --no-default-features)
                no_default_features=1
                ;;
            -j|--jobs)
                shift
                [[ $# -gt 0 ]] || fail "$arg requires a job count"
                cargo_jobs="$1"
                ;;
            -j[0-9]*)
                cargo_jobs="${arg#-j}"
                ;;
            --jobs=*)
                cargo_jobs="${arg#--jobs=}"
                ;;
            --timings)
                cargo_timings=1
                ;;
            --)
                break
                ;;
            *)
                # Non-build run_game.sh aliases / game args do not affect cargo build.
                ;;
        esac
        shift
    done

    local cmd=(cargo build -p ambition_sandbox --bin ambition_sandbox)
    if [[ "$no_default_features" -eq 1 ]]; then
        cmd+=(--no-default-features)
    fi
    if [[ -n "$cargo_jobs" ]]; then
        require_positive_integer "--jobs" "$cargo_jobs"
        cmd+=(--jobs "$cargo_jobs")
    fi
    if [[ "$cargo_timings" -eq 1 ]]; then
        cmd+=(--timings)
    fi

    local features=()
    if [[ "$hot_reload" -eq 1 ]]; then
        features+=(dev_hot_reload)
    fi
    local feature_list
    for feature_list in "${extra_features[@]}"; do
        if [[ -n "$feature_list" ]]; then
            features+=("$feature_list")
        fi
    done
    if [[ "${#features[@]}" -gt 0 ]]; then
        local IFS=,
        cmd+=(--features "${features[*]}")
    fi
    if [[ "$release" -eq 1 ]]; then
        cmd+=(--release)
    fi

    printf '%s\0' "${cmd[@]}"
}

run_warm_build_if_needed() {
    local out_dir="$1"
    local local_mode="$2"
    if ! warm_build_is_enabled_for "$local_mode"; then
        echo "skipped" > "$out_dir/warm-build.status"
        return 0
    fi

    require_tool cargo
    local build_cmd=()
    while IFS= read -r -d '' item; do
        build_cmd+=("$item")
    done < <(derive_cargo_build_cmd "${run_args[@]}")

    echo "$(quote_cmd "${build_cmd[@]}")" > "$out_dir/warm-build-command.txt"
    log "warm-building before profiling: $(quote_cmd "${build_cmd[@]}")"

    set +e
    (cd "$repo_root" && "${build_cmd[@]}") > "$out_dir/warm-build.stdout" 2> "$out_dir/warm-build.stderr"
    local status=$?
    set -e
    echo "$status" > "$out_dir/warm-build.status"
    if [[ "$status" -ne 0 ]]; then
        log "warm build failed with status $status; see $out_dir/warm-build.stderr"
        return "$status"
    fi
}

write_metadata() {
    local out_dir="$1"
    local local_mode="$2"
    {
        echo "mode=$local_mode"
        echo "utc_stamp=$stamp"
        echo "repo_root=$repo_root"
        echo "output_dir=$out_dir"
        echo "duration_seconds=$duration"
        echo "sampling_frequency_hz=$freq"
        echo "stat_interval_ms=$interval_ms"
        echo "run_command=$(quote_cmd "${run_cmd[@]}")"
        echo "warm_build_setting=$warm_build"
        echo "script_command=$(quote_cmd "$0" "${original_args[@]}")"
        echo "hostname=$(hostname 2>/dev/null || true)"
        echo "uname=$(uname -a 2>/dev/null || true)"
        echo "git_head=$(cd "$repo_root" && git rev-parse --short=12 HEAD 2>/dev/null || true)"
        echo "git_status_porcelain_begin"
        (cd "$repo_root" && git status --short 2>/dev/null || true)
        echo "git_status_porcelain_end"
        echo "perf_version=$(perf --version 2>/dev/null || true)"
        echo "strace_version=$(strace --version 2>/dev/null | head -1 || true)"
        echo "python3_version=$(python3 --version 2>/dev/null || true)"
    } > "$out_dir/metadata.txt"
}

package_dir() {
    local out_dir="$1"
    local tarball="$out_dir.tar.gz"
    printf '%s\n' "$tarball" > "$out_dir/package-path.txt"
    tar -czf "$tarball" -C "$(dirname "$out_dir")" "$(basename "$out_dir")"
    log "wrote $tarball"
    printf '%s\n' "$tarball"
}

run_capture_command() {
    local status_file="$1"
    shift
    set +e
    "$@"
    local status=$?
    set -e
    echo "$status" > "$status_file"
    # timeout returns 124 when it terminates the capture after duration. That is expected.
    if [[ "$status" -ne 0 && "$status" -ne 124 && "$status" -ne 130 ]]; then
        log "command exited with status $status: $(quote_cmd "$@")"
    fi
    return 0
}

try_perf_report() {
    local data_file="$1"
    local out_file="$2"
    shift 2
    local err_file="$out_file.stderr"

    # Prefer flags that avoid source/inline addr2line work, then fall back for older perf.
    if perf report -i "$data_file" --stdio "$@" --no-inline --no-source > "$out_file" 2> "$err_file"; then
        return 0
    fi
    if perf report -i "$data_file" --stdio "$@" --no-inline > "$out_file" 2> "$err_file"; then
        return 0
    fi
    if perf report -i "$data_file" --stdio "$@" > "$out_file" 2> "$err_file"; then
        return 0
    fi
    log "perf report failed for $out_file; see $err_file"
    return 0
}

write_perf_reports() {
    local out_dir="$1"
    local data_file="$out_dir/perf.data"
    if [[ ! -s "$data_file" ]]; then
        log "perf.data missing or empty; skipping perf reports"
        return 0
    fi

    try_perf_report \
        "$data_file" \
        "$out_dir/perf-report-children-symbols.txt" \
        --children \
        --sort comm,dso,symbol \
        --call-graph graph,0.5,caller,function \
        --percent-limit 0.25

    try_perf_report \
        "$data_file" \
        "$out_dir/perf-report-self-symbols.txt" \
        --no-children \
        --sort comm,dso,symbol \
        --percent-limit 0.25

    try_perf_report \
        "$data_file" \
        "$out_dir/perf-report-flat-symbols.txt" \
        --sort comm,dso,symbol \
        --call-graph none \
        --percent-limit 0.25

    set +e
    perf script -i "$data_file" > "$out_dir/perf-script.txt" 2> "$out_dir/perf-script.stderr"
    local script_status=$?
    set -e
    echo "$script_status" > "$out_dir/perf-script.status"
    if [[ -s "$out_dir/perf-script.txt" ]]; then
        gzip -9 "$out_dir/perf-script.txt"
    fi
}

write_asset_summary() {
    local out_dir="$1"
    local trace_file="$out_dir/strace-assets.txt"
    if [[ ! -s "$trace_file" ]]; then
        log "strace output missing or empty; skipping asset summary"
        return 0
    fi

    python3 - "$trace_file" "$out_dir" <<'PY'
import os
import re
import sys
from collections import Counter

trace_path, out_dir = sys.argv[1:3]
image_exts = (".png", ".jpg", ".jpeg", ".webp", ".ktx2")
asset_exts = image_exts + (".ron", ".ldtk", ".ttf", ".otf", ".wav", ".ogg", ".flac", ".mp3")
open_pat = re.compile(r'openat(?:2)?\([^\"]*\"([^\"]+)\"')
image_counts = Counter()
asset_counts = Counter()
all_counts = Counter()

with open(trace_path, "r", errors="replace") as f:
    for line in f:
        m = open_pat.search(line)
        if not m:
            continue
        path = m.group(1)
        lower = path.lower()
        all_counts[path] += 1
        if lower.endswith(image_exts):
            image_counts[path] += 1
        if lower.endswith(asset_exts):
            asset_counts[path] += 1

def write_counts(filename, counts):
    with open(os.path.join(out_dir, filename), "w") as f:
        f.write("count\tpath\n")
        for path, count in counts.most_common():
            f.write(f"{count}\t{path}\n")

write_counts("image-open-counts.tsv", image_counts)
write_counts("asset-open-counts.tsv", asset_counts)
write_counts("all-open-counts.tsv", all_counts)

# Backward-compatible human-readable format used in the investigation.
with open(os.path.join(out_dir, "image-open-counts.txt"), "w") as f:
    for path, count in image_counts.most_common():
        f.write(f"{count:6d} {path}\n")

with open(os.path.join(out_dir, "asset-trace-summary.md"), "w") as f:
    f.write("# Asset trace summary\n\n")
    f.write(f"Trace file: `{os.path.basename(trace_path)}`\n\n")
    f.write("## Top image opens\n\n")
    if image_counts:
        f.write("```text\n")
        for path, count in image_counts.most_common(40):
            f.write(f"{count:6d} {path}\n")
        f.write("```\n")
    else:
        f.write("No image opens found.\n")
    f.write("\n## Top asset opens\n\n")
    if asset_counts:
        f.write("```text\n")
        for path, count in asset_counts.most_common(60):
            f.write(f"{count:6d} {path}\n")
        f.write("```\n")
    else:
        f.write("No asset opens found.\n")
PY
}

run_perf_record() {
    local local_mode="$1"
    local out_dir="$2"
    require_tool perf
    write_metadata "$out_dir" "$local_mode" "$@"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "perf-attach" ]]; then
        local target_pid
        target_pid="$(find_game_pid)"
        echo "$target_pid" > "$out_dir/pid.txt"
        log "recording perf on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/perf-record.status" \
            perf record -F "$freq" -g --call-graph "$perf_call_graph" -o "$out_dir/perf.data" -p "$target_pid" -- sleep "$duration"
    else
        log "launching under perf for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_capture_command "$out_dir/perf-record.status" \
            timeout --signal=INT "${duration}s" \
            perf record -F "$freq" -g --call-graph "$perf_call_graph" -o "$out_dir/perf.data" -- "${run_cmd[@]}"
    fi
    write_perf_reports "$out_dir"
}

run_perf_stat() {
    local local_mode="$1"
    local out_dir="$2"
    require_tool perf
    write_metadata "$out_dir" "$local_mode" "$@"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "stat-attach" ]]; then
        local target_pid
        target_pid="$(find_game_pid)"
        echo "$target_pid" > "$out_dir/pid.txt"
        log "recording perf stat on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/perf-stat.status" \
            perf stat -p "$target_pid" -I "$interval_ms" -e "$perf_events" -- sleep "$duration" \
            2> "$out_dir/perf-stat-interval.txt"
    else
        log "launching under perf stat for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_capture_command "$out_dir/perf-stat.status" \
            timeout --signal=INT "${duration}s" \
            perf stat -I "$interval_ms" -e "$perf_events" -- "${run_cmd[@]}" \
            2> "$out_dir/perf-stat-interval.txt"
    fi
}

run_asset_trace() {
    local local_mode="$1"
    local out_dir="$2"
    require_tool strace
    require_tool python3
    write_metadata "$out_dir" "$local_mode" "$@"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "asset-attach" ]]; then
        local target_pid
        target_pid="$(find_game_pid)"
        echo "$target_pid" > "$out_dir/pid.txt"
        log "recording strace asset opens on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/strace.status" \
            timeout --signal=INT "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -p "$target_pid" -o "$out_dir/strace-assets.txt"
    else
        log "launching under strace for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_capture_command "$out_dir/strace.status" \
            timeout --signal=INT "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -o "$out_dir/strace-assets.txt" -- "${run_cmd[@]}"
    fi
    write_asset_summary "$out_dir"
}

run_one_mode() {
    local local_mode="$1"
    local out_dir="$2"
    mkdir -p "$out_dir"
    case "$local_mode" in
        perf-run|perf-attach)
            run_perf_record "$local_mode" "$out_dir"
            ;;
        stat-run|stat-attach)
            run_perf_stat "$local_mode" "$out_dir"
            ;;
        asset-run|asset-attach)
            run_asset_trace "$local_mode" "$out_dir"
            ;;
        *)
            fail "unsupported mode '$local_mode'"
            ;;
    esac
}

main() {
    mkdir -p "$out_base"
    cd "$repo_root"

    if [[ "$mode" == "all-run" ]]; then
        local out_dir
        out_dir="$(make_profile_dir "$mode")"
        mkdir -p "$out_dir"
        write_metadata "$out_dir" "$mode" "$@"
        run_one_mode perf-run "$out_dir/perf-run"
        run_one_mode stat-run "$out_dir/stat-run"
        run_one_mode asset-run "$out_dir/asset-run"
        package_dir "$out_dir"
    else
        local out_dir
        out_dir="$(make_profile_dir "$mode")"
        run_one_mode "$mode" "$out_dir"
        package_dir "$out_dir"
    fi
}

main "$@"
