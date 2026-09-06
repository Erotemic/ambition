//! Platform-aware setup for the sandbox.
//!
//! Single entry point ([`PlatformPlugin`]) that wires the right
//! per-platform plugin (desktop vs Android) into a Bevy app. Keeps
//! `#[cfg(target_os = "android")]` guards confined to this module
//! tree rather than spread across gameplay code.

use bevy::prelude::*;

pub mod android;
pub mod desktop;

/// Top-level platform plugin. Picks the right per-platform plugin
/// (desktop vs Android).
///
/// The visible-app builder ([`crate::app::add_simulation_plugins`])
/// adds this plugin once per build; headless / RL builds skip it.
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_os = "android")]
        app.add_plugins(android::AndroidPlatformPlugin);

        #[cfg(not(target_os = "android"))]
        app.add_plugins(desktop::DesktopPlatformPlugin);
    }
}
