//! The authorized room-transition commit: swap room authority, carry the
//! transiting body across, apply the cross-domain per-transition resets, ask for
//! the redraw, and log the landing.

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, ResMut, With};

use ambition_platformer2d_actor_monolith::platformer_runtime::lifecycle::RoomResident;
use ambition_platformer2d_actor_monolith::rooms;
use ambition_platformer2d_world::rooms as world_rooms;

use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d_actor_monolith::world::physics;
use ambition_platformer2d_core::{self as ae, AabbExt, RoomGeometry};
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_time::time_control::ClockResetRequest;
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
    pub respawn_room_visuals: MessageWriter<'w, world_rooms::RespawnRoomVisualsRequested>,
}

/// Sim state plus the clock-reset channel, so a system already at the parameter
/// ceiling can reach both through one slot. The reset is emitted as DATA and
/// consumed by the time-control owner — no system here mutates `time_scale`.
#[derive(SystemParam)]
pub struct RoomClock<'w> {
    pub sim_state:
        ResMut<'w, ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown>,
    pub clock_resets: MessageWriter<'w, ClockResetRequest>,
}

/// Combat state a fresh room must not inherit, plus the feature-overlay read
/// side the landing diagnostic needs.
#[derive(SystemParam)]
pub struct RoomTransitionCombatReset<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub live_projectiles: Query<'w, 's, Entity, With<ambition_projectiles::LiveProjectile>>,
    pub feature_overlay: Res<'w, FeatureEcsWorldOverlay>,
    pub base_gravity: ResMut<'w, ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
}

impl RoomTransitionCombatReset<'_, '_> {
    /// Drop every in-flight projectile and return ambient gravity to its default,
    /// so a fresh room does not inherit combat events or a stale gravity frame
    /// from the one just left.
    pub fn clear_carryover(&mut self) {
        for entity in &self.live_projectiles {
            self.commands.entity(entity).despawn();
        }
        // Resetting the AMBIENT is the real gravity reset; the presentation
        // `GravityField` is a per-tick mirror of the primary body's resolved
        // frame and has exactly one writer (`resolve_active_gravity`).
        *self.base_gravity = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
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

/// THE room-transition application, and there is only one.
///
/// Applying a prepared transition is *"put this RECORDED subject in this
/// PREPARED room"* — one operation, whatever host asks for it. Two hosts ask:
/// the eager one from an ordinary system, the rollback one from
/// `commit_confirmed_lifecycle`'s exclusive world once the frame is confirmed.
/// They differ in WHEN they are allowed to mutate the world, which is a
/// scheduling fact, not a different transition.
///
/// The same copy never recorded the Class-B transit either. Nobody wrote those omissions; they are
/// simply what a second implementation becomes.
///
/// `Query`, not `Single`, for the session world. `SystemState::get_mut`
/// PANICS when a `Single` matches nothing, and the confirmed host reaches this
/// through a `SystemState` on `&mut World` — a missing session root there must
/// be a refusal it can report, not a crash inside an exclusive system.
#[derive(SystemParam)]
pub struct RoomTransitionApplication<'w, 's> {
    commands: Commands<'w, 's>,
    effects: RoomTransitionEffects<'w>,
    bodies: TransitBodies<'w, 's>,
    /// Room authority, held on the session root: the geometry the body will
    /// collide against and the set that names which room is active.
    session: Query<
        'w,
        's,
        (&'static mut RoomGeometry, &'static mut world_rooms::RoomSet),
        With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    >,
    dev_state: ResMut<'w, ambition_dev_tools::DeveloperRuntimeState>,
    clock: RoomClock<'w>,
    moving_platforms: ResMut<'w, ambition_platformer2d_world::collision::MovingPlatformSet>,
    dialogue: ResMut<'w, ambition_dialog::DialogState>,
    conversation: ResMut<'w, ambition_conversation::ActiveConversation>,
    // RESIDENTS, not merely room-scoped. An object a body is carrying is
    // scoped to a room and resident in none — it crosses with whoever holds it —
    // so it is not part of what the room being left retires. The distinction is
    // spelled once, on `RoomResident`; this operation still knows nothing about
    // items, inventories, or who the player is.
    room_visuals:
        Query<'w, 's, (Entity, Option<&'static physics::PhysicsRoomEntity>), RoomResident>,
    tuning: Res<'w, ae::ActiveMovementTuning>,
    feel: Res<'w, Platformer2dFeelTuningMonolith>,
    carryover: RoomTransitionCombatReset<'w, 's>,
}

/// Why an application refused, with the world still whole.
///
/// A transition that fails after `retire_outgoing` has despawned the source room and has
/// nowhere to put the body, which is not a failure a caller can handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomTransitionApplyError {
    /// No session root carries room authority — there is no world to transition.
    NoSessionWorld,
    /// The body that crossed no longer exists. A VOID crossing: the transition
    /// fails rather than substituting whoever happens to be driving now.
    SubjectGone,
    /// The subject is not a body this operation can move.
    SubjectCannotTransit {
        subject: Entity,
        missing: &'static str,
    },
}

impl std::fmt::Display for RoomTransitionApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSessionWorld => {
                write!(f, "no session root carries RoomGeometry + RoomSet")
            }
            Self::SubjectGone => write!(
                f,
                "the body that triggered this room transition no longer exists; \
                 cancelling the crossing rather than transiting a substitute"
            ),
            Self::SubjectCannotTransit { subject, missing } => write!(
                f,
                "transiting body {subject:?} has no {missing} at room commit"
            ),
        }
    }
}

