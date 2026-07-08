//! Room lifecycle flow: sandbox reset, room load, parallax seeding, and the
//! room-transition apply + landing log.
//!
//! Split out of the former 1211-line `world_flow.rs` (2026-06-15).

use bevy::prelude::{
    AssetServer, Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With,
};

use ambition_actors::dev::dev_tools::EditableMovementTuning;
use ambition_actors::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition_actors::rooms;
use ambition_actors::time::feel::SandboxFeelTuning;
use ambition_actors::world::physics;
use ambition_engine_core::RoomGeometry;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_render::rendering::spawn_room_visuals;
use ambition_sfx::SfxMessage;
use ambition_vfx::{ParticleKind, VfxMessage};

use super::super::feedback::SandboxEventWriters;
use super::{ground_gap_below_feet, RoomClock};

pub(crate) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut ambition_actors::SandboxSimState,
    clock: &mut ambition_time::ClockState,
    safety: &mut ambition_actors::player::PlayerSafetyState,
    attack: &mut Option<ambition_actors::MeleeSwing>,
    anim: &mut ambition_actors::player::BodyAnimFacts,
    combat: &mut ambition_characters::actor::BodyCombat,
    interaction: &mut ambition_actors::player::SlotGestures,
    blink_cam: &mut ambition_actors::player::PlayerBlinkCameraState,
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

/// Apply the cross-domain per-transition STATE resets that the space IR
/// (`rooms::load_room_geometry`) deliberately does not touch: blink-camera snap,
/// respawn-safety anchor, hit-flash/combat timers, and dialogue close. These live
/// in the composition tier because they mutate four different domains' state
/// (player / dialog / combat) — no single domain owns the transition, so the
/// caller that composes them does (anti-god rule 6). Derived entirely from the
/// arrival position + edge-exit fact the IR returns, so behavior is byte-identical
/// to when these writes lived inside `load_room_geometry`.
#[allow(clippy::too_many_arguments)]
fn apply_room_transition_resets(
    safety: Option<&mut ambition_actors::player::PlayerSafetyState>,
    dialogue: &mut ambition_actors::dialog::DialogState,
    combat: &mut ambition_characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition_actors::player::PlayerBlinkCameraState>,
    arrival_pos: ae::Vec2,
    edge_exit: bool,
    feel: SandboxFeelTuning,
) {
    if let Some(blink_cam) = blink_cam {
        blink_cam.blink_in_timer = 0.0;
        blink_cam.blink_camera_from = arrival_pos;
        blink_cam.blink_camera_to = arrival_pos;
        blink_cam.camera_snap_timer = if edge_exit {
            0.0
        } else {
            ambition_actors::ROOM_DOOR_CAMERA_SNAP_TIME
        };
    }
    combat.hit_flash = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    combat.hitstop_timer = 0.0;
    combat.damage_invuln_timer = 0.0;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    if let Some(safety) = safety {
        safety.last_safe_pos = arrival_pos;
    }
    dialogue.close();
}

pub(crate) fn load_room(
    commands: &mut Commands,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut ambition_dev_tools::SandboxDevState,
    sim_state: &mut ambition_actors::SandboxSimState,
    clock: &mut ambition_time::ClockState,
    // Home-only presentation state (None when a possessed actor transits).
    safety: Option<&mut ambition_actors::player::PlayerSafetyState>,
    moving_platforms: &mut Vec<ambition_actors::world::platforms::MovingPlatformState>,
    dialogue: &mut ambition_actors::dialog::DialogState,
    combat: &mut ambition_characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition_actors::player::PlayerBlinkCameraState>,
    world: &mut RoomGeometry,
    room_set: &mut rooms::RoomSet,
    placement_lowering: &ambition_actors::world::placements::PlacementLoweringRegistry,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // The transiting body, exempt from the old-room despawn so it rides along.
    carry_body: Option<Entity>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&ambition_actors::assets::game_assets::GameAssets>,
    quality: Option<&ambition_render::quality::ResolvedVisualQuality>,
) {
    // Runtime half: swap geometry, reset the body, rebuild platforms, spawn
    // feature entities. Lives in the world runtime (`ambition_actors`) so
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
        moving_platforms,
        placement_lowering,
        world,
        room_set,
        room_visuals,
        carry_body,
        transition,
        tuning,
        feel,
    );

    // The space IR (`load_room_geometry`) resolved geometry + arrival but does not
    // name higher-tier player/dialog/combat STATE (W1). The composition tier owns
    // the cross-domain per-transition reset (anti-god rule 6: split by who
    // mutates), driven purely by the returned arrival + edge-exit facts.
    apply_room_transition_resets(
        safety,
        dialogue,
        combat,
        blink_cam,
        arrival_pos,
        edge_exit,
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
    mut game_assets: Option<ResMut<ambition_actors::assets::game_assets::GameAssets>>,
    room_set: Res<rooms::RoomSet>,
    sandbox_catalog: Res<ambition_actors::assets::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    quality: Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
) {
    let Some(assets) = game_assets.as_deref_mut() else {
        return;
    };
    for request in requests.read() {
        if let Some(target_spec) = room_set.rooms.get(request.transition.target_room) {
            ambition_actors::assets::game_assets::ensure_parallax_layers_for_room(
                assets,
                &sandbox_catalog,
                &asset_server,
                &target_spec.metadata,
                quality.as_deref().map(|q| &q.budget),
            );
        }
    }
}

