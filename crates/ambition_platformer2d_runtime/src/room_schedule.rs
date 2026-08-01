//! The engine half of the room-transition phase (E5 step 5): detection emits
//! `RoomTransitionRequested`; the feature-side `reset_ecs_room_features`
//! system tears down per-room ECS state.
//!
//! The PREPARE + COMMIT steps — consuming the request, proving target
//! readiness, loading room geometry, and spawning presentation — live in
//! [`RoomTransitionSet::Apply`], the phase between detection and reset. The
//! engine fills it today (`crate::room_transition`); a game replacing the
//! transition policy replaces what is in that set.
//!
//! It used to be described here as a gap a host pins itself into with
//! `.after(detect_room_transition_system).before(reset_ecs_room_features)` — two
//! engine leaf names in a sentence a host had to trust. Naming the phase is the
//! same arrangement with the trust removed.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::gameplay_allowed;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{RoomTransitionSet, Platformer2dSimulationPhase};

/// Registers room-transition detection + the per-room feature reset, and
/// anchors the content room-reset slot. Part of
/// [`crate::PlatformerEnginePlugins`].
pub struct RoomTransitionSchedulePlugin;

impl Plugin for RoomTransitionSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::rooms::detect_room_transition_system
                    .run_if(gameplay_allowed)
                    .in_set(RoomTransitionSet::Detect),
                // One reset over the unified actor cluster (NPCs + enemies).
                // `RoomTransitionSet::Apply` is the phase between them — a real
                // slot now, where the module docs used to describe one.
                ambition_platformer2d_actor_monolith::features::reset_ecs_room_features.in_set(RoomTransitionSet::Reset),
            ),
        );
        // Anchor the content room-reset slot AFTER the engine's feature reset.
        // Content plugins register their reset systems in the slot; generic
        // plugins (gravity, portal RoomReset) order after the SET — nobody
        // names a content system (E5-finish de-weave).
        app.configure_sets(
            sim,
            ambition_platformer2d_actor_monolith::session::reset::ContentRoomResetSet
                .in_set(Platformer2dSimulationPhase::RoomTransition)
                // The PHASE, not the reset system's name.
                .after(RoomTransitionSet::Reset),
        );
    }
}
