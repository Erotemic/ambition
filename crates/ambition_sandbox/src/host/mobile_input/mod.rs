//! Mobile / touch input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + controller-like touch buttons. The
//! Leafwing keyboard/gamepad pipeline is the canonical desktop input
//! surface; this module translates touch joystick + virtual buttons
//! into the same `ControlFrame` resource the simulator already
//! consumes.
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
//! - [`layout`] — touch HUD positions + visible-circle hit testing;
//!   `mobile_touch`-gated.
//! - [`menu_bridge`] — touch/mouse/joystick → `ControlFrame` /
//!   `MenuControlFrame` merge; `mobile_touch`-gated.
//! - [`bevy_plugin`] — system registration, spawning, visuals,
//!   resource/component definitions; `mobile_touch`-gated.
//!
//! Tests live in `mobile_input/tests.rs`. See `TODO.md` →
//! "Android demo touch controls" for the full plan.

mod state;

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
