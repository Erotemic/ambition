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
  --run                  Install and launch the APK on an attached Android device.
  --device SERIAL        Pass -s SERIAL to adb for install/run.
  --rust-debug           Build the Rust shared library without --release.
  --rust-release         Build the Rust shared library with --release (default).
  --apk-release          Run Gradle assembleRelease instead of assembleDebug.
  --apk-debug            Run Gradle assembleDebug (default; debug-signed).
  --target ABI           Android ABI for cargo-ndk. Default: arm64-v8a.
  --min-sdk API          Android minSdk and cargo-ndk platform. Default: 31.
  --compile-sdk API      Android compileSdk. Default: highest installed, else 35.
  --target-sdk API       Android targetSdk. Default: compileSdk.
  --features LIST        Cargo features to add. Default: android.
  --no-static-map        Do not add static_map to the Android cargo features. This also removes the android composite feature if it is still present.
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
  ./build_for_android.sh --doctor
EOF
}

log() { printf '[android-build] %s\n' "$*"; }
warn() { printf '[android-build] warning: %s\n' "$*" >&2; }
fatal() { printf '[android-build] error: %s\n' "$*" >&2; exit 1; }

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
DEVICE_SERIAL=""
GRADLE_USER_HOME_ARG=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install) INSTALL=true ;;
        --fresh-install) FRESH_INSTALL=true; INSTALL=true ;;
        --run) RUN_APP=true; INSTALL=true ;;
        --device) shift; [[ $# -gt 0 ]] || fatal "--device needs a serial"; DEVICE_SERIAL=$1 ;;
        --rust-debug) RUST_PROFILE="debug" ;;
        --rust-release) RUST_PROFILE="release" ;;
        --apk-debug) APK_BUILD_TYPE="debug" ;;
        --apk-release) APK_BUILD_TYPE="release" ;;
        --target) shift; [[ $# -gt 0 ]] || fatal "--target needs an ABI"; TARGET_ABI=$1 ;;
        --min-sdk) shift; [[ $# -gt 0 ]] || fatal "--min-sdk needs an API level"; MIN_SDK=$1 ;;
        --compile-sdk) shift; [[ $# -gt 0 ]] || fatal "--compile-sdk needs an API level"; COMPILE_SDK=$1 ;;
        --target-sdk) shift; [[ $# -gt 0 ]] || fatal "--target-sdk needs an API level"; TARGET_SDK=$1 ;;
        --features) shift; [[ $# -gt 0 ]] || fatal "--features needs a comma-separated or space-separated feature list"; FEATURES=$1 ;;
        --no-static-map)
            FEATURES=$(printf '%s' "$FEATURES" | tr ',' ' ' | awk '{for (i=1;i<=NF;i++) if ($i != "static_map" && $i != "android") printf "%s%s", sep, $i; sep=" "}')
            ;;
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
if [[ "$INSTALL" == true ]]; then
    need_cmd adb "Run: ./scripts/setup_android_prereqs.sh"
fi

log "repo: $ROOT"
log "sdk: $SDK_ROOT"
log "ndk: $NDK_ROOT"
log "target ABI: $TARGET_ABI"
log "minSdk: $MIN_SDK  compileSdk: $COMPILE_SDK  targetSdk: $TARGET_SDK"
log "Rust profile: $RUST_PROFILE  Android APK build type: $APK_BUILD_TYPE"
log "features: ${FEATURES:-<default only>}"
log "app id: $APP_ID"
log "Gradle user home: $GRADLE_USER_HOME"

if [[ "$DOCTOR" == true ]]; then
    log "doctor completed; no build performed"
    exit 0
fi

rustup target add aarch64-linux-android >/dev/null

PROJECT_DIR="$ROOT/target/android/ambition_sandbox_android"
APP_DIR="$PROJECT_DIR/app"
JNI_OUT="$APP_DIR/src/main/jniLibs"
ASSETS_OUT="$APP_DIR/src/main/assets"

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
        android:label="$APP_LABEL"
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

CARGO_NDK_ARGS=(cargo ndk -t "$TARGET_ABI" -P "$MIN_SDK" -o "$JNI_OUT" build -p ambition_sandbox --lib)
if [[ "$RUST_PROFILE" == "release" ]]; then
    CARGO_NDK_ARGS+=(--release)
fi
if [[ -n "$FEATURES" ]]; then
    CARGO_NDK_ARGS+=(--features "$FEATURES")
fi

log "building Rust shared library"
"${CARGO_NDK_ARGS[@]}"

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
    log "logs: adb ${ADB_ARGS[*]} logcat | grep -E 'RustStdoutStderr|ambition|bevy|wgpu'"
fi
