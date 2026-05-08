#!/usr/bin/env bash
set -euo pipefail

# Install/check the Linux prerequisites for Ambition Android builds.
#
# This consolidates the Android SDK/NDK, Gradle, Rust target, and cargo-ndk
# setup that ./build_for_android.sh expects. Defaults are intentionally pinned
# to known-good versions for the generated Gradle project, but can be
# overridden via environment variables.
#
# Usage:
#   ./scripts/setup_android_prereqs.sh
#   ./scripts/setup_android_prereqs.sh --doctor
#   ./scripts/setup_android_prereqs.sh --with-emulator
#
# Environment overrides:
#   ANDROID_API=35
#   ANDROID_BUILD_TOOLS_VERSION=35.0.0
#   ANDROID_NDK_VERSION=27.2.12479018
#   ANDROID_CMDLINE_TOOLS_VERSION=14742923
#   GRADLE_VERSION=8.9
#   ANDROID_SDK_ROOT=$HOME/Android/Sdk
#   GRADLE_ROOT=$HOME/.local/share/gradle

usage() {
    cat <<'EOF'
Usage: ./scripts/setup_android_prereqs.sh [options]

Options:
  --doctor          Check the environment and print missing pieces; do not install.
  --with-emulator   Also install Android emulator packages and create an AVD if missing.
  --skip-apt        Do not install host packages with apt-get.
  --no-profile      Do not update ~/.bashrc with Android/Gradle environment exports.
  -h, --help        Show this help.

Environment overrides:
  ANDROID_API                       Default: 35
  ANDROID_BUILD_TOOLS_VERSION       Default: 35.0.0
  ANDROID_NDK_VERSION               Default: 27.2.12479018
  ANDROID_CMDLINE_TOOLS_VERSION     Default: 14742923
  GRADLE_VERSION                    Default: 8.9
  ANDROID_SDK_ROOT                  Default: $HOME/Android/Sdk
  GRADLE_ROOT                       Default: $HOME/.local/share/gradle
EOF
}

log() { printf '[android-prereq] %s\n' "$*"; }
warn() { printf '[android-prereq] warning: %s\n' "$*" >&2; }
fatal() { printf '[android-prereq] error: %s\n' "$*" >&2; exit 1; }

ANDROID_API="${ANDROID_API:-35}"
ANDROID_BUILD_TOOLS_VERSION="${ANDROID_BUILD_TOOLS_VERSION:-35.0.0}"
ANDROID_NDK_VERSION="${ANDROID_NDK_VERSION:-27.2.12479018}"
ANDROID_CMDLINE_TOOLS_VERSION="${ANDROID_CMDLINE_TOOLS_VERSION:-14742923}"
GRADLE_VERSION="${GRADLE_VERSION:-8.9}"

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}"
export ANDROID_HOME="$ANDROID_SDK_ROOT"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_SDK_ROOT/ndk/$ANDROID_NDK_VERSION}"
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

GRADLE_ROOT="${GRADLE_ROOT:-$HOME/.local/share/gradle}"
export GRADLE_HOME="${GRADLE_HOME:-$GRADLE_ROOT/gradle-$GRADLE_VERSION}"

SDKMANAGER="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager"
AVDMANAGER="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/avdmanager"
GRADLE_BIN="$GRADLE_HOME/bin/gradle"

DOCTOR=false
WITH_EMULATOR=false
SKIP_APT=false
UPDATE_PROFILE=true
while [[ $# -gt 0 ]]; do
    case "$1" in
        --doctor) DOCTOR=true ;;
        --with-emulator) WITH_EMULATOR=true ;;
        --skip-apt) SKIP_APT=true ;;
        --no-profile) UPDATE_PROFILE=false ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

prepend_path() {
    case ":$PATH:" in
        *":$1:"*) ;;
        *) export PATH="$1:$PATH" ;;
    esac
}

setup_path_for_this_shell() {
    prepend_path "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin"
    prepend_path "$ANDROID_SDK_ROOT/platform-tools"
    prepend_path "$ANDROID_SDK_ROOT/emulator"
    prepend_path "$GRADLE_HOME/bin"
}

check_cmd() {
    local name=$1
    if command -v "$name" >/dev/null 2>&1; then
        printf 'ok     %s -> %s\n' "$name" "$(command -v "$name")"
    else
        printf 'missing %s\n' "$name"
        return 1
    fi
}

