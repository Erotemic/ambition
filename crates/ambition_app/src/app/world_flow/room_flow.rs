//! Room lifecycle flow: sandbox reset, room load, parallax seeding, and the
//! room-transition apply + landing log.
//!
//! Split out of the former 1211-line `world_flow.rs` (2026-06-15).

use bevy::prelude::{
    AssetServer, Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With,
};

use ambition_engine_core::{self as ae, AabbExt};
use ambition_gameplay_core::audio::SfxMessage;
use ambition_gameplay_core::dev::dev_tools::EditableMovementTuning;
use ambition_gameplay_core::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_gameplay_core::world::physics;
use ambition_gameplay_core::{rooms, RoomGeometry};
use ambition_render::fx::{ParticleKind, VfxMessage};
use ambition_render::rendering::spawn_room_visuals;

use super::super::feedback::SandboxEventWriters;
use super::{ground_gap_below_feet, RoomClock};

pub(crate) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    attack: &mut Option<ambition_gameplay_core::MeleeSwing>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::actor::BodyCombat,
    interaction: &mut ambition_gameplay_core::player::PlayerInteractionState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_body_clusters(clusters, world.spawn);
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
    combat.hit_flash = feel.reset_flash_time;
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
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut ambition_gameplay_core::SandboxDevState,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    moving_platforms: &mut Vec<ambition_gameplay_core::world::platforms::MovingPlatformState>,
    dialogue: &mut ambition_gameplay_core::dialog::DialogState,
    combat: &mut ambition_gameplay_core::actor::BodyCombat,
    interaction: &mut ambition_gameplay_core::player::PlayerInteractionState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    world: &mut RoomGeometry,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&ambition_gameplay_core::assets::game_assets::GameAssets>,
    quality: Option<&ambition_render::quality::ResolvedVisualQuality>,
) {
    // Runtime half: swap geometry, reset the body, rebuild platforms, spawn
    // feature entities. Lives in the world runtime (`ambition_gameplay_core`) so
    // the headless sim can load rooms without a render dependency.
    let rooms::RoomLoadResult {
        spec,
        arrival_pos,
        edge_exit,
    } = rooms::load_room_geometry(
        commands,
        sfx,
        clusters,
        dev_state,
        sim_state,
        clock,
        safety,
        moving_platforms,
        dialogue,
        combat,
        interaction,
        blink_cam,
        world,
        room_set,
        room_visuals,
        transition,
        tuning,
        feel,
    );

    // Presentation half (host-only): render-side spawns + arrival VFX. These name
    // `ambition_render`, which the world runtime is forbidden from importing, so
    // they stay here in the app where composition with render is allowed.
    ambition_render::rendering::spawn_parallax_layers(
        commands,
        &world.0,
        &spec.metadata,
        assets,
        quality.map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(commands, &spec, physics_settings, assets);
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
    mut game_assets: Option<ResMut<ambition_gameplay_core::assets::game_assets::GameAssets>>,
    room_set: Res<rooms::RoomSet>,
    sandbox_catalog: Res<ambition_gameplay_core::assets::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    quality: Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
) {
    let Some(assets) = game_assets.as_deref_mut() else {
        return;
    };
    for request in requests.read() {
        if let Some(target_spec) = room_set.rooms.get(request.transition.target_room) {
            ambition_gameplay_core::assets::game_assets::ensure_parallax_layers_for_room(
                assets,
                &sandbox_catalog,
                &asset_server,
                &target_spec.metadata,
                quality.as_deref().map(|q| &q.budget),
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
            ae::BodyClusterQueryData,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
        ),
        // PRIMARY-only: a room transition flips the one active room around the
        // camera body crossing an edge/door; the clone rides along in-room.
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
    mut world: ResMut<RoomGeometry>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut dev_state: ResMut<ambition_gameplay_core::SandboxDevState>,
    mut room_clock: RoomClock,
    mut moving_platforms: ResMut<ambition_gameplay_core::MovingPlatformSet>,
    mut dialogue: ResMut<ambition_gameplay_core::dialog::DialogState>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    visual_assets: (
        Option<Res<ambition_gameplay_core::assets::game_assets::GameAssets>>,
        Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
    ),
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
            visual_assets.0.as_deref(),
            visual_assets.1.as_deref(),
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
    feature_overlay: &ambition_gameplay_core::features::FeatureEcsWorldOverlay,
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
