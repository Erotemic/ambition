//! Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.
//!
//! Physical inputs are bound to `Platformer2dInputActionMonolith` with Leafwing Input Manager.
//! The engine-owned compact `ControlFrame` keeps movement physics independent
//! from keyboards, gamepads, UI rebinding, or replay input.
//!
//! This is the upper-sibling input abstraction (ADR 0019). It depends on
//! `ambition_platformer2d_core` for `ControlFrame` and on the input-domain
//! `settings` (deadzones, trigger hysteresis, burst mode). It never depends
//! on `ambition_platformer2d_actor_monolith` or `ambition_characters`.
//!
//! TODO(compat-remove): migrate the remaining `crate::ControlFrame` caller
//! (`ambition_touch_input::bevy_plugin`) to
//! `ambition_platformer2d_core::ControlFrame`, then remove the re-export.
//! That caller would need a new manifest edge onto an "app-thinness" crate
//! (ADR 0019), so it needs an architecture decision, not a rename.

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

/// Directional motion recognition (a rolling input buffer and a subsequence
/// matcher) and the content-owned technique registry. Pure and headless. A
/// game registers its own gestures, and the special-move gate reads them.
pub use motion_input::{
    MotionDirection, MotionInputBuffer, MotionSample, MotionTechnique, MotionTechniqueAppExt,
    MotionTechniqueCatalog,
};

#[cfg(feature = "input")]
pub use actions::Platformer2dInputActionMonolith;
#[cfg(feature = "input")]
pub use active_input::update_seat_active_devices;
pub use active_input::{gamepad_style_of, ActiveDevice, GamepadStyle, SeatActiveDevices};
pub use ambition_platformer2d_core::AttackStrengthHint;
pub use ambition_platformer2d_core::ControlFrame;
/// Maps each local source to a control channel, so a lobby's sparse source
/// numbers stay out of the rollback host's dense handles.
pub use channels::{LocalChannelPlan, LocalInputSource};
#[cfg(feature = "input")]
pub use control::{
    read_gameplay_control_frame, read_gameplay_control_frame_with_settings, read_menu_control_frame,
};
#[cfg(feature = "input")]
pub use glyphs::glyph_for;
#[cfg(feature = "input")]
pub use local_seats::{
    assign_local_seat_devices, track_local_device_order, LocalDeviceOrder, LocalSeatTopology,
    SeatDeviceOwnership as LocalSeatDeviceOwnership,
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
    action_for_slot, action_name, action_named, publish_seat_bindings, rebuild_maps_from_recipes,
    swallow_the_rebinds_own_edges, ActionBindings, BindingRecipe, BindingSources, PhysicalControl,
    Rebound, SeatBindings,
};
pub use cues::{ActiveUiCues, UiCue};
#[cfg(feature = "input")]
pub use layout::{BindingLayout, DeclaredBindingLayout, PadSlot};
pub use menu::{
    analog_to_dir, MenuControlFrame, MenuDir, MenuInputFrame, MenuInputState, SeatMenuFrames,
};
pub use participant::{
    resolve_active_input_context, ActiveInputContext, ContextClaim, InputContextId,
    InputParticipant, ParticipantContexts, ParticipantId, SeatInputContexts, CUTSCENE_CONTEXT,
    DEBUG_CONTEXT, DIALOGUE_CONTEXT, GAMEPLAY_CONTEXT, INVENTORY_CONTEXT, LAUNCHER_CONTEXT,
    PAUSE_CONTEXT, SELECT_CONTEXT, STARTUP_ACKNOWLEDGE_CONTEXT,
};
#[cfg(feature = "input")]
pub use rebind::{also_bound_to, bindable, capture, pressed_controls_this_frame};
pub use settings::{BindingOverride, ControlFilters, OverrideControl, OverrideDeviceClass};
/// How local sources become participants, and who owns the keyboard.
pub use sources::{InputAssignmentPolicy, KeyboardOwner};
// Export only `key_name`, not the whole module: a HUD legend needs only it.
pub use presets::{key_name, ActionKeys, KeyboardPreset, MovementKeys, PresetId};
pub use semantic::{
    ActionConflict, ActionControlKind, ActionRegistry, InstalledActions, SemanticActionDef,
    SemanticActionId, ENGINE_ACTIONS, ENGINE_CAPABILITY,
};
// The provider-action road: a registered action reaches a device binding and
// a press comes back out. Gated with the rest of the leafwing surface.
#[cfg(feature = "input")]
pub use semantic::{
    install_provider_bindings_on_seats, publish_provider_action_edges, ProviderAction,
    ProviderBindings, SemanticActionPressed,
};

