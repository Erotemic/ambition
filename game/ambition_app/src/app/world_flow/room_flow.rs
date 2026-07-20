//! Room lifecycle flow: sandbox reset, room load, parallax seeding, and the
//! authorized room-transition commit + landing log.
//!
//! Split out of the former 1211-line `world_flow.rs` (2026-06-15).

use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, ResMut, With};

use ambition::actors::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition::actors::rooms;
use ambition::actors::time::feel::SandboxFeelTuning;
use ambition::actors::time::time_control::{ClockRequester, ClockResetRequest};
use ambition::actors::world::physics;
use ambition::engine_core::RoomGeometry;
use ambition::engine_core::{self as ae, AabbExt};
use ambition::render::rendering::spawn_room_visuals;
use ambition::sfx::{SfxMessage, SfxWriter};
use ambition::vfx::{ParticleKind, VfxMessage};

use super::super::feedback::SandboxEventWriters;
use super::{ground_gap_below_feet, RoomClock};

pub(crate) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut ambition::actors::SandboxSimState,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut ambition::actors::avatar::PlayerSafetyState,
    attack: &mut Option<ambition::actors::MeleeSwing>,
    anim: &mut ambition::actors::actor::BodyAnimFacts,
    combat: &mut ambition::characters::actor::BodyCombat,
    interaction: &mut ambition::actors::control::SlotGestures,
    blink_cam: &mut ambition::actors::avatar::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_body_clusters(motion_model, clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "sandbox_reset",
    ));
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
/// (`rooms::commit_room_transition_geometry`) deliberately does not touch: blink-camera snap,
/// respawn-safety anchor, hit-flash/combat timers, and dialogue close. These live
/// in the composition tier because they mutate four different domains' state
/// (player / dialog / combat) — no single domain owns the transition, so the
/// caller that composes them does (anti-god rule 6). Derived entirely from the
/// arrival position + edge-exit fact the IR returns, so behavior is byte-identical
/// to when these writes lived inside the former direct room loader.
#[allow(clippy::too_many_arguments)]
fn apply_room_transition_resets(
    safety: Option<&mut ambition::actors::avatar::PlayerSafetyState>,
    dialogue: &mut ambition::dialog::DialogState,
    combat: &mut ambition::characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition::actors::avatar::PlayerBlinkCameraState>,
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
            ambition::actors::ROOM_DOOR_CAMERA_SNAP_TIME
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
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut ambition::dev_tools::SandboxDevState,
    sim_state: &mut ambition::actors::SandboxSimState,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    // Home-only presentation state (None when a possessed actor transits).
    safety: Option<&mut ambition::actors::avatar::PlayerSafetyState>,
    moving_platforms: &mut Vec<ambition::actors::world::platforms::MovingPlatformState>,
    dialogue: &mut ambition::dialog::DialogState,
    combat: &mut ambition::characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition::actors::avatar::PlayerBlinkCameraState>,
    world: &mut RoomGeometry,
    room_set: &mut rooms::RoomSet,
    construction_plan: &rooms::RoomConstructionPlan,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // The transiting body, exempt from the old-room despawn so it rides along.
    carry_body: Option<Entity>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&ambition::sprite_sheet::game_assets::GameAssets>,
    quality: Option<&ambition::render::quality::ResolvedVisualQuality>,
) {
    // Runtime half: swap geometry, reset the body, rebuild platforms, spawn
    // feature entities. Lives in the world runtime (`ambition::actors`) so
    // the headless sim can load rooms without a render dependency.
    let rooms::RoomLoadResult {
        spec,
        arrival_pos,
        edge_exit,
    } = rooms::commit_room_transition_geometry(
        commands,
        sfx,
        motion_model,
        clusters,
        dev_state,
        sim_state,
        clock_resets,
        moving_platforms,
        construction_plan,
        world,
        room_set,
        room_visuals,
        carry_body,
        transition,
        tuning,
        feel,
    );

    // The space IR (`commit_room_transition_geometry`) resolved geometry + arrival but does not
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
    // `ambition::render`, which the world runtime is forbidden from importing, so
    // they stay here in the app where composition with render is allowed.
    ambition::render::rendering::spawn_parallax_layers(
        commands,
        construction_plan.session_scope(),
        &world.0,
        &spec.metadata,
        assets,
        quality.map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        commands,
        construction_plan.session_scope(),
        &spec,
        physics_settings,
        assets,
    );
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

/// The bodies a room transition can relocate, bundled into one `SystemParam` to
/// keep `commit_ready_room_transition_system` under Bevy's 16-param limit.
///
/// A transition moves the CONTROLLED (observed) body — the home avatar during
/// normal play, or a possessed actor. `clusters` is body-generic (`ae::BodyClusterQueryData`
/// matches every body: the home avatar AND actors carry the same movement clusters),
/// so one `get_mut(subject)` relocates whichever body is driven. `presentation`
/// holds the home-only blink-camera + respawn-point state (a possessed actor has
/// neither); `primary` is the startup-frame fallback subject.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TransitBodies<'w, 's> {
    controlled: Option<Res<'w, ambition::platformer::markers::ControlledSubject>>,
    clusters: Query<'w, 's, ae::BodyClusterQueryData>,
    /// The transiting body's movement policy — a room transition is a discrete
    /// TRANSIT (ADR 0024 authority) and must reconcile model-private attachment.
    motion_models: Query<'w, 's, &'static mut ambition::actors::features::MotionModel>,
    combat: Query<'w, 's, &'static mut ambition::characters::actor::BodyCombat>,
    /// The transiting body's resolved gravity frame — read (before the mutable
    /// cluster borrow) so the landing diagnostic probes along the body's own
    /// gravity, not world-down.
    motion_frames: Query<'w, 's, &'static ambition::actors::physics::ResolvedMotionFrame>,
    presentation: Query<
        'w,
        's,
        (
            &'static mut ambition::actors::avatar::PlayerBlinkCameraState,
            &'static mut ambition::actors::avatar::PlayerSafetyState,
        ),
        ambition::actors::actor::PrimaryPlayerOnly,
    >,
    primary: Query<'w, 's, Entity, ambition::actors::actor::PrimaryPlayerOnly>,
    /// The Class-B transit ledger (`collision-and-ccd.md` §3.2). It rides in
    /// this param because a room transition IS one of the four Class-B
    /// authorities, and this struct is the one that names the body it moves.
    /// `Option`, and bundled here rather than added to the system's signature —
    /// `commit_ready_room_transition_system` already sits at Bevy's 16-param ceiling.
    class_b: Option<ResMut<'w, ambition::platformer::class_b::ClassBRemapLog>>,
}

