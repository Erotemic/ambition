//! Platform-aware setup for the sandbox.
//!
//! Hosts cross-cutting "where am I running and what does that
//! mean" concerns: power policy, window-focus tracking, default
//! aggressiveness for nonessential per-frame work, and a single
//! entry point ([`PlatformPlugin`]) that wires the right
//! per-platform plugin (desktop vs Android) into a Bevy app.
//!
//! The point is to keep `#[cfg(target_os = "android")]` guards
//! confined to this module tree rather than spread across gameplay
//! code. A new "what does the phone need this frame?" question
//! should grow as a function under [`platform::power`] or
//! [`platform::android`], not as a fresh cfg branch in some
//! gameplay system.
//!
//! ## Submodule layout
//!
//! - [`power`] — `PowerProfile`, `WindowFocusState`,
//!   `track_window_focus`, `should_pause_nonessential_work`.
//! - [`android`] — Android-only platform plugin and defaults.
//! - [`desktop`] — Desktop-only platform plugin and defaults.

use bevy::prelude::*;

pub mod android;
pub mod desktop;
pub mod power;

use power::{track_window_focus, WindowFocusState};

/// Top-level platform plugin. Picks the right per-platform plugin
/// (desktop vs Android), wires the focus-tracking system, and
/// inserts the [`WindowFocusState`] resource.
///
/// The visible-app builder ([`crate::app::add_simulation_plugins`])
/// adds this plugin once per build; headless / RL builds skip it
/// because they don't have a window to track focus on.
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowFocusState>();
        app.add_systems(Update, track_window_focus);

        #[cfg(target_os = "android")]
        app.add_plugins(android::AndroidPlatformPlugin);

        #[cfg(not(target_os = "android"))]
        app.add_plugins(desktop::DesktopPlatformPlugin);
    }
}