/// What a successful application did, for the caller's diagnostics.
pub struct AppliedRoomTransition {
    pub subject: Entity,
    pub arrival_pos: ae::Vec2,
}

impl RoomTransitionApplication<'_, '_> {
    /// Read the room authority the transaction's staleness checks compare
    /// against. `None` when no session root carries it, which
    /// [`Self::apply`] refuses on for the same reason.
    pub fn room_set(&self) -> Option<&world_rooms::RoomSet> {
        self.session.iter().next().map(|(_, room_set)| room_set)
    }

    /// Resolve the EXACT body a transition recorded, or `None`.
    ///
    /// An id that still resolves gives that body; one that no longer resolves
    /// gives `None` — never a substitute. The crossing body being gone is a void
    /// crossing, not a licence to move somebody else into the room the player
    /// walked to.
    pub fn subject_entity(
        &self,
        subject: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Option<Entity> {
        self.bodies.subject_entity(subject)
    }

    /// Apply a prepared transition to a resolved subject.
    ///
    /// The order is load-bearing and is the same order both hosts always needed:
    /// resolve and preflight everything that can fail, then mutate. Past the
    /// preflight block nothing here returns `Err`, so the source room is never
    /// retired for a crossing that then cannot complete.
    pub fn apply(
        &mut self,
        plan: &rooms::RoomConstructionPlan,
        subject: Entity,
        target_room: usize,
        arrival_at: ae::Vec2,
        edge_exit: bool,
        zone_sfx: Option<&str>,
    ) -> Result<AppliedRoomTransition, RoomTransitionApplyError> {
        // ── PREFLIGHT ────────────────────────────────────────────────────────
        if self.session.iter().next().is_none() {
            return Err(RoomTransitionApplyError::NoSessionWorld);
        }
        for (missing, present) in [
            ("MotionModel", self.bodies.motion_models.contains(subject)),
            (
                "complete actor cluster",
                self.bodies.clusters.contains(subject),
            ),
            ("BodyCombat", self.bodies.combat.contains(subject)),
        ] {
            if !present {
                return Err(RoomTransitionApplyError::SubjectCannotTransit { subject, missing });
            }
        }

        // Read-only facts gathered before the mutable borrows below.
        let subject_gravity_dir = self
            .bodies
            .motion_frames
            .get(subject)
            .map(|frame| frame.down())
            .unwrap_or(ae::Vec2::new(0.0, 1.0));
        let tuning = self.tuning.0;
        let feel = *self.feel;
        // THE BODY GOING THROUGH THE DOOR IS NOT RETIRED WITH THE ROOM IT IS
        // LEAVING. Stated about the SUBJECT, which is the only thing that
        // matters here, rather than about what kind of body it is.
        //
        // The proxy bought nothing and could only ever lose: when it answered correctly the
        // exemption was a no-op, and the one way it could answer WRONG is by calling a room-scoped
        // body a home body, which despawns the thing mid-transition.  an unconditional exemption
        // is strictly safer, because `retire_outgoing` skips an entity that is not in its roster
        // and exempting an absent entity is already a no-op.
        //
        // and it removes a player-centrism: the transition no longer has an
        // opinion about which body is the protagonist.
        let carry_body = Some(subject);

        // ── MUTATION ─────────────────────────────────────────────────────────
        // Nothing below may fail.

        // A fresh room inherits neither hostile shots nor the gravity frame of
        // the one just left.
        self.carryover.clear_carryover();

        let Ok((mut geometry, mut room_set)) = self.session.single_mut() else {
            // Unreachable: the preflight above proved exactly one match.
            return Err(RoomTransitionApplyError::NoSessionWorld);
        };
        let Ok(mut motion_model) = self.bodies.motion_models.get_mut(subject) else {
            return Err(RoomTransitionApplyError::SubjectCannotTransit {
                subject,
                missing: "MotionModel",
            });
        };
        let Ok(mut cluster_item) = self.bodies.clusters.get_mut(subject) else {
            return Err(RoomTransitionApplyError::SubjectCannotTransit {
                subject,
                missing: "complete actor cluster",
            });
        };
        let mut clusters = cluster_item.as_clusters_mut();

        // The door makes a sound, at the body's position BEFORE the transit.
        if let Some(cue) = zone_sfx {
            self.effects.sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::new(cue),
                pos: clusters.kinematics.pos,
            });
        }

