//! The authorized room-transition commit: swap room authority, carry the
//! transiting body across, apply the cross-domain per-transition resets, ask for
//! the redraw, and log the landing.

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, ResMut, With};


use ambition_platformer2d_publication as publication;
use ambition_platformer2d_shared_tangle::lifecycle::RoomResident;
use ambition_platformer2d_world::rooms as world_rooms;

use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_time::time_control::ClockResetRequest;
use ambition_vfx::{ParticleKind, VfxMessage};
use ambition_vfx::VfxWriter;

/// The sim → presentation channels a committed transition writes: the zone
/// sound, the arrival puff, and the request to rebuild the destination room's
/// static visuals.
///
/// Bundled to keep the commit system under Bevy's 16-`SystemParam` ceiling,
/// which it sits at.
#[derive(SystemParam)]
pub struct RoomTransitionEffects<'w> {
    pub sfx: SfxWriter<'w>,
    pub vfx: VfxWriter<'w>,
}

/// Sim state plus the clock-reset channel, so a system already at the parameter
/// ceiling can reach both through one slot. The reset is emitted as DATA and
/// consumed by the time-control owner — no system here mutates `time_scale`.
#[derive(SystemParam)]
pub struct RoomClock<'w, 's> {
    pub sim_state: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        'w,
        's,
        ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown,
    >,
    pub clock_resets: MessageWriter<'w, ClockResetRequest>,
}

/// State a fresh room must not inherit: combat in flight, the ambient gravity
/// frame, and the collision overlay of the room just left.
#[derive(SystemParam)]
pub struct RoomTransitionCombatReset<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub live_projectiles: Query<
        'w,
        's,
        (Entity, Option<&'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>),
        With<ambition_projectiles::LiveProjectile>,
    >,
    pub base_gravity: ResMut<'w, ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
}

impl RoomTransitionCombatReset<'_, '_> {
    /// Drop the in-flight projectiles of the room left behind and return
    /// ambient gravity to its default, so a fresh room does not inherit combat
    /// events or a stale gravity frame. The walls of the room just left are
    /// retracted by the publication itself, on the root whose geometry it
    /// replaces.
    ///
    /// ⭐ ONLY WHAT THE CROSSING LEAVES BEHIND (OW1, customer 2). With no
    /// other live room standing, the crossing replaces the whole world: every
    /// projectile goes, and every ambient gravity goes back to its default.
    /// While another live room stays (another player is in the room left, or
    /// in a room of its own), its shots and its gravity are that room's: only
    /// the shots stamped into the room left go, and only the gravity of the
    /// room left is forgotten, and only when that room retires. The room
    /// entered has a new identity, so it starts under the default.
    pub fn clear_carryover(&mut self, scope: &CrossingScope) {
        for (entity, stamp) in &self.live_projectiles {
            let left_behind = !scope.other_rooms_stay
                || stamp.is_some_and(|stamp| scope.retiring.contains(&stamp.0));
            if left_behind {
                self.commands.entity(entity).despawn();
            }
        }
        // Resetting the AMBIENT is the real gravity reset; the presentation
        // `GravityField` is a per-tick mirror of the primary body's resolved
        // frame and has exactly one writer (`resolve_active_gravity`).
        if !scope.other_rooms_stay {
            *self.base_gravity = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
        } else {
            for room in &scope.retiring { self.base_gravity.forget(Some(*room)); }
        }
    }
}

/// What a crossing leaves standing, decided when it is staged (OW1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrossingScope {
    /// The source rooms that have no remaining claim after this transfer.
    pub retiring: Vec<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    /// Whether a live room stays standing through the crossing: the room
    /// left (another player stays in it), or another live room. Then the
    /// crossing does not replace the world: the world's shared sim clock is
    /// not reset, and each room that stays keeps its ambient gravity.
    pub other_rooms_stay: bool,
}

