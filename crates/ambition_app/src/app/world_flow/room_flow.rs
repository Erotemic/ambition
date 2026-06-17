//! Room lifecycle flow: sandbox reset, room load, parallax seeding, and the
//! room-transition apply + landing log.
//!
//! Split out of the former 1211-line `world_flow.rs` (2026-06-15).

use super::*;

pub(crate) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut ambition_sandbox::SandboxSimState,
    clock: &mut ambition_sandbox::time::clock_state::ClockState,
    safety: &mut ambition_sandbox::player::PlayerSafetyState,
    attack: &mut Option<ambition_sandbox::PlayerAttackState>,
    anim: &mut ambition_sandbox::player::PlayerAnimState,
    combat: &mut ambition_sandbox::player::PlayerCombatState,
    interaction: &mut ambition_sandbox::player::PlayerInteractionState,
    blink_cam: &mut ambition_sandbox::player::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_player_clusters(clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    *attack = None;
    anim.reset();
    combat.reset();
    combat.flash_timer = feel.reset_flash_time;
    interaction.reset();
    blink_cam.reset();
    let reset_to = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: reset_to });
    vfx.write(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

pub(crate) fn load_room(
    commands: &mut Commands,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    dev_state: &mut ambition_sandbox::SandboxDevState,
    sim_state: &mut ambition_sandbox::SandboxSimState,
    clock: &mut ambition_sandbox::time::clock_state::ClockState,
    safety: &mut ambition_sandbox::player::PlayerSafetyState,
    moving_platforms: &mut Vec<ambition_sandbox::world::platforms::MovingPlatformState>,
    dialogue: &mut ambition_sandbox::dialog::DialogState,
    combat: &mut ambition_sandbox::player::PlayerCombatState,
    interaction: &mut ambition_sandbox::player::PlayerInteractionState,
    blink_cam: &mut ambition_sandbox::player::PlayerBlinkCameraState,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&ambition_sandbox::assets::game_assets::GameAssets>,
) {
    let old_velocity = clusters.kinematics.vel;
    let fly_enabled = clusters.flight.fly_enabled;
    let player_size = clusters.kinematics.size;
    let edge_exit = matches!(
        transition.zone.activation,
        rooms::LoadingZoneActivation::EdgeExit
    );

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
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, player_size);
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
        ambition_sandbox::ROOM_DOOR_CAMERA_SNAP_TIME
    };
    combat.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    combat.hitstop_timer = 0.0;
    combat.damage_invuln_timer = 0.0;
    combat.hitstun_timer = 0.0;
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

    ambition_render::rendering::spawn_parallax_layers(commands, &world.0, &spec.metadata, assets);
    spawn_room_visuals(commands, &spec, physics_settings, assets);
    platforms::spawn_moving_platforms(commands, &world.0, moving_platforms);
    let arrival_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: arrival_pos });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.write(VfxMessage::Burst {
            pos: arrival_pos,
            count: 18,
            speed: 260.0,
            color: [0.35, 0.95, 1.0, 0.75],
            kind: ParticleKind::Dust,
        });
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        vfx.write(VfxMessage::ResetEffects {
            from: arrival_pos,
            to: arrival_pos,
        });
    }
}

/// Bevy system: reads `RoomTransitionRequested` messages written by
/// `detect_room_transition_system` and applies the room load.
///
/// Runs immediately after the player tick in the `CoreSimulation` chain
/// so the player position, world, and room_set are updated before any
/// other post-sim systems run in the same frame.
pub fn ensure_requested_room_parallax_system(
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut game_assets: Option<ResMut<ambition_sandbox::assets::game_assets::GameAssets>>,
    room_set: Res<rooms::RoomSet>,
    sandbox_catalog: Res<ambition_sandbox::assets::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
) {
    let Some(assets) = game_assets.as_deref_mut() else {
        return;
    };
    for request in requests.read() {
        if let Some(target_spec) = room_set.rooms.get(request.transition.target_room) {
            ambition_sandbox::assets::game_assets::ensure_parallax_layers_for_room(
                assets,
                &sandbox_catalog,
                &asset_server,
                &target_spec.metadata,
            );
        }
    }
}