        debug_assert_eq!(plan.target_index(), target_room);
        let player_size = clusters.kinematics.size;
        plan.retire_outgoing(
            &mut self.commands,
            self.room_visuals
                .iter()
                .map(|(entity, physics)| (entity, physics.is_some())),
            carry_body,
        );
        plan.commit_deferred(
            &mut self.commands,
            &mut room_set,
            &mut geometry,
            &mut self.moving_platforms.0,
        );

        // The authored arrival, validated against the NOW-target geometry using
        // the body's own size, so the body is never placed inside a solid or out
        // of bounds.
        let arrival = ambition_platformer2d_world::rooms::validated_spawn(
            &geometry.0,
            arrival_at,
            player_size,
        );
        ae::arrive_body_in_room(
            &mut motion_model,
            &mut clusters,
            arrival,
            tuning.air_jumps,
            if edge_exit {
                ae::ArrivalMomentum::Preserve
            } else {
                ae::ArrivalMomentum::Reset
            },
        );
        let arrival_pos = clusters.kinematics.pos;

        self.clock.clock_resets.write(ClockResetRequest::sim_clock(
            ambition_time::time_control::ClockRequester::Engine,
            "room_transition",
        ));
        self.clock.sim_state.remaining = if edge_exit {
            feel.edge_transition_cooldown
        } else {
            feel.door_transition_cooldown
        };
        self.dev_state.preset_flash = 1.0;

