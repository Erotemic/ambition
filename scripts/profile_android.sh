#!/usr/bin/env bash
# Collect parseable Android profiling artifacts for Ambition.
#
# This script wraps adb/simpleperf and writes text reports under
# target/profiles/. It is intended to be a one-command capture that packages
# results for upload / analysis. It does not build or install the APK; use the
# normal Android build path first.
set -euo pipefail

original_args=("$@")
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
mode="all"
package="${ANDROID_APP_ID:-org.erotemic.ambition.sandbox}"
duration="30"
freq="99"
event="cpu-clock"
stat_events="cpu-clock,context-switches,page-faults"
out_base="${AMBITION_PROFILE_BASE:-$repo_root/target/profiles}"
profile_name=""
adb_bin="${ADB:-adb}"
serial=""
launch_mode="auto"
warmup_seconds="0"
device_perf="/data/local/tmp/ambition.simpleperf.data"
keep_device_file="no"

usage() {
    cat <<'USAGE'
Usage:
  scripts/profile_android.sh [MODE] [OPTIONS]

Modes:
  record      Record a simpleperf CPU profile, pull perf.data, and emit reports.
  stat        Run simpleperf stat and package the text output.
  gfxinfo     Reset and capture dumpsys gfxinfo for the app.
  all         Run record, stat, and gfxinfo in one output directory. Default.

Options:
  -h, --help             Show this help.
  -p, --package PKG      Android package id. Default: org.erotemic.ambition.sandbox.
  -d, --duration SEC     Capture duration in seconds. Default: 30.
  -F, --freq HZ          Sampling frequency for simpleperf record. Default: 99.
  -e, --event EVENT      simpleperf record event. Default: cpu-clock.
  --stat-events LIST     simpleperf stat events. Default: cpu-clock,context-switches,page-faults.
  -o, --out DIR          Output base directory. Default: target/profiles.
  --name NAME            Output directory name suffix. Default: MODE-UTC_TIMESTAMP.
  -s, --serial SERIAL    adb device serial.
  --launch               Always launch the app with monkey before capture.
  --no-launch            Do not launch the app; attach/profile only if already running.
  --auto-launch          Launch only if the app is not already running. Default.
  --warmup SEC           Sleep before capture after launch/check. Default: 0.
  --device-perf PATH     Device-side perf.data path. Default: /data/local/tmp/ambition.simpleperf.data.
  --keep-device-file     Do not remove the device-side perf.data at the end.

Examples:
  # One-and-done Android CPU profile + stats + frame info.
  scripts/profile_android.sh all --duration 30

  # Record only, assuming the app is already running in the slow state.
  scripts/profile_android.sh record --no-launch --duration 30

  # Force launch and wait 10 seconds before capturing.
  scripts/profile_android.sh all --launch --warmup 10

Notes:
  - This uses simpleperf --app with the cpu-clock software event by default,
    which avoids hardware counter permission failures on many devices.
  - If reports show libambition_sandbox.so[+offset] instead of Rust symbols,
    install a no-strip/profileable APK or use simpleperf symbolization later.
  - gfxinfo is collected with dumpsys and is useful for frame timing context.
USAGE
}

fail() {
    echo "profile_android.sh: $*" >&2
    exit 2
}

log() {
    printf '[profile-android] %s\n' "$*" >&2
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
        record|stat|gfxinfo|all)
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
        -p|--package)
            shift
            [[ $# -gt 0 ]] || fail "--package requires a value"
            package="$1"
            ;;
        --package=*)
            package="${1#--package=}"
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
        -e|--event)
            shift
            [[ $# -gt 0 ]] || fail "--event requires a value"
            event="$1"
            ;;
        --event=*)
            event="${1#--event=}"
            ;;
        --stat-events)
            shift
            [[ $# -gt 0 ]] || fail "--stat-events requires a value"
            stat_events="$1"
            ;;
        --stat-events=*)
            stat_events="${1#--stat-events=}"
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
        -s|--serial)
            shift
            [[ $# -gt 0 ]] || fail "--serial requires a value"
            serial="$1"
            ;;
        --serial=*)
            serial="${1#--serial=}"
            ;;
        --launch)
            launch_mode="yes"
            ;;
        --no-launch)
            launch_mode="no"
            ;;
        --auto-launch)
            launch_mode="auto"
            ;;
        --warmup)
            shift
            [[ $# -gt 0 ]] || fail "--warmup requires a value"
            [[ "$1" =~ ^[0-9]+$ ]] || fail "--warmup must be a non-negative integer"
            warmup_seconds="$1"
            ;;
        --warmup=*)
            warmup_seconds="${1#--warmup=}"
            [[ "$warmup_seconds" =~ ^[0-9]+$ ]] || fail "--warmup must be a non-negative integer"
            ;;
        --device-perf)
            shift
            [[ $# -gt 0 ]] || fail "--device-perf requires a path"
            device_perf="$1"
            ;;
        --device-perf=*)
            device_perf="${1#--device-perf=}"
            ;;
        --keep-device-file)
            keep_device_file="yes"
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

