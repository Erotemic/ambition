#!/usr/bin/env bash
# Collect parseable desktop profiling artifacts for Ambition.
#
# Design goals:
# - wrap ./run_game.sh so profiling follows the normal launch path;
# - warm-build launch modes so compilation does not pollute captures;
# - never hang forever in perf report / addr2line / perf script;
# - emit small text summaries by default, not SVGs or giant raw archives.
set -euo pipefail

original_args=("$@")
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
mode="perf-run"
duration="30"
freq="99"
interval_ms="1000"
pid=""
out_base="${AMBITION_PROFILE_BASE:-$repo_root/target/profiles}"
profile_name=""
perf_call_graph="dwarf,8192"
perf_events="task-clock,cycles,instructions,context-switches,cpu-migrations,page-faults,cache-misses"
run_args=()
warm_build="auto"
report_preset="fast"
report_timeout="45"
include_raw_data="no"
include_perf_script="no"

usage() {
    cat <<'USAGE'
Usage:
  scripts/profile_desktop.sh [MODE] [OPTIONS] [-- RUN_GAME_ARGS ...]

Modes:
  perf-run        Launch ./run_game.sh under perf record, then emit bounded text reports.
  perf-attach     Attach perf record to an already-running ambition_game process.
  stat-run        Launch ./run_game.sh under perf stat with interval output.
  stat-attach     Attach perf stat to an already-running ambition_game process.
  asset-run       Launch ./run_game.sh under strace and summarize repeated asset opens.
  asset-attach    Attach strace to an already-running ambition_game process.
  all-run         Run perf-run, stat-run, then asset-run sequentially.

  Default mode: perf-run

Options:
  -h, --help              Show this help.
  -d, --duration SEC      Capture duration in seconds. Default: 30.
  -F, --freq HZ           Sampling frequency for perf record. Default: 99.
  -I, --interval MS       perf stat interval in milliseconds. Default: 1000.
  -p, --pid PID           PID to attach to. If omitted, newest ambition_game_bin PID is used.
  -o, --out DIR           Output base directory. Default: target/profiles.
  --name NAME             Output directory name suffix. Default: MODE-UTC_TIMESTAMP.
  --events LIST           perf stat events. Default: task-clock,cycles,instructions,...
  --call-graph SPEC       perf call graph spec. Default: dwarf,8192.
  --report-preset PRESET  none, fast, or full. Default: fast.
                           fast: one flat symbol report + summary.
                           full: flat + children + self reports.
  --report-timeout SEC    Max seconds per perf report command. Default: 45.
  --include-perf-script   Also run bounded perf script and gzip it. Default: off.
  --include-raw-data      Include perf.data in the upload tarball. Default: off.
  --no-raw-data           Do not include perf.data in the tarball. Default.
  --warm-build            Build the game before launch-based captures.
  --no-warm-build         Skip the pre-profile build step for launch-based captures.
  --                      Arguments after -- are passed to ./run_game.sh for run modes.

Examples:
  scripts/profile_desktop.sh perf-run --duration 30
  scripts/profile_desktop.sh perf-run --report-preset full --duration 30
  scripts/profile_desktop.sh perf-attach --duration 30
  scripts/profile_desktop.sh asset-run --duration 30
  scripts/profile_desktop.sh stat-attach --duration 30

Notes:
  - Report generation is time-limited. If a report times out, the script packages
    the status/stderr and keeps going.
  - Raw perf.data is excluded from tarballs by default because DWARF stacks can
    produce hundreds of MB. Re-run with --include-raw-data when raw data is needed.
  - Launch modes warm-build first by default so compile time is not profiled.
  - The default DWARF stack dump is capped to 8 KiB to avoid huge perf.data files.
USAGE
}

