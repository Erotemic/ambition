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
pub mod cues;
mod menu;
mod motion_input;
pub mod participant;
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

/// Schedule contract for the participant input pipeline (one frame, in
/// order): device adapters complete before routing, and every routed output
/// completes before the shell/menu consumers read it — an edge produced this
/// frame is consumed this frame, never "at worst a frame late".
///
/// The stages, chained by the host input plugin:
///
/// 1. [`InputSet::Collect`] — device and virtual-device adapters produce this
///    frame's raw device state (touch state, joystick messages). Physical
///    devices are read upstream by bevy/leafwing in `PreUpdate`; this is the
///    `Update`-side adapter stage.
/// 2. [`InputSet::ResolveActions`] — bindings resolve device state into the
///    participant's `ActionState` (virtual-device merges into leafwing's
///    already-ticked state land here).
/// 3. [`InputSet::ResolveContext`] — surfaces declare/retract their
///    [`participant::ContextClaim`]s; [`participant::ActiveInputContext`]
///    resolves at the end of the set.
/// 4. [`InputSet::Route`] — actions + the active context route into the
///    semantic seams. Every system that WRITES the `ControlFrame` resource
///    (device populate, touch fold, portal movement-intent brackets,
///    edge-derived flags) and the `MenuControlFrame` lives here; every system
///    that READS them to drive gameplay runs after it. The sandbox pins
///    `Route` before its gameplay consumer (`populate_slot_controls`), so a
///    writer can never "float" past the consume boundary and stamp stale
///    input over the fresh frame — the regression that once killed the Move
///    axis.
/// 5. [`InputSet::PublishCues`] — resolved cue read-models publish for
///    presenters (labels, glyphs, touch-button contracts).
/// 6. [`InputSet::Consume`] — shell/menu consumers of the routed semantics.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputSet {
    /// Device and virtual-device adapters produce raw device state.
    Collect,
    /// Bindings resolve device state into participant `ActionState`.
    ResolveActions,
    /// Context claims are declared; the active context resolves.
    ResolveContext,
    /// Actions + context route into `ControlFrame` / `MenuControlFrame` /
    /// semantic UI commands. All `ControlFrame`-writing systems live here.
    Route,
    /// Resolved cue read-models publish for presenters.
    PublishCues,
    /// Shell/menu consumers of the routed semantics.
    Consume,
}
pub use cues::{ActiveUiCues, UiCue};
pub use menu::{analog_to_dir, MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState};
pub use participant::{
    resolve_active_input_context, ActiveInputContext, ContextClaim, InputContextId,
    InputParticipant, ParticipantContexts, ParticipantId, GAMEPLAY_CONTEXT, LAUNCHER_CONTEXT,
    STARTUP_ACKNOWLEDGE_CONTEXT,
};
pub use presets::{ActionKeys, KeyboardPreset, MovementKeys, PresetId, GAMEPAD_MAP};