adb_cmd=("$adb_bin")
if [[ -n "$serial" ]]; then
    adb_cmd+=(-s "$serial")
fi

make_profile_dir() {
    local local_mode="$1"
    local suffix
    if [[ -n "$profile_name" ]]; then
        suffix="$profile_name"
    else
        suffix="$local_mode-$stamp"
    fi
    local dir="$out_base/android-$suffix"
    mkdir -p "$dir"
    printf '%s\n' "$dir"
}

run_logged() {
    local out_dir="$1"
    local name="$2"
    shift 2
    echo "$(quote_cmd "$@")" > "$out_dir/$name.command.txt"
    set +e
    "$@" > "$out_dir/$name.stdout" 2> "$out_dir/$name.stderr"
    local status=$?
    set -e
    echo "$status" > "$out_dir/$name.status"
    if [[ "$status" -ne 0 ]]; then
        log "$name exited with status $status: $(quote_cmd "$@")"
    fi
    return 0
}

get_pid() {
    local found
    found="$("${adb_cmd[@]}" shell pidof "$package" 2>/dev/null | tr -d '\r' | awk '{print $1}' || true)"
    printf '%s\n' "$found"
}

wait_for_pid() {
    local timeout_seconds="${1:-20}"
    local start now found
    start="$(date +%s)"
    while true; do
        found="$(get_pid)"
        if [[ -n "$found" ]]; then
            printf '%s\n' "$found"
            return 0
        fi
        now="$(date +%s)"
        if (( now - start >= timeout_seconds )); then
            return 1
        fi
        sleep 1
    done
}

prepare_app() {
    local out_dir="$1"
    local before_pid after_pid
    before_pid="$(get_pid)"
    echo "$before_pid" > "$out_dir/pid-before.txt"

    case "$launch_mode" in
        yes)
            log "launching $package"
            run_logged "$out_dir" launch-monkey "${adb_cmd[@]}" shell monkey -p "$package" 1
            ;;
        auto)
            if [[ -z "$before_pid" ]]; then
                log "$package is not running; launching"
                run_logged "$out_dir" launch-monkey "${adb_cmd[@]}" shell monkey -p "$package" 1
            fi
            ;;
        no)
            ;;
        *)
            fail "invalid launch mode '$launch_mode'"
            ;;
    esac

    after_pid="$(wait_for_pid 20 || true)"
    echo "$after_pid" > "$out_dir/pid-capture.txt"
    if [[ -z "$after_pid" ]]; then
        log "warning: no running PID found for $package; simpleperf --app may still wait for/profile the app if it starts"
    fi

    if [[ "$warmup_seconds" -gt 0 ]]; then
        log "warming up for ${warmup_seconds}s before capture"
        sleep "$warmup_seconds"
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
        echo "package=$package"
        echo "duration_seconds=$duration"
        echo "sampling_frequency_hz=$freq"
        echo "record_event=$event"
        echo "stat_events=$stat_events"
        echo "device_perf=$device_perf"
        echo "launch_mode=$launch_mode"
        echo "warmup_seconds=$warmup_seconds"
        echo "script_command=$(quote_cmd "$0" "${original_args[@]}")"
        echo "hostname=$(hostname 2>/dev/null || true)"
        echo "uname=$(uname -a 2>/dev/null || true)"
        echo "git_head=$(cd "$repo_root" && git rev-parse --short=12 HEAD 2>/dev/null || true)"
        echo "git_status_porcelain_begin"
        (cd "$repo_root" && git status --short 2>/dev/null || true)
        echo "git_status_porcelain_end"
        echo "adb_command=$(quote_cmd "${adb_cmd[@]}")"
    } > "$out_dir/metadata.txt"

    run_logged "$out_dir" adb-version "${adb_cmd[@]}" version
    run_logged "$out_dir" adb-devices "${adb_cmd[@]}" devices -l
    run_logged "$out_dir" simpleperf-version "${adb_cmd[@]}" shell simpleperf --version
    run_logged "$out_dir" getprop-summary "${adb_cmd[@]}" shell getprop
    run_logged "$out_dir" package-dumpsys "${adb_cmd[@]}" shell dumpsys package "$package"
}

