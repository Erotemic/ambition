//! Room construction transaction boundary.
//!
//! One room load captures a baseline before construction and verifies the
//! completed room before publishing `RoomLoaded`; feature plans are participants,
//! not owners of that outer transaction. The same open/close bracket serves
//! deferred and exclusive-world execution. Verification can withhold publication
//! but cannot undo already-applied Bevy commands, so this is consistency checking,
//! not atomic rollback.

use std::collections::BTreeSet;

use bevy::ecs::resource::Resource;
use bevy::prelude::{Commands, World};

use ambition_platformer2d_shared_tangle::construction::{
    BaselineCaptureError, ProjectionViolation, PublicationEffects, RosterViolation,
    TransactionBaseline,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;

/// The baseline captured at the head of a construction transaction, waiting for
/// the verification pass at its tail.
///
/// A resource because the two ends are separate commands in one queue and nothing else can
/// carry a value between them.
#[derive(Resource)]
pub(crate) struct PendingConstructionBaseline(Result<OpenedTransaction, OpenRefused>);

/// What [`open`] establishes and [`verify_and_publish`] is owed: the world this
/// transaction opened against, and what it DECLARED it would do to that world.
///
/// ⛔⛤ **THE DECLARATION IS MADE AT THE HEAD OF THE TRANSACTION, NOT READ OFF
/// THE WORLD AT ITS TAIL.** A10's verifier asks *"would the authoritative world
/// be valid if this published?"*, and a declaration derived at the tail from
/// whatever construction happened to leave standing cannot answer that — it
/// would agree with anything. Deriving it here, from the plan and from the
/// baseline captured before a single root was built, is what makes the
/// projection falsifiable.
pub(crate) struct OpenedTransaction {
    baseline: TransactionBaseline,
    effects: PublicationEffects,
}

/// ⛔⛤ **A10'S BRACKET FLAG, ON SINCE 2026-09-14.** `true` builds every root in
/// every lane as an `InactiveCandidate`, so a room that fails verification is
/// DROPPED rather than left standing half-built, and the projected
/// post-publication roster below has a candidate population to project.
///
/// ⚠ It is a constant rather than a literal because it is read THREE times now —
/// the bracket's opening refusal, the spawn's visibility, and whether [`close`]
/// asks the projected question at all — and two spellings of one decision is how
/// the ends come to disagree about whether a room is a candidate.
///
/// ⛔ **IT IS STILL A CONSTANT, AND THAT IS DELIBERATE.** Nothing chooses per
/// room or per composition: a candidate world that some rooms opt out of is two
/// lifecycles, and the refusal path is only trustworthy if every room takes it.
/// The parameter exists so the harness in `construction/tests.rs` can commit the
/// same plan LIVE and prove what the bracket changes — see
/// `commit_bracketed`.
pub(crate) const ROOM_CANDIDATE_BRACKET: bool = true;

/// Why a room transaction refused before it built anything.
///
/// ⛔⛤ **THE SECOND VARIANT COLLAPSES A RULE A DOC COMMENT USED TO ASK CALLERS TO
/// OBEY.** `ConstructionPlan::commit_inactive` REFUSES when
/// `register_inactive_candidate_filter` was never called, because an
/// unregistered `InactiveCandidate` is an ordinary inert component — every
/// "candidate" would be stamped and every one of them would be LIVE, which is
/// the dangerous direction and is silent. The room road cannot call
/// `commit_inactive`: it builds through deferred `Commands` and `commit_hidden`
/// has no `&mut World` to ask with, so its doc said *"the caller owes that
/// check"*.
///
/// ⚠ **THE CALLER DID NOT OWE IT — NOBODY DID.** MEASURED at HEAD 2026-09-13:
/// `inactive_candidate_filter_installed` has exactly one caller, inside
/// `commit_inactive` itself, and `register_inactive_candidate_filter` has exactly
/// one production caller, in `ambition_platformer2d_runtime`'s plugin — a
/// different crate from the room transaction the doc named. So the check lived
/// only on the road with no production traffic.
///
/// ⇒ The room transaction opens with `&mut World` in hand and asks there. A
/// composition that would build invisible candidates into a world that cannot
/// hide them refuses the room instead, on the road production actually uses.
#[derive(Debug)]
pub(crate) enum OpenRefused {
    /// The world already held one identity on two entities.
    Baseline(BaselineCaptureError),
    /// The bracket is on and `InactiveCandidate` is not a disabling component in
    /// this world, so nothing it stamps would actually be hidden.
    CandidateFilterNotInstalled,
}

impl std::fmt::Display for OpenRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Baseline(error) => write!(f, "{error}"),
            Self::CandidateFilterNotInstalled => write!(
                f,
                "this room builds inactive candidates, but `InactiveCandidate` is not \
                 registered as a disabling component in this world, so every candidate \
                 would be LIVE while it was being validated. Call \
                 `register_inactive_candidate_filter` at composition build."
            ),
        }
    }
}

