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
python3 -m unittest scripts.tests.test_package_asset_guard
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

## Assets: two roots in, one tree out

A dev checkout composes **two** asset roots. A package has **one** assets dir,
and both roots merge into it, content last:

| Source | Checkout root | In the APK |
| --- | --- | --- |
| default | `crates/ambition_actors/assets` | `app/src/main/assets/` |
| `game://` | `game/ambition_content/assets` | the same dir, overlaid |

Merging in that order reproduces the precedence `ProviderGameAssetReader`
applies in a checkout (authored content wins, the shared generated tree backs
it), and it is the same merge `deploy_to_steamdeck.sh` performs. Because the two
roots collapse to one, the packaged build resolves `game://` through the
platform's own reader — the APK AssetManager — instead of the checkout-only
two-directory fallback; see `game_asset_source_builder` in
[`game/ambition_app/src/app/cli.rs`](../../game/ambition_app/src/app/cli.rs).

`ambition_content/assets/sprites` is a symlink into the actors tree for LDtk's
relative tileset paths. Packaging skips it: those images already arrive from the
first root, and AssetManager cannot follow a symlink.

Payload PNGs/audio are git-ignored but expected on disk (see `AGENTS.md`). Before
Rust compilation, `scripts/package_asset_guard.py` composes both roots and emits
a path-and-SHA-256 contract. The build fails on missing declared assets,
case-colliding names, symlinks, conflicting two-root paths, or copied bytes that
do not match the desktop source view. After Gradle finishes, the script opens
the final APK and verifies every contracted file under `assets/`; a healthy
staging directory is not accepted as proof that the APK is healthy.

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
python3 -m unittest scripts.tests.test_package_asset_guard
```

For a failure, capture the exact script command, selected ABI/profile/features,
`adb` serial, and filtered logcat output. Inspect the script and generated
project rather than adding undocumented manual steps.
