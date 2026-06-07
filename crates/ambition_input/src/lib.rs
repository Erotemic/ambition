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
mod active_input;
mod control;
mod menu;
mod presets;
pub mod settings;

#[cfg(test)]
mod tests;

#[cfg(feature = "input")]
pub use actions::SandboxAction;
pub use active_input::{update_active_input_kind, ActiveInputKind};
pub use control::{ControlFrame, PlayerDashTriggerState};

/// Schedule contract for the per-frame [`ControlFrame`] input window.
///
/// Every system that WRITES the `ControlFrame` resource (device populate,
/// touch fold, portal movement-intent brackets, edge-derived flags) runs in
/// [`InputSet::Populate`]; every system that READS `ControlFrame` to drive
/// gameplay runs after it. The sandbox pins `Populate` before its gameplay
/// consumer (`sync_local_player_input_frame`), so a writer can never "float"
/// past the consume boundary and stamp stale input over the fresh frame — the
/// regression that recently killed the Move axis.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputSet {
    /// All `ControlFrame`-writing systems live here.
    Populate,
}
pub use menu::{analog_to_dir, MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState};
pub use presets::{ActionKeys, KeyboardPreset, MovementKeys, PresetId, GAMEPAD_MAP};
