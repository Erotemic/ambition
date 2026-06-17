#!/usr/bin/env bash
# Collect parseable Android profiling artifacts for Ambition.
#
# This script is intended to be one-and-done:
# - optionally build/install a no-strip profiling APK;
# - launch or attach to the app;
# - record simpleperf CPU data with permission-friendly defaults;
# - generate bounded device-side and optional host-side reports;
# - collect gfxinfo/stat context;
# - package a small upload tarball.
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
device_heap_trace="/data/misc/perfetto-traces/ambition.heap.${stamp}.perfetto-trace"
keep_device_file="no"
report_timeout="45"
profile_build="no"
build_rust_mode="android-profile"
build_extra_args=()
symfs_dirs=()
include_symbol_candidates="no"
include_symbol_files="no"
symfs_auto="yes"
include_raw_data="yes"
include_large_reports="no"
large_report_limit_mb="64"
callgraph_head_lines="40000"
symbol_lib_name="libambition_app.so"
heap_sampling_interval_bytes="4096"
heap_dump_interval_ms="5000"

usage() {
    cat <<'USAGE'
Usage:
  scripts/profile_android.sh [MODE] [OPTIONS]

Modes:
  record      Record a simpleperf CPU profile, pull perf.data, and emit reports.
  heap        Record a Perfetto/heapprofd native heap allocation trace.
  stat        Run simpleperf stat and package the text output.
  gfxinfo     Reset and capture dumpsys gfxinfo for the app.
  prepare     Build/install a symbol-friendly profiling APK and exit.
  all         Run record, stat, and gfxinfo-style reports in one output directory. Default.

Common one-command workflows:
  # Symbol-friendly build/install, then profile. This is the default recommendation.
  scripts/profile_android.sh all --profile-build --duration 30

  # App is already running in the slow state; do not relaunch.
  scripts/profile_android.sh all --no-launch --duration 30

  # App is already running in the slow state; capture allocation callstacks.
  scripts/profile_android.sh heap --no-launch --duration 30

  # Attach to the currently running app without rebuilding, reinstalling, or relaunching.
  scripts/profile_android.sh heap --no-launch --duration 60 --heap-sampling-interval 4096

  # Build/install symbols first, then manually open the menu and attach.
  scripts/profile_android.sh prepare --profile-build
  scripts/profile_android.sh all --no-launch --duration 30

  # One command after reinstall: launch, wait while you open the bad menu, then record.
  scripts/profile_android.sh all --profile-build --launch --warmup 20 --duration 30

Options:
  -h, --help              Show this help.
  -p, --package PKG       Android package id. Default: org.erotemic.ambition.sandbox.
  -d, --duration SEC      Capture duration in seconds. Default: 30.
  -F, --freq HZ           Sampling frequency for simpleperf record. Default: 99.
  -e, --event EVENT       simpleperf record event. Default: cpu-clock.
  --stat-events LIST      simpleperf stat events. Default: cpu-clock,context-switches,page-faults.
  -o, --out DIR           Output base directory. Default: target/profiles.
  --name NAME             Output directory name suffix. Default: MODE-UTC_TIMESTAMP.
  -s, --serial SERIAL     adb device serial.
  --launch                Always launch the app with monkey before capture.
  --no-launch             Do not launch the app; profile only if already running.
  --auto-launch           Launch only if the app is not already running. Default.
  --warmup SEC            Sleep before capture after launch/check. Default: 0.
  --device-perf PATH      Device-side perf.data path. Default: /data/local/tmp/ambition.simpleperf.data.
  --device-heap-trace PATH
                           Device-side Perfetto heap trace path.
                           Default: /data/misc/perfetto-traces/ambition.heap.${stamp}.perfetto-trace.
  --keep-device-file      Do not remove the device-side perf.data at the end.
  --report-timeout SEC    Max seconds per report command. Default: 45.

Symbol/profile-build options:
  --profile-build         Run ./build_for_android.sh before capture with --no-strip --fresh-install --no-logs.
  --build-profile NAME    With --profile-build, use a Cargo profile. Default: android-profile.
  --build-release         With --profile-build, build Rust release native library.
  --build-debug           With --profile-build, build Rust dev native library.
  --build-extra ARG       Extra argument passed to ./build_for_android.sh. Repeatable.
  --symfs DIR             Add host simpleperf --symfs DIR. Repeatable.
  --no-auto-symfs         Do not build an output symfs mirror for host simpleperf.
  --include-symbol-candidates
                           Copy discovered libambition_app.so candidates into output.
  --include-symbol-files  Include heavy .so/symfs symbol files in the upload tarball.
                           By default, symbols stay in the local output dir only.

Heap profiling options:
  --heap-sampling-interval BYTES
                           Native heap sampling interval for heapprofd. Default: 4096.
                           Increase, e.g. 16384, if heapprofd reports buffer overruns.
  --heap-dump-interval-ms MS
                           Continuous heap dump interval. Default: 5000.
                           Set 0 to keep only the final dump.

Packaging options:
  --include-raw-data      Include perf.data in upload tarball. Default for Android.
  --no-raw-data           Exclude perf.data from upload tarball.
  --include-large-reports Include full callgraph reports even when very large.
                           By default, huge callgraphs stay local and a .head.txt excerpt is packaged.
  --large-report-limit-mb MB
                           Size threshold for excluding huge reports from upload tarball. Default: 64.
  --callgraph-head-lines N
                           Lines to copy into .head.txt excerpts for huge callgraph reports. Default: 40000.

Notes:
  - simpleperf uses --app with cpu-clock by default to avoid hardware counter permission failures.
  - heap mode writes the device trace under /data/misc/perfetto-traces and
    copies it back as heap.perfetto-trace. Open it at https://ui.perfetto.dev,
    click the Native heap profile track, and switch between Total Malloc Size
    and Total Malloc Count to find allocation churn.
  - --profile-build installs a symbol-friendly APK and therefore restarts/kills the app.
  - Do not combine --profile-build with --no-launch. To profile the current
    running app state, first run prepare --profile-build if needed, navigate on
    the phone, then run heap/all with --no-launch and without --profile-build.
  - --profile-build also forces an ELF Build ID on the aarch64 app library so
    Perfetto/simpleperf can match captured mappings to local symbols.
  - If device reports still show libambition_app.so[+offset], install with --profile-build
    and ensure host simpleperf is available on PATH, or pass --symfs to the unstripped build tree.

Jon's local Android tool paths, because future-you will forget:
  export PATH="/home/joncrall/Android/Sdk/ndk/27.2.12479018/simpleperf/bin/linux/x86_64:$PATH"
  /home/joncrall/Android/Sdk/ndk/27.2.12479018/simpleperf/bin/linux/x86_64/simpleperf
  /data/tmp/shitspotter-app-toolchain/android-sdk/ndk/26.3.11579264/simpleperf/bin/linux/x86_64/simpleperf
  Do not use the bin/android/* simpleperf binaries on the host; those run on devices.
USAGE
}

fail() { echo "profile_android.sh: $*" >&2; exit 2; }
log() { printf '[profile-android] %s\n' "$*" >&2; }
require_tool() { command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' not found"; }
is_positive_int() { [[ "$1" =~ ^[1-9][0-9]*$ ]]; }
is_nonnegative_int() { [[ "$1" =~ ^[0-9]+$ ]]; }
quote_cmd() { printf '%q ' "$@"; }

parse_mode_or_option() {
    case "$1" in record|heap|stat|gfxinfo|prepare|all) mode="$1"; return 0 ;; *) return 1 ;; esac
}

while [[ $# -gt 0 ]]; do
    if parse_mode_or_option "$1"; then shift; continue; fi
    case "$1" in
        -h|--help) usage; exit 0 ;;
        -p|--package) shift; [[ $# -gt 0 ]] || fail "--package requires a value"; package="$1" ;;
        --package=*) package="${1#--package=}" ;;
        -d|--duration) shift; [[ $# -gt 0 ]] || fail "--duration requires a value"; is_positive_int "$1" || fail "--duration must be positive"; duration="$1" ;;
        --duration=*) duration="${1#--duration=}"; is_positive_int "$duration" || fail "--duration must be positive" ;;
        -F|--freq) shift; [[ $# -gt 0 ]] || fail "--freq requires a value"; is_positive_int "$1" || fail "--freq must be positive"; freq="$1" ;;
        --freq=*) freq="${1#--freq=}"; is_positive_int "$freq" || fail "--freq must be positive" ;;
        -e|--event) shift; [[ $# -gt 0 ]] || fail "--event requires a value"; event="$1" ;;
        --event=*) event="${1#--event=}" ;;
        --stat-events) shift; [[ $# -gt 0 ]] || fail "--stat-events requires a value"; stat_events="$1" ;;
        --stat-events=*) stat_events="${1#--stat-events=}" ;;
        -o|--out) shift; [[ $# -gt 0 ]] || fail "--out requires a directory"; out_base="$1" ;;
        --out=*) out_base="${1#--out=}" ;;
        --name) shift; [[ $# -gt 0 ]] || fail "--name requires a value"; profile_name="$1" ;;
        --name=*) profile_name="${1#--name=}" ;;
        -s|--serial) shift; [[ $# -gt 0 ]] || fail "--serial requires a value"; serial="$1" ;;
        --serial=*) serial="${1#--serial=}" ;;
        --launch) launch_mode="yes" ;;
        --no-launch) launch_mode="no" ;;
        --auto-launch) launch_mode="auto" ;;
        --warmup) shift; [[ $# -gt 0 ]] || fail "--warmup requires a value"; is_nonnegative_int "$1" || fail "--warmup must be non-negative"; warmup_seconds="$1" ;;
        --warmup=*) warmup_seconds="${1#--warmup=}"; is_nonnegative_int "$warmup_seconds" || fail "--warmup must be non-negative" ;;
        --device-perf) shift; [[ $# -gt 0 ]] || fail "--device-perf requires a path"; device_perf="$1" ;;
        --device-perf=*) device_perf="${1#--device-perf=}" ;;
        --device-heap-trace) shift; [[ $# -gt 0 ]] || fail "--device-heap-trace requires a path"; device_heap_trace="$1" ;;
        --device-heap-trace=*) device_heap_trace="${1#--device-heap-trace=}" ;;
        --keep-device-file) keep_device_file="yes" ;;
        --report-timeout) shift; [[ $# -gt 0 ]] || fail "--report-timeout requires a value"; is_positive_int "$1" || fail "--report-timeout must be positive"; report_timeout="$1" ;;
        --report-timeout=*) report_timeout="${1#--report-timeout=}"; is_positive_int "$report_timeout" || fail "--report-timeout must be positive" ;;
        --profile-build) profile_build="yes" ;;
        --build-profile) shift; [[ $# -gt 0 ]] || fail "--build-profile requires a value"; build_rust_mode="$1" ;;
        --build-profile=*) build_rust_mode="${1#--build-profile=}" ;;
        --build-release) build_rust_mode="release" ;;
        --build-debug) build_rust_mode="debug" ;;
        --build-extra) shift; [[ $# -gt 0 ]] || fail "--build-extra requires a value"; build_extra_args+=("$1") ;;
        --build-extra=*) build_extra_args+=("${1#--build-extra=}") ;;
        --symfs) shift; [[ $# -gt 0 ]] || fail "--symfs requires a directory"; symfs_dirs+=("$1") ;;
        --symfs=*) symfs_dirs+=("${1#--symfs=}") ;;
        --no-auto-symfs) symfs_auto="no" ;;
        --include-symbol-candidates) include_symbol_candidates="yes" ;;
        --include-symbol-files) include_symbol_files="yes" ;;
        --include-raw-data) include_raw_data="yes" ;;
        --no-raw-data) include_raw_data="no" ;;
        --include-large-reports) include_large_reports="yes" ;;
        --large-report-limit-mb) shift; [[ $# -gt 0 ]] || fail "--large-report-limit-mb requires a value"; is_positive_int "$1" || fail "--large-report-limit-mb must be positive"; large_report_limit_mb="$1" ;;
        --large-report-limit-mb=*) large_report_limit_mb="${1#--large-report-limit-mb=}"; is_positive_int "$large_report_limit_mb" || fail "--large-report-limit-mb must be positive" ;;
        --callgraph-head-lines) shift; [[ $# -gt 0 ]] || fail "--callgraph-head-lines requires a value"; is_positive_int "$1" || fail "--callgraph-head-lines must be positive"; callgraph_head_lines="$1" ;;
        --callgraph-head-lines=*) callgraph_head_lines="${1#--callgraph-head-lines=}"; is_positive_int "$callgraph_head_lines" || fail "--callgraph-head-lines must be positive" ;;
        --heap-sampling-interval) shift; [[ $# -gt 0 ]] || fail "--heap-sampling-interval requires a value"; is_positive_int "$1" || fail "--heap-sampling-interval must be positive"; heap_sampling_interval_bytes="$1" ;;
        --heap-sampling-interval=*) heap_sampling_interval_bytes="${1#--heap-sampling-interval=}"; is_positive_int "$heap_sampling_interval_bytes" || fail "--heap-sampling-interval must be positive" ;;
        --heap-dump-interval-ms) shift; [[ $# -gt 0 ]] || fail "--heap-dump-interval-ms requires a value"; is_nonnegative_int "$1" || fail "--heap-dump-interval-ms must be non-negative"; heap_dump_interval_ms="$1" ;;
        --heap-dump-interval-ms=*) heap_dump_interval_ms="${1#--heap-dump-interval-ms=}"; is_nonnegative_int "$heap_dump_interval_ms" || fail "--heap-dump-interval-ms must be non-negative" ;;
        --*) fail "unknown option '$1'" ;;
        *) fail "unknown mode or option '$1'" ;;
    esac
    shift
done

adb_cmd=("$adb_bin")
if [[ -n "$serial" ]]; then adb_cmd+=(-s "$serial"); fi
if [[ "$mode" == "prepare" && "$profile_build" != "yes" ]]; then
    fail "prepare mode requires --profile-build"
fi
if [[ "$profile_build" == "yes" && "$launch_mode" == "no" && "$mode" != "prepare" ]]; then
    fail "--profile-build installs/replaces the APK, which kills the running app. To attach to the current app state, omit --profile-build: scripts/profile_android.sh $mode --no-launch --duration $duration. If symbols are not installed yet, run scripts/profile_android.sh prepare --profile-build first, navigate on the phone, then attach with --no-launch."
fi

make_profile_dir() {
    local local_mode="$1" suffix
    if [[ -n "$profile_name" ]]; then suffix="$profile_name"; else suffix="$local_mode-$stamp"; fi
    local dir="$out_base/android-$suffix"
    mkdir -p "$dir"
    printf '%s\n' "$dir"
}

run_with_tee() {
    local stdout_file="$1" stderr_file="$2"; shift 2
    set +e
    "$@" > >(tee "$stdout_file") 2> >(tee "$stderr_file" >&2)
    local status=$?
    set -e
    return "$status"
}

run_logged() {
    local out_dir="$1" name="$2"; shift 2
    echo "$(quote_cmd "$@")" > "$out_dir/$name.command.txt"
    set +e
    "$@" > "$out_dir/$name.stdout" 2> "$out_dir/$name.stderr"
    local status=$?
    set -e
    echo "$status" > "$out_dir/$name.status"
    if [[ "$status" -ne 0 ]]; then log "$name exited with status $status: $(quote_cmd "$@")"; fi
    return 0
}

run_logged_stream() {
    local out_dir="$1" name="$2"; shift 2
    echo "$(quote_cmd "$@")" > "$out_dir/$name.command.txt"
    log "running $name: $(quote_cmd "$@")"
    run_with_tee "$out_dir/$name.stdout" "$out_dir/$name.stderr" "$@"
    local status=$?
    echo "$status" > "$out_dir/$name.status"
    if [[ "$status" -ne 0 ]]; then log "$name exited with status $status: $(quote_cmd "$@")"; fi
    return 0
}

run_timed_logged() {
    local out_dir="$1" name="$2"; shift 2
    echo "$(quote_cmd timeout --kill-after=5s "${report_timeout}s" "$@")" > "$out_dir/$name.command.txt"
    log "running $name with ${report_timeout}s timeout: $(quote_cmd "$@")"
    run_with_tee "$out_dir/$name.stdout" "$out_dir/$name.stderr" timeout --kill-after=5s "${report_timeout}s" "$@"
    local status=$?
    echo "$status" > "$out_dir/$name.status"
    if [[ "$status" -eq 124 || "$status" -eq 137 ]]; then log "$name timed out after ${report_timeout}s"; elif [[ "$status" -ne 0 ]]; then log "$name exited with status $status"; fi
    return 0
}

get_pid() { "${adb_cmd[@]}" shell pidof "$package" 2>/dev/null | tr -d '\r' | awk '{print $1}' || true; }
wait_for_pid() {
    local timeout_seconds="${1:-20}" start now found
    start="$(date +%s)"
    while true; do
        found="$(get_pid)"
        if [[ -n "$found" ]]; then printf '%s\n' "$found"; return 0; fi
        now="$(date +%s)"
        (( now - start >= timeout_seconds )) && return 1
        sleep 1
    done
}

run_profile_build() {
    local out_dir="$1"
    [[ "$profile_build" == "yes" ]] || { echo "skipped" > "$out_dir/profile-build.status"; return 0; }
    if [[ "$launch_mode" == "no" && "$mode" != "prepare" ]]; then
        fail "--profile-build installs/replaces the APK, which kills the running app. Use: scripts/profile_android.sh prepare --profile-build; then open the menu; then scripts/profile_android.sh $mode --no-launch --duration $duration. Or use --profile-build --launch --warmup SEC."
    fi
    local script="$repo_root/build_for_android.sh"
    [[ -x "$script" ]] || fail "build script not found/executable: $script"
    local cmd=("$script")
    case "$build_rust_mode" in
        debug) cmd+=(--rust-debug) ;;
        release) cmd+=(--rust-release) ;;
        *) cmd+=(--cargo-profile "$build_rust_mode") ;;
    esac
    # The no-strip profile APK is large enough that Android's package manager
    # can fail a replace install while trying to keep old + new copies staged.
    cmd+=(--apk-debug --no-strip --fresh-install --no-logs)
    if [[ -n "$serial" ]]; then cmd+=(--device "$serial"); fi
    if [[ "${#build_extra_args[@]}" -gt 0 ]]; then cmd+=("${build_extra_args[@]}"); fi
    local android_rustflags="${CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS:-}"
    if [[ "$android_rustflags" != *"--build-id"* ]]; then
        android_rustflags="${android_rustflags:+$android_rustflags }-C link-arg=-Wl,--build-id=sha1"
    fi
    echo "CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS=$(printf '%q' "$android_rustflags") CARGO_PROFILE_RELEASE_STRIP=none CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO=off $(quote_cmd "${cmd[@]}")" > "$out_dir/profile-build.command.txt"
    log "building/installing symbol-friendly Android profile APK: $(quote_cmd "${cmd[@]}")"
    log "profile-build Android Rust flags: $android_rustflags"
    printf '[profile-android] profile-build: %s\n' "$(quote_cmd "${cmd[@]}")"
    set +e
    (cd "$repo_root" && \
        CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="$android_rustflags" \
        CARGO_PROFILE_RELEASE_STRIP=none \
        CARGO_PROFILE_RELEASE_DEBUG=1 \
        CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO=off \
        "${cmd[@]}") > >(tee "$out_dir/profile-build.stdout") 2> >(tee "$out_dir/profile-build.stderr" >&2)
    local status=$?
    set -e
    echo "$status" > "$out_dir/profile-build.status"
    if [[ "$status" -ne 0 ]]; then log "profile build failed; see $out_dir/profile-build.stderr"; return "$status"; fi
}

prepare_app() {
    local out_dir="$1" before_pid after_pid
    before_pid="$(get_pid)"; echo "$before_pid" > "$out_dir/pid-before-prebuild.txt"
    run_profile_build "$out_dir"
    find_symbol_candidates "$out_dir"
    prepare_auto_symfs "$out_dir"
    before_pid="$(get_pid)"; echo "$before_pid" > "$out_dir/pid-before.txt"
    case "$launch_mode" in
        yes) log "launching $package"; run_logged_stream "$out_dir" launch-monkey "${adb_cmd[@]}" shell monkey -p "$package" 1 ;;
        auto) if [[ -z "$before_pid" ]]; then log "$package is not running; launching"; run_logged_stream "$out_dir" launch-monkey "${adb_cmd[@]}" shell monkey -p "$package" 1; fi ;;
        no) ;;
        *) fail "invalid launch mode '$launch_mode'" ;;
    esac
    after_pid="$(wait_for_pid 20 || true)"; echo "$after_pid" > "$out_dir/pid-capture.txt"
    if [[ -z "$after_pid" ]]; then log "warning: no running PID found for $package"; fi
    if [[ "$warmup_seconds" -gt 0 ]]; then log "warming up for ${warmup_seconds}s"; sleep "$warmup_seconds"; fi
}

find_symbol_candidates() {
    local out_dir="$1"
    {
        echo "# $symbol_lib_name candidates"
        find "$repo_root/target" -type f -name "$symbol_lib_name" -print 2>/dev/null | sort
        find "$repo_root" -path "*/jniLibs/*/$symbol_lib_name" -type f -print 2>/dev/null | sort
    } | awk '!seen[$0]++' > "$out_dir/symbol-candidates.txt"
    if command -v file >/dev/null 2>&1; then
        while IFS= read -r path; do
            [[ -f "$path" ]] || continue
            printf '%s\n' "== $path ==" >> "$out_dir/symbol-candidates-file.txt"
            file "$path" >> "$out_dir/symbol-candidates-file.txt" 2>&1 || true
        done < "$out_dir/symbol-candidates.txt"
    fi
    if command -v readelf >/dev/null 2>&1; then
        while IFS= read -r path; do
            [[ -f "$path" ]] || continue
            printf '%s\n' "== $path ==" >> "$out_dir/symbol-candidates-buildid.txt"
            readelf -n "$path" 2>/dev/null | grep -A3 'Build ID' >> "$out_dir/symbol-candidates-buildid.txt" 2>/dev/null || true
        done < "$out_dir/symbol-candidates.txt"
    fi
    if [[ "$include_symbol_candidates" == "yes" ]]; then
        mkdir -p "$out_dir/symbol-candidates"
        local n=0 path dest
        while IFS= read -r path; do
            [[ -f "$path" ]] || continue
            n=$((n + 1)); dest="$out_dir/symbol-candidates/${symbol_lib_name%.so}.$n.so"
            cp -f "$path" "$dest" || true
            echo "$dest <- $path" >> "$out_dir/symbol-candidates/copied.txt"
        done < "$out_dir/symbol-candidates.txt"
    fi
}

best_symbol_candidate() {
    local path verdict
    # Prefer the jniLibs copy because build_for_android.sh copies the cargo-ndk output there before Gradle packaging.
    while IFS= read -r path; do
        [[ -f "$path" ]] || continue
        case "$path" in
            */app/src/main/jniLibs/*/"$symbol_lib_name") ;;
            *) continue ;;
        esac
        if command -v file >/dev/null 2>&1; then
            verdict="$(file "$path" 2>/dev/null || true)"
            [[ "$verdict" == *"not stripped"* || "$verdict" == *"with debug_info"* ]] || continue
        fi
        printf '%s
' "$path"; return 0
    done < "$out_dir/symbol-candidates.txt"
    while IFS= read -r path; do
        [[ -f "$path" ]] || continue
        if command -v file >/dev/null 2>&1; then
            verdict="$(file "$path" 2>/dev/null || true)"
            [[ "$verdict" == *"not stripped"* || "$verdict" == *"with debug_info"* ]] || continue
        fi
        printf '%s
' "$path"; return 0
    done < "$out_dir/symbol-candidates.txt"
    return 1
}

prepare_auto_symfs() {
    local out_dir="$1"
    [[ "$symfs_auto" == "yes" ]] || return 0
    [[ -s "$out_dir/symbol-candidates.txt" ]] || find_symbol_candidates "$out_dir"
    local src pkg_path app_dir dest_dir src_best
    src_best="$(best_symbol_candidate "$out_dir" || true)"
    if [[ -z "$src_best" ]]; then
        log "no unstripped $symbol_lib_name candidate found for auto symfs"
        echo "no unstripped $symbol_lib_name candidate found" > "$out_dir/auto-symfs.stderr"
        return 0
    fi
    pkg_path="$(${adb_cmd[@]} shell pm path "$package" 2>/dev/null | tr -d '
' | sed -n 's/^package://p' | head -1 || true)"
    if [[ -z "$pkg_path" ]]; then
        log "could not determine installed package path for auto symfs"
        echo "could not determine installed package path" > "$out_dir/auto-symfs.stderr"
        return 0
    fi
    app_dir="${pkg_path%/base.apk}"
    dest_dir="$out_dir/symfs$app_dir/lib/arm64"
    mkdir -p "$dest_dir"
    cp -f "$src_best" "$dest_dir/$symbol_lib_name"
    echo "$src_best -> $dest_dir/$symbol_lib_name" > "$out_dir/auto-symfs.txt"
    symfs_dirs+=("$out_dir/symfs")
}

write_metadata() {
    local out_dir="$1" local_mode="$2"
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
        echo "device_heap_trace=$device_heap_trace"
        echo "heap_sampling_interval_bytes=$heap_sampling_interval_bytes"
        echo "heap_dump_interval_ms=$heap_dump_interval_ms"
        echo "launch_mode=$launch_mode"
        echo "warmup_seconds=$warmup_seconds"
        echo "report_timeout_seconds=$report_timeout"
        echo "profile_build=$profile_build"
        echo "build_rust_mode=$build_rust_mode"
        echo "symfs_dirs=${symfs_dirs[*]:-}"
        echo "symfs_auto=$symfs_auto"
        echo "include_raw_data=$include_raw_data"
        echo "include_symbol_files=$include_symbol_files"
        echo "include_large_reports=$include_large_reports"
        echo "large_report_limit_mb=$large_report_limit_mb"
        echo "callgraph_head_lines=$callgraph_head_lines"
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
    run_logged "$out_dir" simpleperf-version-device "${adb_cmd[@]}" shell simpleperf --version
    run_logged "$out_dir" perfetto-version-device "${adb_cmd[@]}" shell perfetto --version
    run_logged "$out_dir" getprop-summary "${adb_cmd[@]}" shell getprop
    run_logged "$out_dir" package-dumpsys "${adb_cmd[@]}" shell dumpsys package "$package"
    if command -v simpleperf >/dev/null 2>&1; then
        run_logged "$out_dir" simpleperf-version-host simpleperf --version
    else
        echo "host simpleperf not found on PATH" > "$out_dir/simpleperf-version-host.stderr"
        echo "127" > "$out_dir/simpleperf-version-host.status"
    fi
    find_symbol_candidates "$out_dir"
}

try_device_report() {
    local out_dir="$1" name="$2"; shift 2
    local out_file="$out_dir/$name.txt" err_file="$out_dir/$name.stderr" status_file="$out_dir/$name.status"
    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf report -i "$device_perf" "$@")" > "$out_dir/$name.command.txt"
    log "generating $name with ${report_timeout}s timeout"
    set +e
    timeout --kill-after=5s "${report_timeout}s" "${adb_cmd[@]}" shell simpleperf report -i "$device_perf" "$@" > "$out_file" 2> >(tee "$err_file" >&2)
    local status=$?
    set -e
    echo "$status" > "$status_file"
    return "$status"
}

try_host_report() {
    local out_dir="$1" name="$2"; shift 2
    command -v simpleperf >/dev/null 2>&1 || { echo "127" > "$out_dir/$name.status"; echo "host simpleperf not found" > "$out_dir/$name.stderr"; return 0; }
    local cmd=(simpleperf report -i "$out_dir/perf.data")
    local s
    for s in "${symfs_dirs[@]}"; do cmd+=(--symfs "$s"); done
    cmd+=("$@")
    echo "$(quote_cmd "${cmd[@]}")" > "$out_dir/$name.command.txt"
    log "generating $name with ${report_timeout}s timeout"
    set +e
    timeout --kill-after=5s "${report_timeout}s" "${cmd[@]}" > "$out_dir/$name.txt" 2> >(tee "$out_dir/$name.stderr" >&2)
    local status=$?
    set -e
    echo "$status" > "$out_dir/$name.status"
    return 0
}

write_reports() {
    local out_dir="$1"
    if [[ ! -s "$out_dir/perf.data" ]]; then log "perf.data missing or empty; skipping report generation"; return 0; fi
    stat -c '%s' "$out_dir/perf.data" > "$out_dir/perf.data.bytes" 2>/dev/null || true
    run_logged "$out_dir" simpleperf-report-help-device "${adb_cmd[@]}" shell simpleperf report --help

    try_device_report "$out_dir" simpleperf-device-callgraph -g --percent-limit 0.25 || try_device_report "$out_dir" simpleperf-device-callgraph -g || true
    try_device_report "$out_dir" simpleperf-device-flat-comm-dso-symbol --sort comm,dso,symbol --percent-limit 0.25 || try_device_report "$out_dir" simpleperf-device-flat-comm-dso-symbol --sort comm,dso,symbol || true
    try_device_report "$out_dir" simpleperf-device-flat-dso-symbol --sort dso,symbol --percent-limit 0.25 || try_device_report "$out_dir" simpleperf-device-flat-dso-symbol --sort dso,symbol || true
    try_device_report "$out_dir" simpleperf-device-basic || true

    # Host-side simpleperf can use host symbols/symfs if available.
    try_host_report "$out_dir" simpleperf-host-callgraph -g --percent-limit 0.25 || true
    try_host_report "$out_dir" simpleperf-host-flat-comm-dso-symbol --sort comm,dso,symbol --percent-limit 0.25 || true
    try_host_report "$out_dir" simpleperf-host-flat-dso-symbol --sort dso,symbol --percent-limit 0.25 || true
}

record_cpu() {
    local out_dir="$1"
    log "recording Android CPU profile for ${duration}s"
    run_logged "$out_dir" rm-old-device-perf "${adb_cmd[@]}" shell rm -f "$device_perf"
    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf record --app "$package" -e "$event" -f "$freq" -g --duration "$duration" -o "$device_perf")" > "$out_dir/simpleperf-record.command.txt"
    set +e
    "${adb_cmd[@]}" shell simpleperf record --app "$package" -e "$event" -f "$freq" -g --duration "$duration" -o "$device_perf" > >(tee "$out_dir/simpleperf-record.stdout") 2> >(tee "$out_dir/simpleperf-record.stderr" >&2)
    local status=$?
    set -e
    echo "$status" > "$out_dir/simpleperf-record.status"
    if [[ "$status" -ne 0 ]]; then log "simpleperf record exited with status $status"; fi
    run_logged "$out_dir" device-perf-ls "${adb_cmd[@]}" shell ls -lh "$device_perf"
    set +e
    "${adb_cmd[@]}" pull "$device_perf" "$out_dir/perf.data" > >(tee "$out_dir/adb-pull-perf.stdout") 2> >(tee "$out_dir/adb-pull-perf.stderr" >&2)
    local pull_status=$?
    set -e
    echo "$pull_status" > "$out_dir/adb-pull-perf.status"
    if [[ ! -s "$out_dir/perf.data" ]]; then log "perf.data missing or empty after pull"; fi
    write_reports "$out_dir"
}

write_heap_config() {
    local out_dir="$1" duration_ms
    duration_ms=$((duration * 1000))
    {
        cat <<EOF
buffers {
  size_kb: 32768
  fill_policy: RING_BUFFER
}
duration_ms: $duration_ms
data_sources {
  config {
    name: "android.heapprofd"
    target_buffer: 0
    heapprofd_config {
      sampling_interval_bytes: $heap_sampling_interval_bytes
      shmem_size_bytes: 8388608
      process_cmdline: "$package"
EOF
        if [[ "$heap_dump_interval_ms" -gt 0 ]]; then
            cat <<EOF
      continuous_dump_config {
        dump_interval_ms: $heap_dump_interval_ms
      }
EOF
        fi
        cat <<'EOF'
    }
  }
}
EOF
    } > "$out_dir/heapprofd-config.textproto"
}

record_heap() {
    local out_dir="$1" status pull_status rm_status
    log "recording Android native heap profile for ${duration}s"
    write_heap_config "$out_dir"

    # Perfetto's Android daemon is constrained by SELinux and cannot reliably
    # create traces under /data/local/tmp on all devices. The default path is
    # under /data/misc/perfetto-traces, which is the Android/Perfetto-supported
    # location for command-line traces. Keep it unique to avoid stale ownership
    # conflicts across runs.
    run_logged "$out_dir" rm-old-device-heap-trace "${adb_cmd[@]}" shell rm -f "$device_heap_trace"
    rm_status="$(cat "$out_dir/rm-old-device-heap-trace.status" 2>/dev/null || true)"
    if [[ -n "$rm_status" && "$rm_status" != "0" ]]; then
        log "warning: could not remove old heap trace path before recording: $device_heap_trace"
    fi

    echo "$(quote_cmd "${adb_cmd[@]}" shell perfetto --txt -c - -o "$device_heap_trace") < $out_dir/heapprofd-config.textproto" > "$out_dir/perfetto-heap.command.txt"
    set +e
    "${adb_cmd[@]}" shell perfetto --txt -c - -o "$device_heap_trace" < "$out_dir/heapprofd-config.textproto" > >(tee "$out_dir/perfetto-heap.stdout") 2> >(tee "$out_dir/perfetto-heap.stderr" >&2)
    status=$?
    set -e
    echo "$status" > "$out_dir/perfetto-heap.status"
    if [[ "$status" -ne 0 ]]; then
        log "perfetto heap profile exited with status $status"
        run_logged "$out_dir" device-heap-trace-ls-after-failure "${adb_cmd[@]}" shell ls -lh "$device_heap_trace"
        {
            echo "Perfetto heap profiling failed with status $status."
            echo "Device trace path: $device_heap_trace"
            echo "For non-root Android devices, Perfetto traces should normally be written under /data/misc/perfetto-traces."
            echo "If you passed --device-heap-trace, try omitting it or choose a unique path under /data/misc/perfetto-traces."
            echo "See perfetto-heap.stderr for the original error."
        } > "$out_dir/heap-failure-hint.txt"
        return 0
    fi

    run_logged "$out_dir" device-heap-trace-ls "${adb_cmd[@]}" shell ls -lh "$device_heap_trace"

    # Use exec-out + cat instead of adb pull. Perfetto trace files are commonly
    # mode 0600 on user devices; the shell user can read/cat them, while direct
    # adb pull may fail or be less portable. Avoid tee here because this is a
    # binary trace file.
    echo "$(quote_cmd "${adb_cmd[@]}" exec-out cat "$device_heap_trace") > $out_dir/heap.perfetto-trace" > "$out_dir/adb-pull-heap-trace.command.txt"
    set +e
    "${adb_cmd[@]}" exec-out cat "$device_heap_trace" > "$out_dir/heap.perfetto-trace" 2> >(tee "$out_dir/adb-pull-heap-trace.stderr" >&2)
    pull_status=$?
    set -e
    echo "$pull_status" > "$out_dir/adb-pull-heap-trace.status"
    if [[ "$pull_status" -ne 0 ]]; then
        log "copying heap.perfetto-trace from device exited with status $pull_status"
    fi
    if [[ ! -s "$out_dir/heap.perfetto-trace" ]]; then
        log "heap.perfetto-trace missing or empty after device cat"
    fi
    if [[ "$keep_device_file" != "yes" ]]; then
        run_logged "$out_dir" rm-device-heap-trace "${adb_cmd[@]}" shell rm -f "$device_heap_trace"
    fi
}

run_stat() {
    local out_dir="$1"
    log "recording Android simpleperf stat for ${duration}s"
    set +e
    "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e "$stat_events" > >(tee "$out_dir/simpleperf-stat.txt") 2> >(tee "$out_dir/simpleperf-stat.stderr" >&2)
    local status=$?
    set -e
    echo "$status" > "$out_dir/simpleperf-stat.status"
    echo "$(quote_cmd "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e "$stat_events")" > "$out_dir/simpleperf-stat.command.txt"
    if [[ "$status" -ne 0 ]]; then
        log "simpleperf stat failed; trying cpu-clock only"
        set +e
        "${adb_cmd[@]}" shell simpleperf stat --app "$package" --duration "$duration" -e cpu-clock > >(tee "$out_dir/simpleperf-stat-fallback.txt") 2> >(tee "$out_dir/simpleperf-stat-fallback.stderr" >&2)
        local fstatus=$?
        set -e
        echo "$fstatus" > "$out_dir/simpleperf-stat-fallback.status"
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
import os, re, sys
out = sys.argv[1]
def read(name):
    try:
        with open(os.path.join(out, name), 'r', errors='replace') as f: return f.read()
    except FileNotFoundError: return ''
def status(name):
    txt = read(name + '.status').strip(); return txt or 'missing'
lines = ['# Android profile summary', '']
lines.append('## Status')
for name in ['profile-build','simpleperf-record','adb-pull-perf','simpleperf-device-callgraph','simpleperf-device-flat-comm-dso-symbol','simpleperf-host-callgraph','simpleperf-host-flat-comm-dso-symbol','perfetto-heap','adb-pull-heap-trace','simpleperf-stat','gfxinfo','gfxinfo-framestats','gfxinfo-after-record','gfxinfo-framestats-after-record']:
    if os.path.exists(os.path.join(out, name + '.status')): lines.append(f'- {name}: {status(name)}')
perf = os.path.join(out, 'perf.data')
if os.path.exists(perf): lines.append(f'- perf.data bytes: {os.path.getsize(perf)}')
heap_trace = os.path.join(out, 'heap.perfetto-trace')
if os.path.exists(heap_trace):
    lines.append(f'- heap.perfetto-trace bytes: {os.path.getsize(heap_trace)}')
    lines += ['', '## Heap profile trace', '```text']
    lines.append('Open heap.perfetto-trace at https://ui.perfetto.dev')
    lines.append('Click the Native heap profile track.')
    lines.append('Use Total Malloc Size for allocation bytes and Total Malloc Count for allocation churn.')
    lines.append('Use Unreleased Malloc Size/Count for retained allocations.')
    lines.append('```')
for label, fname in [('record stdout','simpleperf-record.stdout'), ('record stderr','simpleperf-record.stderr')]:
    txt = read(fname); interesting = []
    for line in txt.splitlines():
        if any(k in line.lower() for k in ['samples','event','warning','error','permission','failed']): interesting.append(line)
    if interesting:
        lines += ['', f'## Interesting {label}']
        lines.extend(f'- {x[:220]}' for x in interesting[:30])
report = read('simpleperf-host-flat-comm-dso-symbol.txt') or read('simpleperf-device-flat-comm-dso-symbol.txt') or read('simpleperf-device-basic.txt')
if report:
    lines += ['', '## Top simpleperf report lines', '```text']
    n = 0
    for line in report.splitlines():
        if re.match(r'\s*[0-9]+(\.[0-9]+)?%', line):
            lines.append(line[:240]); n += 1
            if n >= 60: break
    lines.append('```')
stat = read('simpleperf-stat.txt') or read('simpleperf-stat-fallback.txt')
if stat:
    lines += ['', '## simpleperf stat excerpt', '```text']
    lines.extend([x[:220] for x in stat.splitlines() if x.strip()][:80])
    lines.append('```')
gfx = read('gfxinfo-after-record.stdout') or read('gfxinfo.stdout')
if gfx:
    lines += ['', '## gfxinfo excerpt', '```text']
    wanted = []
    for line in gfx.splitlines():
        if any(k in line for k in ['Total frames rendered','Janky frames','50th percentile','90th percentile','95th percentile','99th percentile','Number Missed Vsync','HISTOGRAM']): wanted.append(line)
    lines.extend(wanted[:100]); lines.append('```')
sym = read('symbol-candidates.txt')
if sym:
    lines += ['', '## Symbol candidates', '```text']
    lines.extend(sym.splitlines()[:80]); lines.append('```')
with open(os.path.join(out, 'android-profile-summary.md'), 'w') as f: f.write('\n'.join(lines) + '\n')
PY
}

prepare_large_report_excerpts() {
    local out_dir="$1" limit_bytes report size excerpt
    limit_bytes=$((large_report_limit_mb * 1024 * 1024))
    : > "$out_dir/large-report-packaging.txt"
    for report in "$out_dir"/simpleperf-*-callgraph.txt; do
        [[ -f "$report" ]] || continue
        size="$(stat -c '%s' "$report" 2>/dev/null || echo 0)"
        if [[ "$size" -gt "$limit_bytes" ]]; then
            excerpt="$report.head.txt"
            if [[ ! -f "$excerpt" ]]; then
                head -n "$callgraph_head_lines" "$report" > "$excerpt" || true
            fi
            printf 'large callgraph kept local: %s bytes %s\n' "$size" "${report#$out_dir/}" >> "$out_dir/large-report-packaging.txt"
            printf 'packaged excerpt: %s\n' "${excerpt#$out_dir/}" >> "$out_dir/large-report-packaging.txt"
        fi
    done
}

package_dir() {
    local out_dir="$1" tarball="$out_dir.tar.gz" base tar_args report size limit_bytes
    base="$(basename "$out_dir")"
    if [[ "$keep_device_file" != "yes" ]]; then run_logged "$out_dir" rm-device-perf-final "${adb_cmd[@]}" shell rm -f "$device_perf"; fi
    prepare_large_report_excerpts "$out_dir"
    printf '%s\n' "$tarball" > "$out_dir/package-path.txt"

    tar_args=(-czf "$tarball")
    if [[ "$include_raw_data" != "yes" ]]; then tar_args+=(--exclude='*/perf.data'); fi
    if [[ "$include_symbol_files" != "yes" ]]; then
        tar_args+=(--exclude='*/symfs/*' --exclude='*/symbol-candidates/*.so' --exclude='*/libambition_app*.so' --exclude='*.so')
    fi
    if [[ "$include_large_reports" != "yes" ]]; then
        limit_bytes=$((large_report_limit_mb * 1024 * 1024))
        for report in "$out_dir"/simpleperf-*-callgraph.txt; do
            [[ -f "$report" ]] || continue
            size="$(stat -c '%s' "$report" 2>/dev/null || echo 0)"
            if [[ "$size" -gt "$limit_bytes" ]]; then
                tar_args+=(--exclude="$base/${report#$out_dir/}")
            fi
        done
    fi
    tar_args+=(-C "$(dirname "$out_dir")" "$base")
    tar "${tar_args[@]}"
    log "wrote $tarball"
    printf '%s\n' "$tarball"
}

main() {
    require_tool "$adb_bin"; require_tool python3
    local out_dir; out_dir="$(make_profile_dir "$mode")"
    write_metadata "$out_dir" "$mode"
    prepare_app "$out_dir"
    case "$mode" in
        prepare) ;;
        record) record_cpu "$out_dir" ;;
        heap) record_heap "$out_dir" ;;
        stat) run_stat "$out_dir" ;;
        gfxinfo) run_gfxinfo "$out_dir" ;;
        all)
            run_logged "$out_dir" gfxinfo-reset-before-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" reset
            record_cpu "$out_dir"
            run_logged "$out_dir" gfxinfo-after-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package"
            run_logged "$out_dir" gfxinfo-framestats-after-record "${adb_cmd[@]}" shell dumpsys gfxinfo "$package" framestats
            run_stat "$out_dir"
            ;;
        *) fail "unknown mode '$mode'" ;;
    esac
    write_summary "$out_dir"
    package_dir "$out_dir"
}
main
