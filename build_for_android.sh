#!/usr/bin/env bash
set -euo pipefail

# Build an Android APK for the Ambition sandbox.
#
# This script follows Bevy's current Android path: build the Rust crate as an
# Android shared library with cargo-ndk, place it under a generated Gradle
# project's app/src/main/jniLibs/, copy runtime assets into app/src/main/assets/,
# then ask Gradle to assemble an installable debug APK.
#
# Default: Rust release build + Android debug APK (debug-signed, easy to adb
# install, but the native library is optimized enough to test gameplay).

usage() {
    cat <<'EOF'
Usage: ./build_for_android.sh [options]

Options:
  --install              Install the APK on an attached Android device with adb.
  --fresh-install        Uninstall the app for the current user before installing.
  --run                  Install, launch, and follow filtered logcat by default.
  --logs                 Follow filtered logcat after --run / --install.
  --no-logs              Do not follow logcat after --run.
  --log-filter FILTER    Logcat filter spec. Default: RustStdoutStderr:I event:W AndroidRuntime:E *:S
  --device SERIAL        Pass -s SERIAL to adb for install/run/logs.
  --list-emulators       List available Android Virtual Devices and exit.
  --emulator NAME        Start/use the named AVD before install/run. Defaults target ABI to x86_64.
  --create-emulator NAME Create the named AVD if missing, then start/use it.
  --emulator-api API     System-image API for --create-emulator. Default: compileSdk.
  --rust-debug           Build the Rust shared library with Cargo dev profile.
  --rust-release         Build the Rust shared library with Cargo release profile (default).
  --size-profile         Build the Rust shared library with Cargo profile android-size.
  --cargo-profile NAME   Build the Rust shared library with a named Cargo profile.
  --apk-release          Run Gradle assembleRelease instead of assembleDebug.
  --apk-debug            Run Gradle assembleDebug (default; debug-signed).
  --target ABI           Android ABI for cargo-ndk. Default: arm64-v8a.
  --min-sdk API          Android minSdk and cargo-ndk platform. Default: 31.
  --compile-sdk API      Android compileSdk. Default: highest installed, else 35.
  --target-sdk API       Android targetSdk. Default: compileSdk.
  --features LIST        Cargo features to add. Default: android.
  --use-default-features Also enable ambition_sandbox default features. Off by default for Android.
  --no-default-features  Disable default features (default for Android builds).
  --static-map           Add static_map to the Android cargo features.
  --no-static-map        Remove static_map from the Android cargo features.
  --static-sfx-bank      Embed assets/audio/sfx.bank into the native library.
  --no-static-sfx-bank   Do not embed assets/audio/sfx.bank (default is auto).
  --strip                Strip the final native library after cargo-ndk (default).
  --no-strip             Do not strip the final native library.
  --size-report          Print native/APK size diagnostics. Enabled by --size-profile.
  --clean                Delete the generated Android project before building.
  --doctor               Check tools/environment and print what would be used.
  --gradle-user-home DIR  Gradle cache dir. Default: target/android/gradle-user-home.
  -h, --help             Show this help.

Environment overrides:
  ANDROID_SDK_ROOT / ANDROID_HOME   Android SDK root.
  ANDROID_NDK_ROOT / ANDROID_NDK_HOME Android NDK root. If unset, the newest
                                      SDK side-by-side NDK is used.
  ANDROID_APP_ID                    Default: org.erotemic.ambition.sandbox
  ANDROID_APP_LABEL                 Default: Ambition Sandbox
  ANDROID_GRADLE_PLUGIN_VERSION     Default: 8.7.3
  ANDROID_GAMES_ACTIVITY_VERSION    Default: 4.0.0
  GRADLE                            Gradle command. Default: gradle
  GRADLE_USER_HOME                  Gradle cache dir. Default: target/android/gradle-user-home

Examples:
  ./build_for_android.sh
  ./build_for_android.sh --install
  ./build_for_android.sh --run --device 0123456789ABCDEF
  ./build_for_android.sh --run --emulator ambition_pixel
  ./build_for_android.sh --run --create-emulator ambition_pixel
  ./build_for_android.sh --doctor
  ./build_for_android.sh --size-profile --fresh-install
EOF
}

log() { printf '[android-build] %s\n' "$*"; }
warn() { printf '[android-build] warning: %s\n' "$*" >&2; }
fatal() { printf '[android-build] error: %s\n' "$*" >&2; exit 1; }


registered_character_sprite_filenames() {
    local registry_src="$ROOT/crates/ambition_sandbox/src/character_sprites/assets.rs"
    if [[ ! -f "$registry_src" ]]; then
        warn "character sprite registry source not found at $registry_src; skipping registered sprite presence check"
        return 0
    fi
    # The Rust-side character sprite registry is the source of truth for
    # animated character PNGs. Parse the filename literals here instead of
    # maintaining a second build-script list. This is intentionally warning-only:
    # missing art should not block Android iteration, but it should be visible.
    grep -Eo '"[A-Za-z0-9_./-]+_spritesheet\.png"' "$registry_src" \
        | tr -d '"' \
        | sort -u
}

