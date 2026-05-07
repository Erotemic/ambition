#!/usr/bin/env bash
# Desktop-only run script. Builds for the host platform (Linux x86-64
# in this dev VM), NOT for Android. Default features include
# `mobile_touch` (pulls `virtual_joystick`) and `rl` (SandboxSim
# binaries) -- both compile cleanly on desktop and are useful even
# without a phone in the loop. To strip them for a smaller / faster
# build, switch the cargo line to:
#   cargo run -p ambition_sandbox --bin ambition_sandbox \
#       --no-default-features --features visible,dev_hot_reload --release
#
# An actual Android APK build is NOT produced by this script and
# would require a separate `cargo apk` / `cargo ndk` toolchain plus
# an Android NDK install. Nothing here invokes either of those.
set -euo pipefail
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload --release
