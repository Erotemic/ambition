//! The engine half of the room-transition phase (E5 step 5): detection emits
//! `RoomTransitionRequested`; the feature-side `reset_ecs_room_features`
//! system tears down per-room ECS state.
//!
//! The APPLY step — consuming the request, loading the room geometry, and
//! spawning presentation — is the host/composition tier's job (the W1
//! composer): the Ambition app registers its
//! (`ensure_requested_room_parallax_system`, `apply_room_transition_system`)
//! pair `.after(detect_room_transition_system)
//! .before(reset_ecs_room_features)` in `SandboxSet::RoomTransition`. A demo
//! host registers its own composer in the same gap.

use bevy::prelude::*;

use ambition_actors::game_mode::gameplay_allowed;
use ambition_actors::schedule::SandboxSet;

/// Registers room-transition detection + the per-room feature reset, and
/// anchors the content room-reset slot. Part of
/// [`crate::PlatformerEnginePlugins`].
pub struct RoomTransitionSchedulePlugin;

impl Plugin for RoomTransitionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                ambition_actors::rooms::detect_room_transition_system.run_if(gameplay_allowed),
                // One reset over the unified actor cluster (NPCs + enemies).
                // The host's transition APPLY slots in between (module docs).
                ambition_actors::features::reset_ecs_room_features,
            )
                .chain()
                .in_set(SandboxSet::RoomTransition),
        );
        // Anchor the content room-reset slot AFTER the engine's feature reset.
        // Content plugins register their reset systems in the slot; generic
        // plugins (gravity, portal RoomReset) order after the SET — nobody
        // names a content system (E5-finish de-weave).
        app.configure_sets(
            Update,
            ambition_actors::session::reset::ContentRoomResetSet
                .in_set(SandboxSet::RoomTransition)
                .after(ambition_actors::features::reset_ecs_room_features),
        );
    }
}