/// Probe along the body's gravity direction from its feet for the nearest
/// landing face of the room geometry (within 256 px). `None` means nothing: the
/// body is over a pit.
///
/// The collision overlay is not probed. At the commit it holds no contribution
/// for the arrival room yet (the publication retracts the old room's), so a probe of
/// it could only report the room the body left.
///
/// Frame-relative: "below feet" is +gravity, not world-down, so the diagnostic
/// stays meaningful under a gravity flip (identity under normal gravity).
fn ground_gap_below_feet(
    body: &ae::Aabb,
    gravity_dir: ae::Vec2,
    world: &ae::World,
) -> Option<f32> {
    const MAX_PROBE_PX: f32 = 256.0;
    // Side axis ⊥ gravity (`gravity_half(side)` reuses the projection to get an
    // AABB's extent along it).
    let side = ae::Vec2::new(gravity_dir.y, -gravity_dir.x);
    let feet = body.feet_coord(gravity_dir);
    let body_side = body.center().dot(side);
    let body_side_half = body.gravity_half(side);
    let mut best: Option<f32> = None;
    for block in &world.blocks {
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
    continuity: ambition_platformer2d_session::room_preparation::RoomContinuity<'w, 's>,
    bodies: TransitBodies<'w, 's>,
    /// Room authority, held on the session root: the set that names which
    /// room is active. The geometry is on the live room's own root.
    session: Query<
        'w,
        's,
        &'static world_rooms::RoomSet,
        With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    >,
    parties: Query<'w, 's, (
        &'static ambition_platformer2d_session::party_travel::PartyTravel,
        &'static ambition_platformer2d_session::participants::SessionParticipants,
    ), With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>>,
    network: Res<'w, world_rooms::GateNetworkState>,
    subject_room: Query<'w, 's, (Entity, &'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance)>,
    collision: ambition_platformer2d_world::collision::CollisionWorld<'w, 's>,
    /// The bodies players drive, and the live room each is in: a room a
    /// crossing leaves stays live while another player's body is in it (OW1
    /// cut 6c).
    drivers: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_characters::control::DrivingParticipant,
            Option<&'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
    >,
    /// Which room of the set each live room is (OW1 cut 5e).
    definitions: Query<
        'w,
        's,
        (&'static world_rooms::LiveRoomInstance, &'static world_rooms::LiveRoomDefinition),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    // ⛔⛤ **THE EFFECT CHANNELS ARE GONE FROM THIS PARAM, AND THEIR ABSENCE IS
    // THE POINT.** The sfx/vfx writers, the clock, the developer overlay, the
    // dialogue and the conversation moved to `RoomTransitionFinalize`, which only
    // a published crossing reaches. Staging cannot express *"the crossing
    // happened"* any more — the compiler says so.
    // RESIDENTS, not merely room-scoped. An object a body is carrying is
    // scoped to a room and resident in none — it crosses with whoever holds it —
    // so it is not part of what the room being left retires. The distinction is
    // spelled once, on `RoomResident`; this operation still knows nothing about
    // items, inventories, or who the player is.
    room_visuals: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static ambition_platformer2d_physics_debris::PhysicsRoomEntity>,
            Option<&'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        RoomResident,
    >,
    tuning: Res<'w, ae::ActiveMovementTuning>,
}

/// Why an application refused, with the world still whole.
///
/// A transition that fails after the room sweep has despawned the source room
/// and has nowhere to put the body, which is not a failure a caller can handle.
/// The sweep is this module's own `room_visuals` roster — the `RoomResident`
/// alias — handed to `replace_live_world`. ⚠ It named a `retire_outgoing` until  <!-- cite-ok: names the method A10 DELETED on 2026-09-14; this sentence RECORDS the dead name, so a resolvable citation here would mean the deletion did not happen -->
/// 2026-09-19; no such function has ever existed in this workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomTransitionApplyError {
    /// No session root carries room authority — there is no world to transition.
    NoSessionWorld,
    /// A consumed occurrence or mint recipe changed after preparation.
    StaleContinuity,
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
            Self::StaleContinuity => write!(f, "the prepared destination no longer matches its occurrence continuity"),
            Self::NoSessionWorld => {
                write!(f, "no session root carries a RoomSet")
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

impl RoomTransitionApplication<'_, '_> {
    /// Read the room authority the transaction's staleness checks compare
    /// against. `None` when no session root carries it, which
    /// [`Self::apply`] refuses on for the same reason.
    pub fn room_set(&self) -> Option<&world_rooms::RoomSet> {
        self.session.iter().next()
    }

    /// Which room of the set a crossing by `subject` leaves: the live room
    /// the subject was recorded in (OW1 cut 6b). With no subject, or a
    /// subject in no live room, the sole live room, and none when two are
    /// live.
    pub fn definition_left_by(
        &self,
        subject: Option<&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId>,
    ) -> Option<world_rooms::LiveRoomDefinition> {
        match subject.and_then(|subject| subject.room) {
            Some(room) => self
                .definitions
                .iter()
                .find(|(live, _)| **live == room)
                .map(|(_, definition)| *definition),
            None => self.definitions.single().ok().map(|(_, definition)| *definition),
        }
    }

    /// Resolve the EXACT body a transition recorded, or `None`.
    ///
    /// An id that still resolves gives that body; one that no longer resolves
    /// gives `None` — never a substitute. The crossing body being gone is a void
    /// crossing, not a licence to move somebody else into the room the player
    /// walked to.
    pub fn subject_entity(
        &self,
        subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId,
    ) -> Option<Entity> {
        self.bodies.subject_entity(subject)
    }

    fn prepare_member(
        &self,
        subject: Entity,
        participant: Option<ambition_characters::control::PlayerSlot>,
    ) -> Result<StagedRoomMember, RoomTransitionApplyError> {
        for (missing, present) in [
            ("MotionModel", self.bodies.motion_models.contains(subject)),
            ("complete actor cluster", self.bodies.clusters.contains(subject)),
            ("BodyCombat", self.bodies.combat.contains(subject)),
        ] {
            if !present { return Err(RoomTransitionApplyError::SubjectCannotTransit { subject, missing }); }
        }
        let body = self.bodies.clusters.get(subject).expect("the complete body was checked");
        let size = body.kinematics.size;
        let arrival_pos = body.kinematics.pos;
        let gravity_dir = self.bodies.motion_frames.get(subject).map(|frame| frame.down())
            .unwrap_or(ae::Vec2::new(0.0, 1.0));
        Ok(StagedRoomMember { subject, participant, arrival_pos, size, gravity_dir })
    }

    fn arrival_world<'a>(
        &'a self,
        plan: &'a publication::RoomConstructionPlan,
        live: Option<world_rooms::LiveRoomInstance>,
        subject: Entity,
    ) -> std::borrow::Cow<'a, ae::World> {
        let stamp = live.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let base = stamp.as_ref().and_then(|stamp| self.collision.room(Some(stamp)))
            .and_then(|room| room.solids_for(subject))
            .unwrap_or(std::borrow::Cow::Borrowed(&plan.spec().world));
        let rooms = self.room_set().expect("staging checked the session room set");
        let (Ok((party, admitted)), Ok((_, driver, _)), Some(definition)) = (
            self.parties.single(), self.drivers.get(subject), rooms.definition(plan.target_index()),
        ) else { return base; };
        // Incoming bodies may have no wall contribution in an already-live room.
        let mut walls = ambition_platformer2d_session::rooms::party_exit_walls(party, admitted, driver.0, rooms, definition, &self.network);
        walls.retain(|block| !base.blocks.iter().any(|existing| existing.id == block.id));
        if walls.is_empty() { return base; }
        let mut result = base.into_owned();
        result.blocks.add_all(walls);
        std::borrow::Cow::Owned(result)
    }

    fn member_box(&self, member: &StagedRoomMember) -> ae::Aabb {
        let shape = self.bodies.motion_models.get(member.subject).expect("the movement model was checked")
            .spec().spawn_shape(member.size, member.gravity_dir);
        ae::Aabb::new(member.arrival_pos, shape.body_half)
    }

    /// Stage every selected body through one destination publication.
    pub fn stage(
        &mut self,
        plan: &publication::RoomConstructionPlan,
        intent: &ambition_platformer2d_session::lifecycle_commit::LifecycleIntent,
        target_room: usize,
        restart: bool,
        restore: Option<ambition_platformer2d_session::checkpoint::CheckpointOperationKey>,
    ) -> Result<StagedRoomTransition, RoomTransitionApplyError> {
        use std::collections::BTreeSet;
        let Some(room_set) = self.session.iter().next() else {
            return Err(RoomTransitionApplyError::NoSessionWorld);
        };
        debug_assert_eq!(plan.target_index(), target_room);
        let target = room_set.definition(target_room);
        let mints = room_set.next_live_room();
        let mut members = Vec::new();
        let mut seen = BTreeSet::new();
        for transfer in intent.transfers() {
            let subject = self.subject_entity(&transfer.subject).ok_or(RoomTransitionApplyError::SubjectGone)?;
            if !seen.insert(subject) {
                return Err(RoomTransitionApplyError::SubjectCannotTransit { subject, missing: "unique transfer membership" });
            }
            members.push(self.prepare_member(subject, transfer.participant)?);
        }
        let room_plan = ambition_platformer2d_session::rooms::room_transfer_plan(
            members.iter().map(|member| (Some(member.subject), member.participant,
                self.subject_room.get(member.subject).ok().map(|(_, room)| room.0))),
            self.drivers.iter().map(|(entity, driver, room)| (entity, driver.0, room.map(|room| room.0))),
            self.definitions.iter().map(|(live, definition)| (*live, Some(*definition))),
            target, intent.rebuilds_source_room(), restart,
        );
        if room_plan.joins.is_none() {
            let continuity = self.continuity.select(intent, restore)
                .map_err(|_| RoomTransitionApplyError::StaleContinuity)?;
            let outlook = continuity.occurrences.as_ref().map(|ledger| ledger.outlook_for(plan.room_id())).unwrap_or_default();
            if !plan.matches_continuity(&outlook, continuity.minted.as_ref()) {
                return Err(RoomTransitionApplyError::StaleContinuity);
            }
        }
        let transfers: Vec<_> = intent.transfers().collect();
        let mut resolved = vec![false; members.len()];
        for (index, transfer) in transfers.iter().enumerate() {
            if let ambition_platformer2d_session::lifecycle_commit::RoomArrival::At(at) = transfer.arrival {
                if !at.is_finite() {
                    return Err(RoomTransitionApplyError::SubjectCannotTransit {
                        subject: members[index].subject, missing: "finite arrival point",
                    });
                }
                let walls = self.arrival_world(plan, room_plan.joins, members[index].subject);
                members[index].arrival_pos = world_rooms::validated_spawn(&walls, at, members[index].size);
                resolved[index] = true;
            }
        }
        while resolved.iter().any(|done| !done) {
            let mut progress = false;
            for (index, transfer) in transfers.iter().enumerate() {
                if resolved[index] { continue; }
                let ambition_platformer2d_session::lifecycle_commit::RoomArrival::NearBody(reference) = &transfer.arrival
                    else { unreachable!("point arrivals were resolved"); };
                let anchor = self.subject_entity(reference).ok_or(RoomTransitionApplyError::SubjectGone)?;
                let reference = if let Some(anchor_index) = members.iter().position(|member| member.subject == anchor) {
                    if !resolved[anchor_index] { continue; }
                    self.member_box(&members[anchor_index])
                } else {
                    if room_plan.joins.is_none() || self.subject_room.get(anchor).ok().map(|(_, room)| room.0) != room_plan.joins {
                        return Err(RoomTransitionApplyError::SubjectCannotTransit {
                            subject: anchor, missing: "arrival reference in the destination room",
                        });
                    }
                    self.member_box(&self.prepare_member(anchor, None)?)
                };
                let mut occupied: Vec<_> = members.iter().zip(&resolved)
                    .filter(|(_, done)| **done).map(|(member, _)| self.member_box(member)).collect();
                if let Some(target) = room_plan.joins {
                    occupied.extend(self.subject_room.iter()
                        .filter(|(entity, room)| room.0 == target && !seen.contains(entity))
                        .filter_map(|(entity, _)| self.prepare_member(entity, None).ok())
                        .map(|member| self.member_box(&member)));
                }
                let member = &mut members[index];
                let frame = ae::AccelerationFrame::new(member.gravity_dir);
                let shape = self.bodies.motion_models.get(member.subject).expect("the movement model was checked")
                    .spec().spawn_shape(member.size, member.gravity_dir);
                let walls = self.arrival_world(plan, room_plan.joins, member.subject);
                member.arrival_pos = world_rooms::spawn_near_body(&walls, reference, frame, shape, &occupied)
                    .ok_or(RoomTransitionApplyError::SubjectCannotTransit {
                        subject: member.subject, missing: "clear arrival space",
                    })?;
                resolved[index] = true;
                progress = true;
            }
            if !progress {
                return Err(RoomTransitionApplyError::SubjectCannotTransit {
                    subject: members[resolved.iter().position(|done| !done).unwrap()].subject,
                    missing: "resolved arrival reference",
                });
            }
        }
        let departing = room_plan.departing;
        if intent.rebuilds_source_room() && !restart {
            let mut survivors: Vec<_> = self.drivers.iter()
                .filter(|(entity, _, room)| !seen.contains(entity) && room.map(|room| room.0) == departing)
                .filter_map(|(entity, driver, _)| self.bodies.live_bodies.id_of(entity)
                    .map(|id| (id, entity, driver.0))).collect();
            survivors.sort_by(|(a, ..), (b, ..)| a.sim_id.as_str().cmp(b.sim_id.as_str()));
            for (_, subject, participant) in survivors {
                seen.insert(subject);
                let mut member = self.prepare_member(subject, Some(participant))?;
                let walls = self.arrival_world(plan, room_plan.joins, member.subject);
                member.arrival_pos = world_rooms::validated_spawn(&walls, member.arrival_pos, member.size);
                members.push(member);
            }
        }
        let retiring = room_plan.retiring;
        let keeps_left = departing.is_some_and(|room| !retiring.contains(&room));
        let succession = departing.map(|leaves| publication::LiveRoomSuccession::for_crossing(
            leaves, mints, keeps_left, room_plan.joins,
        ));
        let retires_beside: Vec<_> = retiring.iter().copied().filter(|live| Some(*live) != departing).collect();
        let other_rooms_stay = room_plan.other_rooms_stay;
        let door_sfx = intent.zone_sfx().zip(members.first()).and_then(|(cue, member)| {
            self.bodies.clusters.get(member.subject).ok().map(|body| (cue.to_string(), body.kinematics.pos))
        });
        let publication = plan.replace_live_world_restoring(
            &mut self.commands,
            self.room_visuals.iter().filter(|(_, _, room)| {
                room.map_or(!other_rooms_stay, |room| retiring.contains(&room.0))
            }).map(|(entity, physics, _)| (entity, physics.is_some())),
            members.iter().map(|member| member.subject),
            None,
            members.iter().map(|member| publication::StagedArrival {
                subject: member.subject, arrival: member.arrival_pos, air_jumps: self.tuning.0.air_jumps,
                momentum: if intent.edge_exit() { ae::ArrivalMomentum::Preserve } else { ae::ArrivalMomentum::Reset },
            }),
            succession, retires_beside, restore,
        );
        Ok(StagedRoomTransition {
            publication, scope: CrossingScope { retiring: retiring.into_iter().collect(), other_rooms_stay },
            members, target_room, edge_exit: intent.edge_exit(), door_sfx,
            target_world: plan.spec().world.clone(),
            resets_clock: intent.rebuilds_source_room(),
        })
    }

}

/// A crossing that has been prepared and whose candidate room is built but
/// HIDDEN, waiting on its publication's verdict.
///
/// ⚠ **PLAIN DATA, HOST-LOCAL.** It holds a [`publication::PublicationHandle`], which
/// is control-plane identity and never canonical rollback state, plus the facts
/// finalization cannot re-derive afterwards — the body's pre-transit position
/// for the door cue, its size and gravity frame for the landing diagnostic, and
/// the TARGET room's geometry, which the live component does not hold until the
/// verdict.
pub struct StagedRoomTransition {
    pub publication: publication::PublicationHandle,
    /// What the crossing leaves standing.
    pub scope: CrossingScope,
    pub members: Vec<StagedRoomMember>,
    pub target_room: usize,
    pub edge_exit: bool,
    /// The zone cue and the position to play it at: where the body stood BEFORE
    /// the transit.
    pub door_sfx: Option<(String, ae::Vec2)>,
    pub target_world: ae::World,
    resets_clock: bool,
}

/// Per-body facts frozen before the destination publishes.
pub struct StagedRoomMember {
    pub subject: Entity,
    pub participant: Option<ambition_characters::control::PlayerSlot>,
    pub arrival_pos: ae::Vec2,
    pub size: ae::Vec2,
    pub gravity_dir: ae::Vec2,
}

/// Everything a crossing does that MEANS the crossing happened.
///
/// Separate from [`RoomTransitionApplication`] because these are exactly the
/// writes that must not happen until the room published. Both hosts reach them
/// through [`finalize_room_transition`].
#[derive(SystemParam)]
pub struct RoomTransitionFinalize<'w, 's> {
    effects: RoomTransitionEffects<'w>,
    clock: RoomClock<'w, 's>,
    dev_state: ResMut<'w, ambition_dev_tools::DeveloperRuntimeState>,
    // `Option`: the dialogue capability is removable (`Capability::Dialogue`), and a
    // transition in a composition without it has no text box or conversation to close.
    dialogue: Option<ResMut<'w, ambition_dialog::DialogState>>,
    conversation: Option<ResMut<'w, ambition_conversation::ActiveConversation>>,
    carryover: RoomTransitionCombatReset<'w, 's>,
    bodies: TransitBodies<'w, 's>,
    /// The room set AFTER publication — the landing diagnostic reports against
    /// the room the body actually arrived in.
    session: Query<
        'w,
        's,
        &'static world_rooms::RoomSet,
        With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    >,
    /// The live rooms AFTER publication: the arrival effect is drawn in the
    /// live room the body arrived in.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'w, 's>,
    feel: Res<'w, Platformer2dFeelTuningMonolith>,
    /// A body that came through a passage comes out of its door or gate: it
    /// is not flashed in.
    passages: Query<'w, 's, &'static mut ambition_platformer2d_session::rooms::passage::ZonePassage>,
}

impl RoomTransitionFinalize<'_, '_> {
    /// Apply what a published crossing means, and say the live room it
    /// arrived in: the subject's, or for a rebuild with nobody in it, the sole
    /// live room.
    fn apply_crossing(
        &mut self,
        staged: &StagedRoomTransition,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
        let feel = *self.feel;
        self.carryover.clear_carryover(&staged.scope);
        if let Some((cue, pos)) = &staged.door_sfx {
            self.effects.sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::new(cue), emitter: (*pos).into(),
            });
        }
        // A normal crossing does not reset session time control.
        if staged.resets_clock && !staged.scope.other_rooms_stay {
            self.clock.clock_resets.write(ClockResetRequest::sim_clock(
                ambition_time::time_control::ClockRequester::Engine, "room_transition",
            ));
        }
        self.dev_state.preset_flash = 1.0;
        for member in &staged.members {
            let subject = member.subject;
            let arrival_pos = member.arrival_pos;
            if let Some(seat) = member.participant {
                self.clock.sim_state.hold(usize::from(seat.0), if staged.edge_exit {
                    feel.edge_transition_cooldown
                } else { feel.door_transition_cooldown });
            }
            let through_a_passage = match self.passages.get_mut(subject) {
                Ok(mut passage) => {
                    let facing = self.bodies.clusters.get(subject).map(|body| body.kinematics.facing).unwrap_or(1.0);
                    passage.arrived(facing, Some(arrival_pos));
                    true
                }
                Err(_) => false,
            };
            if let Ok(mut combat) = self.bodies.combat.get_mut(subject) {
                combat.reset();
                combat.hit_flash = if through_a_passage { 0.0 }
                    else if staged.edge_exit { feel.edge_transition_flash }
                    else { feel.door_transition_flash };
            }
            if let Ok((mut blink_cam, mut safety)) = self.bodies.presentation.get_mut(subject) {
                blink_cam.blink_in_timer = 0.0;
                blink_cam.blink_camera_from = arrival_pos;
                blink_cam.camera_snap_timer = if staged.edge_exit { 0.0 } else { crate::ROOM_DOOR_CAMERA_SNAP_TIME };
                safety.last_safe_pos = arrival_pos;
            }
            if let Some(log) = self.bodies.class_b.as_mut() {
                log.record(subject, ambition_platformer2d_shared_tangle::class_b::ClassBRemap::RoomTransition);
            }
            let arrival_room = self.rooms.of(subject);
            if staged.edge_exit {
                self.effects.vfx.for_room(arrival_room).write(VfxMessage::Burst {
                    pos: arrival_pos, count: 18, speed: 260.0,
                    color: [0.35, 0.95, 1.0, 0.75], kind: ParticleKind::Dust,
                });
            } else if !through_a_passage {
                self.effects.vfx.for_room(arrival_room).write(VfxMessage::ResetEffects { from: arrival_pos, to: arrival_pos });
            }
            self.effects.sfx.write(SfxMessage::Reset { pos: arrival_pos });
            if let Some(room_set) = self.session.iter().next() {
                log_room_transition_landing(staged.target_room, room_set, arrival_pos,
                    member.size, member.gravity_dir, &staged.target_world);
            }
        }
        if let Some(dialogue) = self.dialogue.as_deref_mut() { dialogue.close(); }
        if let Some(conversation) = self.conversation.as_deref_mut() { conversation.close(); }
        match staged.members.first() {
            Some(member) => self.rooms.of(member.subject),
            None => self.rooms.sole(),
        }
    }

}