/// The live world this room transaction WOULD become, held off to the side
/// until its verdict.
///
/// ⛔⛤ **THE HALF OF A10 THAT IS NOT ENTITIES.** `RoomConstructionPlan::
/// commit_deferred` is four statements and only the last one builds the
/// candidate; the other three — `rooms.set_active`, `geometry.0 = ..`,
/// `*moving_platforms = ..` — write the LIVE world, and `replace_live_world`
/// retired the OUTGOING room ahead of all of it. So a refusal used to leave the
/// session pointed at a room index with no room in it: the strictly worse
/// outcome A10 exists to delete, arrived at by the candidate bracket working.
///
/// ⇒ Every one of those four is now staged here and applied by the ONE
/// publication authority in [`verify_and_publish`], after the candidate has been
/// admitted. A refusal drops this resource and the live world never learns the
/// room was attempted.
///
/// ⚠ **AND STAGING IT CHANGES WHAT THE BASELINE SEES, WHICH IS THE POINT.** The
/// outgoing room is still standing when the transaction opens, so the baseline
/// holds it, the projection can be asked what publication would do to it, and
/// `LiveLostWithoutDeclaration` can notice construction destroying a piece of it.
/// Under the old order the outgoing room was already gone before the baseline was
/// taken and none of those questions were askable.
/// Where the transiting body lands when — and only when — the room it is
/// arriving into publishes.
///
/// ⛔⛤ **THE BODY IS PART OF THE PUBLICATION, NOT PART OF THE ATTEMPT.** Placing
/// it before the verdict is the same defect as writing the geometry before the
/// verdict, and it is the half a 2026-09-14 app-level arm caught still standing:
/// a refused transition left the player at the REFUSED room's arrival
/// coordinates, inside the geometry of the room they never left. The world
/// survived and the body was somewhere it had no reason to be.
///
/// ⚠ **DATA, NOT A CLOSURE**, because the caller lives a crate above this one.
/// The transition computes the arrival against its own plan — it is the only
/// thing that knows the authored door and the body's size — and states the
/// result; `apply_world_replacement` performs it through the same
/// `arrive_body_in_room` authority the caller used to call directly.
pub struct StagedArrival {
    pub subject: bevy::ecs::entity::Entity,
    pub arrival: ambition_platformer2d_core::Vec2,
    pub air_jumps: u8,
    pub momentum: ambition_platformer2d_core::movement::ArrivalMomentum,
}

#[derive(Resource)]
pub(crate) struct PendingWorldReplacement {
    /// The outgoing room's bodies, and whether each is a physics entity — the
    /// flag decides which retirement they take.
    outgoing: Vec<(bevy::ecs::entity::Entity, bool)>,
    /// A room SET replacement (a hot reload re-reads content); `None` when the
    /// caller walks within the set it already has.
    next_rooms: Option<ambition_platformer2d_world::rooms::RoomSet>,
    /// Which room in that set becomes active.
    target_index: usize,
    /// The geometry the session collides against afterwards.
    geometry: ambition_platformer2d_core::World,
    /// The moving-platform bodies' starting state.
    moving_platforms: Vec<ambition_platformer2d_world::platforms::MovingPlatformState>,
    /// Where the transiting body lands, if one is crossing.
    arrival: Option<StagedArrival>,
}

impl PendingWorldReplacement {
    pub(crate) fn new(
        outgoing: Vec<(bevy::ecs::entity::Entity, bool)>,
        next_rooms: Option<ambition_platformer2d_world::rooms::RoomSet>,
        target_index: usize,
        geometry: ambition_platformer2d_core::World,
        moving_platforms: Vec<ambition_platformer2d_world::platforms::MovingPlatformState>,
    ) -> Self {
        Self {
            outgoing,
            next_rooms,
            target_index,
            geometry,
            moving_platforms,
            arrival: None,
        }
    }

    /// State that a body is crossing into this room, and where it lands.
    pub(crate) fn arriving(mut self, arrival: StagedArrival) -> Self {
        self.arrival = Some(arrival);
        self
    }

    /// Every outgoing body, so the transaction can DECLARE what publication is
    /// about to do to the identities standing on them.
    fn outgoing_entities(&self) -> BTreeSet<bevy::ecs::entity::Entity> {
        self.outgoing.iter().map(|(entity, _)| *entity).collect()
    }
}

/// Why the staged world may not be published.
///
/// ⛔⛤ **THE PROJECTED VERIFIER VALIDATES A ROSTER, AND THE ROOM IS NOT ONLY A
/// ROSTER.** `verify_projected_roster` asks what identities the authoritative
/// world would hold; the staged replacement also names WHICH ROOM the session
/// becomes and WHAT GEOMETRY it collides against, and nothing was asking whether
/// those two agree with each other or with the set they index into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StagedWorldViolation {
    /// The staged active-room index is not a valid index into the room set that
    /// would be live.
    ///
    /// ⛔⛤ **AND THE CONSEQUENCE IS SILENT, WHICH IS WHY THIS EXISTS.**
    /// `RoomSet::set_active` is `self.active = index.min(len - 1)` — an
    /// out-of-range index does not panic, it CLAMPS, and the session wakes up in
    /// the last room of the set with the geometry of the one it was told to
    /// build. Measured by accident 2026-09-14: a poison that staged
    /// `usize::MAX` moved the active room rather than failing.
    TargetRoomOutOfRange { target: usize, rooms: usize },
    /// The staged geometry is not the geometry of the staged room.
    ///
    /// The two travel together from one plan, so this is a caller pairing a plan
    /// with an index into a different set — the hot-reload road replaces the SET
    /// as well as the room, and is the one place they can disagree.
    GeometryIsNotTheTargetRoom {
        target: String,
        geometry: String,
    },
    /// A world was staged and there is no live session root to publish it into.
    ///
    /// ⛔⛤ **FAIL-CLOSED, BECAUSE THE OTHER ANSWER IS SILENT AND WRONG.**
    /// `apply_world_replacement` writes through `session_world_component_mut`,
    /// which returns `None` when no root is live — so without this the room would
    /// PUBLISH, report `room-loaded`, and leave the geometry, the active room and
    /// the platform state exactly as they were. A caller that staged a whole
    /// world and got nothing would have no way to tell that from success.
    ///
    /// ⚠ It is reachable by ORDERING, not only by misuse: session activation
    /// queues its room build BEFORE it spawns the session root, which is why
    /// activation commits through `spawn_contents` and stages no world at all.
    NoSessionRootToPublishInto,
}