warn_missing_registered_character_sprites() {
    local src_dir="$ROOT/crates/ambition_sandbox/assets/sprites"
    local apk_dir="$ASSETS_OUT/sprites"
    local -a missing_src=()
    local -a missing_apk=()
    local -a registered=()
    local filename

    mapfile -t registered < <(registered_character_sprite_filenames)
    if [[ "${#registered[@]}" -eq 0 ]]; then
        warn "no registered character sprites were discovered; Android sprite presence check could not verify animated NPC/enemy sheets"
        return 0
    fi

    for filename in "${registered[@]}"; do
        if [[ ! -f "$src_dir/$filename" ]]; then
            missing_src+=("sprites/$filename")
        fi
        if [[ ! -f "$apk_dir/$filename" ]]; then
            missing_apk+=("sprites/$filename")
        fi
    done

    if [[ "${#missing_src[@]}" -gt 0 ]]; then
        warn "registered character sprite PNGs missing from source assets (${#missing_src[@]}): ${missing_src[*]}"
    fi
    if [[ "${#missing_apk[@]}" -gt 0 ]]; then
        warn "registered character sprite PNGs missing from generated APK assets (${#missing_apk[@]}): ${missing_apk[*]}"
    fi
    if [[ "${#missing_src[@]}" -eq 0 && "${#missing_apk[@]}" -eq 0 ]]; then
        log "verified ${#registered[@]} registered character sprite PNGs in source assets and generated APK assets"
    fi
}

repo_root() {
    local root
    root=$(git rev-parse --show-toplevel 2>/dev/null || true)
    if [[ -z "$root" ]]; then
        fatal "run this script from inside the Ambition git checkout"
    fi
    printf '%s\n' "$root"
}