pub(crate) fn commit_ready_room_transition_system(
    mut commands: Commands,
    mut event_writers: SandboxEventWriters,
    mut transit: TransitBodies,
    mut world: ambition::platformer::lifecycle::SessionWorldMut<RoomGeometry>,
    mut room_set: ambition::platformer::lifecycle::SessionWorldMut<rooms::RoomSet>,
    mut dev_state: ResMut<ambition::dev_tools::SandboxDevState>,
    mut room_clock: RoomClock,
    mut moving_platforms: ResMut<ambition::world::collision::MovingPlatformSet>,
    mut dialogue: ResMut<ambition::dialog::DialogState>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    load_resources: (
        Option<Res<ambition::sprite_sheet::game_assets::GameAssets>>,
        Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
        Option<Res<ambition::platformer::lifecycle::ActiveSessionScope>>,
        Res<super::RoomTransitionContentEpoch>,
        ResMut<super::RoomTransitionLoadState>,
        ResMut<ambition::load::LoadCoordinator>,
        MessageWriter<ambition::load::LoadEvent>,
        ResMut<bevy::prelude::NextState<ambition::platformer::schedule::GameMode>>,
        Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    ),
    mut combat_reset: super::super::feedback::CombatRoomReset,
) {
    let (
        assets,
        quality,
        active_session,
        content_epoch,
        mut transition_state,
        mut loads,
        mut load_events,
        mut next_mode,
        real_time,
    ) = load_resources;

    let Some(active) = transition_state
        .active
        .as_ref()
        .filter(|active| {
            active.phase
                == super::room_transition_loading::RoomTransitionLoadPhase::CommitAuthorized
        })
        .cloned()
    else {
        return;
    };

    let target_still_matches = room_set
        .rooms
        .get(active.request.transition.target_room)
        .is_some_and(|room| {
            room.id == active.target_room_id
                && active
                    .construction_plan
                    .as_ref()
                    .is_some_and(|plan| plan.matches_room_spec(room))
        });
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    if active.content_epoch != content_epoch.get()
        || active.session_scope != current_session
        || room_set.active != active.source_room
        || !target_still_matches
    {
        let detail = format!(
            "discarding stale room transition {}: expected epoch {}, session {:?}, source '{}' at index {}, and target '{}'; current epoch is {}, session is {:?}, active index is {}",
            active.sequence,
            active.content_epoch,
            active.session_scope,
            active.source_room_id,
            active.source_room,
            active.target_room_id,
            content_epoch.get(),
            current_session,
            room_set.active,
        );
        for event in loads.apply(ambition::load::LoadCommand::Cancel {
            load_id: active.barrier.load_id.clone(),
        }) {
            load_events.write(event);
        }
        loads.retire(&active.barrier.load_id);
        transition_state.active = None;
        next_mode.set(ambition::platformer::schedule::GameMode::Playing);
        bevy::log::warn!(target: "ambition::room_transition", "{detail}");
        return;
    }

    let Some(construction_plan) = active.construction_plan.as_ref() else {
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            "authorized room transition has no prepared construction plan".to_string(),
        );
        return;
    };
    if construction_plan.target_index() != active.request.transition.target_room
        || construction_plan.room_id() != active.target_room_id
    {
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!(
                "prepared construction plan {} targets '{}' at index {}, not '{}' at index {}",
                construction_plan.id().as_str(),
                construction_plan.room_id(),
                construction_plan.target_index(),
                active.target_room_id,
                active.request.transition.target_room,
            ),
        );
        return;
    }

    let request = active.request;
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
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            "authorized room transition has no controlled or primary body".to_string(),
        );
        return;
    };
    let subject_gravity_dir = transit
        .motion_frames
        .get(subject)
        .map(|frame| frame.down())
        .unwrap_or(ae::Vec2::new(0.0, 1.0));
    let Ok(mut motion_model) = transit.motion_models.get_mut(subject) else {
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!("controlled body {subject:?} has no MotionModel at room commit"),
        );
        return;
    };
    let Ok(mut cluster_item) = transit.clusters.get_mut(subject) else {
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!("controlled body {subject:?} has no complete actor cluster at room commit"),
        );
        return;
    };
    let Ok(mut combat) = transit.combat.get_mut(subject) else {
        super::room_transition_loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!("controlled body {subject:?} has no BodyCombat at room commit"),
        );
        return;
    };
    let (mut blink_opt, mut safety_opt) = match transit.presentation.get_mut(subject).ok() {
        Some((blink, safety)) => (Some(blink), Some(safety)),
        None => (None, None),
    };
    let carry_body = if blink_opt.is_some() {
        None
    } else {
        Some(subject)
    };

    combat_reset.clear_carryover();
    let mut clusters = cluster_item.as_clusters_mut();
    let pos_before = clusters.kinematics.pos;
    if let Some(sfx_id) = &request.zone_sfx {
        event_writers.sfx.write(SfxMessage::Play {
            id: ambition::sfx::SfxId::new(sfx_id.as_str()),
            pos: pos_before,
        });
    }

    let target_room = request.transition.target_room;
    #[cfg(not(target_arch = "wasm32"))]
    let commit_started = std::time::Instant::now();
    load_room(
        &mut commands,
        &mut event_writers.sfx,
        &mut event_writers.vfx,
        &mut motion_model,
        &mut clusters,
        &mut dev_state,
        &mut room_clock.sim_state,
        &mut room_clock.clock_resets,
        safety_opt.as_deref_mut(),
        &mut moving_platforms.0,
        &mut dialogue,
        &mut combat,
        blink_opt.as_deref_mut(),
        &mut world,
        &mut room_set,
        construction_plan,
        &room_visuals,
        carry_body,
        request.transition.clone(),
        active_tuning.0,
        *feel_tuning,
        *physics_settings,
        assets.as_deref(),
        quality.as_deref(),
    );
    #[cfg(not(target_arch = "wasm32"))]
    let commit_duration = Some(commit_started.elapsed());
    #[cfg(target_arch = "wasm32")]
    let commit_duration = None;
    if let Some(current) = transition_state
        .active
        .as_mut()
        .filter(|current| current.sequence == active.sequence)
    {
        current.commit_duration = commit_duration;
        current.committed_at = real_time.as_deref().map(|time| time.elapsed());
    }

    if let Some(log) = transit.class_b.as_mut() {
        log.record(
            subject,
            ambition::platformer::class_b::ClassBRemap::RoomTransition,
        );
    }
    log_room_transition_landing(
        target_room,
        &room_set,
        clusters.kinematics.pos,
        clusters.kinematics.size,
        subject_gravity_dir,
        &world.0,
        &combat_reset.feature_overlay,
    );
    if active.cover_required {
        if let Some(current) = transition_state
            .active
            .as_mut()
            .filter(|current| current.sequence == active.sequence)
        {
            current.phase = super::room_transition_loading::RoomTransitionLoadPhase::Committed;
        }
    } else {
        loads.retire(&active.barrier.load_id);
        transition_state.active = None;
        next_mode.set(ambition::platformer::schedule::GameMode::Playing);
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
/// Cheap: runs once per committed room transition, iterates blocks once
/// to find the highest top-below-feet, no per-frame cost. Filter the
/// browser console / log file with target `ambition::room_transition`.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &rooms::RoomSet,
    pos: ae::Vec2,
    size: ae::Vec2,
    gravity_dir: ae::Vec2,
    world: &ae::World,
    feature_overlay: &ambition::platformer::feature_overlay::FeatureEcsWorldOverlay,
) {
    let target_id = room_set
        .rooms
        .get(target_room)
        .map(|spec| spec.id.clone())
        .unwrap_or_else(|| format!("<index {target_room}>"));
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
    let gap = ground_gap_below_feet(&body, gravity_dir, world, feature_overlay);
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