try_report() {
    local out_dir="$1"
    local name="$2"
    shift 2
    local out_file="$out_dir/$name.txt"
    local err_file="$out_dir/$name.stderr"
    local status_file="$out_dir/$name.status"
    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf report -i "$device_perf" "$@")" > "$out_dir/$name.command.txt"
    set +e
    "${adb_cmd[@]}" shell simpleperf report -i "$device_perf" "$@" > "$out_file" 2> "$err_file"
    local status=$?
    set -e
    echo "$status" > "$status_file"
    return "$status"
}

write_reports() {
    local out_dir="$1"
    local data_file="$out_dir/perf.data"
    if [[ ! -s "$data_file" ]]; then
        log "perf.data missing or empty; skipping report generation"
        return 0
    fi

    run_logged "$out_dir" simpleperf-report-help "${adb_cmd[@]}" shell simpleperf report --help

    if ! try_report "$out_dir" simpleperf-report-callgraph -g --percent-limit 0.25; then
        try_report "$out_dir" simpleperf-report-callgraph -g || true
    fi
    if ! try_report "$out_dir" simpleperf-report-flat-comm-dso-symbol --sort comm,dso,symbol --percent-limit 0.25; then
        try_report "$out_dir" simpleperf-report-flat-comm-dso-symbol --sort comm,dso,symbol || true
    fi
    if ! try_report "$out_dir" simpleperf-report-flat-dso-symbol --sort dso,symbol --percent-limit 0.25; then
        try_report "$out_dir" simpleperf-report-flat-dso-symbol --sort dso,symbol || true
    fi
    if ! try_report "$out_dir" simpleperf-report-basic; then
        log "basic simpleperf report failed; see $out_dir/simpleperf-report-basic.stderr"
    fi
}

record_cpu() {
    local out_dir="$1"
    log "recording Android CPU profile for ${duration}s"
    run_logged "$out_dir" rm-old-device-perf "${adb_cmd[@]}" shell rm -f "$device_perf"

    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf record --app "$package" -e "$event" -f "$freq" -g --duration "$duration" -o "$device_perf")" > "$out_dir/simpleperf-record.command.txt"
    set +e
    "${adb_cmd[@]}" shell simpleperf record --app "$package" -e "$event" -f "$freq" -g --duration "$duration" -o "$device_perf" > "$out_dir/simpleperf-record.stdout" 2> "$out_dir/simpleperf-record.stderr"
    local record_status=$?
    set -e
    echo "$record_status" > "$out_dir/simpleperf-record.status"
    if [[ "$record_status" -ne 0 ]]; then
        log "simpleperf record exited with status $record_status"
    fi

    run_logged "$out_dir" device-perf-ls "${adb_cmd[@]}" shell ls -lh "$device_perf"

    set +e
    "${adb_cmd[@]}" pull "$device_perf" "$out_dir/perf.data" > "$out_dir/adb-pull-perf.stdout" 2> "$out_dir/adb-pull-perf.stderr"
    local pull_status=$?
    set -e
    echo "$pull_status" > "$out_dir/adb-pull-perf.status"
    if [[ "$pull_status" -ne 0 ]]; then
        log "adb pull failed with status $pull_status"
    fi

    if [[ ! -s "$out_dir/perf.data" ]]; then
        log "perf.data is missing or empty after pull"
    fi

    write_reports "$out_dir"
}

run_stat() {
    local out_dir="$1"
    log "recording Android simpleperf stat for ${duration}s"
    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e "$stat_events")" > "$out_dir/simpleperf-stat.command.txt"
    set +e
    "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e "$stat_events" > "$out_dir/simpleperf-stat.txt" 2> "$out_dir/simpleperf-stat.stderr"
    local status=$?
    set -e
    echo "$status" > "$out_dir/simpleperf-stat.status"
    if [[ "$status" -ne 0 ]]; then
        log "simpleperf stat exited with status $status; trying cpu-clock only"
        echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e cpu-clock)" > "$out_dir/simpleperf-stat-fallback.command.txt"
        set +e
        "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e cpu-clock > "$out_dir/simpleperf-stat-fallback.txt" 2> "$out_dir/simpleperf-stat-fallback.stderr"
        local fallback_status=$?
        set -e
        echo "$fallback_status" > "$out_dir/simpleperf-stat-fallback.status"
    fi
}

run_gfxinfo() {
    local out_dir="$1"
    log "capturing gfxinfo for ${duration}s"
    run_logged "$out_dir" gfxinfo-reset "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" reset
    sleep "$duration"
    run_logged "$out_dir" gfxinfo "${adb_cmd[@]}" shell dumpsys gfxinfo "$package"
    run_logged "$out_dir" gfxinfo-framestats "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" framestats
}