/// The bodies a room transition can relocate, bundled into one `SystemParam` to
/// keep `apply_room_transition_system` under Bevy's 16-param limit.
///
/// A transition moves the CONTROLLED (observed) body — the home avatar during
/// normal play, or a possessed actor. `clusters` is body-generic (`ae::BodyClusterQueryData`
/// matches every body: the home avatar AND actors carry the same movement clusters),
/// so one `get_mut(subject)` relocates whichever body is driven. `presentation`
/// holds the home-only blink-camera + respawn-point state (a possessed actor has
/// neither); `primary` is the startup-frame fallback subject.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TransitBodies<'w, 's> {
    controlled:
        Option<Res<'w, ambition_platformer_primitives::markers::ControlledSubject>>,
    clusters: Query<'w, 's, ae::BodyClusterQueryData>,
    combat: Query<'w, 's, &'static mut ambition_characters::actor::BodyCombat>,
    presentation: Query<
        'w,
        's,
        (
            &'static mut ambition_actors::player::PlayerBlinkCameraState,
            &'static mut ambition_actors::player::PlayerSafetyState,
        ),
        ambition_actors::actor::PrimaryPlayerOnly,
    >,
    primary: Query<'w, 's, Entity, ambition_actors::actor::PrimaryPlayerOnly>,
}

pub(crate) fn apply_room_transition_system(
    mut commands: Commands,
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut event_writers: SandboxEventWriters,
    mut transit: TransitBodies,
    mut world: ResMut<RoomGeometry>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut dev_state: ResMut<ambition_dev_tools::SandboxDevState>,
    mut room_clock: RoomClock,
    mut moving_platforms: ResMut<ambition_actors::MovingPlatformSet>,
    mut dialogue: ResMut<ambition_actors::dialog::DialogState>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    load_resources: (
        Res<ambition_actors::world::placements::PlacementLoweringRegistry>,
        Option<Res<ambition_actors::assets::game_assets::GameAssets>>,
        Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
    ),
    mut combat_reset: super::super::feedback::CombatRoomReset,
) {
    for request in requests.read() {
        // The transition relocates the CONTROLLED body — the body the local player
        // is driving (home avatar or possessed actor), falling back to the primary
        // player at startup. This is the same subject the detect side resolves, so
        // the body that CROSSED the seam is the body that ARRIVES.
        let Some(subject) = transit
            .controlled
            .as_deref()
            .and_then(|c| c.0)
            .or_else(|| transit.primary.single().ok())
        else {
            continue;
        };
        let Ok(mut cluster_item) = transit.clusters.get_mut(subject) else {
            continue;
        };
        let Ok(mut combat) = transit.combat.get_mut(subject) else {
            continue;
        };
        // Home-only presentation: a possessed actor has no blink-camera / respawn
        // point, and — being room-scoped — must be CARRIED through the seam instead
        // of despawned with the old room. The home avatar is never room-scoped, so
        // it needs no carry (None) and keeps its presentation resets.
        let (mut blink_opt, mut safety_opt) = match transit.presentation.get_mut(subject).ok() {
            Some((blink, safety)) => (Some(blink), Some(safety)),
            None => (None, None),
        };
        let carry_body = if blink_opt.is_some() {
            None
        } else {
            Some(subject)
        };
        // Any enemy volleys still in flight from the previous room
        // would otherwise sail across the seam and hit the player
        // mid-transition. The slot board is per-target and the live
        // actor list is about to be torn down + rebuilt, so drop
        // every reservation now and let the next tick rebuild.
        combat_reset.clear_carryover();
        let mut clusters = cluster_item.as_clusters_mut();
        // Play the zone-entry SFX at the pre-load body position so it sounds
        // like it originates from the door/edge the body walked through.
        let pos_before = clusters.kinematics.pos;
        if let Some(sfx_id) = &request.zone_sfx {
            event_writers.sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::new(sfx_id.as_str()),
                pos: pos_before,
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
            safety_opt.as_deref_mut(),
            &mut moving_platforms.0,
            &mut dialogue,
            &mut combat,
            blink_opt.as_deref_mut(),
            &mut world,
            &mut room_set,
            &load_resources.0,
            &room_visuals,
            carry_body,
            request.transition.clone(),
            editable_tuning.as_engine(),
            *feel_tuning,
            *physics_settings,
            load_resources.1.as_deref(),
            load_resources.2.as_deref(),
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
    feature_overlay: &ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay,
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