        // ── CROSS-DOMAIN PER-TRANSITION RESETS ───────────────────────────────
        // Four different domains' state, so no single domain owns this and the
        // operation that composes them does (anti-god rule 6). Optional
        // components are absent for a possessed non-home body, which is allowed.
        if let Ok(mut combat) = self.bodies.combat.get_mut(subject) {
            // The arrival flash is the one thing a transition adds.
            combat.reset();
            combat.hit_flash = if edge_exit {
                feel.edge_transition_flash
            } else {
                feel.door_transition_flash
            };
        }
        if let Ok((mut blink_cam, mut safety)) = self.bodies.presentation.get_mut(subject) {
            blink_cam.blink_in_timer = 0.0;
            blink_cam.blink_camera_from = arrival_pos;
            blink_cam.blink_camera_to = arrival_pos;
            blink_cam.camera_snap_timer = if edge_exit {
                0.0
            } else {
                ambition_platformer2d_actor_monolith::ROOM_DOOR_CAMERA_SNAP_TIME
            };
            safety.last_safe_pos = arrival_pos;
        }
        self.dialogue.close();
        // the AUTHORITY too, and it is not the same close. `DialogState` going quiet only
        // takes the text box away; the simulation's conversation names two BODIES, and this
        // transition just despawned the room they were standing in.
        self.conversation.close();

        if let Some(log) = self.bodies.class_b.as_mut() {
            log.record(
                subject,
                ambition_platformer2d_shared_tangle::class_b::ClassBRemap::RoomTransition,
            );
        }

        // ── PRESENTATION: ASK, DON'T DRAW ────────────────────────────────────
        // A room's static visuals + parallax are rebuilt by
        // `ambition_render::rendering::respawn_room_visuals_on_request`, which
        // reads the active room out of `RoomSet` for itself — the same channel
        // the sandbox reset and the room stager already use. Calling
        // `spawn_room_visuals` here instead was what made a room transition name
        // `ambition_platformer2d::render`, and therefore what kept the whole
        // commit chain app-local and unreachable by a demo host. A headless build
        // has no consumer and correctly skips the respawn.
        self.effects
            .respawn_room_visuals
            .write(world_rooms::RespawnRoomVisualsRequested);
        if edge_exit {
            // Edge exits should feel like contiguous room scrolling, not a
            // death-like teleport. Only an arrival puff in the new room, because
            // `from` would be expressed in the previous room's coordinate space.
            self.effects.vfx.write(VfxMessage::Burst {
                pos: arrival_pos,
                count: 18,
                speed: 260.0,
                color: [0.35, 0.95, 1.0, 0.75],
                kind: ParticleKind::Dust,
            });
        } else {
            // Door transitions are discrete interactions, so a teleport-like
            // effect is acceptable; use the destination for both endpoints to
            // avoid mixing coordinate systems from two rooms.
            self.effects.vfx.write(VfxMessage::ResetEffects {
                from: arrival_pos,
                to: arrival_pos,
            });
        }
        self.effects
            .sfx
            .write(SfxMessage::Reset { pos: arrival_pos });

        log_room_transition_landing(
            target_room,
            &room_set,
            arrival_pos,
            player_size,
            subject_gravity_dir,
            &geometry.0,
            &self.carryover.feature_overlay,
        );

        Ok(AppliedRoomTransition {
            subject,
            arrival_pos,
        })
    }
}