run_doctor() {
    setup_path_for_this_shell
    local missing=0
    echo "[android-prereq] environment"
    echo "  ANDROID_SDK_ROOT=$ANDROID_SDK_ROOT"
    echo "  ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"
    echo "  GRADLE_HOME=$GRADLE_HOME"
    echo
    check_cmd java || missing=1
    check_cmd rustup || missing=1
    check_cmd cargo || missing=1
    check_cmd cargo-ndk || missing=1
    check_cmd adb || missing=1
    check_cmd gradle || missing=1
    [[ -x "$SDKMANAGER" ]] || { echo "missing sdkmanager at $SDKMANAGER"; missing=1; }
    [[ -d "$ANDROID_NDK_ROOT" ]] || { echo "missing NDK dir at $ANDROID_NDK_ROOT"; missing=1; }
    if rustup target list --installed | grep -qx 'aarch64-linux-android'; then
        echo "ok     rust target aarch64-linux-android"
    else
        echo "missing rust target aarch64-linux-android"
        missing=1
    fi
    if [[ -e "$HOME/.gradle" && ! -w "$HOME/.gradle" ]]; then
        echo "warning ~/.gradle exists but is not writable: $HOME/.gradle"
        echo "        Fix with: sudo chown -R "$USER:$USER" "$HOME/.gradle""
    else
        echo "ok     user Gradle cache writable or absent: $HOME/.gradle"
    fi
    echo
    if [[ $missing -eq 0 ]]; then
        echo "[android-prereq] doctor passed"
    else
        echo "[android-prereq] doctor found missing prerequisites"
    fi
    return "$missing"
}

install_host_packages() {
    if [[ "$SKIP_APT" == true ]]; then
        log "skipping apt host package install"
        return 0
    fi
    if command -v apt-get >/dev/null 2>&1; then
        log "installing host packages via apt"
        sudo apt-get update
        sudo apt-get install -y \
            curl \
            unzip \
            zip \
            ca-certificates \
            openjdk-17-jdk \
            pkg-config
    else
        warn "apt-get not found; assuming curl/unzip/zip/JDK are already installed"
    fi
}

install_cmdline_tools() {
    mkdir -p "$ANDROID_SDK_ROOT/cmdline-tools"
    if [[ -x "$SDKMANAGER" ]]; then
        log "Android command-line tools already installed"
        return 0
    fi

    log "installing Android command-line tools $ANDROID_CMDLINE_TOOLS_VERSION"
    local tmpdir zip_path
    tmpdir=$(mktemp -d)
    zip_path="$tmpdir/commandlinetools-linux.zip"
    curl -fL \
        "https://dl.google.com/android/repository/commandlinetools-linux-${ANDROID_CMDLINE_TOOLS_VERSION}_latest.zip" \
        -o "$zip_path"

    rm -rf "$ANDROID_SDK_ROOT/cmdline-tools/latest" "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools"
    unzip -q "$zip_path" -d "$ANDROID_SDK_ROOT/cmdline-tools"
    mv "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools" "$ANDROID_SDK_ROOT/cmdline-tools/latest"
    rm -rf "$tmpdir"
}

install_sdk_packages() {
    mkdir -p "$ANDROID_SDK_ROOT"
    touch "$ANDROID_SDK_ROOT/repositories.cfg"
    setup_path_for_this_shell

    log "accepting Android SDK licenses"
    yes | "$SDKMANAGER" --sdk_root="$ANDROID_SDK_ROOT" --licenses >/dev/null || true

    local packages=(
        "platform-tools"
        "platforms;android-${ANDROID_API}"
        "build-tools;${ANDROID_BUILD_TOOLS_VERSION}"
        "ndk;${ANDROID_NDK_VERSION}"
    )
    if [[ "$WITH_EMULATOR" == true ]]; then
        packages+=(
            "emulator"
            "system-images;android-${ANDROID_API};google_apis;x86_64"
        )
    fi

    log "installing SDK packages"
    "$SDKMANAGER" --sdk_root="$ANDROID_SDK_ROOT" "${packages[@]}"
}

install_gradle() {
    mkdir -p "$GRADLE_ROOT"
    if [[ -x "$GRADLE_BIN" ]]; then
        log "Gradle already installed at $GRADLE_BIN"
        return 0
    fi

    log "installing Gradle $GRADLE_VERSION"
    local zip_path
    zip_path="/tmp/gradle-$GRADLE_VERSION-bin.zip"
    curl -fL "https://services.gradle.org/distributions/gradle-$GRADLE_VERSION-bin.zip" -o "$zip_path"
    rm -rf "$GRADLE_HOME"
    unzip -q "$zip_path" -d "$GRADLE_ROOT"
}

