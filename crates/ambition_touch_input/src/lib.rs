//! Touch input adapter and on-screen controls.
//!
//! The always-built [`state`] module contains pure touch state and deadzone logic. The
//! `mobile_touch` feature adds layout, menu gestures, virtual-device bindings, and the
//! Bevy presentation plugin. Touch is lowered through the same participant bindings and
//! active input contexts as keyboard/gamepad input.

// The pure touch STATE vocabulary. Its consumers (`bevy_plugin`,
// `virtual_device`, `menu_bridge`) are `mobile_touch`-gated, but the module
// compiles unconditionally so its unit tests run in every build.
#[cfg_attr(not(feature = "mobile_touch"), allow(dead_code))]
mod state;

#[cfg(feature = "mobile_touch")]
pub mod layout;

#[cfg(feature = "mobile_touch")]
pub mod placement;

#[cfg(feature = "mobile_touch")]
pub mod menu_bridge;

#[cfg(feature = "mobile_touch")]
pub mod virtual_device;

#[cfg(test)]
mod tests;

// `TouchButton` is referenced by `bevy_plugin::super::TouchButton`; keep
// it re-exported so the plugin can construct buttons without a deeper
// import path.
pub use state::{apply_deadzone, TouchButton, TouchInputState};

/// Bevy plugin wiring `virtual_joystick` into the participant action seam.
/// Gated behind the `mobile_touch` feature so desktop / gamepad /
/// headless / RL builds don't pull in `virtual_joystick` and don't
/// register the touch systems.
#[cfg(feature = "mobile_touch")]
pub mod bevy_plugin;

/// The touch-controls Bevy plugin — the single entry point the host adds. Re-exported
/// at the crate root so the host wires `crate::TouchControlsPlugin`
/// without reaching into the submodule.
#[cfg(feature = "mobile_touch")]
pub use bevy_plugin::TouchControlsPlugin;