pub(crate) fn apply_room_transition_system(
    mut commands: Commands,
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut event_writers: SandboxEventWriters,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerInteractionState,
            &mut ambition_sandbox::player::PlayerBlinkCameraState,
            &mut ambition_sandbox::player::PlayerSafetyState,
        ),
        // PRIMARY-only: a room transition flips the one active room around the
        // camera body crossing an edge/door; the clone rides along in-room.
        ambition_sandbox::player::PrimaryPlayerOnly,
    >,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut dev_state: ResMut<ambition_sandbox::SandboxDevState>,
    mut room_clock: RoomClock,
    mut moving_platforms: ResMut<ambition_sandbox::MovingPlatformSet>,
    mut dialogue: ResMut<ambition_sandbox::dialog::DialogState>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    game_assets: Option<Res<ambition_sandbox::assets::game_assets::GameAssets>>,
    mut combat_reset: super::super::feedback::CombatRoomReset,
) {
    for request in requests.read() {
        let Ok((mut cluster_item, mut combat, mut interaction, mut blink_cam, mut safety)) =
            player_q.single_mut()
        else {
            continue;
        };
        // Any enemy volleys still in flight from the previous room
        // would otherwise sail across the seam and hit the player
        // mid-transition. The slot board is per-target and the live
        // actor list is about to be torn down + rebuilt, so drop
        // every reservation now and let the next tick rebuild.
        combat_reset.clear_carryover();
        let mut clusters = cluster_item.as_clusters_mut();
        // Play the zone-entry SFX at the pre-load player position so it sounds
        // like it originates from the door/edge the player walked through.
        let player_pos_before = clusters.kinematics.pos;
        if let Some(sfx_id) = request.zone_sfx {
            event_writers.sfx.write(SfxMessage::Play {
                id: sfx_id,
                pos: player_pos_before,
            });
        }
        let target_room = request.transition.target_room;
        load_room(
            &mut commands,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
            &mut clusters,
            &mut dev_state,
            &mut room_clock.sim_state,
            &mut room_clock.clock,
            &mut safety,
            &mut moving_platforms.0,
            &mut dialogue,
            &mut combat,
            &mut interaction,
            &mut blink_cam,
            &mut world,
            &mut room_set,
            &room_visuals,
            request.transition.clone(),
            editable_tuning.as_engine(),
            *feel_tuning,
            *physics_settings,
            game_assets.as_deref(),
        );
        log_room_transition_landing(
            target_room,
            &room_set,
            clusters.kinematics.pos,
            clusters.kinematics.size,
            &world.0,
            &combat_reset.feature_overlay,
        );
    }
}

/// One-line diagnostic emitted on every room transition. Goal: when
/// "player fell through the floor in <room>" reports come in we have
/// the signals on disk / in the browser console to tell apart the
/// usual suspects:
///
/// - `world_blocks` == 0 → `to_room_set()` didn't populate this room's
///   `world.blocks` (LDtk load / merge issue).
/// - `overlay_blocks` == 0 in a room whose floor is breakable / actor
///   / boss → ECS feature spawn raced the post-transition sim tick.
/// - `gap_below_feet` large or `none` → `validated_spawn` placed the
///   player above the floor (`world.0`-only collision check missed the
///   overlay floor) and gravity is about to pull them through.
///
/// Cheap: runs once per RoomTransitionRequested, iterates blocks once
/// to find the highest top-below-feet, no per-frame cost. Filter the
/// browser console / log file with target `ambition::room_transition`.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &rooms::RoomSet,
    pos: ae::Vec2,
    size: ae::Vec2,
    world: &ae::World,
    feature_overlay: &ambition_sandbox::features::FeatureEcsWorldOverlay,
) {
    let target_id = room_set
        .rooms
        .get(target_room)
        .map(|spec| spec.id.clone())
        .unwrap_or_else(|| format!("<index {target_room}>"));
    let feet_y = pos.y + size.y * 0.5;
    let body = ae::Aabb::new(pos, size * 0.5);
    let overlapping_world = world
        .blocks
        .iter()
        .filter(|b| b.aabb.strict_intersects(body))
        .count();
    let overlapping_overlay = feature_overlay
        .blocks
        .iter()
        .filter(|b| b.aabb.strict_intersects(body))
        .count();
    let gap = ground_gap_below_feet(feet_y, &body, world, feature_overlay);
    let gap_desc = match gap {
        Some((distance, source)) => format!("{distance:.1}px ({source})"),
        None => "none within 256px".to_string(),
    };
    bevy::log::info!(
        target: "ambition::room_transition",
        "room transition: target={target_id} player_pos=({:.1},{:.1}) \
         world_blocks={} overlay_blocks={} gap_below_feet={gap_desc} \
         body_overlaps[world={overlapping_world}, overlay={overlapping_overlay}]",
        pos.x,
        pos.y,
        world.blocks.len(),
        feature_overlay.blocks.len(),
    );
}