/// Install the input pipeline's set order and device-ownership systems.
///
/// This crate owns the `InputSet` chain and the systems, so compositions do
/// not restate the order.
///
/// Keep `configure_sets` with the systems. Without it the sets are unordered
/// and fail silently: an edge produced this frame may be consumed a frame
/// late. The order is the contract: device adapters complete before routing,
/// and routed semantics before shell/menu consumers.
///
/// The device systems run in `PreUpdate` before
/// `InputManagerSystem::Update`, because the seat/device association is an
/// input to leafwing's action resolution. After it, a joining seat reads its
/// controller a frame late and the join press reaches no seat.
#[cfg(feature = "input")]
pub fn install_input_pipeline(app: &mut bevy::prelude::App) {
    use bevy::prelude::{IntoScheduleConfigs as _, PreUpdate, Update};

    app.configure_sets(
        Update,
        (
            InputSet::Collect,
            InputSet::ResolveActions,
            InputSet::ResolveContext,
            InputSet::Route,
            InputSet::PublishCues,
            InputSet::Consume,
        )
            .chain(),
    );
    app.init_resource::<LocalDeviceOrder>();
    // Which pad each seat holds, kept across disconnects.
    // `assign_local_seat_devices` panics without it.
    app.init_resource::<LocalSeatDeviceOwnership>();
    app.add_systems(
        PreUpdate,
        (track_local_device_order, assign_local_seat_devices)
            .chain()
            .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
    );
}

/// Install the provider action road: seat bindings in, semantic edges out.
///
/// It uses two schedules. The provider map must be on the seat before
/// leafwing resolves in `PreUpdate`. The edge is a routed semantic, so it
/// goes in `InputSet::Route`, which is configured in `Update`. An `in_set` on
/// the `PreUpdate` half would order nothing; with both in `PreUpdate` the
/// press never publishes.
#[cfg(feature = "input")]
pub fn install_provider_action_road(app: &mut bevy::prelude::App) {
    use bevy::prelude::{IntoScheduleConfigs as _, PostUpdate, PreUpdate, Update};

    // Add the per-action systems, not `InputManagerPlugin::<ProviderAction>`.
    // That plugin adds `clear_central_input_store` and `filter_captured_input`
    // without a guard, so a second action type registers them twice, and the
    // doubled clear drains the store. Guarded by
    // `no_system_is_registered_twice_in_one_schedule`. These are the generic
    // half of that plugin, in the same sets.
    //
    // `ProviderAction` is a second map component on the same participant
    // entity. The seats, resolve pass, and readers are shared.
    app.add_systems(
        PreUpdate,
        (
            leafwing_input_manager::systems::tick_action_state::<ProviderAction>
                .in_set(leafwing_input_manager::plugin::InputManagerSystem::Tick)
                .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
            leafwing_input_manager::systems::update_action_state::<ProviderAction>
                .in_set(leafwing_input_manager::plugin::InputManagerSystem::Update),
        ),
    );
    app.add_systems(
        PostUpdate,
        leafwing_input_manager::systems::release_on_input_map_removed::<ProviderAction>,
    );
    app.init_resource::<ProviderBindings>();
    app.add_message::<SemanticActionPressed>();

    app.add_systems(
        PreUpdate,
        install_provider_bindings_on_seats
            .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
    );
    app.add_systems(
        Update,
        publish_provider_action_edges.in_set(InputSet::Route),
    );
}

/// Publish which physical device each seat is currently driving.
///
/// It runs in `InputSet::Route` because the seat's device is a routed fact,
/// not a device reading.
#[cfg(feature = "input")]
pub fn install_seat_device_tracking(app: &mut bevy::prelude::App) {
    use bevy::prelude::{IntoScheduleConfigs as _, Update};

    app.add_systems(Update, update_seat_active_devices.in_set(InputSet::Route));
}

/// Swallow the edges a REBIND itself produced, one frame later.
///
/// `rebuild_maps_from_recipes` clears the seat's `ActionState` so no press
/// latches across a rebind. Leafwing re-reads the devices in the next
/// `PreUpdate` and would report an unmoved control as a fresh press. This
/// system runs in leafwing's `ManualControl` set to remove those edges.
///
/// It cannot chain onto the rebuild: the edges do not exist until leafwing
/// re-reads the devices.
#[cfg(feature = "input")]
pub fn install_rebind_edge_swallow(app: &mut bevy::prelude::App) {
    use bevy::prelude::{IntoScheduleConfigs as _, PreUpdate};

    app.add_systems(
        PreUpdate,
        swallow_the_rebinds_own_edges
            .in_set(leafwing_input_manager::plugin::InputManagerSystem::ManualControl)
            .after(leafwing_input_manager::plugin::InputManagerSystem::Update),
    );
}