highest_installed_sdk() {
    local sdk_root=$1
    local max=""
    if [[ -d "$sdk_root/platforms" ]]; then
        local d n
        for d in "$sdk_root"/platforms/android-*; do
            [[ -d "$d" ]] || continue
            n=${d##*/android-}
            [[ "$n" =~ ^[0-9]+$ ]] || continue
            if [[ -z "$max" || "$n" -gt "$max" ]]; then
                max=$n
            fi
        done
    fi
    printf '%s\n' "$max"
}

newest_ndk_root() {
    local sdk_root=$1
    local newest=""
    if [[ -d "$sdk_root/ndk" ]]; then
        local d
        for d in "$sdk_root"/ndk/*; do
            [[ -d "$d" ]] || continue
            newest=$d
        done
    fi
    printf '%s\n' "$newest"
}

need_cmd() {
    local cmd=$1
    local hint=$2
    if ! command -v "$cmd" >/dev/null 2>&1; then
        fatal "missing '$cmd'. $hint"
    fi
}

sdk_tool() {
    local name=$1
    local candidate
    for candidate in \
        "$ANDROID_SDK_ROOT/emulator/$name" \
        "$ANDROID_SDK_ROOT/platform-tools/$name" \
        "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/$name" \
        "$ANDROID_SDK_ROOT/tools/bin/$name"
    do
        if [[ -x "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    command -v "$name" 2>/dev/null || return 1
}

need_sdk_tool() {
    local name=$1
    local hint=$2
    local path
    if path=$(sdk_tool "$name"); then
        printf '%s\n' "$path"
        return 0
    fi
    fatal "missing Android SDK tool '$name'. $hint"
}

rust_target_for_abi() {
    case "$1" in
        arm64-v8a) printf 'aarch64-linux-android\n' ;;
        armeabi-v7a) printf 'armv7-linux-androideabi\n' ;;
        x86) printf 'i686-linux-android\n' ;;
        x86_64) printf 'x86_64-linux-android\n' ;;
        *) fatal "unsupported Android ABI for rustup target mapping: $1" ;;
    esac
}

copy_tree_contents() {
    local src=$1
    local dst=$2
    mkdir -p "$dst"
    if [[ ! -d "$src" ]]; then
        warn "asset source not found: $src"
        return 0
    fi
    if command -v rsync >/dev/null 2>&1; then
        rsync -a --delete \
            --exclude '.git/' \
            --exclude '.DS_Store' \
            "$src"/ "$dst"/
    else
        rm -rf "$dst"
        mkdir -p "$dst"
        (cd "$src" && tar cf - .) | (cd "$dst" && tar xf -)
    fi
}

feature_list_has() {
    local needle=$1
    local word
    for word in $(printf '%s' "${FEATURES:-}" | tr ',' ' '); do
        [[ "$word" == "$needle" ]] && return 0
    done
    return 1
}

add_feature() {
    local name=$1
    if ! feature_list_has "$name"; then
        if [[ -z "${FEATURES:-}" ]]; then
            FEATURES="$name"
        else
            FEATURES="$FEATURES $name"
        fi
    fi
}

remove_feature() {
    local name=$1
    FEATURES=$(printf '%s' "${FEATURES:-}" | tr ',' ' ' | awk -v name="$name" '{sep=""; for (i=1;i<=NF;i++) if ($i != name) {printf "%s%s", sep, $i; sep=" "}}')
}

human_size() {
    local path=$1
    if [[ -e "$path" ]]; then
        du -h "$path" | awk '{print $1}'
    else
        printf 'missing'
    fi
}

dir_size() {
    local path=$1
    if [[ -d "$path" ]]; then
        du -sh "$path" | awk '{print $1}'
    else
        printf 'missing'
    fi
}

file_bytes() {
    local path=$1
    if [[ -e "$path" ]]; then
        wc -c < "$path" | tr -d ' '
    else
        printf '0'
    fi
}

print_apk_size_report() {
    local apk_path=$1
    [[ -f "$apk_path" ]] || return 0
    log "largest APK entries"
    if command -v unzip >/dev/null 2>&1; then
        unzip -l "$apk_path"             | awk 'NF >= 4 && $1 ~ /^[0-9]+$/ {print $1 " " $4}'             | sort -n             | tail -30             | awk '{size=$1; name=$2; for (i=3;i<=NF;i++) name=name " " $i; printf "%10.1f MiB  %s\n", size / 1048576, name}'
    else
        warn "unzip not found; skipping APK entry report"
    fi
}

avd_exists() {
    local name=$1
    local emulator_tool
    emulator_tool=$(need_sdk_tool emulator "Install emulator packages with: ./scripts/setup_android_prereqs.sh --with-emulator")
    "$emulator_tool" -list-avds | grep -Fxq "$name"
}

create_android_avd() {
    local name=$1
    local api=$2
    local sdkmanager avdmanager package
    sdkmanager=$(need_sdk_tool sdkmanager "Install Android command-line tools.")
    avdmanager=$(need_sdk_tool avdmanager "Install Android command-line tools.")
    package="system-images;android-$api;google_apis;x86_64"
    log "installing emulator system image: $package"
    "$sdkmanager" --install "$package"
    if avd_exists "$name"; then
        log "AVD already exists: $name"
    else
        log "creating AVD: $name"
        printf 'no\n' | "$avdmanager" create avd --force --name "$name" --package "$package" --device "pixel_6"
    fi
}

start_android_emulator() {
    local name=$1
    local emulator_tool
    emulator_tool=$(need_sdk_tool emulator "Install emulator packages with: ./scripts/setup_android_prereqs.sh --with-emulator")
    avd_exists "$name" || fatal "AVD not found: $name. Create it with: ./build_for_android.sh --create-emulator $name --doctor"
    log "starting Android emulator: $name"
    "$emulator_tool" -avd "$name" -netdelay none -netspeed full >/tmp/ambition-emulator-$name.log 2>&1 &
    log "emulator stdout/stderr: /tmp/ambition-emulator-$name.log"
}

select_started_emulator_serial() {
    local serial
    serial=$(adb devices | awk '/^emulator-[0-9]+[[:space:]]+device$/ {print $1; exit}')
    printf '%s\n' "$serial"
}

wait_for_android_device() {
    local adb_args=()
    if [[ -n "$DEVICE_SERIAL" ]]; then
        adb_args=(-s "$DEVICE_SERIAL")
    fi
    log "waiting for adb device"
    adb "${adb_args[@]}" wait-for-device
    local i booted
    for i in $(seq 1 90); do
        booted=$(adb "${adb_args[@]}" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
        [[ "$booted" == "1" ]] && return 0
        sleep 1
    done
    warn "device did not report sys.boot_completed=1 within timeout; continuing"
}

follow_android_logs() {
    local filter_args=()
    read -r -a filter_args <<< "$LOG_FILTER"
    log "following Android logs; press Ctrl-C to stop"
    adb "${ADB_ARGS[@]}" logcat -c || true
    adb "${ADB_ARGS[@]}" logcat -v time "${filter_args[@]}"
}

find_ndk_tool() {
    local tool=$1
    local host_tag="linux-x86_64"
    local candidate="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$host_tag/bin/$tool"
    if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
    fi
    if command -v "$tool" >/dev/null 2>&1; then
        command -v "$tool"
        return 0
    fi
    return 1
}

strip_native_library() {
    local so_path=$1
    [[ -f "$so_path" ]] || fatal "native library not found after cargo-ndk build: $so_path"
    local before after strip_tool
    before=$(human_size "$so_path")
    if strip_tool=$(find_ndk_tool llvm-strip); then
        log "stripping native library with $strip_tool"
        "$strip_tool" --strip-unneeded "$so_path"
        after=$(human_size "$so_path")
        log "native library size: $before -> $after"
    else
        warn "llvm-strip not found; native library remains unstripped at $before"
    fi
}

print_native_size_report() {
    local so_path=$1
    [[ -f "$so_path" ]] || return 0
    local size_tool
    if size_tool=$(find_ndk_tool llvm-size); then
        log "native library section-size tail from $size_tool"
        "$size_tool" -A "$so_path" | sort -k2 -n | tail -40 || true
    else
        warn "llvm-size not found; skipping native section-size report"
    fi
}

INSTALL=false
FRESH_INSTALL=false
RUN_APP=false
CLEAN=false
DOCTOR=false
TARGET_ABI="arm64-v8a"
MIN_SDK="31"
COMPILE_SDK=""
TARGET_SDK=""
RUST_PROFILE="release"
APK_BUILD_TYPE="debug"
FEATURES="android"
USE_DEFAULT_FEATURES=false
STRIP_NATIVE=true
SIZE_REPORT=false
STATIC_SFX_BANK="auto"
STATIC_SFX_BANK_PATH=""
DEVICE_SERIAL=""
GRADLE_USER_HOME_ARG=""
FOLLOW_LOGS="auto"
LOG_FILTER="RustStdoutStderr:I event:W AndroidRuntime:E *:S"
LIST_EMULATORS=false
EMULATOR_NAME=""
CREATE_EMULATOR_NAME=""
EMULATOR_API=""
TARGET_ABI_EXPLICIT=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install) INSTALL=true ;;
        --fresh-install) FRESH_INSTALL=true; INSTALL=true ;;
        --run) RUN_APP=true; INSTALL=true ;;
        --logs) FOLLOW_LOGS=true ;;
        --no-logs) FOLLOW_LOGS=false ;;
        --log-filter) shift; [[ $# -gt 0 ]] || fatal "--log-filter needs a logcat filter spec"; LOG_FILTER=$1 ;;
        --device) shift; [[ $# -gt 0 ]] || fatal "--device needs a serial"; DEVICE_SERIAL=$1 ;;
        --list-emulators) LIST_EMULATORS=true ;;
        --emulator) shift; [[ $# -gt 0 ]] || fatal "--emulator needs an AVD name"; EMULATOR_NAME=$1 ;;
        --create-emulator) shift; [[ $# -gt 0 ]] || fatal "--create-emulator needs an AVD name"; CREATE_EMULATOR_NAME=$1; EMULATOR_NAME=$1 ;;
        --emulator-api) shift; [[ $# -gt 0 ]] || fatal "--emulator-api needs an API level"; EMULATOR_API=$1 ;;
        --rust-debug) RUST_PROFILE="debug" ;;
        --rust-release) RUST_PROFILE="release" ;;
        --size-profile) RUST_PROFILE="android-size"; STRIP_NATIVE=true; SIZE_REPORT=true ;;
        --cargo-profile) shift; [[ $# -gt 0 ]] || fatal "--cargo-profile needs a profile name"; RUST_PROFILE=$1 ;;
        --apk-debug) APK_BUILD_TYPE="debug" ;;
        --apk-release) APK_BUILD_TYPE="release" ;;
        --target) shift; [[ $# -gt 0 ]] || fatal "--target needs an ABI"; TARGET_ABI=$1; TARGET_ABI_EXPLICIT=true ;;
        --min-sdk) shift; [[ $# -gt 0 ]] || fatal "--min-sdk needs an API level"; MIN_SDK=$1 ;;
        --compile-sdk) shift; [[ $# -gt 0 ]] || fatal "--compile-sdk needs an API level"; COMPILE_SDK=$1 ;;
        --target-sdk) shift; [[ $# -gt 0 ]] || fatal "--target-sdk needs an API level"; TARGET_SDK=$1 ;;
        --features) shift; [[ $# -gt 0 ]] || fatal "--features needs a comma-separated or space-separated feature list"; FEATURES=$1 ;;
        --use-default-features) USE_DEFAULT_FEATURES=true ;;
        --no-default-features) USE_DEFAULT_FEATURES=false ;;
        --static-map) add_feature static_map ;;
        --no-static-map) remove_feature static_map ;;
        --static-sfx-bank) STATIC_SFX_BANK=true ;;
        --no-static-sfx-bank) STATIC_SFX_BANK=false; remove_feature static_sfx_bank ;;
        --strip) STRIP_NATIVE=true ;;
        --no-strip) STRIP_NATIVE=false ;;
        --size-report) SIZE_REPORT=true ;;
        --clean) CLEAN=true ;;
        --doctor) DOCTOR=true ;;
        --gradle-user-home) shift; [[ $# -gt 0 ]] || fatal "--gradle-user-home needs a path"; GRADLE_USER_HOME_ARG=$1 ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

ROOT=$(repo_root)
cd "$ROOT"

APP_ID=${ANDROID_APP_ID:-org.erotemic.ambition.sandbox}
APP_LABEL=${ANDROID_APP_LABEL:-Ambition Sandbox}
AGP_VERSION=${ANDROID_GRADLE_PLUGIN_VERSION:-8.7.3}
GAMES_ACTIVITY_VERSION=${ANDROID_GAMES_ACTIVITY_VERSION:-4.0.0}
GRADLE_CMD=${GRADLE:-gradle}
if [[ -n "$GRADLE_USER_HOME_ARG" ]]; then
    export GRADLE_USER_HOME="$GRADLE_USER_HOME_ARG"
else
    export GRADLE_USER_HOME="${GRADLE_USER_HOME:-$ROOT/target/android/gradle-user-home}"
fi
mkdir -p "$GRADLE_USER_HOME" || fatal "could not create GRADLE_USER_HOME: $GRADLE_USER_HOME"
if [[ ! -w "$GRADLE_USER_HOME" ]]; then
    fatal "GRADLE_USER_HOME is not writable: $GRADLE_USER_HOME"
fi

SDK_ROOT=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
if [[ -z "$SDK_ROOT" && -d "$HOME/Android/Sdk" ]]; then
    SDK_ROOT="$HOME/Android/Sdk"
fi
[[ -n "$SDK_ROOT" ]] || fatal "ANDROID_SDK_ROOT is not set and ~/Android/Sdk was not found. Run: ./scripts/setup_android_prereqs.sh"
[[ -d "$SDK_ROOT" ]] || fatal "ANDROID_SDK_ROOT does not exist: $SDK_ROOT"
export ANDROID_SDK_ROOT="$SDK_ROOT"
export ANDROID_HOME="$SDK_ROOT"

if [[ "$LIST_EMULATORS" == true ]]; then
    emulator_tool=$(need_sdk_tool emulator "Install emulator packages with: ./scripts/setup_android_prereqs.sh --with-emulator")
    "$emulator_tool" -list-avds
    exit 0
fi

NDK_ROOT=${ANDROID_NDK_ROOT:-${ANDROID_NDK_HOME:-}}
if [[ -z "$NDK_ROOT" ]]; then
    NDK_ROOT=$(newest_ndk_root "$SDK_ROOT")
fi
[[ -n "$NDK_ROOT" ]] || fatal "Android NDK not found. Run: ./scripts/setup_android_prereqs.sh, or set ANDROID_NDK_ROOT."
[[ -d "$NDK_ROOT" ]] || fatal "ANDROID_NDK_ROOT does not exist: $NDK_ROOT"
export ANDROID_NDK_ROOT="$NDK_ROOT"
export ANDROID_NDK_HOME="$NDK_ROOT"

if [[ -z "$COMPILE_SDK" ]]; then
    COMPILE_SDK=$(highest_installed_sdk "$SDK_ROOT")
    [[ -n "$COMPILE_SDK" ]] || COMPILE_SDK="35"
fi
if [[ -z "$TARGET_SDK" ]]; then
    TARGET_SDK="$COMPILE_SDK"
fi
if [[ -z "$EMULATOR_API" ]]; then
    EMULATOR_API="$COMPILE_SDK"
fi
if [[ -n "$EMULATOR_NAME" && "$TARGET_ABI_EXPLICIT" != true ]]; then
    TARGET_ABI="x86_64"
fi
if [[ "$FOLLOW_LOGS" == "auto" ]]; then
    if [[ "$RUN_APP" == true ]]; then
        FOLLOW_LOGS=true
    else
        FOLLOW_LOGS=false
    fi
fi

need_cmd rustup "Install Rust via rustup."
need_cmd cargo "Install Rust/Cargo via rustup."
need_cmd cargo-ndk "Run: ./scripts/setup_android_prereqs.sh"
if ! command -v "$GRADLE_CMD" >/dev/null 2>&1; then
    fallback_gradle="$HOME/.local/share/gradle/gradle-8.9/bin/gradle"
    if [[ -x "$fallback_gradle" ]]; then
        GRADLE_CMD="$fallback_gradle"
    else
        fatal "missing 'gradle'. Run: ./scripts/setup_android_prereqs.sh"
    fi
fi
if [[ "$INSTALL" == true || -n "$EMULATOR_NAME" ]]; then
    need_cmd adb "Run: ./scripts/setup_android_prereqs.sh"
fi
if [[ -n "$CREATE_EMULATOR_NAME" ]]; then
    create_android_avd "$CREATE_EMULATOR_NAME" "$EMULATOR_API"
fi
if [[ -n "$EMULATOR_NAME" ]]; then
    start_android_emulator "$EMULATOR_NAME"
    wait_for_android_device
    if [[ -z "$DEVICE_SERIAL" ]]; then
        DEVICE_SERIAL=$(select_started_emulator_serial)
        [[ -n "$DEVICE_SERIAL" ]] || fatal "could not determine emulator serial after starting $EMULATOR_NAME"
        log "selected emulator device: $DEVICE_SERIAL"
    fi
fi

DEFAULT_SFX_BANK_PATH="$ROOT/crates/ambition_sandbox/assets/audio/sfx.bank"
if [[ -n "${AMBITION_SFX_BANK_PATH:-}" ]]; then
    DEFAULT_SFX_BANK_PATH="$AMBITION_SFX_BANK_PATH"
fi
case "$STATIC_SFX_BANK" in
    true)
        [[ -f "$DEFAULT_SFX_BANK_PATH" ]] || fatal "--static-sfx-bank requested but no sfx bank exists at $DEFAULT_SFX_BANK_PATH. Generate it first, or use --no-static-sfx-bank."
        STATIC_SFX_BANK_PATH="$DEFAULT_SFX_BANK_PATH"
        add_feature static_sfx_bank
        ;;
    false)
        remove_feature static_sfx_bank
        ;;
    auto)
        if [[ -f "$DEFAULT_SFX_BANK_PATH" ]]; then
            STATIC_SFX_BANK_PATH="$DEFAULT_SFX_BANK_PATH"
            add_feature static_sfx_bank
        else
            remove_feature static_sfx_bank
        fi
        ;;
    *) fatal "internal error: bad STATIC_SFX_BANK value: $STATIC_SFX_BANK" ;;
esac

log "repo: $ROOT"
log "sdk: $SDK_ROOT"
log "ndk: $NDK_ROOT"
log "target ABI: $TARGET_ABI"
log "minSdk: $MIN_SDK  compileSdk: $COMPILE_SDK  targetSdk: $TARGET_SDK"
log "Rust profile: $RUST_PROFILE  Android APK build type: $APK_BUILD_TYPE"
log "default features: $USE_DEFAULT_FEATURES"
log "features: ${FEATURES:-<default only>}"
log "strip native library: $STRIP_NATIVE  size report: $SIZE_REPORT"
if [[ -n "$STATIC_SFX_BANK_PATH" ]]; then
    log "static sfx bank: $STATIC_SFX_BANK_PATH ($(human_size "$STATIC_SFX_BANK_PATH"))"
else
    log "static sfx bank: disabled (no bank found at $DEFAULT_SFX_BANK_PATH)"
fi
log "app id: $APP_ID"
if [[ "$FOLLOW_LOGS" == true ]]; then
    log "logcat filter: $LOG_FILTER"
fi
log "Gradle user home: $GRADLE_USER_HOME"

if [[ "$DOCTOR" == true ]]; then
    log "doctor completed; no build performed"
    exit 0
fi

RUST_TARGET=$(rust_target_for_abi "$TARGET_ABI")
rustup target add "$RUST_TARGET" >/dev/null

PROJECT_DIR="$ROOT/target/android/ambition_sandbox_android"
APP_DIR="$PROJECT_DIR/app"
JNI_OUT="$APP_DIR/src/main/jniLibs"
ASSETS_OUT="$APP_DIR/src/main/assets"
ANDROID_ICON_SRC="$ROOT/assets/icons/android_icon2.png"
# Adaptive icon (mipmap-anydpi-v26): the launcher composites a
# background drawable + foreground drawable inside its own mask shape
# (circle / squircle / etc). Without an adaptive icon, Android wraps a
# square PNG in a white border on most launchers. With one, the icon's
# own dark backdrop blends seamlessly with the launcher's masked frame.
ANDROID_ICON_RES_DIR_DRAWABLE="$APP_DIR/src/main/res/drawable"
ANDROID_ICON_RES_DIR_MIPMAP_ANYDPI="$APP_DIR/src/main/res/mipmap-anydpi-v26"
ANDROID_ICON_RES_DIR_MIPMAP_XXXHDPI="$APP_DIR/src/main/res/mipmap-xxxhdpi"
# Sampled from the icon's edge pixels — the source PNG has a near-pure
# black backdrop, so the launcher's masked background matches it.
ANDROID_ICON_BACKGROUND_HEX="#000000"
ANDROID_ICON_MANIFEST_ATTR=""
if [[ -f "$ANDROID_ICON_SRC" ]]; then
    ANDROID_ICON_MANIFEST_ATTR=$'\n        android:icon="@mipmap/ic_launcher"\n        android:roundIcon="@mipmap/ic_launcher"'
fi

if [[ "$CLEAN" == true ]]; then
    rm -rf "$PROJECT_DIR"
fi

mkdir -p "$APP_DIR/src/main"
cat > "$PROJECT_DIR/settings.gradle" <<EOF
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement { repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories { google(); mavenCentral() } }
rootProject.name = 'AmbitionSandboxAndroid'
include ':app'
EOF

cat > "$PROJECT_DIR/build.gradle" <<EOF
plugins {
    id 'com.android.application' version '$AGP_VERSION' apply false
}
EOF

cat > "$PROJECT_DIR/gradle.properties" <<'EOF'
# GameActivity is published as an AndroidX artifact. AGP fails metadata
# validation unless this flag is enabled in the generated project.
android.useAndroidX=true

# Keep the generated Android project local and predictable. The build script
# already points GRADLE_USER_HOME at target/android/gradle-user-home by default.
org.gradle.jvmargs=-Xmx4096m -Dfile.encoding=UTF-8
android.javaCompile.suppressSourceTargetDeprecationWarning=true
EOF


cat > "$APP_DIR/build.gradle" <<EOF
plugins { id 'com.android.application' }

android {
    namespace '$APP_ID'
    compileSdk $COMPILE_SDK

    defaultConfig {
        applicationId '$APP_ID'
        minSdk $MIN_SDK
        targetSdk $TARGET_SDK
        versionCode 1
        versionName '0.1.0'
    }

    packagingOptions {
        jniLibs { useLegacyPackaging = true }
    }
}

// Keep transitive Kotlin artifacts aligned. Some AndroidX/GameActivity
// dependency combinations pull kotlin-stdlib 1.8.x together with older
// kotlin-stdlib-jdk7/jdk8 1.6.x artifacts, which duplicate JDK extension
// classes at package time. For this generated Java-only shell we do not use
// Kotlin directly, but aligning the transitive runtime avoids duplicate-class
// failures without introducing a checked-in Gradle project.
configurations.configureEach {
    resolutionStrategy.eachDependency { details ->
        if (details.requested.group == 'org.jetbrains.kotlin') {
            details.useVersion '1.8.22'
            details.because 'Align Kotlin stdlib transitive dependencies for AndroidX/GameActivity.'
        }
    }
    // Kotlin 1.8 folds the JDK7/JDK8 extension classes into kotlin-stdlib.
    // Some AndroidX transitive dependency combinations still request the old
    // kotlin-stdlib-jdk7/jdk8 artifacts, which then duplicate those classes.
    // Exclude the compatibility artifacts and keep the unified stdlib.
    exclude group: 'org.jetbrains.kotlin', module: 'kotlin-stdlib-jdk7'
    exclude group: 'org.jetbrains.kotlin', module: 'kotlin-stdlib-jdk8'
}

dependencies {
    implementation platform('org.jetbrains.kotlin:kotlin-bom:1.8.22')
    implementation 'org.jetbrains.kotlin:kotlin-stdlib:1.8.22'
    implementation 'androidx.games:games-activity:$GAMES_ACTIVITY_VERSION'
    // GameActivity extends AppCompatActivity and also uses AndroidX Core.
    // Keep these explicit so the generated project is self-contained even if
    // future GameActivity POMs do not pull them transitively.
    implementation 'androidx.appcompat:appcompat:1.7.0'
    implementation 'androidx.core:core:1.13.1'
}
EOF

cat > "$APP_DIR/src/main/AndroidManifest.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x00400000" android:required="false" />
    <uses-feature android:name="android.hardware.touchscreen" android:required="false" />

    <application
        android:allowBackup="false"
        android:hasCode="true"
        android:label="$APP_LABEL"${ANDROID_ICON_MANIFEST_ATTR}
        android:theme="@style/Theme.Ambition">
        <activity
            android:name=".MainActivity"
            android:configChanges="keyboard|keyboardHidden|orientation|screenLayout|screenSize|smallestScreenSize|uiMode"
            android:exported="true"
            android:screenOrientation="landscape">
            <meta-data android:name="android.app.lib_name" android:value="ambition_sandbox" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
EOF

JAVA_PACKAGE_PATH=$(printf '%s' "$APP_ID" | tr '.' '/')
JAVA_SRC_DIR="$APP_DIR/src/main/java/$JAVA_PACKAGE_PATH"
mkdir -p "$JAVA_SRC_DIR"
cat > "$JAVA_SRC_DIR/MainActivity.java" <<EOF
package $APP_ID;

import com.google.androidgamesdk.GameActivity;

/**
 * Thin launcher activity for the generated Ambition Android project.
 *
 * GameActivity's Java class lives in the com.google.androidgamesdk package.
 * Keep this app-local subclass so Android launches a class packaged in this APK
 * and so the native Rust library is loaded before GameActivity enters native
 * code.
 */
public class MainActivity extends GameActivity {
    static {
        System.loadLibrary("ambition_sandbox");
    }
}
EOF

mkdir -p "$APP_DIR/src/main/res/values"
if [[ -f "$ANDROID_ICON_SRC" ]]; then
    # Foreground PNG: drop the source icon into mipmap-xxxhdpi (Android
    # scales down for lower densities). Adaptive icons render the
    # foreground inside a 108×108 dp canvas where only the inner 66 dp
    # circle is the "safe zone"; the outer 21 dp on each side can be
    # cropped by launcher zoom / parallax. Pad the source artwork into a
    # 432×432 canvas at ~72% scale so the visible artwork stays inside
    # the safe zone with the icon's own dark backdrop filling the
    # bleed region.
    mkdir -p "$ANDROID_ICON_RES_DIR_MIPMAP_XXXHDPI"
    if command -v python3 >/dev/null 2>&1 && python3 -c "import PIL" >/dev/null 2>&1; then
        python3 - "$ANDROID_ICON_SRC" "$ANDROID_ICON_RES_DIR_MIPMAP_XXXHDPI/ic_launcher_foreground.png" <<'PY'
import sys
from PIL import Image
src_path, dst_path = sys.argv[1], sys.argv[2]
src = Image.open(src_path).convert("RGBA")
# Adaptive-icon target: 432×432 px (mipmap-xxxhdpi == 4× 108 dp).
canvas_size = 432
inner_size = int(canvas_size * 0.72)
inner = src.resize((inner_size, inner_size), Image.LANCZOS)
canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
offset = (canvas_size - inner_size) // 2
canvas.paste(inner, (offset, offset), inner)
canvas.save(dst_path, "PNG")
PY
    else
        # Fallback: PIL isn't available — copy the raw source. The
        # launcher will likely still clip the corners but the icon is
        # already corner-darkened so the cropping reads as black.
        cp "$ANDROID_ICON_SRC" "$ANDROID_ICON_RES_DIR_MIPMAP_XXXHDPI/ic_launcher_foreground.png"
    fi

    # Adaptive icon XML — launcher composites background + foreground
    # inside its mask shape (circle / squircle / etc).
    mkdir -p "$ANDROID_ICON_RES_DIR_MIPMAP_ANYDPI"
    cat > "$ANDROID_ICON_RES_DIR_MIPMAP_ANYDPI/ic_launcher.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
EOF
    cat > "$ANDROID_ICON_RES_DIR_MIPMAP_ANYDPI/ic_launcher_round.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
EOF

    # Background color resource referenced by the adaptive icon XML.
    cat > "$APP_DIR/src/main/res/values/colors.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">$ANDROID_ICON_BACKGROUND_HEX</color>
</resources>
EOF

    # Legacy fallback: pre-API-26 launchers (and Gradle's resource
    # linter) still want a plain bitmap @drawable/android_icon, so keep
    # the old drawable around even though the manifest now points at
    # @mipmap/ic_launcher.
    mkdir -p "$ANDROID_ICON_RES_DIR_DRAWABLE"
    cp "$ANDROID_ICON_SRC" "$ANDROID_ICON_RES_DIR_DRAWABLE/android_icon.png"

    log "using Android app icon (adaptive): $ANDROID_ICON_SRC (bg $ANDROID_ICON_BACKGROUND_HEX)"
else
    log "Android app icon: default generated icon (no assets/android_icon.png found)"
fi
cat > "$APP_DIR/src/main/res/values/styles.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <!-- GameActivity extends AppCompatActivity, so the launched activity must
         use an AppCompat-derived theme. Keep it fullscreen/no-title for the
         game surface while satisfying AppCompatDelegate's runtime check. -->
    <style name="Theme.Ambition" parent="Theme.AppCompat.NoActionBar">
        <item name="windowActionBar">false</item>
        <item name="windowNoTitle">true</item>
        <item name="android:windowActionBar">false</item>
        <item name="android:windowNoTitle">true</item>
        <item name="android:windowFullscreen">true</item>
    </style>
</resources>
EOF

log "copying runtime assets into generated Android project"
rm -rf "$ASSETS_OUT"
copy_tree_contents "$ROOT/crates/ambition_sandbox/assets" "$ASSETS_OUT"
if [[ -d "$ASSETS_OUT/sprites" ]]; then
    sprite_png_count=$(find "$ASSETS_OUT/sprites" -type f -name '*.png' | wc -l | tr -d ' ')
    if [[ "$sprite_png_count" == "0" ]]; then
        warn "no sprite PNGs were copied into the APK assets; the game will use colored-rectangle sprite fallbacks. Regenerate/copy sprites under crates/ambition_sandbox/assets/sprites before building Android."
    else
        log "copied $sprite_png_count sprite PNGs into APK assets"
    fi
else
    warn "no sprites/ asset directory copied into APK; sprite art will fall back to colored rectangles"
fi
warn_missing_registered_character_sprites
if [[ -f "$ASSETS_OUT/audio/sfx.bank" ]]; then
    log "copied sfx.bank into APK assets: $(human_size "$ASSETS_OUT/audio/sfx.bank")"
else
    warn "no sfx.bank copied into APK assets; generated/fundsp SFX fallback will be used unless static_sfx_bank is enabled"
fi
log "APK asset tree size: $(dir_size "$ASSETS_OUT")"

CARGO_NDK_ARGS=(cargo ndk -t "$TARGET_ABI" -P "$MIN_SDK" -o "$JNI_OUT" build -p ambition_sandbox --lib)
case "$RUST_PROFILE" in
    debug) ;;
    release) CARGO_NDK_ARGS+=(--release) ;;
    *) CARGO_NDK_ARGS+=(--profile "$RUST_PROFILE") ;;
