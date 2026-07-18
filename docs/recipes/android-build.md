---
status: current
last_verified: 2026-07-18
---

# Android build and sideload

The supported interface is the repository-root build script. Its `--help` and
`--doctor` output are authoritative.

## Prerequisites

```bash
./scripts/setup_android_prereqs.sh --doctor
./scripts/setup_android_prereqs.sh
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

## Architecture rules

- Platform/device policy belongs in host/platform composition.
- Game/provider assets are packaged through the same provider/runtime contracts
  used on desktop and web.
- Touch emits semantic control actions; gameplay does not branch on Android.
- Missing audio/window/device capabilities may degrade presentation but cannot
  alter authoritative simulation.

## Validate before device testing

```bash
./run_tests.sh -p ambition_touch_input
./run_tests.sh -p ambition_input
./run_tests.sh -p ambition_content
./build_for_android.sh --doctor
```

For a failure, capture the exact script command, selected ABI/profile/features,
`adb` serial, and filtered logcat output. Inspect the script and generated
project rather than adding undocumented manual steps.
