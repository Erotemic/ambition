//! The authorized room-transition commit: swap room authority, carry the
//! transiting body across, apply the cross-domain per-transition resets, ask for
//! the redraw, and log the landing.
//!
//! Came from `ambition_app::app::world_flow::room_flow` (2026-07-25). It was
//! stuck app-side for one reason — it DREW the new room (`spawn_room_visuals`),
//! which named `ambition_render`. Now it writes `RespawnRoomVisualsRequested`
//! like the sandbox reset and the room stager already did, and nothing here is
//! beyond an engine crate's reach. `reset_sandbox` made the same journey in
//! 2026-07-21 (tracks §2.5) for the same reason.

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, ResMut, With};

use ambition_platformer2d_actor_monolith::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition_platformer2d_actor_monolith::rooms;
use ambition_platformer2d_actor_monolith::time::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d_actor_monolith::time::time_control::ClockResetRequest;
use ambition_platformer2d_actor_monolith::world::physics;
use ambition_platformer2d_core::{self as ae, AabbExt, RoomGeometry};
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::{ParticleKind, VfxMessage};

/// The sim → presentation channels a committed transition writes: the zone
/// sound, the arrival puff, and the request to rebuild the destination room's
/// static visuals.
///
/// Bundled to keep the commit system under Bevy's 16-`SystemParam` ceiling,
/// which it sits at.
#[derive(SystemParam)]
pub struct RoomTransitionEffects<'w> {
    pub sfx: SfxWriter<'w>,
    pub vfx: MessageWriter<'w, VfxMessage>,
    pub respawn_room_visuals: MessageWriter<'w, rooms::RespawnRoomVisualsRequested>,
}

/// Sim state plus the clock-reset channel, so a system already at the parameter
/// ceiling can reach both through one slot. The reset is emitted as DATA and
/// consumed by the time-control owner — no system here mutates `time_scale`.
#[derive(SystemParam)]
pub struct RoomClock<'w> {
    pub sim_state: ResMut<'w, ambition_platformer2d_actor_monolith::RoomTransitionCooldown>,
    pub clock_resets: MessageWriter<'w, ClockResetRequest>,
}

/// Combat state a fresh room must not inherit, plus the feature-overlay read
/// side the landing diagnostic needs.
#[derive(SystemParam)]
pub struct RoomTransitionCombatReset<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub enemy_projectiles:
        Query<'w, 's, Entity, With<ambition_projectiles::enemy::EnemyProjectile>>,
    pub slot_board: ResMut<'w, ambition_platformer2d_actor_monolith::combat::slots::CombatSlotsRes>,
    pub feature_overlay: Res<'w, FeatureEcsWorldOverlay>,
    pub base_gravity: ResMut<'w, ambition_platformer2d_actor_monolith::physics::BaseGravity>,
}

impl RoomTransitionCombatReset<'_, '_> {
    /// Drop every in-flight enemy projectile and every slot reservation, and
    /// return ambient gravity to its default, so a fresh room does not inherit
    /// hostile shots or stale assignments from the one just left.
    pub fn clear_carryover(&mut self) {
        for entity in &self.enemy_projectiles {
            self.commands.entity(entity).despawn();
        }
        self.slot_board.0.clear_assignments();
        // Resetting the AMBIENT is the real gravity reset; the presentation
        // `GravityField` is a per-tick mirror of the primary body's resolved
        // frame and has exactly one writer (`resolve_active_gravity`).
        *self.base_gravity = ambition_platformer2d_actor_monolith::physics::BaseGravity::default();
    }
}