fail() { echo "profile_desktop.sh: $*" >&2; exit 2; }
log() { printf '[profile-desktop] %s\n' "$*" >&2; }
require_tool() { command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' not found"; }
is_positive_int() { [[ "$1" =~ ^[1-9][0-9]*$ ]]; }
quote_cmd() { printf '%q ' "$@"; }

parse_mode_or_option() {
    case "$1" in
        perf-run|perf-attach|stat-run|stat-attach|asset-run|asset-attach|all-run)
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
run_cmd=("$repo_root/run_game.sh" "${run_args[@]}")

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
    for candidate in ambition_game_bin ambition_actors; do
        found="$(pgrep -n -x "$candidate" 2>/dev/null || true)"
        if [[ -n "$found" ]]; then
            printf '%s\n' "$found"
            return 0
        fi
    done
    for candidate in ambition_game_bin ambition_actors; do
        found="$(pgrep -n -f "$candidate" 2>/dev/null || true)"
        if [[ -n "$found" ]]; then
            printf '%s\n' "$found"
            return 0
        fi
    done
    fail "could not find ambition_game_bin or ambition_actors; pass --pid or use a *-run mode"
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

mode_uses_launch() { case "$1" in perf-run|stat-run|asset-run) return 0 ;; *) return 1 ;; esac; }
warm_build_is_enabled_for() {
    case "$warm_build" in
        yes) return 0 ;;
        no) return 1 ;;
        auto) mode_uses_launch "$1" ;;
        *) fail "invalid warm_build setting '$warm_build'" ;;
    esac
}

derive_cargo_build_cmd() {
    local release=0 hot_reload=0 no_default_features=0 cargo_jobs="" cargo_timings=0
    local extra_features=() arg
    while [[ $# -gt 0 ]]; do
        arg="$1"
        case "$arg" in
            -r|--release|release) release=1 ;;
            --debug|debug|dev) release=0 ;;
            --hot|--hot-reload|--dev-hot-reload|hot|hot-reload|dev-hot-reload) hot_reload=1 ;;
            --no-hot-reload) hot_reload=0 ;;
            --features) shift; [[ $# -gt 0 ]] || fail "--features requires value"; extra_features+=("$1") ;;
            --features=*) extra_features+=("${arg#--features=}") ;;
            --no-default-features) no_default_features=1 ;;
            -j|--jobs) shift; [[ $# -gt 0 ]] || fail "$arg requires value"; cargo_jobs="$1" ;;
            -j[0-9]*) cargo_jobs="${arg#-j}" ;;
            --jobs=*) cargo_jobs="${arg#--jobs=}" ;;
            --timings) cargo_timings=1 ;;
            --) break ;;
            *) ;;
        esac
        shift
    done
    local cmd=(cargo build -p ambition_app --bin ambition_game_bin)
    [[ "$no_default_features" -eq 1 ]] && cmd+=(--no-default-features)
    [[ -n "$cargo_jobs" ]] && cmd+=(--jobs "$cargo_jobs")
    [[ "$cargo_timings" -eq 1 ]] && cmd+=(--timings)
    local features=()
    [[ "$hot_reload" -eq 1 ]] && features+=(dev_hot_reload)
    local feature_list
    for feature_list in "${extra_features[@]}"; do [[ -n "$feature_list" ]] && features+=("$feature_list"); done
    if [[ "${#features[@]}" -gt 0 ]]; then local IFS=,; cmd+=(--features "${features[*]}"); fi
    [[ "$release" -eq 1 ]] && cmd+=(--release)
    printf '%s\0' "${cmd[@]}"
}

