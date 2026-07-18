//! Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine-owned compact `ControlFrame` keeps movement physics independent
//! from keyboards, gamepads, UI rebinding, or replay input.
//!
//! This is the upper-sibling input abstraction (ADR 0019): it depends DOWN on
//! `ambition_engine_core` for the `ControlFrame` vocabulary and on the
//! input-domain `settings` (deadzones / trigger hysteresis / dash mode), but
//! NEVER on `ambition_actors` or `ambition_characters`. The legacy
//! `ambition_input::ControlFrame` path remains as a re-export for app/input
//! adapters; reusable brains import the lower engine-core vocabulary directly.

use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::*;

mod actions;
mod active_input;
mod control;
mod menu;
mod motion_input;
mod presets;
pub mod settings;

#[cfg(test)]
mod tests;

/// Directional motion recognition (a rolling input buffer + a generic
/// subsequence matcher) and the open, content-owned technique registry. Pure +
/// headless; a game registers its own named gestures and the special-move gate
/// consumes them.
pub use motion_input::{
    MotionDirection, MotionInputBuffer, MotionSample, MotionTechnique, MotionTechniqueAppExt,
    MotionTechniqueCatalog,
};

#[cfg(feature = "input")]
pub use actions::SandboxAction;
pub use active_input::{update_active_input_kind, ActiveInputKind};
pub use ambition_engine_core::ControlFrame;
pub use control::PlayerDashTriggerState;
#[cfg(feature = "input")]
pub use control::{
    read_gameplay_control_frame, read_gameplay_control_frame_with_settings, read_menu_control_frame,
};

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