/// Probe along the body's gravity direction from its feet for the nearest
/// landing face (within 256 px). Returns `(distance, source)` where `source` is
/// `"world"`, `"overlay"`, or `"both"`. `None` means nothing — the body is over
/// a pit, or the floor has not materialised yet.
///
/// Frame-relative: "below feet" is +gravity, not world-down, so the diagnostic
/// stays meaningful under a gravity flip (identity under normal gravity).
fn ground_gap_below_feet(
    body: &ae::Aabb,
    gravity_dir: ae::Vec2,
    world: &ae::World,
    feature_overlay: &FeatureEcsWorldOverlay,
) -> Option<(f32, &'static str)> {
    const MAX_PROBE_PX: f32 = 256.0;
    // Side axis ⊥ gravity (`gravity_half(side)` reuses the projection to get an
    // AABB's extent along it).
    let side = ae::Vec2::new(gravity_dir.y, -gravity_dir.x);
    let feet = body.feet_coord(gravity_dir);
    let body_side = body.center().dot(side);
    let body_side_half = body.gravity_half(side);
    let probe = |blocks: &[ae::Block]| {
        let mut best: Option<f32> = None;
        for block in blocks {
            // The body's cross-section (⊥ gravity) must overlap the block's.
            let block_side = block.aabb.center().dot(side);
            if (block_side - body_side).abs() >= body_side_half + block.aabb.gravity_half(side) {
                continue;
            }
            // Only consider blocks whose landing face is at/below the feet along
            // gravity.
            let gap = block.aabb.head_coord(gravity_dir) - feet;
            if gap < 0.0 || gap > MAX_PROBE_PX {
                continue;
            }
            best = Some(best.map_or(gap, |b| b.min(gap)));
        }
        best
    };
    let world_gap = probe(&world.blocks);
    let overlay_gap = probe(&feature_overlay.blocks);
    match (world_gap, overlay_gap) {
        (Some(a), Some(b)) if (a - b).abs() < 0.5 => Some((a.min(b), "both")),
        (Some(a), Some(b)) if a <= b => Some((a, "world")),
        (Some(_), Some(b)) => Some((b, "overlay")),
        (Some(a), None) => Some((a, "world")),
        (None, Some(b)) => Some((b, "overlay")),
        (None, None) => None,
    }
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
    safety: Option<&mut ambition_platformer2d_actor_monolith::avatar::PlayerSafetyState>,
    dialogue: &mut ambition_dialog::DialogState,
    conversation: &mut ambition_platformer2d_actor_monolith::conversation::ActiveConversation,
    combat: &mut ambition_characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition_platformer2d_actor_monolith::avatar::PlayerBlinkCameraState>,
    arrival_pos: ae::Vec2,
    edge_exit: bool,
    feel: Platformer2dFeelTuningMonolith,
) {
    if let Some(blink_cam) = blink_cam {
        blink_cam.blink_in_timer = 0.0;
        blink_cam.blink_camera_from = arrival_pos;
        blink_cam.blink_camera_to = arrival_pos;
        blink_cam.camera_snap_timer = if edge_exit {
            0.0
        } else {
            ambition_platformer2d_actor_monolith::ROOM_DOOR_CAMERA_SNAP_TIME
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
    // ⛔ **the AUTHORITY too, and it is not the same close.** `DialogState` going
    // quiet only takes the text box away; the simulation's conversation names
    // two BODIES, and this transition is about to despawn the room they were
    // standing in. A conversation that survived would point at dead entities and
    // hold an NPC that no longer exists.
    conversation.close();
}

pub fn load_room(
    commands: &mut Commands,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    respawn_visuals: &mut MessageWriter<rooms::RespawnRoomVisualsRequested>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut ambition_dev_tools::DeveloperRuntimeState,
    sim_state: &mut ambition_platformer2d_actor_monolith::RoomTransitionCooldown,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    // Home-only presentation state (None when a possessed actor transits).
    safety: Option<&mut ambition_platformer2d_actor_monolith::avatar::PlayerSafetyState>,
    moving_platforms: &mut Vec<
        ambition_platformer2d_actor_monolith::world::platforms::MovingPlatformState,
    >,
    dialogue: &mut ambition_dialog::DialogState,
    conversation: &mut ambition_platformer2d_actor_monolith::conversation::ActiveConversation,
    combat: &mut ambition_characters::actor::BodyCombat,
    blink_cam: Option<&mut ambition_platformer2d_actor_monolith::avatar::PlayerBlinkCameraState>,
    world: &mut RoomGeometry,
    room_set: &mut rooms::RoomSet,
    construction_plan: &rooms::RoomConstructionPlan,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // The transiting body, exempt from the old-room despawn so it rides along.
    carry_body: Option<Entity>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
) {
    // Runtime half: swap geometry, reset the body, rebuild platforms, spawn
    // feature entities. Lives in the world runtime (`ambition_platformer2d_actor_monolith`) so
    // the headless sim can load rooms without a render dependency.
    let rooms::RoomLoadResult {
        spec: _,
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
        conversation,
        combat,
        blink_cam,
        arrival_pos,
        edge_exit,
        feel,
    );

    // Presentation half: ASK, don't draw. A room's static visuals + parallax are
    // rebuilt by `ambition_render::rendering::respawn_room_visuals_on_request`,
    // which reads the active room out of `RoomSet` for itself — the same channel
    // the sandbox reset (`session::reset`, step 7) and the room stager already
    // use. Calling `spawn_room_visuals` here instead was the lone holdout: it is
    // what made a room transition name `ambition_platformer2d::render`, and therefore what
    // kept the whole commit chain app-local and unreachable by a demo host.
    // A headless build has no consumer and correctly skips the visual respawn.
    respawn_visuals.write(rooms::RespawnRoomVisualsRequested);
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
pub struct TransitBodies<'w, 's> {
    controlled: Option<Res<'w, ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    clusters: Query<'w, 's, ae::BodyClusterQueryData>,
    /// The transiting body's movement policy — a room transition is a discrete
    /// TRANSIT (ADR 0024 authority) and must reconcile model-private attachment.
    motion_models:
        Query<'w, 's, &'static mut ambition_platformer2d_actor_monolith::features::MotionModel>,
    combat: Query<'w, 's, &'static mut ambition_characters::actor::BodyCombat>,
    /// The transiting body's resolved gravity frame — read (before the mutable
    /// cluster borrow) so the landing diagnostic probes along the body's own
    /// gravity, not world-down.
    motion_frames:
        Query<'w, 's, &'static ambition_platformer2d_actor_monolith::physics::ResolvedMotionFrame>,
    presentation: Query<
        'w,
        's,
        (
            &'static mut ambition_platformer2d_actor_monolith::avatar::PlayerBlinkCameraState,
            &'static mut ambition_platformer2d_actor_monolith::avatar::PlayerSafetyState,
        ),
        ambition_platformer2d_actor_monolith::actor::PrimaryPlayerOnly,
    >,
    primary: Query<'w, 's, Entity, ambition_platformer2d_actor_monolith::actor::PrimaryPlayerOnly>,
    /// The Class-B transit ledger (`collision-and-ccd.md` §3.2). It rides in
    /// this param because a room transition IS one of the four Class-B
    /// authorities, and this struct is the one that names the body it moves.
    /// `Option`, and bundled here rather than added to the system's signature —
    /// `commit_ready_room_transition_system` already sits at Bevy's 16-param ceiling.
    class_b: Option<ResMut<'w, ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
}

pub fn commit_ready_room_transition_system(
    mut commands: Commands,
    mut event_writers: RoomTransitionEffects,
    mut transit: TransitBodies,
    mut world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<RoomGeometry>,
    mut room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<rooms::RoomSet>,
    mut dev_state: ResMut<ambition_dev_tools::DeveloperRuntimeState>,
    mut room_clock: RoomClock,
    mut moving_platforms: ResMut<ambition_platformer2d_world::collision::MovingPlatformSet>,
    mut dialogue: ResMut<ambition_dialog::DialogState>,
    mut conversation: ResMut<
        ambition_platformer2d_actor_monolith::conversation::ActiveConversation,
    >,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<Platformer2dFeelTuningMonolith>,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    load_resources: (
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
        Res<super::loading::RoomTransitionContentEpoch>,
        ResMut<super::loading::RoomTransitionLoadState>,
        ResMut<ambition_load::LoadCoordinator>,
        MessageWriter<ambition_load::LoadEvent>,
        ResMut<bevy::prelude::NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
        Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    ),
    mut combat_reset: RoomTransitionCombatReset,
) {
    let (
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
        .filter(|active| active.phase == super::loading::RoomTransitionLoadPhase::CommitAuthorized)
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
        for event in loads.apply(ambition_load::LoadCommand::Cancel {
            load_id: active.barrier.load_id.clone(),
        }) {
            load_events.write(event);
        }
        loads.retire(&active.barrier.load_id);
        transition_state.active = None;
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
        bevy::log::warn!(target: "ambition_platformer2d::room_transition", "{detail}");
        return;
    }

    let Some(construction_plan) = active.construction_plan.as_ref() else {
        super::loading::fail_room_transition_commit_precondition(
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
        super::loading::fail_room_transition_commit_precondition(
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
        super::loading::fail_room_transition_commit_precondition(
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
        super::loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!("controlled body {subject:?} has no MotionModel at room commit"),
        );
        return;
    };
    let Ok(mut cluster_item) = transit.clusters.get_mut(subject) else {
        super::loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            format!("controlled body {subject:?} has no complete actor cluster at room commit"),
        );
        return;
    };
    let Ok(mut combat) = transit.combat.get_mut(subject) else {
        super::loading::fail_room_transition_commit_precondition(
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
            id: ambition_sfx::SfxId::new(sfx_id.as_str()),
            pos: pos_before,
        });
    }

    let target_room = request.transition.target_room;
    // AMBITION_REVIEW(determinism): wall clock, and deliberately so — this
    // measures how long the commit TOOK for `commit_duration`, a write-only
    // diagnostic field on `ActiveRoomTransitionLoad`. It is never read back, and
    // `RoomTransitionLoadState` is not rollback-registered, so no sim decision
    // can observe it. Timing a transaction with `SimTick` would measure the
    // wrong thing: the point is wall-clock cost to the player.
    #[cfg(not(target_arch = "wasm32"))]
    let commit_started = std::time::Instant::now();
    load_room(
        &mut commands,
        &mut event_writers.sfx,
        &mut event_writers.vfx,
        &mut event_writers.respawn_room_visuals,
        &mut motion_model,
        &mut clusters,
        &mut dev_state,
        &mut room_clock.sim_state,
        &mut room_clock.clock_resets,
        safety_opt.as_deref_mut(),
        &mut moving_platforms.0,
        &mut dialogue,
        &mut conversation,
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
            ambition_platformer2d_shared_tangle::class_b::ClassBRemap::RoomTransition,
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
            current.phase = super::loading::RoomTransitionLoadPhase::Committed;
        }
    } else {
        loads.retire(&active.barrier.load_id);
        transition_state.active = None;
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
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
/// browser console / log file with target `ambition_platformer2d::room_transition`.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &rooms::RoomSet,
    pos: ae::Vec2,
    size: ae::Vec2,
    gravity_dir: ae::Vec2,
    world: &ae::World,
    feature_overlay: &ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay,
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
        target: "ambition_platformer2d::room_transition",
        "room transition: target={target_id} player_pos=({:.1},{:.1}) \
         world_blocks={} overlay_blocks={} gap_below_feet={gap_desc} \
         body_overlaps[world={overlapping_world}, overlay={overlapping_overlay}]",
        pos.x,
        pos.y,
        world.blocks.len(),
        feature_overlay.blocks.len(),
    );
}
