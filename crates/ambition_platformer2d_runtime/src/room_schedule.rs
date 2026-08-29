//! Room-transition schedule anchors. Detection emits the request, apply prepares
//! and commits it, and reset tears down per-room ECS state afterward.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{RoomTransitionSet, Platformer2dSimulationPhaseMonolith};

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
                    .in_set(GameplayGated)
                    .in_set(RoomTransitionSet::Detect),
                ambition_platformer2d_actor_monolith::features::reset_ecs_room_features.in_set(RoomTransitionSet::Reset),
            ),
        );
        // Content-specific room resets run after the engine feature reset;
        // generic plugins order against this set rather than naming content systems.
        app.configure_sets(
            sim,
            ambition_platformer2d_actor_monolith::session::reset::ContentRoomResetSet
                .in_set(Platformer2dSimulationPhaseMonolith::RoomTransition)
                // The PHASE, not the reset system's name.
                .after(RoomTransitionSet::Reset),
        );
    }
}