impl std::fmt::Display for StagedWorldViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetRoomOutOfRange { target, rooms } => write!(
                f,
                "this room would become active room {target} of a set holding \
                 {rooms}; `set_active` CLAMPS rather than failing, so publishing \
                 would silently seat the session in a different room"
            ),
            Self::GeometryIsNotTheTargetRoom { target, geometry } => write!(
                f,
                "this room would seat the session in `{target}` while publishing \
                 the geometry of `{geometry}`"
            ),
            Self::NoSessionRootToPublishInto => write!(
                f,
                "this room staged a whole world and there is no live session root \
                 to publish it into, so publishing would change nothing and say \
                 it had succeeded"
            ),
        }
    }
}

impl std::error::Error for StagedWorldViolation {}

/// Would the staged world be coherent if it published?
///
/// ⚠ **THE ARRIVAL IS DELIBERATELY NOT CHECKED HERE, AND THAT IS A STATEMENT
/// ABOUT THE CHECK RATHER THAN ABOUT THE RISK.** `validated_spawn` already clamps
/// the arrival into the plan's own world, and the plan's world is what is staged
/// — so an in-bounds assertion could not fail on any road that exists. A check
/// that cannot fail reads as coverage. If an arrival ever comes from somewhere
/// other than the staged plan, it becomes checkable and belongs here.
fn verify_staged_world(
    world: &World,
    pending: &PendingWorldReplacement,
) -> Result<(), Vec<StagedWorldViolation>> {
    use ambition_platformer2d_world::rooms::RoomSet;

    let mut violations = Vec::new();
    // The set that would be live: the staged replacement's, or the one already
    // on the session root when this transaction replaces only the active room.
    let staged_rooms = pending.next_rooms.as_ref();
    let live_rooms = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        RoomSet,
    >(world);
    let rooms = match (staged_rooms, live_rooms) {
        (Some(next), _) => Some(&next.rooms),
        (None, Some(live)) => Some(&live.rooms),
        (None, None) => None,
    };
    // ⛔ ASKED SEPARATELY FROM THE SET, because a replacement that BRINGS its own
    // room set still needs a root to put it on.
    if ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(world).is_none() {
        violations.push(StagedWorldViolation::NoSessionRootToPublishInto);
    }
    if let Some(rooms) = rooms {
        match rooms.get(pending.target_index) {
            None => violations.push(StagedWorldViolation::TargetRoomOutOfRange {
                target: pending.target_index,
                rooms: rooms.len(),
            }),
            Some(spec) if spec.world.name != pending.geometry.name => {
                violations.push(StagedWorldViolation::GeometryIsNotTheTargetRoom {
                    target: spec.world.name.clone(),
                    geometry: pending.geometry.name.clone(),
                })
            }
            Some(_) => {}
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Make the staged world live: retire the outgoing room, then publish the
/// world-defining state.
///
/// ⛔ CALLED ONLY FROM THE ADMISSION ARM, and only after `publish_candidate`.
/// The order inside is the one the three former call sites each kept privately —
/// retire, then commit — and it is safe to read as atomic because nothing is
/// SCHEDULED inside an exclusive-world call. It is not atomic to hooks or
/// observers; see `publish_candidate` for the measurement behind that wording.
fn apply_world_replacement(world: &mut World, pending: PendingWorldReplacement) {
    {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, world);
        for (entity, is_physics) in &pending.outgoing {
            if *is_physics {
                crate::world::physics::retire_physics_entity(&mut commands, *entity);
            } else {
                // `try_despawn`: the outgoing roster was collected before this
                // frame's commands flushed, so an entity in it can already have
                // been despawned by something else in the same frame — an actor
                // death, a session teardown racing a transition. Retiring a body
                // that is already gone is the outcome this wants.
                commands.entity(*entity).try_despawn();
            }
        }
        queue.apply(world);
    }
    if let Some(next) = pending.next_rooms {
        if let Some(mut rooms) = ambition_platformer2d_shared_tangle::lifecycle::
            session_world_component_mut::<ambition_platformer2d_world::rooms::RoomSet>(world)
        {
            *rooms = next;
        }
    }
    if let Some(mut rooms) = ambition_platformer2d_shared_tangle::lifecycle::
        session_world_component_mut::<ambition_platformer2d_world::rooms::RoomSet>(world)
    {
        rooms.set_active(pending.target_index);
    }
    if let Some(mut geometry) = ambition_platformer2d_shared_tangle::lifecycle::
        session_world_component_mut::<ambition_platformer2d_core::RoomGeometry>(world)
    {
        geometry.0 = pending.geometry;
    }
    if let Some(mut platforms) = world
        .get_resource_mut::<ambition_platformer2d_world::collision::MovingPlatformSet>()
    {
        platforms.0 = pending.moving_platforms;
    }
    // ⛔ LAST, AND AFTER THE GEOMETRY. The arrival was validated against the
    // plan's world by the caller, and the body is placed into a world that is
    // already the one it was validated against.
    if let Some(arrival) = pending.arrival {
        apply_staged_arrival(world, arrival);
    }
}

/// Place the transiting body. Separate from its caller only so the poison that
/// proves the staging matters can run THIS and nothing else — a poison that also
/// moved the room set would redden the same arm for a different reason.
fn apply_staged_arrival(world: &mut World, arrival: StagedArrival) {
    let mut bodies = world.query::<(
        ambition_platformer2d_core::BodyClusterQueryData,
        &mut ambition_platformer2d_core::MotionModel,
    )>();
    if let Ok((mut item, mut model)) = bodies.get_mut(world, arrival.subject) {
        let mut clusters = item.as_clusters_mut();
        ambition_platformer2d_core::movement::arrive_body_in_room(
            &mut model,
            &mut clusters,
            arrival.arrival,
            arrival.air_jumps,
            arrival.momentum,
        );
    }
}

/// What the last construction transaction's verification concluded.
///
/// Developer evidence and a test seam, kept for the same reason
/// [`LastRoomConstructionCommit`](super::LastRoomConstructionCommit) is: a room
/// that failed verification is a fact worth being able to query rather than only
/// to read in a log.
#[derive(Resource, Clone, Debug, Default)]
pub struct LastConstructionVerification {
    pub room_id: String,
    /// Every construction invariant the transaction found violated.
    pub violations: Vec<RosterViolation>,
    /// Every reason the world PUBLICATION WOULD PRODUCE was invalid.
    ///
    /// ⛔ A separate field rather than a `RosterViolation` variant because the
    /// two are answers to different questions about different worlds, and a
    /// reader that cannot tell *"the room I built is wrong"* from *"the room I
    /// built is fine and publishing it would break the world"* has lost the
    /// distinction A10 is made of. Empty whenever the candidate bracket is off,
    /// because then there is no candidate world to project.
    pub projection_violations: Vec<ProjectionViolation>,
    /// Every reason the staged non-entity world — which room becomes active, and
    /// what geometry it collides against — would be incoherent. See
    /// [`StagedWorldViolation`].
    pub staged_violations: Vec<StagedWorldViolation>,
    /// Whether `RoomLoaded` was written.
    pub published: bool,
}


/// Did the room transaction for `room_id` publish?
///
/// ⛔⛤ **FOR A CALLER WHOSE OWN WRITES ONLY MAKE SENSE IF THE ROOM ARRIVED.** The
/// dev hot reload is the first: it advances the session's content generation —
/// the live binding, the installed LDtk index, the prepared content and its
/// identity — and made those writes unconditionally. A REFUSED reload therefore
/// left the session claiming a generation whose room does not exist: the old
/// room's contents running under the new epoch's name, and every later room
/// transaction refused as stale against a binding nothing built.
///
/// ⚠ **IT ASKS BY ROOM ID, AND THAT IS THE WHOLE FUNCTION.**
/// [`LastConstructionVerification`] is last-writer-wins, so *"is there a verdict
/// and does it say published"* would accept a DIFFERENT room's success — the
/// session handoff road commits two rooms in quick succession and is exactly
/// where that would bite. A caller owns one room's transaction and names it.
///
/// ⚠ **ABSENT IS `false`, NOT A WAIVER.** No verdict means no room transaction
/// ran, and a caller whose writes are conditional on one has nothing to be
/// conditional on.
pub fn room_publication_succeeded(world: &World, room_id: &str) -> bool {
    world
        .get_resource::<LastConstructionVerification>()
        .is_some_and(|verdict| verdict.published && verdict.room_id == room_id)
}

/// Open the transaction: queue the baseline capture.
///
/// Queued before anything the transaction constructs, so what it sees at flush
/// is what was live when the transaction opened.
/// Open the transaction: queue the baseline capture, DECLARING what this room
/// intends to rebuild.
///
/// ⛔⛤ **`TransactionBaseline::retiring` AND `::reconstructing` HAD ZERO
/// PRODUCTION CALLERS, MEASURED AT HEAD 2026-09-13.** Both were reached only from
/// tests. So every shipped room opened a baseline declaring NOTHING, and
/// `verify_committed_roster` then judged the finished room against a claim nobody
/// had made — for the life of the road.
///
/// ⇒ **THE CONSEQUENCE IS NOT THAT NOTHING WAS CHECKED. IT IS THAT THE CHECK
/// COULD NOT NAME WHAT IT FOUND.** A room rebuild legitimately replaces the
/// bodies of the identities it authors; with no declaration that reads as
/// `PlannedOverBaseline` — *"you rebuilt something you never said you would"* —
/// which is true of every room reset in the game and therefore says nothing. With
/// the declaration, the same situation reports `ReconstructedOldSurvived`, which
/// is the precise and ACTIONABLE statement: *"you said you would replace this
/// identity, and the old body is still here."*
///
/// ⭐ **THAT IS EXACTLY THE `Q124` DEFECT, NEWLY LEGIBLE.** A death-reset carries
/// a placement held in the player's custody across the "door" (`RoomResident`
/// excludes `InCustodyOf`) and then re-mints it, so two entities wear one
/// authored `SimId`. What the ruling in `Q124` decides is what SHOULD happen to
/// the held one; what this decides is that the verifier can say which of the two
/// is unexpected.
///
/// ⚠ **RECONSTRUCTING, NOT RETIRING.** A room plan says what the room WILL
/// contain; it never declares an identity gone. `retiring` therefore still has no
/// production caller, and that is a statement about room construction rather than
/// an omission here — a road that genuinely retires an identity (an encounter
/// clearing its rewards, say) owes its own declaration.
///
/// ⛔⛤ **IT TAKES THE PLAN AND DERIVES THE DECLARATION ITSELF, RATHER THAN
/// ACCEPTING ONE.** The first version took a `BTreeSet<SimId>` from the caller,
/// and the test harness in `construction/tests.rs` promptly grew its own copy of
/// `plan.planned_sim_ids()` beside `spawn_contents`'s — TWO spellings of one
/// fact. MEASURED: poisoning the production declaration to declare NOTHING left
/// `a_room_that_fails_verification_is_not_published` green, because the harness
/// was declaring for itself and the arm never reached the production road at all.
///
/// ⇒ **A CALLER CANNOT NOW DECLARE SOMETHING OTHER THAN WHAT IT IS ABOUT TO
/// BUILD**, and the same poison reddens the arm. `close` already took the plan;
/// the two ends of the bracket are symmetric again.
pub(crate) fn open(
    commands: &mut Commands,
    plan: &crate::features::RoomFeatureConstructionPlan,
    candidate_bracket: bool,
) {
    let planned = plan.planned_sim_ids();
    commands.queue(move |world: &mut World| {
        // ⛔ ASKED BEFORE THE BASELINE, because a world that cannot hide a
        // candidate must refuse the room rather than build one it will then
        // validate in plain sight. See `OpenRefused`.
        let captured = if candidate_bracket
            && !ambition_platformer2d_shared_tangle::construction::inactive_candidate_filter_installed(
                world,
            ) {
            Err(OpenRefused::CandidateFilterNotInstalled)
        } else {
            TransactionBaseline::capture(world)
                .map(|baseline| {
                    // ⛔ THE PLAN'S OWN PREDICTED ROSTER, not a hand-kept list
                    // beside it: `predicted_authoritative_ids` is the same set
                    // the receipt is `debug_assert`ed against, so the
                    // declaration and the execution cannot drift apart without
                    // that assertion firing first.
                    //
                    // ⛔⛤ **AND THE BASELINE SPLITS IT IN TWO, PER IDENTITY.**
                    // A planned identity NOBODY currently holds is a plain
                    // reconstruction — "the old body should already be gone",
                    // which for these is vacuously so and still catches a stray
                    // wearing the name. A planned identity SOMETHING STILL
                    // HOLDS is a SUPERSESSION: the live body is meant to keep
                    // standing until this room publishes.
                    //
                    // ⭐ That is `Q124`'s ruling implemented rather than
                    // averaged: the checkpoint baseline decides PER ITEM, and
                    // two placements in one death reconstruction are free to
                    // land in different halves. Nothing here asks a blanket
                    // question about custody.
                    //
                    // ⚠ **AND THE SPLIT IS ONLY AVAILABLE UNDER THE BRACKET.**
                    // A supersession states *"a HIDDEN candidate stands beside
                    // the live body until publication"*. Without the bracket
                    // this transaction spawns its roots VISIBLE, so there is no
                    // beside — the two bodies are both authoritative the
                    // instant the second one lands, which is the ordinary
                    // in-place rebuild `reconstructing` already describes.
                    // Declaring supersession there would be a claim about a
                    // world this road does not build, and it would mean the
                    // live road stopped reporting the coexistence at all.
                    let (superseding, reconstructing): (Vec<_>, Vec<_>) = planned
                        .iter()
                        .cloned()
                        .partition(|sim_id| candidate_bracket && baseline.contains(sim_id));
                    let mut effects = superseding.iter().fold(
                        PublicationEffects::new(),
                        |effects, sim_id| effects.superseding(sim_id.clone(), sim_id.clone()),
                    );
                    // ⛔⛤ **AND THE OUTGOING ROOM IS DECLARED TOO — A
                    // RETIREMENT IS NOT A SUPERSESSION.** The staged replacement
                    // is going to sweep every body of the room being left, and
                    // most of them are identities this plan does NOT re-author:
                    // an enemy that wandered in, a thrown item, a body the
                    // previous room owned. Omission means RETAINED in the
                    // projection, so leaving them undeclared would have the
                    // verifier certify a post-publication world still holding a
                    // room that publication is about to destroy.
                    //
                    // ⚠ Declared in the EFFECTS only, never in the baseline.
                    // `TransactionBaseline::retiring` means *"already gone by the
                    // time you verify"*, and under A10 they are deliberately
                    // still standing — that is the whole point of staging the
                    // sweep behind the verdict.
                    if let Some(outgoing) = world
                        .get_resource::<PendingWorldReplacement>()
                        .map(PendingWorldReplacement::outgoing_entities)
                    {
                        let planned: BTreeSet<_> = planned.iter().collect();
                        for (sim_id, entry) in baseline.entries() {
                            if outgoing.contains(&entry.entity) && !planned.contains(sim_id) {
                                effects = effects.retiring(sim_id.clone());
                            }
                        }
                    }
                    OpenedTransaction {
                        baseline: baseline
                            .reconstructing(reconstructing)
                            .superseding(superseding),
                        effects,
                    }
                })
                .map_err(OpenRefused::Baseline)
        };
        world.insert_resource(PendingConstructionBaseline(captured));
    });
}

/// Close the transaction: queue the verification that publishes the room, or
/// refuses to.
///
/// Queued last, so every command the transaction issued has applied by the time
/// it runs — which is the only moment at which "what did this transaction
/// actually build" is a question the world can answer.
pub(crate) fn close(
    commands: &mut Commands,
    plan: &crate::features::RoomFeatureConstructionPlan,
    receipt: &crate::features::RoomFeatureConstructionReceipt,
    room_id: String,
    session: SessionSpawnScope,
    candidate_bracket: bool,
) {
    let plan = plan.clone();
    let receipt = receipt.clone();
    commands.queue(move |world: &mut World| {
        verify_and_publish(world, &plan, &receipt, room_id, session, candidate_bracket);
    });
}

/// The content generation the SESSION is live under — the commit boundary's
/// comparison value for [`RosterViolation::ContentBindingMismatch`].
///
/// Written by the content activation authorities: session setup inserts it from
/// the construction context it was handed, and a hot-reload commit that
/// allocates a new epoch updates it. Room transitions and resets do not change
/// content, so they never write it. Absent (headless fixtures, unit tests
/// without a session) the boundary check is vacuous — an honest gap, not a
/// waiver: a fixture with no content authority has nothing to be stale
/// against.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveContentBinding(pub ambition_platformer2d_shared_tangle::construction::ContentBinding);

impl ActiveContentBinding {
    /// The binding for one exact prepared-content generation — the app-side
    /// spelling for "the session now runs under this epoch".
    pub fn content(epoch: ambition_platformer2d_core::ContentEpoch) -> Self {
        Self(ambition_platformer2d_shared_tangle::construction::ContentBinding::Content(epoch))
    }
}

fn verify_and_publish(
    world: &mut World,
    plan: &crate::features::RoomFeatureConstructionPlan,
    receipt: &crate::features::RoomFeatureConstructionReceipt,
    room_id: String,
    session: SessionSpawnScope,
    candidate_bracket: bool,
) {
    let refuse = |world: &mut World, room_id: String| {
        // ⛔ THE STAGED WORLD GOES WITH THE CANDIDATE. A refusal that left it
        // behind would hand the NEXT room transaction a replacement prepared for
        // a room that never published.
        world.remove_resource::<PendingWorldReplacement>();
        world.insert_resource(LastConstructionVerification {
            room_id,
            violations: Vec::new(),
            projection_violations: Vec::new(),
            staged_violations: Vec::new(),
            published: false,
        });
    };

    let OpenedTransaction { baseline, effects } = match world
        .remove_resource::<PendingConstructionBaseline>()
    {
        Some(PendingConstructionBaseline(Ok(opened))) => opened,
        Some(PendingConstructionBaseline(Err(error))) => {
            // Publishing a room on top of that would bury the earlier fault.
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "room `{room_id}` cannot be verified: its opening baseline was invalid: {error}"
            );
            refuse(world, room_id);
            return;
        }
        None => {
            // Nothing queued a capture, so there is no transaction to verify.
            // Refusing here rather than verifying against an empty baseline: an
            // empty baseline would call every persistent entity unplanned.
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "room `{room_id}` reached verification without an opening baseline"
            );
            refuse(world, room_id);
            return;
        }
    };

    let mut violations = plan.verify_committed_construction(receipt, &baseline, world, session);

    // The commit-boundary staleness check: every lane was prepared against the
    // same content generation, and the room may publish only into that exact
    // generation. The room transaction owns this comparison because it owns
    // publication; individual construction domains do not.
    // ⛔⛤ **AN ABSENT BINDING USED TO MEAN "NO COMPARISON", WHICH IS FAIL-OPEN —
    // AND A 2026-09-13 REVIEW NAMED WHY THAT IS NOT A GAP BUT A HOLE.** This
    // resource's own doc said the vacuous branch was *"an honest gap, not a
    // waiver: a fixture with no content authority has nothing to be stale
    // against."* True of a fixture. The `Option` could not tell that fixture from
    // **a live shell session that lost its canonical content authority**, and in
    // the second case a room publishes into a generation nobody can name.
    //
    // ⭐ **THE DISCRIMINATOR ALREADY EXISTED AND IS NOT A NEW CONCEPT.**
    // `SessionGatedSimulation` is installed by `ambition_game_shell`'s session
    // plugin and, in its own words, *"never inserted by direct-entry apps or
    // headless harnesses"* — two other sites in `lifecycle/session.rs` already
    // branch on exactly it. ⇒ Composition mode is asked, rather than inferred
    // from whether a resource happens to be there.
    match world.get_resource::<ActiveContentBinding>().copied() {
        Some(live) => {
            let planned = plan.construction_binding();
            if planned != live.0 {
                violations.push(
                    ambition_platformer2d_shared_tangle::construction::RosterViolation::ContentBindingMismatch {
                        planned,
                        live: live.0,
                    },
                );
            }
        }
        None if world.contains_resource::<
            ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation,
        >() => {
            // A shell-routed composition owes this resource. Publishing here
            // would admit a room whose staleness nothing checked.
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "room `{room_id}` cannot be verified: this composition routes \
                 gameplay through a shell session, so `ActiveContentBinding` is a \
                 canonical authority it must hold, and it is absent"
            );
            refuse(world, room_id);
            return;
        }
        // A direct-entry fixture states no binding and means it.
        None => {}
    }
    violations.sort_by_key(|violation| format!("{violation:?}"));
    violations.dedup();

    for violation in &violations {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` failed construction verification: {violation}"
        );
    }

    // ⛔⛤ **A10: THE ROOM WAS BUILT AS A CANDIDATE, SO THIS IS WHERE IT BECOMES
    // REAL — OR CEASES TO EXIST.** `spawn_contents` commits every root, in every
    // lane, stamped `InactiveCandidate` at mint. Until this line nothing in the
    // room is visible to an ordinary query, which is why the verification above
    // is allowed to say no.
    //
    // ⛔ THE OLD REFUSAL MESSAGE SAID WHAT THIS DELETES: *"The world has already
    // been mutated and cannot be rolled back."* It can now: a refused room is
    // DROPPED.
    //
    // ⛔⛤ **THIS IS NOT THE LAST-GOOD-WORLD GUARANTEE, AND THE DIFFERENCE IS NOT
    // "ONE ORDERING" — THAT SENTENCE STOOD HERE AND WAS FALSE.** It said moving
    // `retire_outgoing` after this point was the remaining step. Two reviews
    // (2026-09-13) measured otherwise, and this comment sits exactly where an
    // implementation agent will work, so it is more dangerous than stale prose
    // elsewhere.
    //
    // ⇒ **HIDDEN ENTITIES ARE NOT A CANDIDATE WORLD.** What is still published
    // before this verification runs:
    //   * `commit_deferred` writes `RoomSet`, `RoomGeometry` and the
    //     moving-platform state — none of them entities, none of them bracketed;
    //   * `replace_live_world` retires the OUTGOING room first, so a refusal
    //     leaves the session with no room rather than with its previous one;
    //   * the hot-reload caller then queues `ActiveContentBinding`, transits the
    //     player, resets movement/combat and replaces `ldtk_index` /
    //     `prepared_identity` / `prepared_content` — **none of which receives the
    //     verdict computed here**;
    //   * and the verifier has no way to validate N+1 *while intentionally
    //     retaining N*: it calls the coexistence an accidental duplicate.
    //
    // ⇒ What the bracket buys TODAY is narrower and still worth having: a refused
    // room leaves no debris — no half-built scene standing beside a world that
    // never accepted it. The rest is a candidate WORLD/SESSION transaction; see
    // `docs/planning/queue.md`'s A10 rows, which carry the packet order.
    // ⛔ EVERY LANE'S TRANSACTION, not just the actor lane's — see
    // `construction_transactions`, and the measurement that corrected me.
    let transactions = plan.construction_transactions(session);

    // ═══════════════════════════════════════════════════════════════════════
    // A10: WOULD THE AUTHORITATIVE WORLD BE VALID IF THIS ROOM PUBLISHED?
    // ═══════════════════════════════════════════════════════════════════════
    //
    // ⛔⛤ **EVERYTHING ABOVE JUDGES THE WORLD AS IT IS. THIS JUDGES THE WORLD
    // PUBLICATION WOULD PRODUCE**, and the difference is the whole of A10: the
    // candidates are hidden, the live world is deliberately still the old one,
    // and a refusal here costs nothing because nothing has been retired to make
    // room for them.
    //
    // ⚠ **ONLY UNDER THE BRACKET, AND THAT IS NOT A CONVENIENCE.** Without it
    // every root is spawned VISIBLE, so there is no candidate population to
    // project: each declared supersession would report
    // `SupersedingCandidateMissing` about a root that is standing right there.
    // A projection of a world with no candidates in it is not a weaker check,
    // it is a different and false one.
    // ⛔ THE OTHER HALF OF THE PROJECTION: the staged world, not the roster.
    // Asked BEFORE the verdict is taken, so an incoherent staged world refuses
    // the room exactly as an incoherent roster does.
    let staged_violations = match world.get_resource::<PendingWorldReplacement>() {
        Some(pending) => verify_staged_world(world, pending).err().unwrap_or_default(),
        None => Vec::new(),
    };
    for violation in &staged_violations {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` staged an incoherent world: {violation}"
        );
    }

    let mut effects = effects;
    let projection_violations = if candidate_bracket {
        use ambition_platformer2d_shared_tangle::construction::{
            project_post_publication_roster, verify_projected_roster, AuthoritativeScope,
        };
        // ⛔ THE PUBLICATION OWNS EVERY LANE'S TRANSACTION. A room commits the
        // actor lane and one transaction per capability lane under ONE verdict;
        // a publication that claimed only the lane it gathered with would call
        // every other lane's candidate stolen.
        effects = transactions
            .iter()
            .cloned()
            .fold(effects, PublicationEffects::owned_by);
        // ⚠ The gather's transaction argument selects nothing here — the
        // projection reads VISIBILITY and `PresentationOnly`, and ownership is
        // asked of `effects.owners()` above. It is the actor lane's because a
        // scope has to be gathered against some transaction, not because that
        // lane is privileged.
        let scope = AuthoritativeScope::gather(world, &transactions[0]);
        let projection = project_post_publication_roster(&scope, &effects);
        verify_projected_roster(&projection, &effects, &baseline, &scope, world)
            .err()
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for violation in &projection_violations {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` would not publish into a valid world: {violation}"
        );
    }

    let published =
        violations.is_empty() && projection_violations.is_empty() && staged_violations.is_empty();
    if published {
        let admitted: usize = transactions
            .iter()
            .map(|transaction| {
                ambition_platformer2d_shared_tangle::construction::publish_candidate(
                    world,
                    transaction,
                )
            })
            .sum();
        // ⛔⛤ **AND ONLY NOW IS N RETIRED.** Every candidate this room built is
        // authoritative as of the line above; the bodies it declared it was
        // replacing go on the line below, in that order and never the other
        // one. See `retire_superseded`.
        //
        // ⚠ **THE SWEEP RUNS BEFORE THE BACKSTOP**, and the reason is simply that
        // a domain-aware retirement should precede a declaration-driven one — NOT,
        // as I first wrote, because `retire_superseded` could otherwise despawn a
        // physics body past its grace period. MEASURED: a physics room entity
        // carries `RoomVisual` and `PhysicsRoomEntity` and no `SimId`, so it is
        // invisible to `TransactionBaseline::capture` and out of
        // `retire_superseded`'s reach entirely.
        // ⛔⛤ **AND ONLY NOW DOES THE LIVE WORLD CHANGE AT ALL.** The outgoing
        // room is swept and the world-defining state published here, after the
        // candidate became authoritative — never before it, which is the order
        // `replace_live_world` used to name as a destructive window it could only
        // give one address to.
        if let Some(pending) = world.remove_resource::<PendingWorldReplacement>() {
            apply_world_replacement(world, pending);
        }
        let superseded = ambition_platformer2d_shared_tangle::construction::retire_superseded(
            world, &effects, &baseline,
        );
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-loaded {room_id} ({admitted} roots admitted, \
             {} declared departures retired, {} left to their custodian)",
            superseded.retired, superseded.left_to_custodian
        ));
        world.write_message(ambition_platformer2d_world::rooms::RoomLoaded {
            room_id: room_id.clone(),
        });
    } else {
        let failure_count =
            violations.len() + projection_violations.len() + staged_violations.len();
        // ⭐ THE LAST-GOOD-WORLD GUARANTEE, IN ONE STATEMENT: the room the
        // session is playing was never touched, so there is nothing to recover.
        let staged = world.remove_resource::<PendingWorldReplacement>().is_some();
        let dropped: usize = transactions
            .iter()
            .map(|transaction| {
                ambition_platformer2d_shared_tangle::construction::retire_candidate(
                    world,
                    transaction,
                )
            })
            .sum();
        // ⛔⛤ **A REFUSAL IS A WORLD EVENT, NOT ONLY A LOG LINE — 2026-09-14.**
        // This path wrote `bevy::log::error!` alone, and a harness without a log
        // plugin surfaces nothing: measured, flipping `ROOM_CANDIDATE_BRACKET` on
        // makes `death_restores_the_checkpoint` fail with the object NOWHERE and
        // prints no violation at all, so *"no violations printed"* could not be
        // told from *"the transaction published"*. A10 cannot be implemented
        // against an invisible refusal — the publication side has said
        // `room-loaded` on the same channel since it existed.
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-refused {room_id} ({failure_count} violation(s), {dropped} roots dropped, \
             live world {}): {}",
            if staged { "kept" } else { "was not staged" },
            violations
                .iter()
                .map(|violation| format!("{violation:?}"))
                .chain(
                    projection_violations
                        .iter()
                        .map(|violation| format!("{violation:?}")),
                )
                .chain(
                    staged_violations
                        .iter()
                        .map(|violation| format!("{violation:?}")),
                )
                .collect::<Vec<_>>()
                .join(", ")
        ));
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` was NOT published: {failure_count} construction violation(s). \
             The candidate was never visible and its {dropped} roots are dropped."
        );
    }
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations,
        projection_violations,
        staged_violations,
        published,
    });
}
