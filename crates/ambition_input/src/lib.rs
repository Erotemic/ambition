//! Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.
//!
//! Physical inputs are bound to `Platformer2dInputActionMonolith` with Leafwing Input Manager.
//! The engine-owned compact `ControlFrame` keeps movement physics independent
//! from keyboards, gamepads, UI rebinding, or replay input.
//!
//! This is the upper-sibling input abstraction (ADR 0019): it depends DOWN on
//! `ambition_platformer2d_core` for the `ControlFrame` vocabulary and on the
//! input-domain `settings` (deadzones / trigger hysteresis / burst mode), but
//! NEVER on `ambition_platformer2d_actor_monolith` or `ambition_characters`.
//!
//! TODO(compat-remove): migrate remaining `crate::ControlFrame` callers to
//! `ambition_platformer2d_core::ControlFrame`, then remove the re-export.

use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::*;

mod actions;
mod active_input;
#[cfg(feature = "input")]
mod bindings;
pub mod channels;
mod control;
pub mod cues;
#[cfg(feature = "input")]
mod glyphs;
#[cfg(feature = "input")]
pub mod layout;
#[cfg(feature = "input")]
mod local_seats;
mod menu;
mod motion_input;
pub mod participant;
mod presets;
#[cfg(feature = "input")]
mod rebind;
pub mod seating;
pub mod semantic;
pub mod settings;
pub mod sources;

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
pub use actions::Platformer2dInputActionMonolith;
#[cfg(feature = "input")]
pub use active_input::update_seat_active_devices;
pub use active_input::{ActiveDevice, GamepadStyle, SeatActiveDevices, gamepad_style_of};
pub use ambition_platformer2d_core::ControlFrame;
/// Which local source drives which control channel — the map that keeps a
/// lobby's sparse source numbers out of the rollback host's dense handles.
pub use channels::{LocalChannelPlan, LocalInputSource};
#[cfg(feature = "input")]
pub use control::{
    read_gameplay_control_frame, read_gameplay_control_frame_with_settings, read_menu_control_frame,
};
#[cfg(feature = "input")]
pub use glyphs::glyph_for;
#[cfg(feature = "input")]
pub use local_seats::{
    LocalDeviceOrder, LocalSeatTopology, SeatDeviceOwnership as LocalSeatDeviceOwnership,
    assign_local_seat_devices, track_local_device_order,
};
pub use seating::{LocalSeatOffer, SessionSeatingSource};

/// Ordered participant-input pipeline. The host chains
/// `Collect -> ResolveActions -> ResolveContext -> Route -> PublishCues -> Consume`.
/// [`InputSet::Route`] is the publication boundary: every system that shapes a seat's semantic
/// frame runs inside it, and gameplay consumers run after it so they cannot observe stale input.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputSet {
    /// Device and virtual-device adapters produce raw device state.
    Collect,
    /// Bindings resolve device state into participant `ActionState`.
    ResolveActions,
    /// Context claims are declared; the active context resolves.
    ResolveContext,
    /// Actions + context shape each seat's frame, and the `MenuControlFrame` /
    /// semantic UI commands. Every stage that must run before the publication
    /// boundary lives here.
    Route,
    /// Resolved cue read-models publish for presenters.
    PublishCues,
    /// Shell/menu consumers of the routed semantics.
    Consume,
}
#[cfg(feature = "input")]
pub use bindings::{
    ActionBindings, BindingRecipe, BindingSources, PhysicalControl, SeatBindings, action_for_slot,
    action_name, action_named, publish_seat_bindings, rebuild_maps_from_recipes,
};
pub use cues::{ActiveUiCues, UiCue};
#[cfg(feature = "input")]
pub use layout::{BindingLayout, DeclaredBindingLayout, PadSlot};
pub use menu::{
    MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState, SeatMenuFrames, analog_to_dir,
};
pub use participant::{
    ActiveInputContext, CUTSCENE_CONTEXT, ContextClaim, DEBUG_CONTEXT, DIALOGUE_CONTEXT,
    GAMEPLAY_CONTEXT, INVENTORY_CONTEXT, InputContextId, InputParticipant, LAUNCHER_CONTEXT,
    PAUSE_CONTEXT, ParticipantContexts, ParticipantId, SELECT_CONTEXT, STARTUP_ACKNOWLEDGE_CONTEXT,
    SeatInputContexts, resolve_active_input_context,
};
#[cfg(feature = "input")]
pub use rebind::{also_bound_to, bindable, capture, pressed_controls_this_frame};
pub use settings::{BindingOverride, ControlFilters, OverrideControl, OverrideDeviceClass};
/// HOW LOCAL SOURCES BECOME PARTICIPANTS, and who owns the keyboard when
/// that is a question.
pub use sources::{InputAssignmentPolicy, KeyboardOwner};
// `key_name` joins this list rather than the module being opened: the crate
// exposes a chosen surface, and a HUD legend needs exactly one function from it.
pub use presets::{ActionKeys, KeyboardPreset, MovementKeys, PresetId, key_name};
pub use semantic::{
    ActionConflict, ActionControlKind, ActionRegistry, ENGINE_ACTIONS, ENGINE_CAPABILITY,
    InstalledActions, SemanticActionDef, SemanticActionId,
};
