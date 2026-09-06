//! Room-transition schedule anchors. Detection emits the request, apply prepares
//! and commits it, and `Reset` retires what an ADMITTED same-room replay
//! invalidates.
//!
//! ⭐ `Reset` NO LONGER BUILDS ANYTHING. A same-room replay is admitted in
//! `sandbox_reset::admit_room_replay` and rebuilt by the transition road like
//! any other room change; what runs here is the one sweep that road cannot make
//! — the previous attempt's session-scoped residue. See
//! `ambition_platformer2d_actor_monolith::rooms::retire_the_previous_attempt`.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, RoomTransitionSet,
};

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
                ambition_platformer2d_actor_monolith::rooms::retire_the_previous_attempt
                    .in_set(RoomTransitionSet::Reset),
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