/// The bodies a room transition can relocate, bundled into one `SystemParam` to
/// keep `commit_ready_room_transition_system` under Bevy's 16-param limit.
///
/// A transition moves the body the REQUEST NAMED — the home avatar during normal
/// play, a possessed actor, or whatever else crossed. `clusters` is body-generic
/// (`ae::BodyClusterQueryData` matches every body: the home avatar AND actors
/// carry the same movement clusters), so one `get_mut(subject)` relocates
/// whichever body it is. `presentation` holds the home-only blink-camera +
/// respawn-point state (a possessed actor has neither).
///
/// They answered *"who is driving right now"*, which is a different question from *"who walked
/// through the door"* the moment readiness takes more than one frame. The request carries the
/// answer; this only resolves it.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TransitBodies<'w, 's> {
    clusters: Query<'w, 's, ae::BodyClusterQueryData>,
    /// The transiting body's movement policy — a room transition is a discrete
    /// TRANSIT (ADR 0024 authority) and must reconcile model-private attachment.
    motion_models: Query<'w, 's, &'static mut ambition_platformer2d_core::movement::MotionModel>,
    combat: Query<'w, 's, &'static mut ambition_characters::actor::BodyCombat>,
    /// The transiting body's resolved gravity frame — read (before the mutable
    /// cluster borrow) so the landing diagnostic probes along the body's own
    /// gravity, not world-down.
    motion_frames:
        Query<'w, 's, &'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    presentation: Query<
        'w,
        's,
        (
            &'static mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
            &'static mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    /// Stable body identity — how the transiting subject is NAMED. The request
    /// records a `SimId` at detection and this resolves it at commit; see
    /// [`Self::subject_entity`].
    sim_ids: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_platformer2d_shared_tangle::sim_id::SimId,
        ),
    >,
    /// The Class-B transit ledger (`docs/concepts/movement-collision.md`). It rides in
    /// this param because a room transition IS one of the four Class-B
    /// authorities, and this struct is the one that names the body it moves.
    /// `Option`, and bundled here rather than added to the system's signature —
    /// `commit_ready_room_transition_system` already sits at Bevy's 16-param ceiling.
    class_b: Option<ResMut<'w, ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
}

impl TransitBodies<'_, '_> {
    /// Resolve the EXACT body a transition request recorded, or `None`.
    ///
    /// Mirrors the confirmed side's `resolve_transition_subject` in both
    /// behaviour and REFUSAL: an id that still resolves gives that body, and one
    /// that no longer resolves gives `None` — never a substitute. The crossing
    /// body being gone is a void crossing, not a licence to move somebody else
    /// into the room the participant crossed toward.
    pub fn subject_entity(
        &self,
        subject: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Option<Entity> {
        self.sim_ids
            .iter()
            .find(|(_, id)| *id == subject)
            .map(|(entity, _)| entity)
    }
}

/// Retire one eager-host transaction while leaving the still-authoritative
/// source room intact.
///
/// The pending rollback-state intent is deliberately NOT touched here; callers
/// decide whether this is a transient transaction replacement (keep the intent)
/// or a terminal crossing cancellation (clear the exact intent first).
fn cancel_eager_room_transition_transaction(
    transition_state: &mut super::loading::RoomTransitionLoadState,
    loads: &mut ambition_load::LoadCoordinator,
    load_events: &mut MessageWriter<ambition_load::LoadEvent>,
    next_mode: &mut bevy::prelude::NextState<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >,
    active: &super::loading::ActiveRoomTransitionLoad,
    mode_cause: &'static str,
) {
    for event in loads.apply(ambition_load::LoadCommand::Cancel {
        load_id: active.barrier.load_id.clone(),
    }) {
        load_events.write(event);
    }
    loads.retire(&active.barrier.load_id);
    if transition_state
        .active
        .as_ref()
        .is_some_and(|current| current.sequence == active.sequence)
    {
        transition_state.active = None;
    }
    ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
        ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
        mode_cause,
    );
    next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
}