esac
if [[ "$USE_DEFAULT_FEATURES" != true ]]; then
    CARGO_NDK_ARGS+=(--no-default-features)
fi
if [[ -n "$FEATURES" ]]; then
    CARGO_NDK_ARGS+=(--features "$FEATURES")
fi

log "building Rust shared library"
if [[ -n "$STATIC_SFX_BANK_PATH" ]]; then
    AMBITION_ANDROID_APP_ID="$APP_ID" AMBITION_STATIC_SFX_BANK_PATH="$STATIC_SFX_BANK_PATH" "${CARGO_NDK_ARGS[@]}"
else
    AMBITION_ANDROID_APP_ID="$APP_ID" "${CARGO_NDK_ARGS[@]}"
fi

SO_PATH="$JNI_OUT/$TARGET_ABI/libambition_sandbox.so"
if [[ "$STRIP_NATIVE" == true ]]; then
    strip_native_library "$SO_PATH"
else
    log "native library size: $(human_size "$SO_PATH") (strip disabled)"
fi
if [[ "$SIZE_REPORT" == true ]]; then
    print_native_size_report "$SO_PATH"
fi

GRADLE_TASK="assembleDebug"
if [[ "$APK_BUILD_TYPE" == "release" ]]; then
    GRADLE_TASK="assembleRelease"