write_summary() {
    local out_dir="$1"
    python3 - "$out_dir" <<'PY'
import os
import re
import sys

out_dir = sys.argv[1]
summary = []
summary.append('# Android profile summary')
summary.append('')

def read(name):
    path = os.path.join(out_dir, name)
    try:
        with open(path, 'r', errors='replace') as f:
            return f.read()
    except FileNotFoundError:
        return ''

def status(name):
    txt = read(name + '.status').strip()
    return txt if txt else 'missing'

summary.append('## Status')
for name in [
    'simpleperf-record',
    'adb-pull-perf',
    'simpleperf-stat',
    'gfxinfo',
    'gfxinfo-framestats',
]:
    summary.append(f'- {name}: {status(name)}')

perf_path = os.path.join(out_dir, 'perf.data')
if os.path.exists(perf_path):
    summary.append(f'- perf.data bytes: {os.path.getsize(perf_path)}')
else:
    summary.append('- perf.data bytes: missing')

record_stdout = read('simpleperf-record.stdout')
record_stderr = read('simpleperf-record.stderr')
for label, text in [('record stdout', record_stdout), ('record stderr', record_stderr)]:
    interesting = []
    for line in text.splitlines():
        if any(key in line.lower() for key in ['samples', 'event', 'warning', 'error', 'permission', 'failed']):
            interesting.append(line)
    if interesting:
        summary.append('')
        summary.append(f'## Interesting {label}')
        summary.extend(f'- {line}' for line in interesting[:25])

report = read('simpleperf-report-flat-comm-dso-symbol.txt') or read('simpleperf-report-basic.txt')
if report:
    summary.append('')
    summary.append('## Top simpleperf report lines')
    count = 0
    for line in report.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if re.match(r'^[0-9]+(\.[0-9]+)?%', stripped):
            summary.append('```text')
            break
    else:
        summary.append('```text')
    for line in report.splitlines():
        stripped = line.strip()
        if re.match(r'^[0-9]+(\.[0-9]+)?%', stripped):
            summary.append(line[:200])
            count += 1
            if count >= 40:
                break
    summary.append('```')

stat = read('simpleperf-stat.txt') or read('simpleperf-stat-fallback.txt')
if stat:
    summary.append('')
    summary.append('## simpleperf stat excerpt')
    summary.append('```text')
    lines = [line for line in stat.splitlines() if line.strip()]
    summary.extend(line[:200] for line in lines[:60])
    summary.append('```')

gfx = read('gfxinfo.txt')
if gfx:
    summary.append('')
    summary.append('## gfxinfo excerpt')
    summary.append('```text')
    lines = []
    for line in gfx.splitlines():
        if any(key in line for key in ['Total frames rendered', 'Janky frames', '50th percentile', '90th percentile', '95th percentile', '99th percentile', 'Number Missed Vsync', 'HISTOGRAM']):
            lines.append(line)
    summary.extend(lines[:80])
    summary.append('```')

with open(os.path.join(out_dir, 'android-profile-summary.md'), 'w') as f:
    f.write('\n'.join(summary) + '\n')
PY
}

package_dir() {
    local out_dir="$1"
    local tarball="$out_dir.tar.gz"
    if [[ "$keep_device_file" != "yes" ]]; then
        run_logged "$out_dir" rm-device-perf-final "${adb_cmd[@]}" shell rm -f "$device_perf"
    fi
    printf '%s\n' "$tarball" > "$out_dir/package-path.txt"
    tar -czf "$tarball" -C "$(dirname "$out_dir")" "$(basename "$out_dir")"
    log "wrote $tarball"
    printf '%s\n' "$tarball"
}

main() {
    require_tool "$adb_bin"
    require_tool python3

    local out_dir
    out_dir="$(make_profile_dir "$mode")"
    write_metadata "$out_dir" "$mode"
    prepare_app "$out_dir"

    case "$mode" in
        record)
            record_cpu "$out_dir"
            ;;
        stat)
            run_stat "$out_dir"
            ;;
        gfxinfo)
            run_gfxinfo "$out_dir"
            ;;
        all)
            run_logged "$out_dir" gfxinfo-reset-before-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" reset
            record_cpu "$out_dir"
            run_logged "$out_dir" gfxinfo-after-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package"
            run_logged "$out_dir" gfxinfo-framestats-after-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" framestats
            run_stat "$out_dir"
            ;;
        *)
            fail "unknown mode '$mode'"
            ;;
    esac

    write_summary "$out_dir"
    package_dir "$out_dir"
}

main
