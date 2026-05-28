//! Keyboard/gamepad semantic input model for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine still consumes a compact `ControlFrame`, which keeps movement
//! physics independent from keyboards, gamepads, UI rebinding, or replay input.

use crate::engine_core as ae;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::*;

mod actions;
mod control;
mod menu;
mod presets;

#[cfg(test)]
mod tests;

#[cfg(feature = "input")]
pub use actions::SandboxAction;
pub use control::{ControlFrame, PlayerDashTriggerState};
pub use menu::{analog_to_dir, MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState};
pub use presets::{ActionKeys, KeyboardPreset, MovementKeys, PresetId, GAMEPAD_MAP};
