# Android build / sideload workflow

This is the first practical Android packaging path for testing the sandbox on a
phone. It uses Bevy's Android flow: build the Rust crate as an Android shared
library with `cargo-ndk`, place that library in a generated Gradle project, copy
runtime assets into the APK asset directory, then let Gradle produce an
installable APK.

## One-time prerequisites

Run the checked-in prereq helper:

```bash
./scripts/setup_android_prereqs.sh
source ~/.bashrc
```

The script installs/checks:

- Android command-line tools
- platform-tools / `adb`
- Android platform and build-tools
- Android NDK
- Gradle
- Rust `aarch64-linux-android` target
- `cargo-ndk`

Check the environment without installing anything:

```bash
./scripts/setup_android_prereqs.sh --doctor
./build_for_android.sh --doctor
```

Optional emulator support:

```bash
./scripts/setup_android_prereqs.sh --with-emulator
```

## Build

Build a debug-signed APK with an optimized Rust shared library:

```bash
./build_for_android.sh
```

The generated Android project is intentionally not checked in. It lives under:

```text
target/android/ambition_sandbox_android/
```

The generated project writes its own `gradle.properties` with:

```properties
android.useAndroidX=true
```

This is required because `androidx.games:games-activity` is an AndroidX
dependency; without it Gradle fails `checkDebugAarMetadata`.

The final APK is copied to:

```text
target/android/apks/ambition_sandbox-debug-arm64-v8a.apk
```

The build script copies `crates/ambition_sandbox/assets/` into the generated
Gradle project's `app/src/main/assets/`, so LDtk, RON, dialogue, sprites, fonts,
and audio assets are packaged into the APK rather than needing loose files next
to the app.

## Install / run on a phone

On the phone:

1. Enable Developer options.
2. Enable USB debugging.
3. Plug in USB and accept the RSA prompt.

Then:

```bash
adb devices
./build_for_android.sh --install
```

Build, install, and launch:

```bash
./build_for_android.sh --run
```

If more than one device/emulator is connected:

```bash
adb devices
./build_for_android.sh --run --device <serial>
```

Watch logs:

```bash
adb logcat | grep -E 'RustStdoutStderr|ambition|bevy|wgpu'
```

## Gradle cache permissions

`build_for_android.sh` uses a repo-local Gradle cache by default:

```text
target/android/gradle-user-home
```

This avoids failures caused by stale/root-owned files under `~/.gradle`. If you
still want to use the global Gradle cache, pass `--gradle-user-home ~/.gradle`
or set `GRADLE_USER_HOME=~/.gradle`. If Gradle reports that it cannot create a
cache directory under `~/.gradle`, repair ownership with:

```bash
sudo chown -R "$USER:$USER" "$HOME/.gradle"
```

### GameActivity launcher class

The generated project uses a tiny app-local `MainActivity` Java class that
extends `com.google.androidgamesdk.GameActivity` and loads
`libambition_sandbox.so`. Do not point `AndroidManifest.xml` directly at
`androidx.games.activity.GameActivity`; the runtime class provided by the
GameActivity AAR is in the `com.google.androidgamesdk` package, and using the
wrong manifest class crashes before Rust starts.



### GameActivity dependencies

The generated Gradle project uses a tiny `MainActivity` subclass of
`com.google.androidgamesdk.GameActivity`. GameActivity itself is distributed in
`androidx.games:games-activity`, and it extends `AppCompatActivity`, so the
generated app declares `androidx.appcompat:appcompat` and `androidx.core:core`
explicitly. If either dependency is missing, Android/Gradle can fail before the
Rust library is ever loaded.


### Kotlin duplicate-class failures

The generated Android shell is Java-only, but AndroidX/GameActivity dependencies
can pull Kotlin runtime artifacts transitively. If Gradle reports duplicate
classes involving `kotlin-stdlib`, `kotlin-stdlib-jdk7`, and
`kotlin-stdlib-jdk8`, the generated `app/build.gradle` aligns all
`org.jetbrains.kotlin` artifacts to Kotlin `1.8.22` and imports the Kotlin BOM.
This keeps transitive AndroidX dependencies compatible without checking in a
full Android project.

### AppCompat theme requirement

`GameActivity` extends `AppCompatActivity`, so the generated manifest applies
`@style/Theme.Ambition`, and that style must inherit from an AppCompat theme
(for example `Theme.AppCompat.NoActionBar`). If Android crashes with
`You need to use a Theme.AppCompat theme`, regenerate the Android project with
`./build_for_android.sh --clean` after updating this script.
