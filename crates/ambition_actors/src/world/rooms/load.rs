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
use crate::features;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::time::feel::SandboxFeelTuning;
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::platforms::{self, MovingPlatformState};
use crate::SandboxSimState;
use ambition_dev_tools::SandboxDevState;
use ambition_engine_core as ae;
use ambition_engine_core::RoomGeometry;
use ambition_sfx::SfxMessage;
use ambition_time::ClockState;

/// What [`load_room_geometry`] hands back to the composition layer so the host
/// can spawn parallax/room visuals + arrival VFX and apply the cross-domain
/// per-transition resets without re-deriving room state: the now-active room
/// spec, the validated arrival position, and whether this was an edge exit
/// (contiguous scroll) versus a door (discrete teleport).
///
/// `arrival_pos` and `edge_exit` are the sole inputs the caller needs for the
/// player/dialog/combat resets (see `apply_room_transition_resets` in the
/// composition tier) — the world IR resolves geometry, the composition tier owns
/// the multi-domain state reset (anti-god rule 6: split by who mutates).
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
/// per-room clock/cooldown state it owns.
///
/// It does NOT touch the higher-tier player/dialog/combat STATE (blink camera,
/// respawn safety, dialogue, hit-flash/timers) — those are live SIM state the
/// space IR must never name (W1). The caller in the composition tier applies
/// them from the returned [`RoomLoadResult`]. Emits only the room-reset
/// `SfxMessage` (a sim fact); all render-side spawning and arrival VFX are the
/// host's job.
#[allow(clippy::too_many_arguments)]
pub fn load_room_geometry(
    commands: &mut Commands,
    sfx: &mut MessageWriter<SfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut SandboxDevState,
    sim_state: &mut SandboxSimState,
    clock: &mut ClockState,
    moving_platforms: &mut Vec<MovingPlatformState>,
    placement_lowering: &crate::world::placements::PlacementLoweringRegistry,
    world: &mut RoomGeometry,
    room_set: &mut RoomSet,
    room_visuals: &Query<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // The body transiting INTO the target room. It rides along (like the home body,
    // which is never room-scoped) instead of being torn down with the old room's
    // scenery — so a possessed actor carries itself through the door.
    carry_body: Option<Entity>,
    transition: RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) -> RoomLoadResult {
    let old_velocity = clusters.kinematics.vel;
    let fly_enabled = clusters.flight.fly_enabled;
    let player_size = clusters.kinematics.size;
    let edge_exit = matches!(transition.zone.activation, LoadingZoneActivation::EdgeExit);

    for (entity, physics_entity) in room_visuals.iter() {
        // The transiting body is the protagonist crossing the seam, not room
        // scenery — never despawn it (the home body is exempt by never being
        // room-scoped; this extends the same treatment to a possessed actor).
        if carry_body == Some(entity) {
            continue;
        }
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
    ae::reset_body_clusters(clusters, arrival);
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
    clock.time_scale = 1.0;
    *moving_platforms = platforms::moving_platforms_for_room(&spec);
    features::spawn_room_feature_entities_with_registry(commands, &spec, placement_lowering);
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
