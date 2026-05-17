//! Android-specific platform setup.
//!
//! Today this is a stub: the `android_main` entrypoint lives at
//! `crate::lib`'s root (where `#[bevy_main]` can attach to it), and
//! the build script + `target_os = "android"` cfg guards are scattered
//! across the rest of the sandbox. This module is the home for any
//! *future* Android-only systems — phone-side battery hooks, doze /
//! suspend handling, app-internal-storage path resolution, audio focus
//! integration, etc.
//!
//! The aim is to keep `target_os = "android"` cfg guards inside this
//! file rather than scattered across gameplay code. A new "what
//! does the phone need this frame?" question should grow as a
//! function here.

use bevy::prelude::*;

use super::power::PowerProfile;

/// Pick a sensible default `PowerProfile` for the Android build.
///
/// Default is `BatterySaver`: phones run on battery, and the user
/// can flip to `Performance` if they want maximum FPS while plugged
/// in.
pub fn default_power_profile() -> PowerProfile {
    PowerProfile::BatterySaver
}

/// Bevy plugin for Android-only setup. Today this only sets the
/// initial [`PowerProfile`] resource; future hooks (audio focus
/// listener, OS suspend → app pause) live here.
pub struct AndroidPlatformPlugin;

impl Plugin for AndroidPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(default_power_profile());
    }
}