run_warm_build_if_needed() {
    local out_dir="$1" local_mode="$2"
    if ! warm_build_is_enabled_for "$local_mode"; then echo "skipped" > "$out_dir/warm-build.status"; return 0; fi
    require_tool cargo
    local build_cmd=() item
    while IFS= read -r -d '' item; do build_cmd+=("$item"); done < <(derive_cargo_build_cmd "${run_args[@]}")
    echo "$(quote_cmd "${build_cmd[@]}")" > "$out_dir/warm-build-command.txt"
    log "warm-building: $(quote_cmd "${build_cmd[@]}")"
    set +e
    (cd "$repo_root" && "${build_cmd[@]}") > >(tee "$out_dir/warm-build.stdout") 2> >(tee "$out_dir/warm-build.stderr" >&2)
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
        echo "duration_seconds=$duration"
        echo "sampling_frequency_hz=$freq"
        echo "stat_interval_ms=$interval_ms"
        echo "perf_call_graph=$perf_call_graph"
        echo "perf_events=$perf_events"
        echo "report_preset=$report_preset"
        echo "report_timeout_seconds=$report_timeout"
        echo "include_raw_data=$include_raw_data"
        echo "include_perf_script=$include_perf_script"
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
        echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || true)"
        echo "strace_version=$(strace --version 2>/dev/null | head -1 || true)"
        echo "python3_version=$(python3 --version 2>/dev/null || true)"
    } > "$out_dir/metadata.txt"
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
    run_with_tee "$stem.stdout" "$stem.stderr" "$@"
    local status=$?
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
    local out_dir="$1" data_file="$out_dir/perf.data"
    if [[ ! -s "$data_file" ]]; then log "perf.data missing or empty; skipping perf reports"; return 0; fi
    stat -c '%s' "$data_file" > "$out_dir/perf.data.bytes" 2>/dev/null || true
    [[ "$report_preset" == "none" ]] && return 0

    # Fast, robust, no callgraph report first. This is the one most likely to finish.
    run_timed_report "$out_dir" perf-report-flat-fast \
        perf report -i "$data_file" --stdio --sort comm,dso,symbol --call-graph none --percent-limit 0.25 --no-inline --no-source

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
    local out_dir="$1" trace_file="$out_dir/strace-assets.txt"
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
with open(os.path.join(out_dir, 'image-open-counts.txt'), 'w') as f:
    for path, count in image_counts.most_common(): f.write(f'{count:6d} {path}\n')
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

write_summary() {
    local out_dir="$1"
    python3 - "$out_dir" <<'PY'
import os, re, sys
out = sys.argv[1]
def read(name):
    try:
        with open(os.path.join(out, name), 'r', errors='replace') as f: return f.read()
    except FileNotFoundError: return ''
def status(name):
    txt = read(name + '.status').strip(); return txt or 'missing'
lines = ['# Desktop profile summary', '']
lines.append('## Status')
for name in ['warm-build','perf-record','perf-report-flat-fast','perf-report-self-symbols','perf-report-children-symbols','perf-script','perf-stat','strace']:
    p = os.path.join(out, name + '.status')
    if os.path.exists(p): lines.append(f'- {name}: {status(name)}')
perf = os.path.join(out, 'perf.data')
if os.path.exists(perf): lines.append(f'- perf.data bytes: {os.path.getsize(perf)}')
report = read('perf-report-flat-fast.txt') or read('perf-report-self-symbols.txt') or read('perf-report-children-symbols.txt')
if report:
    lines += ['', '## Top perf report lines', '```text']
    n = 0
    for line in report.splitlines():
        if re.match(r'\s*[0-9]+(\.[0-9]+)?%', line):
            lines.append(line[:220]); n += 1
            if n >= 60: break
    lines.append('```')
asset = read('asset-trace-summary.md')
if asset:
    lines += ['', '## Asset trace summary excerpt']
    lines.extend(asset.splitlines()[:90])
stat = read('perf-stat-interval.txt')
if stat:
    lines += ['', '## perf stat excerpt', '```text']
    lines.extend([x[:220] for x in stat.splitlines()[:80]])
    lines.append('```')
with open(os.path.join(out, 'desktop-profile-summary.md'), 'w') as f:
    f.write('\n'.join(lines) + '\n')
PY
}

