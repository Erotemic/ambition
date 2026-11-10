---
status: current
last_verified: 2026-10-31
---

# Android build and sideload

The supported interface is the repository-root build script. Its `--help` and
`--doctor` output are authoritative.

## Prerequisites

```bash
./scripts/setup/android_prereqs.sh --doctor
./scripts/setup/android_prereqs.sh
./build_for_android.sh --doctor
```

The setup helper manages/checks the Android SDK/NDK, Rust target, `cargo-ndk`,
Gradle, and `adb`. Re-run `--doctor` after shell/environment changes.

## Common workflows

```bash
# Build a debug-signed APK with the default release Rust profile.
./build_for_android.sh

# Build, install, launch, and follow filtered logs on an attached device.
./build_for_android.sh --run

# Select a device or use/create an emulator.
./build_for_android.sh --run --device <SERIAL>
./build_for_android.sh --run --emulator <AVD_NAME>
./build_for_android.sh --run --create-emulator <AVD_NAME>

# Size-oriented build and report.
./build_for_android.sh --size-profile --fresh-install
```

Use `./build_for_android.sh --help` for current ABI, SDK, feature, profile,
static-asset, logging, and output options. Do not manually maintain a parallel
Gradle recipe in documentation.

## Assets: one selected package tree

`scripts/package/package_asset_guard.py` selects the game's files and the content
libraries that its pack declares. It uses the same library inventory as the
desktop package. Game files keep their relative paths. Library files use
`__libraries/<library-id>/` under `app/src/main/assets/`. The runtime resolves
`content://<library-id>/...` through the APK AssetManager at that package path.

The generated Gradle project replaces Android's default asset filter. The
default excludes directories that start with `_`, including `__libraries/`.
The replacement matches only the current directory (`<dir>.`), so Gradle keeps
all files that the package guard selects. An empty ignore list restores the
Android defaults and must not be used to disable the filter.

Generated images and audio are ignored by Git but must be present on the build
host. Before Rust compilation, the package guard writes the selected tree and
a path-and-SHA-256 contract. It asks the content compiler for pack declarations.
See [the package guard guide](../tools/packaged-asset-guard.md) for validation
rules and the policy for absent pack assets.

After Gradle finishes, the build script opens the final APK and checks every
contracted file under `assets/`. It checks both paths and bytes. A correct
staging directory does not prove that the APK contains those files.

The generated project sets `android.experimental.forceDeterministicApk=true`.
Changed package inputs produce a fresh APK. This avoids an integer overflow in
incremental ZIP updates when entry offsets differ by more than 2 GiB. Rust,
asset compression, and Java build outputs remain cached. The build script
requests Gradle stack traces for packaging failures.

## Architecture rules

- Platform/device policy belongs in host/platform composition.
- Game/provider assets are packaged through the same provider/runtime contracts
  used on desktop and web.
- Touch emits semantic control actions; gameplay does not branch on Android.
- Missing audio/window/device capabilities may degrade presentation but cannot
  alter authoritative simulation.

## Validate on the affected device

Use `./build_for_android.sh --doctor` for prerequisite failures. Build and run
the selected profile on the relevant device or emulator, then exercise the
changed input, packaging or presentation behavior. Run focused Rust tests for
shared components when they would catch a specific regression; the touch,
input and content suites are not mandatory for every APK build.

For a failure, record the command, ABI/profile/features, device ID and filtered
logcat output. Inspect the build script and generated project rather than
maintaining a parallel manual build procedure.