fi

log "assembling APK with Gradle task: $GRADLE_TASK"
( cd "$PROJECT_DIR" && GRADLE_USER_HOME="$GRADLE_USER_HOME" "$GRADLE_CMD" --no-daemon "$GRADLE_TASK" )

APK_DIR="$APP_DIR/build/outputs/apk/$APK_BUILD_TYPE"
APK=$(find "$APK_DIR" -maxdepth 1 -type f -name '*.apk' | sort | tail -1)
[[ -n "$APK" ]] || fatal "Gradle finished but no APK was found under $APK_DIR"

OUT_DIR="$ROOT/target/android/apks"
mkdir -p "$OUT_DIR"
OUT_APK="$OUT_DIR/ambition_sandbox-${APK_BUILD_TYPE}-${TARGET_ABI}.apk"
cp "$APK" "$OUT_APK"

log "APK: $OUT_APK"
log "size summary: native=$(human_size "$SO_PATH")  apk=$(human_size "$OUT_APK")  apk-assets=$(dir_size "$ASSETS_OUT")"
if [[ "$SIZE_REPORT" == true ]]; then
    print_apk_size_report "$OUT_APK"
    log "cargo-bloat hint: cargo bloat --profile $RUST_PROFILE --target aarch64-linux-android -p ambition_sandbox --lib --no-default-features --features '${FEATURES}' --crates -n 40"
fi

ADB_ARGS=()
if [[ -n "$DEVICE_SERIAL" ]]; then
    ADB_ARGS=(-s "$DEVICE_SERIAL")
fi

if [[ "$INSTALL" == true ]]; then
    if [[ "$FRESH_INSTALL" == true ]]; then
        log "fresh install requested; removing existing package for current user if present"
        adb "${ADB_ARGS[@]}" shell am force-stop "$APP_ID" >/dev/null 2>&1 || true
        adb "${ADB_ARGS[@]}" shell cmd package uninstall --user 0 "$APP_ID" >/dev/null 2>&1 || true
        adb "${ADB_ARGS[@]}" uninstall "$APP_ID" >/dev/null 2>&1 || true
    fi
    log "installing APK via adb"
    adb "${ADB_ARGS[@]}" install -r -d --install-location 0 "$OUT_APK"
fi

if [[ "$RUN_APP" == true ]]; then
    log "launching app via adb monkey"
    adb "${ADB_ARGS[@]}" shell monkey -p "$APP_ID" 1 >/dev/null
fi

if [[ "$FOLLOW_LOGS" == true ]]; then
    follow_android_logs
fi
