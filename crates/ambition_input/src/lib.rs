//! Device -> `ControlFrame` input layer for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine still consumes a compact `ControlFrame`, which keeps movement
//! physics independent from keyboards, gamepads, UI rebinding, or replay input.
//!
//! This is the upper-sibling input abstraction (ADR 0019): it depends DOWN on
//! `ambition_engine_core` (to map a `ControlFrame` into `engine_core::InputState`)
//! and on the input-domain `settings` (deadzones / trigger hysteresis / dash
//! mode), but NEVER on `ambition_sandbox`. The sandbox re-exports this crate as
//! `crate::input` so all existing `crate::input::{ControlFrame, SandboxAction, …}`
//! paths resolve unchanged.

use ambition_engine_core as ae;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::*;

mod actions;
mod control;
mod menu;
mod presets;
pub mod settings;

#[cfg(test)]
mod tests;

#[cfg(feature = "input")]
pub use actions::SandboxAction;
pub use control::{ControlFrame, PlayerDashTriggerState};
pub use menu::{analog_to_dir, MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState};
pub use presets::{ActionKeys, KeyboardPreset, MovementKeys, PresetId, GAMEPAD_MAP};
