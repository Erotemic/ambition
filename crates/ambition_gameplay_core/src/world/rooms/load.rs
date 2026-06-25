//! Sim-side room load: swap the authored [`RoomGeometry`], reset the controlled
//! body to the validated arrival, rebuild moving platforms, and spawn the room's
//! feature ECS entities. This is the runtime half of a room transition — it holds
//! no render dependency, so the headless simulation path can load rooms without a
//! presentation layer.
//!
//! The presentation half (parallax layers, room visuals, arrival VFX) lives in the
//! host (`ambition_app`), which calls [`load_room_geometry`] and then spawns
//! visuals from the returned [`RoomLoadResult`]. Splitting it this way keeps the
//! render spawns in the only crate allowed to name `ambition_render` while the
//! world-runtime work moves down to where the types it mutates already live.

use bevy::prelude::{Commands, Entity, MessageWriter, Query, With};

use super::{validated_spawn, LoadingZoneActivation, RoomSet, RoomSpec, RoomTransition};
use crate::audio::SfxMessage;
use crate::dialog::DialogState;
use crate::features;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::player::{
    PlayerBlinkCameraState, PlayerCombatState, PlayerInteractionState, PlayerSafetyState,
};
use crate::time::clock_state::ClockState;
use crate::time::feel::SandboxFeelTuning;
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::platforms::{self, MovingPlatformState};
use crate::{RoomGeometry, SandboxDevState, SandboxSimState, ROOM_DOOR_CAMERA_SNAP_TIME};
use ambition_engine_core as ae;

/// What [`load_room_geometry`] hands back to the presentation layer so the host
/// can spawn parallax/room visuals and the arrival VFX without re-deriving room
/// state: the now-active room spec, the validated arrival position, and whether
/// this was an edge exit (contiguous scroll) versus a door (discrete teleport).
pub struct RoomLoadResult {
    pub spec: RoomSpec,
    pub arrival_pos: ae::Vec2,
    pub edge_exit: bool,
}

/// Apply the runtime half of a room transition. Despawns the previous room's
/// scoped/physics entities, swaps `world` to the target room's authored geometry,
/// resets the controlled body to its validated arrival (preserving velocity on
/// edge exits so side-to-side scrolling feels continuous), rebuilds and spawns
/// moving platforms, spawns the room's feature entities, and resets the transient
/// per-room clock/combat/interaction state.
///
/// Emits only the room-reset `SfxMessage` (a sim fact). All render-side spawning
/// and arrival VFX are the host's job — see [`RoomLoadResult`].
#[allow(clippy::too_many_arguments)]
pub fn load_room_geometry(
    commands: &mut Commands,
    sfx: &mut MessageWriter<SfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    dev_state: &mut SandboxDevState,
    sim_state: &mut SandboxSimState,
    clock: &mut ClockState,
    safety: &mut PlayerSafetyState,
    moving_platforms: &mut Vec<MovingPlatformState>,
    dialogue: &mut DialogState,
    combat: &mut PlayerCombatState,
    interaction: &mut PlayerInteractionState,
    blink_cam: &mut PlayerBlinkCameraState,
    world: &mut RoomGeometry,
    room_set: &mut RoomSet,
    room_visuals: &Query<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>,
    transition: RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) -> RoomLoadResult {
    let old_velocity = clusters.kinematics.vel;
    let fly_enabled = clusters.flight.fly_enabled;
    let player_size = clusters.kinematics.size;
    let edge_exit = matches!(transition.zone.activation, LoadingZoneActivation::EdgeExit);

    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let spec = room_set.set_active(transition.target_room).clone();
    world.0 = spec.world.clone();

    // Room transitions are not player deaths/resets. Rebuild transient room
    // state, but preserve ability progression and, for edge exits, preserve
    // velocity so side-to-side room changes feel continuous. Door transitions
    // intentionally zero velocity because they are discrete interactions.
    let arrival = validated_spawn(&world.0, transition.arrival, player_size);
    ae::reset_player_clusters(clusters, arrival);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.flight.fly_enabled = fly_enabled && clusters.abilities.abilities.fly;
    if edge_exit {
        clusters.kinematics.vel = old_velocity;
    }
    blink_cam.blink_in_timer = 0.0;
    blink_cam.blink_camera_from = clusters.kinematics.pos;
    blink_cam.blink_camera_to = clusters.kinematics.pos;
    blink_cam.camera_snap_timer = if edge_exit {
        0.0
    } else {
        ROOM_DOOR_CAMERA_SNAP_TIME
    };
    combat.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    combat.hitstop_timer = 0.0;
    combat.damage_invuln_timer = 0.0;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    safety.last_safe_pos = clusters.kinematics.pos;
    clock.time_scale = 1.0;
    interaction.down_tap_timer = 0.0;
    *moving_platforms = platforms::moving_platforms_for_room(&spec);
    features::spawn_room_feature_entities(commands, &spec);
    dialogue.close();
    // This guard prevents immediate backtracking when arriving inside/near a
    // paired zone. It should not feel like frozen input, so keep it short and
    // rely on validated arrivals to do most of the safety work.
    sim_state.room_transition_cooldown = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    dev_state.preset_flash = 1.0;

    platforms::spawn_moving_platforms(commands, &world.0, moving_platforms);
    let arrival_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: arrival_pos });

    RoomLoadResult {
        spec,
        arrival_pos,
        edge_exit,
    }
}
