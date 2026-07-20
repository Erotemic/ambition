//! Mobile / touch presentation-input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + controller-like touch buttons. This crate owns
//! both the rendered touch HUD and the fold from touch joystick / virtual button
//! state into the same `ControlFrame` resource consumed by the simulator. The
//! Leafwing keyboard/gamepad pipeline remains the canonical desktop input
//! surface.
//!
//! Extracted from `ambition_app::host::mobile_input` (app-thinness, ADR 0019):
//! reusable touch presentation/input infrastructure any platformer host would
//! want, so it lives beside the input/render seams rather than inside the app
//! binary. Track 7 split: the SEMANTIC half (raw touch state → `ControlFrame`,
//! [`mod@state`]) is pure data on the `ambition_input` seam alone — no Bevy,
//! no render stack; every PRESENTATION dependency (`bevy`, `ambition_render`,
//! `ambition_actors`, `ambition_ui_nav`, `ambition_cutscene`,
//! `ambition_persistence`, `virtual_joystick`) is optional and enabled only by
//! the `mobile_touch` overlay feature, whose direct `ambition_render` edge is
//! intentional: the overlay draws its own quads and text.
//!
//! Two layers:
//!
//! 1. **Pure state (always built)** — [`TouchInputState`]/[`TouchButton`]
//!    plus [`apply_deadzone`]: the raw virtual-device state. Pure data,
//!    unit-tested, no Bevy / `virtual_joystick` dep. See [`mod@state`].
//! 2. **Bevy plugin (gated behind `mobile_touch`)** — collects
//!    `virtual_joystick` stick + button UI state into [`mod@state`], then
//!    exposes it to leafwing as VIRTUAL-DEVICE input kinds
//!    ([`mod@virtual_device`]) bound in the persistent participant's
//!    `InputMap` — touch resolves through bindings and the active input
//!    context exactly like a keyboard or gamepad. Lives in
//!    [`mod@bevy_plugin`].
//!
//! ## Submodule layout
//!
//! - [`state`] — pure types ([`TouchInputState`], [`TouchButton`],
//!   [`apply_deadzone`]); always built.
//! - [`exclusion`] — ECS marker + pure hit-test helpers for touch UI
//!   regions that should not become menu drag-scroll gestures;
//!   `mobile_touch`-gated.
//! - [`layout`] — touch HUD positions + visible-circle hit testing;
//!   `mobile_touch`-gated.
//! - [`virtual_device`] — the leafwing input kinds over the touch state +
//!   the participant binding table; `mobile_touch`-gated.
//! - [`menu_bridge`] — the pointer-GESTURE lane (drag-scroll) and the
//!   touch active-input marker; `mobile_touch`-gated.
//! - [`bevy_plugin`] — system registration, spawning, visuals,
//!   resource/component definitions; `mobile_touch`-gated.
//!
//! Tests live in `tests.rs`.

// The pure touch STATE + fold. Its consumers (`bevy_plugin`, `menu_bridge`) are
// `mobile_touch`-gated, but the module compiles unconditionally so its unit tests
// run in every build. Without the feature, most of it is legitimately unreachable.
#[cfg_attr(not(feature = "mobile_touch"), allow(dead_code))]
mod state;

#[cfg(feature = "mobile_touch")]
pub mod exclusion;

#[cfg(feature = "mobile_touch")]
pub mod layout;

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

/// Bevy plugin wiring `virtual_joystick` to the `ControlFrame` seam.
/// Gated behind the `mobile_touch` feature so desktop / gamepad /
/// headless / RL builds don't pull in `virtual_joystick` and don't
/// register the touch systems.
#[cfg(feature = "mobile_touch")]
pub mod bevy_plugin;

/// The touch-controls Bevy plugin — the single entry point the host adds. Re-exported
/// at the crate root so the host wires `ambition_touch_input::TouchControlsPlugin`
/// without reaching into the submodule.
#[cfg(feature = "mobile_touch")]
pub use bevy_plugin::TouchControlsPlugin;