pub fn commit_ready_room_transition_system(
    // What this system owns is the TRANSACTION — the staleness checks, the barrier, the cover
    // bookkeeping — and it reaches the world only through the one operation the confirmed host
    // also calls.
    mut application: RoomTransitionApplication,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    load_resources: (
        Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
        Res<super::loading::RoomTransitionContentEpoch>,
        ResMut<super::loading::RoomTransitionLoadState>,
        ResMut<ambition_load::LoadCoordinator>,
        MessageWriter<ambition_load::LoadEvent>,
        ResMut<bevy::prelude::NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
        Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
        // Whose commit this is. The STABLE simulation host, not the
        // optional boundary of its current session. A rollback session teardown
        // removes `ConfirmedFrameBoundary` but does not turn the app into an
        // eager host. Rollback-host room changes must always go through
        // `commit_confirmed_lifecycle`'s rebase; this system would mutate the
        // world inside the rewound schedule and the next restore would put the
        // old room back.
        Res<crate::SimulationHost>,
        // The crossing's own record, cleared when it lands. Sticky until then, so
        // leaving it set would wedge every later crossing.
        ResMut<
            ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit,
        >,
    ),
) {
    let (
        active_session,
        content_epoch,
        mut transition_state,
        mut loads,
        mut load_events,
        mut next_mode,
        real_time,
        simulation_host,
        mut pending_lifecycle,
    ) = load_resources;
    // the EAGER commit, and only the eager one. A rollback host reaches an
    // identical room change through `commit_confirmed_lifecycle`, which runs
    // outside the rewound schedule and rebases the session afterwards. Both read
    // the same authorized transaction; they differ in what they must do to be
    // allowed to mutate the world at all.
    //
    // Host identity is deliberately NOT inferred from `ConfirmedFrameBoundary`.
    // `stop_session` removes that boundary while `SimulationHost::Rollback` remains
    // installed. Reclassifying that state as eager is exactly how an invalidated
    // rollback session acquired a loading transaction that no schedule could
    // ever commit.
    if simulation_host.is_rollback() {
        return;
    }

    let Some(active) = transition_state
        .active
        .as_ref()
        .filter(|active| active.phase == super::loading::RoomTransitionLoadPhase::CommitAuthorized)
        .cloned()
    else {
        return;
    };

    // The host-side transaction is DERIVED from this exact simulation intent. A different
    // pending intent is equally not this transaction's authority — keep that new intent, retire
    // only the stale transaction, and let readiness open the new one in `Update`.
    let intent_still_pending = pending_lifecycle.pending.as_ref().is_some_and(|pending| {
        matches!(
            &pending.kind,
            ambition_platformer2d_actor_monolith::session::lifecycle_commit::LifecycleIntent::Transition(intent)
                if intent == &active.intent
        )
    });
    if !intent_still_pending {
        cancel_eager_room_transition_transaction(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            &mut next_mode,
            &active,
            "room_commit_intent_retracted",
        );
        bevy::log::info!(
            target: "ambition_platformer2d::room_transition",
            "room transition {} cancelled before commit because its lifecycle intent is no longer pending",
            active.sequence,
        );
        return;
    }

    // The room authority this transaction was opened against. Absent means the
    // session world is gone under it, which is exactly as stale as any other
    // mismatch below.
    let Some(room_set) = application.room_set() else {
        super::loading::fail_room_transition_commit_precondition(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            active.sequence,
            "no session root carries room authority at commit".to_string(),
        );
        return;
    };
    let room_set_active = room_set.active;
    let target_still_matches = room_set.rooms.get(active.target_room).is_some_and(|room| {
        room.id == active.target_room_id
            && active
                .construction_plan
                .as_ref()
                .is_some_and(|plan| plan.matches_room_spec(room))
    });
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    if active.content_epoch != content_epoch.get()
        || active.session_scope != current_session
        || room_set_active != active.source_room
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
            room_set_active,
        );
        cancel_eager_room_transition_transaction(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            &mut next_mode,
            &active,
            "room_commit_failed",
        );
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
    if construction_plan.target_index() != active.target_room
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
                active.target_room,
            ),
        );
        return;
    }

    // Moving the intent out first would partially move the value those arms still need to describe.
    let intent = active.intent.clone();
    // The body that CROSSED is the body that ARRIVES — resolved from the
    // `SimId` the DETECTION recorded, never re-derived here.
    //
    // this asked `ControlledSubject`, falling back to the primary player, under a comment claiming
    // *"this is the same subject the detect side resolves"*. A citation, and a false one across
    // time: readiness takes several frames, and possession changing hands or a death inside that
    // window silently transited a different body than the one that walked through the door.
    //
    // A subject that no longer resolves is a VOID crossing: the crossing body is
    // gone, so the transition FAILS rather than substituting whoever is driving
    // now. Same rule, same words, as `commit_transition`'s confirmed side.
    // The body that CROSSED is the body that ARRIVES — resolved from the
    // `SimId` the DETECTION recorded, never re-derived here.
    //
    // this asked `ControlledSubject`, falling back to the primary player, under a comment claiming
    // *"this is the same subject the detect side resolves"*. A citation, and a false one across
    // time: readiness takes several frames, and possession changing hands or a death inside that
    // window silently transited a different body than the one that walked through the door.
    //
    // A missing subject is terminal. The confirmed host already calls this a
    // `CommitOutcome::Cancelled`; the eager host has no speculative frames, so it
    // can make the same decision here by consuming the exact pending intent and
    // retiring its transaction. Treating this as retryable reopens the same
    // impossible crossing forever. Death-owned crossings are retracted earlier by
    // the eager death lifecycle, before an out-of-play body can reach this point.
    let Some(subject) = application.subject_entity(&intent.subject) else {
        pending_lifecycle.take();
        cancel_eager_room_transition_transaction(
            &mut transition_state,
            &mut loads,
            &mut load_events,
            &mut next_mode,
            &active,
            "room_commit_subject_unavailable",
        );
        bevy::log::warn!(
            target: "ambition_platformer2d::room_transition",
            "the body that triggered room transition {} ({:?}) is gone; cancelling the crossing",
            active.sequence,
            intent.subject,
        );
        return;
    };

    let target_room = active.target_room;
    // AMBITION_REVIEW(determinism): wall clock, and deliberately so — this
    // measures how long the commit TOOK for `commit_duration`, a write-only
    // diagnostic field on `ActiveRoomTransitionLoad`. It is never read back, and
    // `RoomTransitionLoadState` is not rollback-registered, so no sim decision
    // can observe it. Timing a transaction with `SimTick` would measure the
    // wrong thing: the point is wall-clock cost to the player.
    #[cfg(not(target_arch = "wasm32"))]
    let commit_started = std::time::Instant::now();
    // Both hosts call this; what the eager host owns is the TRANSACTION around it — the
    // staleness checks above and the barrier/cover bookkeeping below — not a second idea of
    // what a room transition does to the world.
    if let Err(error) = application.apply(
        construction_plan,
        subject,
        target_room,
        intent.arrival,
        intent.edge_exit,
        intent.zone_sfx.as_deref(),
    ) {
        match error {
            terminal @ RoomTransitionApplyError::SubjectGone
            | terminal @ RoomTransitionApplyError::SubjectCannotTransit { .. } => {
                // The body cannot become eligible later without some other
                // lifecycle operation replacing this crossing. This exact intent
                // is therefore spent as a cancellation, not retained as a retry.
                pending_lifecycle.take();
                cancel_eager_room_transition_transaction(
                    &mut transition_state,
                    &mut loads,
                    &mut load_events,
                    &mut next_mode,
                    &active,
                    "room_commit_subject_unavailable",
                );
                bevy::log::warn!(
                    target: "ambition_platformer2d::room_transition",
                    "room transition {} cancelled: {terminal}",
                    active.sequence,
                );
            }
            transient @ RoomTransitionApplyError::NoSessionWorld => {
                super::loading::fail_room_transition_commit_precondition(
                    &mut transition_state,
                    &mut loads,
                    &mut load_events,
                    active.sequence,
                    transient.to_string(),
                );
            }
        }
        return;
    }
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
    pending_lifecycle.take();
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
        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
            ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
            "room_commit_uncovered",
        );
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
    }
}

/// Emit one landing diagnostic for each committed room transition, including
/// world/overlay collision coverage and the support gap below the arriving body.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &world_rooms::RoomSet,
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