package_dir() {
    local out_dir="$1" tarball="$out_dir.tar.gz" base
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

run_perf_record() {
    local local_mode="$1" out_dir="$2"
    require_tool perf
    ensure_perf_kernel_level
    write_metadata "$out_dir" "$local_mode"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "perf-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; echo "$target_pid" > "$out_dir/pid.txt"
        log "recording perf on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/perf-record.status" \
            perf record -F "$freq" -g --call-graph "$perf_call_graph" -o "$out_dir/perf.data" -p "$target_pid" -- sleep "$duration"
    else
        log "launching under perf for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_capture_command "$out_dir/perf-record.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            perf record -F "$freq" -g --call-graph "$perf_call_graph" -o "$out_dir/perf.data" -- "${run_cmd[@]}"
    fi
    write_perf_reports "$out_dir"
}

run_perf_stat() {
    local local_mode="$1" out_dir="$2"
    require_tool perf
    ensure_perf_kernel_level
    write_metadata "$out_dir" "$local_mode"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "stat-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; echo "$target_pid" > "$out_dir/pid.txt"
        log "recording perf stat on PID $target_pid for ${duration}s"
        set +e
        perf stat -p "$target_pid" -I "$interval_ms" -e "$perf_events" -- sleep "$duration" > >(tee "$out_dir/perf-stat.stdout") 2> >(tee "$out_dir/perf-stat-interval.txt" >&2)
        local status=$?
        set -e; echo "$status" > "$out_dir/perf-stat.status"
    else
        log "launching under perf stat for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        set +e
        timeout --signal=INT --kill-after=5s "${duration}s" perf stat -I "$interval_ms" -e "$perf_events" -- "${run_cmd[@]}" > >(tee "$out_dir/perf-stat.stdout") 2> >(tee "$out_dir/perf-stat-interval.txt" >&2)
        local status=$?
        set -e; echo "$status" > "$out_dir/perf-stat.status"
    fi
}

run_asset_trace() {
    local local_mode="$1" out_dir="$2"
    require_tool strace; require_tool python3
    write_metadata "$out_dir" "$local_mode"
    run_warm_build_if_needed "$out_dir" "$local_mode"
    if [[ "$local_mode" == "asset-attach" ]]; then
        local target_pid; target_pid="$(find_game_pid)"; echo "$target_pid" > "$out_dir/pid.txt"
        log "recording strace asset opens on PID $target_pid for ${duration}s"
        run_capture_command "$out_dir/strace.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -p "$target_pid" -o "$out_dir/strace-assets.txt"
    else
        log "launching under strace for ${duration}s: $(quote_cmd "${run_cmd[@]}")"
        run_capture_command "$out_dir/strace.status" \
            timeout --signal=INT --kill-after=5s "${duration}s" \
            strace -f -yy -tt -s 240 -e trace=openat,openat2,read,pread64,close -o "$out_dir/strace-assets.txt" -- "${run_cmd[@]}"
    fi
    write_asset_summary "$out_dir"
}

run_one_mode() {
    local local_mode="$1" out_dir="$2"
    mkdir -p "$out_dir"
    case "$local_mode" in
        perf-run|perf-attach) run_perf_record "$local_mode" "$out_dir" ;;
        stat-run|stat-attach) run_perf_stat "$local_mode" "$out_dir" ;;
        asset-run|asset-attach) run_asset_trace "$local_mode" "$out_dir" ;;
        *) fail "unsupported mode '$local_mode'" ;;
    esac
    write_summary "$out_dir"
}

main() {
    mkdir -p "$out_base"; cd "$repo_root"
    if [[ "$mode" == "all-run" ]]; then
        local out_dir; out_dir="$(make_profile_dir "$mode")"; mkdir -p "$out_dir"; write_metadata "$out_dir" "$mode"
        run_one_mode perf-run "$out_dir/perf-run"
        run_one_mode stat-run "$out_dir/stat-run"
        run_one_mode asset-run "$out_dir/asset-run"
        write_summary "$out_dir"
        package_dir "$out_dir"
    else
        local out_dir; out_dir="$(make_profile_dir "$mode")"
        run_one_mode "$mode" "$out_dir"
        package_dir "$out_dir"
    fi
}
main
