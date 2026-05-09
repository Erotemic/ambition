# Lessons learned

This journal records unexpected errors encountered while iterating on the Ambition sandbox, especially places where an overlay or generated build script looked reasonable but failed in a real local/device test. The goal is to make future LLM-generated patches less likely to repeat the same mistakes.

## 2026-05-08: Android APK bring-up

### Prefer generated Android projects, but keep the generated Java/Gradle side explicit

The first Android APK path successfully built the Rust shared library with `cargo-ndk`, but Gradle/device launch uncovered several Java-side assumptions. Each failure happened before Bevy gameplay started, so the fix belonged in the generated Android shell rather than in gameplay code.

Observed fixes:

- `android.useAndroidX=true` is required because GameActivity is distributed through AndroidX artifacts.
- The manifest should launch an app-local `.MainActivity`, not a Maven-coordinate-looking class such as `androidx.games.activity.GameActivity`.
- `MainActivity` should extend `com.google.androidgamesdk.GameActivity`.
- GameActivity extends `AppCompatActivity`, so the app needs both `androidx.appcompat:appcompat` and an AppCompat-derived theme.
- Transitive Kotlin dependencies may mix old `kotlin-stdlib-jdk7/jdk8` artifacts with newer `kotlin-stdlib`; the generated Gradle project now aligns Kotlin artifacts and excludes obsolete compatibility jars.
- A repo-local Gradle user home under `target/android/gradle-user-home` avoids unrelated host `~/.gradle` cache permissions breaking this project.

### Do not assume adb install flags are portable

I suggested `adb install --no-stream`, but the target device rejected it as an unknown package-manager option. The build script should prefer conservative install flags (`-r -d --install-location 0`) and provide a `--fresh-install` mode that force-stops/uninstalls first.

### Overlay patches must not clobber platform entrypoints

A later Android usability overlay replaced `crates/ambition_sandbox/src/lib.rs` from a source snapshot that did not contain the Android shared-library entry point. The APK still built and installed, but launch failed with:

```text
UnsatisfiedLinkError: dlopen failed: cannot locate symbol "android_main" referenced by libambition_sandbox.so
```

The lesson is that files touched by multiple overlay series need special care. Before overwriting `lib.rs`, `Cargo.toml`, or generated build scripts, preserve platform-critical entrypoints and feature definitions added by earlier overlays.

For Bevy Android GameActivity builds, the Rust library must export `android_main`. In this project the intended pattern is:

```rust
#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    app::run_visible();
}
```

Desktop still enters through `src/main.rs`; Android packages the library as `libambition_sandbox.so` and needs Bevy's `#[bevy_main]` macro to generate the Android boilerplate.

### Keep asset behavior platform-aware

Android packages runtime assets into the APK. Host-side `CARGO_MANIFEST_DIR/assets/...` existence checks are not valid on-device. On Android, let Bevy's APK asset reader attempt the load; on desktop, host-side existence checks are still useful for clearer diagnostics.

### Treat device logs as the source of truth

The Android sequence progressed through distinct phases:

1. APK installed but manifest activity class was missing.
2. Java activity compiled but AppCompat dependency/theme was missing.
3. Native library loaded but `android_main` was missing.

Each phase required a different layer of the stack to be fixed. Avoid guessing from the symptom alone; use `adb logcat` and identify whether the failure is Gradle, install/package-manager, Java activity startup, native library loading, or Rust/Bevy runtime.

## 2026-05-08: Keep Android HUD defaults and menu toggles separate

The Android build can boot with the same desktop sandbox systems, but phone usability needs
coarse user-facing switches for large overlays. Do not only change `DeveloperTools::default`
when a HUD is too large: add an explicit persisted setting and make the render system clear
its text when the setting is off. Quest/objective UI and debug HUD text should be controlled
separately because the quest panel is useful during play while the debug dump can consume most
of a phone screen.


## 2026-05-08: Android size is a separate profile and platform-composition problem

A large Android APK/native library should not immediately trigger semantic
feature-gate churn. First separate the size mechanics from the gameplay feature
set:

- build Android with `--no-default-features --features android` so desktop-only
  inspector/file-watcher tooling does not enter the phone artifact by default;
- keep the playable sandbox, touch controls, audio, LDtk runtime, UI, and RL/test
  seams in the Android composite feature;
- add a dedicated `android-size` Cargo profile before removing gameplay systems;
- strip the final `.so` explicitly with the NDK `llvm-strip` as a backstop;
- print before/after sizes so future patches compare measurements instead of
  guessing from APK size alone.

The principle is platform composition, not release minimalism: Android can remain
a dev/test build while excluding desktop inspector/editor conveniences that are
not useful on a phone screen.


## 2026-05-08: Android APK assets are not regular files

The Android build copied `assets/audio/sfx.bank` into the APK, but the game still
fell back to generated/fundsp SFX. The reason was that the SFX bank loader used
`std::fs` and normal paths such as `/assets/audio/sfx.bank`; packaged APK assets
are not visible at those paths on-device. Bevy's `AssetServer` can load many
runtime assets from the APK, but this specific SFX-bank path is a synchronous
custom loader built around `BankProvider::from_path` / `from_bytes`.

Temporary fix: let `build_for_android.sh` statically embed the SFX bank with a
separate `static_sfx_bank` feature when the bank exists locally. Long-term fix:
teach the SFX bank loader to read bytes from Android APK assets or route it
through Bevy asset loading, then remove the static embedding workaround.

The lesson is to distinguish "copied into APK assets" from "readable via
`std::fs`". Any custom synchronous loader needs an explicit Android asset path,
static fallback, or Bevy asset pipeline bridge.

## 2026-05-08: Size diagnostics should be automatic for phone builds

A 200 MiB native library became a much more reasonable ~49 MiB `.so` after using
a size-oriented Cargo profile, disabling desktop-only default features for
Android, and stripping with the NDK toolchain. Future Android patches should keep
printing `.so`, APK, and asset-tree sizes so we notice regressions immediately.
