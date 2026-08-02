//! Transition-specific body mutation around canonical room construction.
//!
//! [`RoomConstructionPlan`](super::RoomConstructionPlan) owns room teardown,
//! geometry/platform publication, and feature construction. This module adds
//! only the controlled-body transit and transition-owned clock/feel facts.

use bevy::prelude::{Commands, Entity, MessageWriter, Query, With};

use super::{
    validated_spawn, LoadingZoneActivation, RoomConstructionPlan, RoomSet, RoomSpec, RoomTransition,
};
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::time::feel::Platformer2dFeelTuningMonolith;
use crate::time::time_control::{ClockRequester, ClockResetRequest};
use crate::world::physics::PhysicsRoomEntity;
use crate::world::platforms::MovingPlatformState;
use crate::RoomTransitionCooldown;
use ambition_dev_tools::DeveloperRuntimeState;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::RoomGeometry;
use ambition_sfx::{SfxMessage, SfxWriter};

pub struct RoomLoadResult {
    pub spec: RoomSpec,
    pub arrival_pos: ae::Vec2,
    pub edge_exit: bool,
}

/// Commit a prepared room construction plan and relocate the controlled body.
///
/// Every fallible room/content lookup occurred when the plan was prepared. This
/// function is therefore the short covered commit: retire outgoing room scope,
/// publish the prepared target, enqueue its exact roster, and apply transition
/// body semantics.
#[allow(clippy::too_many_arguments)]
pub fn commit_room_transition_geometry(
    commands: &mut Commands,
    sfx: &mut SfxWriter,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut DeveloperRuntimeState,
    sim_state: &mut RoomTransitionCooldown,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    moving_platforms: &mut Vec<MovingPlatformState>,
    plan: &RoomConstructionPlan,
    world: &mut RoomGeometry,
    room_set: &mut RoomSet,
    room_visuals: &Query<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>,
    carry_body: Option<Entity>,
    transition: RoomTransition,
    tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
) -> RoomLoadResult {
    debug_assert_eq!(plan.target_index(), transition.target_room);
    let player_size = clusters.kinematics.size;
    let edge_exit = matches!(transition.zone.activation, LoadingZoneActivation::EdgeExit);

    plan.retire_outgoing(
        commands,
        room_visuals
            .iter()
            .map(|(entity, physics)| (entity, physics.is_some())),
        carry_body,
    );
    plan.commit_deferred(commands, room_set, world, moving_platforms);

    let arrival = validated_spawn(&world.0, transition.arrival, player_size);
    // The follow-up `refresh_movement_resources_clusters` that used to sit here is
    // GONE: the reset takes the live air-jump count now and already restores dash
    // charges from the same `BodyAbilities`, so the second call restated its
    // answer. Four of five call sites performed that ritual and one did not,
    // which is the whole argument for folding it in.
    ae::arrive_body_in_room(
        motion_model,
        clusters,
        arrival,
        tuning.air_jumps,
        if edge_exit {
            ae::ArrivalMomentum::Preserve
        } else {
            ae::ArrivalMomentum::Reset
        },
    );
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "room_transition",
    ));
    sim_state.remaining = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    dev_state.preset_flash = 1.0;

    let arrival_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: arrival_pos });

    RoomLoadResult {
        spec: plan.spec().clone(),
        arrival_pos,
        edge_exit,
    }
}