/// Ask the exact publication whether the crossing happened, and finish it if it
/// did.
///
/// ⛔⛤ **ONE DEFINITION OF A SUCCESSFUL TRANSITION, FOR BOTH HOSTS.** The eager
/// host calls this from the exclusive system chained after its commit, once this
/// frame's commands have flushed and the verdict exists; the confirmed host calls
/// it immediately after `state.apply(world)`, which flushes the same commands
/// inside its exclusive access. They differ in scheduling mechanics, never in
/// what publication success means.
///
/// ⭐ **AND IT CONSUMES THE RECEIPT EITHER WAY.** The publication was begun with
/// [`publication::PublicationRetention::UntilOwnerRetires`]; this is the owner, and a
/// refused receipt is as consumed as an admitted one.
pub fn finalize_room_transition(
    world: &mut bevy::prelude::World,
    staged: &StagedRoomTransition,
) -> bool {
    publication::settle_publication(
        world,
        staged.publication,
        |world| {
            let mut state: bevy::ecs::system::SystemState<RoomTransitionFinalize> =
                bevy::ecs::system::SystemState::new(world);
            {
                // The same argument the confirmed host makes about
                // `RoomTransitionApplication`: these are the parameters
                // `RoomTransitionPlugin` installs, and a composition that could not
                // fetch them could not have produced the publication being finalized.
                let mut finalize = state.get_mut(world).expect(
                    "RoomTransitionFinalize params are the ones RoomTransitionPlugin installs",
                );
                let arrived_in = finalize.apply_crossing(staged);
                // The transaction keeps the live room it arrived in, so its
                // cover waits on that room: two live rooms can be of one
                // definition.
                if let Some(active) = world
                    .get_resource_mut::<super::loading::RoomTransitionLoadState>()
                    .and_then(|state| state.into_inner().active.as_mut())
                {
                    active.arrived_in = arrived_in;
                }
            }
            state.apply(world);
        },
        |_| {},
    )
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
    /// Live body identity — how the transiting subject is NAMED. The request
    /// records a `LiveBodyId` at detection and this resolves it at commit; see
    /// [`Self::subject_entity`]. The live room is part of the name, so a
    /// duplicate of the subject in another live room is not a substitute.
    live_bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies<'w, 's>,
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
        subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId,
    ) -> Option<Entity> {
        self.live_bodies.entity_of(subject)
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
            ambition_platformer2d_session::lifecycle_commit::PendingLifecycleCommit,
        >,
        // ⛔ WHERE THE STAGED CROSSING IS HANDED OVER. Not a message: a message
        // would be readable by anything, and the finalizer must be the one
        // consumer. The checkpoint restore this commit may owe is recorded by the
        // FINALIZER now, because owing it is a statement that the crossing
        // happened.
        ResMut<PendingRoomTransitionFinalize>,
    ),
    // Whether the checkpoint operation this commit carries is a fresh run (a
    // New Game): then it is a whole-session restart.
    accepted_restores: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_session::checkpoint::AcceptedCheckpointRestore>,
    >,
) {
    let (
        active_session,
        content_epoch,
        mut transition_state,
        mut loads,
        mut load_events,
        mut next_mode,
        simulation_host,
        mut pending_lifecycle,
        mut pending_finalize,
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
    let intent_still_pending = pending_lifecycle
        .pending
        .as_ref()
        .is_some_and(|pending| active.describes(pending));
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
    // The room the crossing leaves: its subject's live room's definition
    // (OW1 cut 6b), not an index on the set (5e).
    let room_set_active = application
        .definition_left_by(active.intent.subject())
        .map(|definition| definition.index());
    let target_still_matches = room_set.rooms.get(active.target_room).is_some_and(|room| {
        room.id == active.target_room_id()
            && active
                .construction_plan
                .as_ref()
                .is_some_and(|plan| plan.matches_room_spec(room))
    });
    let current_session = active_session.as_deref().and_then(|scope| scope.current());
    if active.content_epoch != content_epoch.get()
        || active.session_scope != current_session
        || room_set_active != Some(active.source_room)
        || !target_still_matches
    {
        let detail = format!(
            "discarding stale room transition {}: expected epoch {}, session {:?}, source '{}' at index {}, and target '{}'; current epoch is {}, session is {:?}, live room definition is {:?}",
            active.sequence,
            active.content_epoch,
            active.session_scope,
            active.source_room_id,
            active.source_room,
            active.target_room_id(),
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
        || construction_plan.room_id() != active.target_room_id()
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
                active.target_room_id(),
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
    // ⛔ TWO `None`s, AND THEY MEAN OPPOSITE THINGS. `intent.subject()` being
    // `None` is a room rebuild with NOBODY IN IT — legitimate, and the whole
    // reason `ReconstituteRoom` exists. `subject_entity` answering `None` for a
    // subject the intent DID name is a body that is gone, which is terminal.
    // Collapsing them would turn every dead-body crossing into a silent bodyless
    // rebuild of the destination room.
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
    //
    // ⛔⛤ **STAGE, NOT APPLY — SPLIT 2026-09-14 ON REVIEW.** This called
    // `apply`, which performed the entire crossing, and then treated `Ok` as
    // *"the transition committed"*: it consumed the exact lifecycle intent,
    // recorded the checkpoint restore as owed, and advanced the load barrier and
    // the game mode. But the room is built as a HIDDEN candidate and its verifier
    // has not run yet — `apply` returned before the deferred transaction took its
    // verdict — so a room the transaction went on to REFUSE had already been
    // reported as a completed crossing. Staging builds the candidate and hands
    // the exact publication to the finalizer chained after this system.
    let restart = active
        .checkpoint_operation
        .zip(accepted_restores.as_deref())
        .and_then(|(key, accepted)| accepted.inputs_for_key(key))
        .is_some_and(|accepted| accepted.fresh);
    let staged = match application.stage(
        construction_plan,
        &intent,
        target_room,
        restart,
        active.checkpoint_operation,
    ) {
        Ok(staged) => staged,
        Err(error) => {
            match error {
                terminal @ RoomTransitionApplyError::StaleContinuity
                | terminal @ RoomTransitionApplyError::SubjectGone
                | terminal @ RoomTransitionApplyError::SubjectCannotTransit { .. } => {
                    // A missing body or a changed construction input ends this
                    // prepared crossing. A later attempt must prepare again.
                    pending_lifecycle.take();
                    cancel_eager_room_transition_transaction(
                        &mut transition_state,
                        &mut loads,
                        &mut load_events,
                        &mut next_mode,
                        &active,
                        if matches!(&terminal, RoomTransitionApplyError::StaleContinuity) {
                            "room_commit_continuity_changed"
                        } else { "room_commit_subject_unavailable" },
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
    };

    // ⛔ NOTHING ELSE HAPPENS HERE. Every statement that used to follow —
    // consuming the intent, owing the restore, advancing the barrier and the
    // mode — means *"the crossing happened"*, and whether it happened is a
    // question only this publication's verdict can answer.
    *pending_finalize = PendingRoomTransitionFinalize(Some(PendingCrossing {
        staged,
        active,
        #[cfg(not(target_arch = "wasm32"))]
        commit_started,
    }));
}

/// The crossing this frame staged, waiting on its publication's verdict.
///
/// ⚠ **NOT ROLLBACK STATE, and it must never become any**: it is written by the
/// eager commit and consumed by the exclusive finalizer chained immediately
/// after it, inside one run of the sim schedule. A value that survived a frame
/// here would be a crossing waiting to be finished against a world that had
/// moved on — the same rule `CommittedRoomTransitionRestore` states.
#[derive(bevy::prelude::Resource, Default)]
pub struct PendingRoomTransitionFinalize(Option<PendingCrossing>);

/// Run condition of [`finalize_committed_room_transition`]: a crossing waits
/// for its verdict.
pub fn a_crossing_waits(pending: bevy::prelude::Res<PendingRoomTransitionFinalize>) -> bool {
    pending.0.is_some()
}

/// Run condition of [`apply_committed_room_transition_restore`]: a crossing's
/// restore is owed.
pub fn a_restore_is_owed(committed: bevy::prelude::Res<CommittedRoomTransitionRestore>) -> bool {
    committed.0.is_some()
}

pub struct PendingCrossing {
    staged: StagedRoomTransition,
    /// The transaction this crossing belongs to, so the finalizer can advance it
    /// or cancel it by the SAME sequence the commit opened.
    active: super::loading::ActiveRoomTransitionLoad,
    #[cfg(not(target_arch = "wasm32"))]
    commit_started: std::time::Instant,
}

/// Finish — or refuse — the crossing the eager commit staged, from its exact
/// publication's verdict.
///
/// ⛔⛤ **EXCLUSIVE, AND CHAINED AFTER THE COMMIT, WHICH IS WHAT MAKES THE
/// VERDICT EXIST.** The candidate room is built by commands the commit system
/// queued; the transaction's verifier is the last of them. `.chain()` puts a sync
/// point between the two systems, so by the time this runs the publication either
/// carries a verdict or the transaction never closed.
pub fn finalize_committed_room_transition(world: &mut bevy::prelude::World) {
    // ⛔ THE EAGER HOST ONLY. The confirmed host reaches the identical
    // finalization from `commit_confirmed_lifecycle`, inline, outside the rewound
    // schedule.
    if world
        .get_resource::<crate::SimulationHost>()
        .is_some_and(|host| host.is_rollback())
    {
        return;
    }
    let Some(crossing) = world
        .get_resource_mut::<PendingRoomTransitionFinalize>()
        .and_then(|mut pending| pending.0.take())
    else {
        return;
    };

    // ⭐ THE SHARED OPERATION. Asks the exact publication, applies every effect
    // that means the crossing happened if it published, and consumes the receipt
    // either way.
    let published = finalize_room_transition(world, &crossing.staged);

    let mut state: bevy::ecs::system::SystemState<(
        ResMut<super::loading::RoomTransitionLoadState>,
        ResMut<ambition_load::LoadCoordinator>,
        MessageWriter<ambition_load::LoadEvent>,
        ResMut<bevy::prelude::NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
        ResMut<
            ambition_platformer2d_session::lifecycle_commit::PendingLifecycleCommit,
        >,
        ResMut<CommittedRoomTransitionRestore>,
        Option<Res<bevy::prelude::Time<bevy::prelude::Real>>>,
    )> = bevy::ecs::system::SystemState::new(world);
    {
        let (
            mut transition_state,
            mut loads,
            mut load_events,
            mut next_mode,
            mut pending_lifecycle,
            mut committed_restore,
            real_time,
        ) = state
            .get_mut(world)
            .expect("the eager commit's bookkeeping resources are the ones RoomTransitionPlugin installs");
        let active = &crossing.active;
        if !published {
            // ⛔⛤ **A REFUSED ROOM IS NOT A CROSSING.** The intent is spent — the
            // same terminal policy a body that cannot transit gets — because the
            // plan this transaction authorized is the thing the verifier rejected,
            // and retrying it re-refuses forever. The world the player is standing
            // in was never touched, so there is nothing to recover.
            pending_lifecycle.take();
            cancel_eager_room_transition_transaction(
                &mut transition_state,
                &mut loads,
                &mut load_events,
                &mut next_mode,
                active,
                "room_commit_refused",
            );
            bevy::log::warn!(
                target: "ambition_platformer2d::room_transition",
                "room transition {} was NOT committed: the room transaction refused its \
                 candidate, so the crossing is cancelled and the room the player is in \
                 is untouched",
                active.sequence,
            );
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            let commit_duration = Some(crossing.commit_started.elapsed());
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
            // ⭐ THE DOMAIN RESTORE IS OWED, AND THIS SYSTEM CANNOT PAY IT. Custody
            // materialization spawns and despawns, so it runs in an exclusive schedule
            // the confirmed host calls directly; the eager host records the debt here
            // and `apply_committed_checkpoint_restore_system` settles it a system later,
            // after this frame's structural work has flushed.
            //
            // ⛔ AND IT IS OWED ONLY BY A PUBLISHED CROSSING. A refused room must
            // not restore a checkpoint into the world it did not build.
            committed_restore.0 = active.checkpoint_operation;
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
                next_mode
                    .set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
            }
        }
    }
    state.apply(world);
}

/// The room transition this frame's eager commit landed, if it landed one.
///
/// ⛔ IT EXISTS BECAUSE THE EAGER COMMIT IS A SYSTEM AND THE RESTORE IS
/// EXCLUSIVE. The confirmed host commits from `&mut World` and calls the domain
/// application inline; the eager host cannot, so it names the intent it just
/// committed and an exclusive system ordered immediately after settles it. One
/// system writes this and one reads-and-clears it.
///
/// ⚠ NOT ROLLBACK STATE, and it must never become any: it is set and consumed
/// inside one run of the sim schedule. A value that survived a frame here would
/// be a restore waiting to be applied to a world that had moved on.
#[derive(bevy::prelude::Resource, Default, Clone, Debug, PartialEq)]
pub struct CommittedRoomTransitionRestore(
    pub  Option<
        ambition_platformer2d_session::checkpoint::CheckpointOperationKey,
    >,
);

/// Apply the committed restore the eager commit recorded.
///
/// ⛔⛔ EXCLUSIVE, AND THAT IS THE POINT rather than an inconvenience. The
/// custody reducer materializes occurrences the checkpoint remembers in a hand,
/// which is a spawn; the shared entry point flushes that structural work before
/// returning, so a caller cannot observe a half-applied restore.
pub fn apply_committed_room_transition_restore(world: &mut bevy::prelude::World) {
    // ⛔ THE EAGER HOST ONLY. On a rollback host this same operation is
    // performed by `commit_confirmed_lifecycle` outside the rewound schedule;
    // running it here as well would apply one restore twice.
    if world
        .get_resource::<crate::SimulationHost>()
        .is_some_and(|host| host.is_rollback())
    {
        return;
    }
    let Some(key) = world
        .get_resource_mut::<CommittedRoomTransitionRestore>()
        .and_then(|mut committed| committed.0.take())
    else {
        return;
    };
    ambition_platformer2d_session::checkpoint::apply_committed_checkpoint_restore(
        world, key,
    );
}

/// End any checkpoint operation whose room preparation failed.
///
/// ⛔⛔ **THE EAGER HOST'S HALF OF A SPLIT THE ROLLBACK GUARD FORCED.**
/// `abandon_failed_checkpoint_restore_system` sees the failure in `Update` and
/// may only touch host-side state there; the retraction it implies spends
/// `PendingLifecycleCommit`, which is rollback-registered. This is the eager
/// host's legal moment for that write. The rollback host makes the identical
/// call from `commit_confirmed_lifecycle`, on a confirmed frame.
///
/// ⚠ EXCLUSIVE ONLY TO SHARE ONE ENTRY POINT WITH THAT HOST. There is no
/// structural work here — nothing is spawned and nothing is despawned, because a
/// cancellation is precisely the case where no destructive application ran.
pub fn terminalize_abandoned_checkpoint_restore_system(world: &mut bevy::prelude::World) {
    // ⛔ THE EAGER HOST ONLY, exactly as the restore above: on a rollback host
    // this schedule rewinds, and a retraction performed inside it would be
    // undone by the next restore and re-performed on the next re-simulation.
    if world
        .get_resource::<crate::SimulationHost>()
        .is_some_and(|host| host.is_rollback())
    {
        return;
    }
    // `None`: this host has no rollback, so every frame it simulates is already
    // final and there is no speculative admission to wait out.
    let _ = ambition_platformer2d_session::checkpoint::terminalize_abandoned_checkpoint_restore(
        world, None,
    );
}

/// Emit one landing diagnostic for each committed room transition: the arrival
/// room's collision coverage and the support gap below the arriving body.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &world_rooms::RoomSet,
    pos: ae::Vec2,
    size: ae::Vec2,
    gravity_dir: ae::Vec2,
    world: &ae::World,
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
    let gap = ground_gap_below_feet(&body, gravity_dir, world);
    let gap_desc = match gap {
        Some(distance) => format!("{distance:.1}px"),
        None => "none within 256px".to_string(),
    };
    bevy::log::info!(
        target: "ambition_platformer2d::room_transition",
        "room transition: target={target_id} player_pos=({:.1},{:.1}) \
         world_blocks={} gap_below_feet={gap_desc} \
         body_overlaps_world={overlapping_world}",
        pos.x,
        pos.y,
        world.blocks.len(),
    );
}

#[cfg(test)]
mod another_player_tests {
    use super::*;
    use ambition_platformer2d_session::rooms::rooms_released_by_transfer;
    use ambition_characters::control::PlayerSlot;

    fn entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("a test entity index")
    }

    /// Unselected participants keep their source room. A selected participant's
    /// other controlled bodies do not create another claim for that participant.
    #[test]
    fn a_room_stays_live_for_another_player_and_not_for_the_subjects_own_bodies() {
        let first = world_rooms::LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let (alice, home, bob) = (entity(1), entity(2), entity(3));
        let alone = [(alice, PlayerSlot(0), Some(first)), (home, PlayerSlot(0), Some(first))];
        let slot0 = Some(PlayerSlot(0));
        assert!(
            rooms_released_by_transfer([first], [(Some(alice), slot0)], alone).contains(&first),
            "control: the subject's own slot kept the room it leaves live"
        );
        let with_bob = |room| {
            !rooms_released_by_transfer([first], [(Some(alice), slot0)], alone.into_iter().chain([(bob, PlayerSlot(1), Some(room))])).contains(&first)
        };
        assert!(with_bob(first), "another player's body in the room did not keep it live");
        assert!(!with_bob(second), "another player's body in ANOTHER room kept this one live");
        let bob_stays = [(bob, PlayerSlot(1), Some(first))];
        assert!(
            !rooms_released_by_transfer([first], [(Some(entity(4)), None)], bob_stays).contains(&first),
            "a crossing with no participant retired another body's room"
        );
        assert!(
            !rooms_released_by_transfer([first], [(Some(alice), slot0)], bob_stays).contains(&first),
            "the recorded participant did not decide once the subject lost its slot"
        );
    }
}
