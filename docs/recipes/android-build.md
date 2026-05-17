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

Build, install, launch, and follow filtered logcat:

```bash
./build_for_android.sh --run
```

`--run` now follows the Rust/tracing log stream by default. Press `Ctrl-C`
to stop watching logs; the installed app keeps running. Use `--no-logs` when
you only want to build/install/launch, or pass `--log-filter` to replace the
default logcat filter.

If more than one device/emulator is connected:

```bash
adb devices
./build_for_android.sh --run --device <serial>
```

## Emulator workflow

Install emulator prerequisites once:

```bash
./scripts/setup_android_prereqs.sh --with-emulator
```

List existing Android Virtual Devices:

```bash
./build_for_android.sh --list-emulators
```

Create a Pixel-style x86_64 emulator if needed, build an x86_64 APK, install,
launch, and follow logs:

```bash
./build_for_android.sh --run --create-emulator ambition_pixel
```

Reuse an existing AVD:

```bash
./build_for_android.sh --run --emulator ambition_pixel
```

When an emulator is requested and `--target` is not specified, the script uses
`--target x86_64` so the APK matches the usual desktop emulator system image.

## Android data path

Android builds compile the app id into the Rust library and use
`/data/data/<app-id>/files/ambition/` for settings and sandbox save data. This
avoids attempting to write `./ambition/settings.ron` inside the read-only APK
working directory. `AMBITION_DATA_DIR` still overrides the root for tests and
local experiments.

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

## Phone usability notes

Android builds package the checked-out `crates/ambition_sandbox/assets/` tree into
`app/src/main/assets/`. Generated sprite PNGs are usually ignored by git, so the
build script now prints how many sprite PNGs were copied. If that count is zero,
the APK will still run but character/entity art falls back to colored rectangles.
Regenerate or copy the sprite PNGs into `crates/ambition_sandbox/assets/sprites/`
before rebuilding the APK.

The Android build starts with debug HUD/gizmo overlays disabled so the small
phone screen is not covered by desktop tuning text. Desktop builds keep the
traditional debug-first defaults.

Touch buttons are folded from raw active touches as well as Bevy UI
`Interaction`s. This is intentional: the joystick can own one touch while another
finger taps Jump / Attack / Dash / Blink / Interact / Projectile / Pause.

## Size-focused Android builds

The default Android script uses `--no-default-features --features android`, so it
keeps the playable sandbox, touch controls, audio, LDtk runtime, UI, and RL/test
seams while excluding desktop-only inspector/file-watcher tooling from the phone
artifact.

For a smaller phone-test APK without changing gameplay features, use the
`android-size` Cargo profile:

```bash
./build_for_android.sh --size-profile --fresh-install
```

This profile uses size-oriented optimization, thin LTO, one codegen unit,
`panic = "abort"`, and symbol stripping. The script also runs the NDK
`llvm-strip --strip-unneeded` as a backstop and prints native library before / after
sizes.

Useful variants:

```bash
./build_for_android.sh --size-profile --size-report
./build_for_android.sh --size-profile --static-map
./build_for_android.sh --use-default-features  # intentionally includes desktop defaults
```

If the binary is unexpectedly large, compare the native library and APK sizes
between these commands:

```bash
./build_for_android.sh --clean
ls -lh target/android/ambition_sandbox_android/app/src/main/jniLibs/arm64-v8a/libambition_sandbox.so \
      target/android/apks/ambition_sandbox-debug-arm64-v8a.apk

./build_for_android.sh --clean --size-profile
ls -lh target/android/ambition_sandbox_android/app/src/main/jniLibs/arm64-v8a/libambition_sandbox.so \
      target/android/apks/ambition_sandbox-debug-arm64-v8a.apk
```

Note: the Android composite feature still includes `static_map` today because the
LDtk world is synchronously parsed by sandbox startup code before Bevy's
Android asset pipeline can provide a packaged asset handle. This is only about
~1 MiB of source map data and is not the main native-library size driver. A
future cleanup can teach the LDtk loader to read from Android APK assets and
then make `--no-static-map` the normal Android path.


## SFX bank on Android

The current SFX-bank runtime loader is synchronous and path/byte based. Desktop
can read `assets/audio/sfx.bank` directly from the checkout, but Android APK
assets are not ordinary filesystem files. Until the SFX bank gets an Android
asset-reader backend, `build_for_android.sh` automatically enables the
`static_sfx_bank` Cargo feature when `crates/ambition_sandbox/assets/audio/sfx.bank`
exists and passes the bank path to Rust for `include_bytes!`.

That keeps the phone build using the real SFX bank instead of falling back to
old generated/fundsp sounds. The same bank is still copied into APK assets so a
future APK asset-loader path can stop embedding it in the native library.

Controls:

```bash
./build_for_android.sh --static-sfx-bank
./build_for_android.sh --no-static-sfx-bank
```

## Size diagnostics

`--size-profile` now prints native library, APK, and APK-asset sizes. It also
prints the largest APK entries and an optional `cargo bloat` command to run when
symbol-level attribution is needed.

## Menu/touch input seam

Android touch menus use the same semantic `MenuControlFrame` as desktop
keyboard/gamepad menu navigation. Touch Start folds into pause, Reset folds into
Back, Jump/Interact fold into Confirm, and a one-finger drag outside the fixed
on-screen controls folds into menu scroll. Keep this separate from gameplay
`ControlFrame` so RL/gameplay movement does not learn about UI gestures.

## Android platform feature hygiene

The sandbox crate uses two explicit Bevy platform feature groups:

- `desktop_platform = ["bevy/default_platform"]`
- `android_platform = [...]`

The Android group intentionally does **not** include `bevy/default_platform`,
because that broad Bevy collection includes desktop-oriented features such as
`bevy_gilrs`, X11/Wayland, web platform support, and the sysinfo plugin. Android
still gets GameActivity/winit/std/multi-threaded support, but a healthy launch
should no longer emit the noisy `bevy_gilrs` unsupported-platform error.

## LDtk loading policy on Android

Android builds still package `crates/ambition_sandbox/assets/` into APK assets,
but the startup LDtk JSON is synchronously parsed before the Android AssetServer
path is convenient for world composition. For now, `static_map` Android builds
load the embedded `sandbox.ldtk` first unless an explicit `--ldtk`, `--map`, or
`AMBITION_LDTK` override is provided. Desktop builds continue to default to the
external checked-in LDtk path for iteration and hot reload.

## Bundled UI fonts

The app prefers fonts under:

```text
crates/ambition_sandbox/assets/fonts/bundled/
```

Generate them with:

```bash
./scripts/grab_font_assets.py
```

The font files are git-ignored by default. Review the generated
`FONT_ASSET_MANIFEST.json` and license files, then force-add or IPFS-track the
assets when accepted. The current script downloads Inter Display for UI/dialog
text and JetBrains Mono for debug/monospace text; both are distributed under the
SIL Open Font License.

## Battery / power plan

See `docs/planning/android-power.md` for the current battery-life plan. The first
steps are platform-feature cleanup, static-first Android LDtk loading, bundled
fonts, frame-pacing settings, background throttling, and keeping phone builds
free of desktop debug overlays unless explicitly requested.
