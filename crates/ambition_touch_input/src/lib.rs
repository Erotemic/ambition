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
//! binary. The direct `ambition_render` edge is intentional: the crate draws its
//! own overlay quads and text. The module has no app-only coupling — it
//! reads/writes only the `ambition_input` / `ambition_actors` /
//! `ambition_render` / `ambition_ui_nav` / `ambition_cutscene` library seams.
//!
//! Two layers:
//!
//! 1. **Pure helper (always built)** — [`fold_touch_into_control_frame`]
//!    takes a [`TouchInputState`] plus a deadzone and returns a
//!    `ControlFrame`. Pure data, unit-tested, no Bevy /
//!    `virtual_joystick` dep. RL agents, tests, and the Bevy systems
//!    all share this. See [`mod@state`].
//! 2. **Bevy plugin (gated behind `mobile_touch`)** — wires
//!    `virtual_joystick` Move + Aim sticks plus a small button UI to
//!    the helper, then writes `ControlFrame`. Lives in
//!    [`mod@bevy_plugin`].
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`state`] — pure types ([`TouchInputState`], [`TouchButton`],
//!   [`apply_deadzone`], [`fold_touch_into_control_frame`]); always
//!   built.
//! - [`exclusion`] — ECS marker + pure hit-test helpers for touch UI
//!   regions that should not become menu drag-scroll gestures;
//!   `mobile_touch`-gated.
//! - [`layout`] — touch HUD positions + visible-circle hit testing;
//!   `mobile_touch`-gated.
//! - [`menu_bridge`] — touch/mouse/joystick → `ControlFrame` /
//!   `MenuControlFrame` merge; `mobile_touch`-gated.
//! - [`bevy_plugin`] — system registration, spawning, visuals,
//!   resource/component definitions; `mobile_touch`-gated.
//!
//! Tests live in `tests.rs`.

mod state;

#[cfg(feature = "mobile_touch")]
pub mod exclusion;

#[cfg(feature = "mobile_touch")]
pub mod layout;

#[cfg(feature = "mobile_touch")]
pub mod menu_bridge;

#[cfg(test)]
mod tests;

// `TouchButton` is referenced by `bevy_plugin::super::TouchButton`; keep
// it re-exported so the plugin can construct buttons without a deeper
// import path. `apply_deadzone`/`fold_touch_into_control_frame`/
// `TouchInputState` are exercised only by the tests submodule, which
// reaches them via `super::state::*` and does not need a re-export.
pub use state::TouchButton;

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