install_rust_bits() {
    log "installing Rust Android target"
    rustup target add aarch64-linux-android

    if command -v cargo-ndk >/dev/null 2>&1; then
        log "cargo-ndk already installed"
    else
        log "installing cargo-ndk"
        cargo install cargo-ndk
    fi
}

update_profile() {
    if [[ "$UPDATE_PROFILE" != true ]]; then
        log "not updating shell profile"
        return 0
    fi

    local profile_file marker_begin marker_end env_block
    profile_file="$HOME/.bashrc"
    marker_begin="# >>> ambition android env >>>"
    marker_end="# <<< ambition android env <<<"
    env_block="$marker_begin
export ANDROID_SDK_ROOT=\"\$HOME/Android/Sdk\"
export ANDROID_HOME=\"\$ANDROID_SDK_ROOT\"
export ANDROID_NDK_ROOT=\"\$ANDROID_SDK_ROOT/ndk/$ANDROID_NDK_VERSION\"
export ANDROID_NDK_HOME=\"\$ANDROID_NDK_ROOT\"
export GRADLE_HOME=\"\$HOME/.local/share/gradle/gradle-$GRADLE_VERSION\"
export PATH=\"\$GRADLE_HOME/bin:\$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:\$ANDROID_SDK_ROOT/platform-tools:\$ANDROID_SDK_ROOT/emulator:\$PATH\"
$marker_end"

    if grep -q "$marker_begin" "$profile_file" 2>/dev/null; then
        log "updating Android/Gradle env block in $profile_file"
        python3 - "$profile_file" "$marker_begin" "$marker_end" "$env_block" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
begin = sys.argv[2]
end = sys.argv[3]
block = sys.argv[4]
text = path.read_text() if path.exists() else ""
start = text.find(begin)
stop = text.find(end)
if start >= 0 and stop >= start:
    stop = stop + len(end)
    text = text[:start].rstrip() + "\n" + block + "\n" + text[stop:].lstrip()
else:
    text = text.rstrip() + "\n\n" + block + "\n"
path.write_text(text)
PY
    else
        log "adding Android/Gradle env block to $profile_file"
        {
            echo
            echo "$env_block"
        } >> "$profile_file"
    fi
}

create_emulator_if_requested() {
    if [[ "$WITH_EMULATOR" != true ]]; then
        return 0
    fi
    setup_path_for_this_shell
    local avd_name="ambition_pixel_${ANDROID_API}"
    if "$AVDMANAGER" list avd | grep -q "Name: $avd_name"; then
        log "AVD already exists: $avd_name"
        return 0
    fi
    log "creating AVD: $avd_name"
    yes "no" | "$AVDMANAGER" create avd \
        -n "$avd_name" \
        -k "system-images;android-${ANDROID_API};google_apis;x86_64" \
        -d pixel_6 || true
    log "start emulator with: $ANDROID_SDK_ROOT/emulator/emulator -avd $avd_name -netdelay none -netspeed full"
}

if [[ "$DOCTOR" == true ]]; then
    run_doctor
    exit $?
fi

log "ANDROID_SDK_ROOT=$ANDROID_SDK_ROOT"
log "ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"
log "GRADLE_HOME=$GRADLE_HOME"
log "API=$ANDROID_API build-tools=$ANDROID_BUILD_TOOLS_VERSION ndk=$ANDROID_NDK_VERSION gradle=$GRADLE_VERSION"

install_host_packages
install_cmdline_tools
install_sdk_packages
install_gradle
setup_path_for_this_shell
install_rust_bits
update_profile
create_emulator_if_requested

echo
log "versions"
java -version || true
"$SDKMANAGER" --version || true
adb version || true
gradle --version || true
cargo ndk --version || true

echo
log "done"
echo "For this shell, run:"
echo "  export ANDROID_SDK_ROOT=\"$ANDROID_SDK_ROOT\""
echo "  export ANDROID_HOME=\"\$ANDROID_SDK_ROOT\""
echo "  export ANDROID_NDK_ROOT=\"$ANDROID_NDK_ROOT\""
echo "  export ANDROID_NDK_HOME=\"\$ANDROID_NDK_ROOT\""
echo "  export GRADLE_HOME=\"$GRADLE_HOME\""
echo "  export PATH=\"\$GRADLE_HOME/bin:\$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:\$ANDROID_SDK_ROOT/platform-tools:\$ANDROID_SDK_ROOT/emulator:\$PATH\""
echo
echo "Then run:"
echo "  ./build_for_android.sh --doctor"
echo "  ./build_for_android.sh"
