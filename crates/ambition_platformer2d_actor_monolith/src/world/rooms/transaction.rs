//! Room construction transaction boundary.
//!
//! One room load captures a baseline before construction and verifies the
//! completed room before publishing `RoomLoaded`; feature plans are participants,
//! not owners of that outer transaction. The same open/close bracket serves
//! deferred and exclusive-world execution. Verification can withhold publication
//! but cannot undo already-applied Bevy commands, so this is consistency checking,
//! not atomic rollback.

use std::collections::BTreeSet;

use bevy::ecs::component::Component;
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
/// ⛔⛤ **THE CONTROL PLANE IS EXACT NOW, AND IT WAS A SINGLETON — CHANGED
/// 2026-09-14 ON REVIEW.** The candidate ENTITIES carried exact `TransactionId`s
/// while the baseline, the staged world and the verdict were all *"the pending
/// one"* / *"the last one"*: three App resources a second publication would have
/// overwritten. That is the opposite direction from the rest of A10, and it made
/// multi-region residency a special case before it was written.
///
/// ⇒ One ENTITY is the publication. Everything that belongs to it hangs off that
/// entity — baseline, declared effects, staged world, owning lane transactions,
/// target room, verdict — so `baseline(P)`, `staged_world(P)`, `effects(P)` and
/// `verdict(P)` refer to the same P by construction rather than by a check.
/// [`PublicationHandle`] is what a caller holds to ask about its OWN publication.
#[derive(Component)]
pub(crate) struct PendingConstructionBaseline(Result<OpenedTransaction, OpenRefused>);

/// A caller's exact reference to the publication it started.
///
/// ⚠ **HOST-LOCAL AND CONTROL-PLANE ONLY.** It is an `Entity`, never canonical
/// state, never in a snapshot, never mixed into a `SimId` or a `TransactionId`.
/// The peer-stable identity campaign owns canonical provenance; this is the
/// engine's own bookkeeping and must not become a second identity workaround.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationHandle(pub bevy::ecs::entity::Entity);

/// What a room publication is ABOUT: which room, and which construction lanes
/// it owns.
#[derive(Component)]
pub(crate) struct RoomPublication {
    room_id: String,
    transactions: Vec<ambition_platformer2d_shared_tangle::construction::TransactionId>,
    retention: PublicationRetention,
}

/// How long a publication's RECEIPT outlives the publication.
///
/// ⛔⛤ **AN UNRELATED PUBLICATION MUST NEVER INVALIDATE ANOTHER OWNER'S
/// RECEIPT — CORRECTED 2026-09-14 ON REVIEW.** `begin_publication` used to reap
/// every finished publication in the world, so the validity of A's exact receipt
/// depended on whether B happened to begin: *"A finishes, caller still holds A's
/// handle, B begins, `publication_succeeded(A)` is suddenly false"*. That is
/// ownership by coincidence, and it is exactly what a candidate session holding
/// its first room's receipt across an activation decision cannot tolerate.
///
/// ⇒ Retention is DECLARED at `begin_publication`, by the caller that knows
/// whether anyone is going to read the verdict. Nothing else retires a
/// publication, ever.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationRetention {
    /// Nobody holds this handle: the publication is retired the instant its
    /// verdict is recorded. Session activation's first room is the case — it
    /// commits through `spawn_contents` and drops the handle on the floor.
    UntilTheVerdictIsRecorded,
    /// Retained until its owner calls [`retire_publication`]. The owner reading
    /// the verdict is what ends the publication, so a reader queued behind this
    /// frame's flush — or an activation decision that spans frames — finds the
    /// receipt it was promised.
    UntilOwnerRetires,
}

/// End a publication whose owner has read its verdict.
///
/// ⚠ **IDEMPOTENT, AND DELIBERATELY UNCONDITIONAL.** An owner retires its
/// publication whether it published or refused — a refused receipt is just as
/// consumed as an admitted one — and retiring one that is already gone is not an
/// error, because a caller cannot always know whether its verdict was ever
/// recorded.
pub fn retire_publication(world: &mut World, publication: PublicationHandle) {
    if let Ok(entity) = world.get_entity_mut(publication.0) {
        entity.despawn();
    }
}

/// The verdict of one exact publication.
///
/// ⛔⛤ **PRODUCTION AUTHORIZES FROM THIS, NOT FROM `LastConstructionVerification`.**
/// That resource is last-writer-wins and distinguishes transactions only by room
/// NAME — two operations on one room are indistinguishable in it — and it was
/// nevertheless deciding whether the hot reload could advance the session's
/// content generation and whether the reset could wipe the save. A diagnostic
/// resource is not a transaction receipt. It stays, as diagnostics.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationVerdict {
    pub published: bool,
}

/// Start a publication: the entity everything about it will hang off.
///
/// ⛔⛤ **IT REAPS NOTHING. THE CALLER DECLARES THE RECEIPT'S LIFETIME — see
/// [`PublicationRetention`].** This used to despawn every finished publication
/// in the world, which made one owner's receipt depend on whether an unrelated
/// publication happened to begin.
pub(crate) fn begin_publication(
    commands: &mut Commands,
    room_id: String,
    transactions: Vec<ambition_platformer2d_shared_tangle::construction::TransactionId>,
    retention: PublicationRetention,
) -> PublicationHandle {
    let publication = commands.spawn(RoomPublication {
        room_id,
        transactions,
        retention,
    });
    PublicationHandle(publication.id())
}

/// Did this exact publication publish?
///
/// ⛔ **ABSENT IS `false`, AND SO IS AN UNFINISHED ONE.** A caller whose writes
/// are conditional on a publication has nothing to be conditional on until the
/// verdict exists.
pub fn publication_succeeded(world: &World, publication: PublicationHandle) -> bool {
    world
        .get::<PublicationVerdict>(publication.0)
        .is_some_and(|verdict| verdict.published)
}

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

/// ⛔⛤ **A COMPONENT ON A HIDDEN CANDIDATE ENTITY, NOT A RESOURCE — CHANGED
/// 2026-09-14 TO THE SHAPE THE RULING PREFERS.** It was a `Resource`, which is
/// the *"paired `ActiveFoo`/`CandidateFoo` process global"* shape A10's settled
/// architecture says not to prefer, and it showed: the refusal path had to
/// REMEMBER to remove it, and a replacement whose transaction never closed leaked
/// into the next room's. Held under the candidate it is retired by
/// `retire_candidate` along with everything else the transaction made, and a leak
/// carries the dead transaction's stamp so the next room cannot see it at all.
#[derive(bevy::prelude::Component)]
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
    /// ⚠ It WAS reachable by ORDERING as well as by misuse: session activation
    /// queued its room build before it spawned the session root. That order is
    /// reversed as of 2026-09-14 (`PlatformerSessionBuilder::build`), and
    /// `verify_and_publish` now refuses ANY room publication in a shell-routed
    /// composition that has no root to publish into, staged world or not.
    NoSessionRootToPublishInto,
    /// A world was staged and the composition holds no `MovingPlatformSet` to
    /// publish its platform state into.
    ///
    /// ⛔ **THE SAME FAIL-CLOSED ARGUMENT AS THE ONE ABOVE, AND IT WAS THE LAST
    /// SILENT SKIP IN THE PUBLICATION.** `apply_world_replacement` writes the
    /// platform state through `get_resource_mut`, which answers `None` when the
    /// resource is absent — so a room with authored moving platforms would
    /// publish, report `room-loaded`, and leave the world with no platforms in
    /// it, which is a room the player falls through.
    NoPlatformStateToPublishInto,
    /// A world was staged and the live session root carries no `RoomSet` to
    /// publish the active room into.
    ///
    /// ⚠ **THE THIRD AND LAST ACCESSOR IN `apply_world_replacement` THAT CAN
    /// ANSWER `None`, AND THE ONLY ONE OF THE THREE I CANNOT NAME A PRODUCTION
    /// ROAD TO.** A session root always carries a room set — the provider's
    /// bundle and the direct-entry app both insert one. It is checked anyway
    /// because the alternative is a PARTIAL publication reported as success: the
    /// geometry and the platform state would be written and the active room
    /// silently left where it was, which is a session colliding against one room
    /// while believing it is in another.
    NoRoomSetToPublishInto,
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
            Self::NoPlatformStateToPublishInto => write!(
                f,
                "this room staged moving-platform state and the composition holds \
                 no `MovingPlatformSet`, so publishing would leave the room \
                 without the platforms it authored"
            ),
            Self::NoRoomSetToPublishInto => write!(
                f,
                "this room staged a world and the live session root carries no \
                 `RoomSet`, so publishing would write the geometry and leave the \
                 active room where it was"
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
    // ⛔ ONLY WHEN THERE IS SOMETHING TO PUBLISH. A room with no authored
    // platforms states an empty vector and means it, and a composition that has
    // never needed the resource is not wrong for lacking one.
    if !pending.moving_platforms.is_empty()
        && !world.contains_resource::<ambition_platformer2d_world::collision::MovingPlatformSet>()
    {
        violations.push(StagedWorldViolation::NoPlatformStateToPublishInto);
    }
    if rooms.is_none() {
        violations.push(StagedWorldViolation::NoRoomSetToPublishInto);
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
    publication: PublicationHandle,
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
                    // ⛔⛤ **THE DEPARTURE AUTHORITY IS DECLARED HERE, FROM THE
                    // BASELINE, NOT DISCOVERED AT RETIREMENT.** A predecessor in
                    // another entity's CUSTODY is removed by its custodian —
                    // `restore_custody_to_checkpoint` unequips AND despawns it as
                    // one operation keyed on that entity, so reaching in first
                    // destroys the key the other half is found by. Publication
                    // used to promise it gone and then skip it because of a
                    // component it noticed; now the promise matches what it does.
                    let mut effects = superseding.iter().fold(
                        PublicationEffects::new(),
                        |effects, sim_id| {
                            let departs = baseline
                                .entries()
                                .get(sim_id)
                                .filter(|entry| {
                                    world.get::<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>(
                                        entry.entity,
                                    )
                                    .is_some()
                                })
                                .map_or(
                                    ambition_platformer2d_shared_tangle::construction::DepartureAuthority::Publication,
                                    |_| ambition_platformer2d_shared_tangle::construction::DepartureAuthority::Custodian,
                                );
                            effects.superseding_under(sim_id.clone(), sim_id.clone(), departs)
                        },
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
                    let staged = world
                        .get::<PendingWorldReplacement>(publication.0)
                        .map(PendingWorldReplacement::outgoing_entities);
                    if let Some(outgoing) = staged.as_ref() {
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
        // ⛔ ON THE PUBLICATION, not in a resource: a second publication in
        // flight would have overwritten "the pending baseline".
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            entity.insert(PendingConstructionBaseline(captured));
        }
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
    publication: PublicationHandle,
    plan: &crate::features::RoomFeatureConstructionPlan,
    receipt: &crate::features::RoomFeatureConstructionReceipt,
    session: SessionSpawnScope,
    candidate_bracket: bool,
) {
    let plan = plan.clone();
    let receipt = receipt.clone();
    commands.queue(move |world: &mut World| {
        verify_and_publish(world, publication, &plan, &receipt, session, candidate_bracket);
    });
}

/// The content generation ONE SESSION is live under — the commit boundary's
/// comparison value for [`RosterViolation::ContentBindingMismatch`].
///
/// ⛔⛤ **IT LIVES ON THE SESSION ROOT, NOT IN A PROCESS-GLOBAL RESOURCE —
/// MOVED 2026-09-14 FOR A10.4.** It was a `Resource`, which is one mirror of a
/// per-session fact, and A10.4 needs two sessions to hold their own at once: a
/// candidate session's first room must verify against the CANDIDATE's generation
/// while the outgoing session is still live under its own. A global could only be
/// overwritten before the candidate's room verified, which destroys the live
/// session's answer — the retire-then-overwrite shape A10 exists to remove.
///
/// ⇒ A room transaction reads it off the root it is publishing INTO
/// (`session_root_for_scope`), so "which generation is this room being committed
/// into" is a question about a session rather than about the process.
///
/// Written by the content activation authorities: session setup inserts it on the
/// root it just spawned, and a hot-reload commit that allocates a new epoch
/// updates it behind that room's verdict. Room transitions and resets do not
/// change content, so they never write it. Absent (headless fixtures, unit tests
/// without a session) the boundary check is vacuous — an honest gap, not a
/// waiver: a fixture with no content authority has nothing to be stale
/// against.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
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
    publication: PublicationHandle,
    plan: &crate::features::RoomFeatureConstructionPlan,
    receipt: &crate::features::RoomFeatureConstructionReceipt,
    session: SessionSpawnScope,
    candidate_bracket: bool,
) {
    // ⛔⛤ **WHAT THIS PUBLICATION IS ABOUT IS READ OFF THE PUBLICATION.** The
    // room's name and the set of lane transactions it owns were stated once, at
    // `begin_publication`, by the caller that also built them. Taking the name as
    // a parameter and re-deriving the lanes from the plan here were two more
    // spellings of facts P already holds, and two spellings is how the ends of a
    // bracket come to disagree about which room — or which lanes — a verdict is
    // for. See [`RoomPublication`].
    let Some((room_id, transactions, retention)) = world
        .get::<RoomPublication>(publication.0)
        .map(|about| (about.room_id.clone(), about.transactions.clone(), about.retention))
    else {
        // Not a refusal: there is no publication to refuse. A handle whose entity
        // is gone means the caller closed a publication that was never begun, or
        // one already reaped.
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "a room transaction closed against publication {:?}, which holds no `RoomPublication`",
            publication.0
        );
        return;
    };

    // ⛔ THE VERDICT GOES ON THE PUBLICATION, whatever the outcome. A caller
    // whose follow-up work is conditional on THIS publication reads it there;
    // `LastConstructionVerification` below is diagnostics and stays that way.
    // ⛔ AND THE DECLARED RETENTION IS SETTLED IN THE SAME STATEMENT. A
    // publication nobody holds ends here, where the last thing that will ever
    // touch it is; one with an owner stands until that owner retires it.
    let record = |world: &mut World, published: bool| {
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            match retention {
                PublicationRetention::UntilTheVerdictIsRecorded => entity.despawn(),
                PublicationRetention::UntilOwnerRetires => {
                    entity.insert(PublicationVerdict { published });
                }
            }
        }
    };
    let refuse = |world: &mut World, room_id: String| {
        record(world, false);
        // ⛔ THE STAGED WORLD GOES WITH THE CANDIDATE, and no longer by being
        // remembered here: it is candidate-owned state stamped with this room's
        // transaction, so `retire_candidate` takes it. This early road refuses
        // BEFORE the transactions are known, and a staged world left standing
        // here carries a dead stamp that no later room can find.
        world.insert_resource(LastConstructionVerification {
            room_id,
            violations: Vec::new(),
            projection_violations: Vec::new(),
            staged_violations: Vec::new(),
            published: false,
        });
    };

    let OpenedTransaction { baseline, effects } =
        match world
            .get_entity_mut(publication.0)
            .ok()
            .and_then(|mut entity| entity.take::<PendingConstructionBaseline>())
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

    // ⛔⛤ **AND A ROOM PUBLISHES INTO A SESSION. IN A SHELL-ROUTED COMPOSITION
    // THERE IS NO SUCH THING AS A ROOM WITHOUT ONE — ADDED 2026-09-14 WITH
    // A10.4's ORDERING FIX.** Session activation used to queue its first room's
    // build BEFORE it spawned the session root, so the activating room was the
    // one room publication in the project that took its verdict in a world where
    // its own session did not exist. Everything that publishes THROUGH the root
    // (`apply_world_replacement`, and every `session_world_component_mut` write
    // behind a verdict) answers `None` there and says nothing — the fail-open
    // shape `StagedWorldViolation::NoSessionRootToPublishInto` already refuses
    // for a staged world. The activation road stages no world, so that check
    // could not see it.
    //
    // ⚠ **THE DISCRIMINATOR IS COMPOSITION, NOT PRESENCE** — the same reasoning
    // as the binding check above. A direct-entry fixture legitimately builds
    // rooms with no session at all; a shell-routed host owes one, and "the
    // resource happens to be missing" is not a waiver there.
    //
    // ⛔⛤ **AND IT ASKS FOR *THIS TRANSACTION'S* SESSION, NOT FOR THE LIVE ONE.**
    // `session_world_entity` answers *"which root is live right now"*, which is a
    // different question from *"which root does this publication belong to"* the
    // moment a candidate session exists: its first room is prepared under the
    // candidate's scope while the PREVIOUS session's root is still the live one.
    // `session_root_for_scope` asks by the scope the plan was prepared under and
    // sees hidden candidate roots, which is what makes a first room buildable
    // INTO a candidate session. See A10.4.
    let shell_routed = world.contains_resource::<
        ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation,
    >();
    // ⚠ **AN UNSCOPED TRANSACTION BELONGS TO WHATEVER SINGLE ROOT THE
    // COMPOSITION HAS.** Direct-entry hosts, demos and headless fixtures build
    // with `SessionSpawnScope::UNSCOPED` and still carry a root; there is no
    // candidate session there to disambiguate from, so the live-root question IS
    // the right one. MEASURED: asking `session_root_for_scope` unconditionally
    // made four refusal fixtures PUBLISH, because a `None` scope can match no
    // root and the binding comparison then had nothing to compare against.
    let publishing_into = match session.id() {
        Some(scope) => {
            ambition_platformer2d_shared_tangle::lifecycle::session_root_for_scope(world, scope)
        }
        None => ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(world),
    };
    if shell_routed && publishing_into.is_none() {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` cannot be published: this composition routes gameplay \
             through a shell session and session {:?} carries no root to publish \
             into, so every write through the root would silently do nothing",
            session.id()
        );
        refuse(world, room_id);
        return;
    }

    // ⛔⛤ **AN ABSENT BINDING USED TO MEAN "NO COMPARISON", WHICH IS FAIL-OPEN —
    // AND A 2026-09-13 REVIEW NAMED WHY THAT IS NOT A GAP BUT A HOLE.** The
    // binding's own doc said the vacuous branch was *"an honest gap, not a
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
    // from whether the state happens to be there.
    //
    // ⛔ **AND IT IS READ OFF THE ROOT THIS PUBLICATION IS GOING INTO**, which is
    // the same root the precondition above just found. Two sessions may hold two
    // generations at once; the one that answers here is this room's own.
    match publishing_into.and_then(|root| world.get::<ActiveContentBinding>(root).copied()) {
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
        None if shell_routed => {
            // A shell-routed composition owes this state on its root. Publishing
            // here would admit a room whose staleness nothing checked.
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "room `{room_id}` cannot be verified: this composition routes \
                 gameplay through a shell session, so `ActiveContentBinding` is a \
                 canonical authority its session root must hold, and it is absent"
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
    // DROPPED, and so is the world it staged.
    //
    // ⛔⛤ **THIS COMMENT USED TO SAY "THIS IS NOT THE LAST-GOOD-WORLD GUARANTEE"
    // AND LIST FOUR THINGS PUBLISHED BEFORE THE VERDICT. ALL FOUR ARE CLOSED —
    // 2026-09-14.** They were: `commit_deferred` writing `RoomSet`,
    // `RoomGeometry` and the platform state; `replace_live_world` retiring the
    // OUTGOING room first; the hot-reload caller advancing the session's content
    // generation afterwards; and a verifier with no way to validate N+1 while
    // intentionally RETAINING N. In order: `PendingWorldReplacement` stages all
    // of the first, the sweep moved inside `apply_world_replacement`,
    // `room_publication_succeeded` gates the third, and `superseding` plus
    // `verify_projected_roster` answer the fourth.
    //
    // ⚠ **WHAT IS STILL OUTSIDE THE VERDICT IS NAMED IN `docs/planning/queue.md`'s
    // A10 row** — the whole SESSION scope, which is a different transaction. The
    // hot reload's body transit and presentation spawns joined the verdict on
    // 2026-09-14 (`reload_ldtk_world_from_disk`). Do not read the absence of a
    // warning here as their absence.

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
    let staged_entity = world
        .get::<PendingWorldReplacement>(publication.0)
        .map(|_| publication.0);
    let staged_violations = match staged_entity
        .and_then(|entity| world.get::<PendingWorldReplacement>(entity))
    {
        Some(pending) => {
            // The borrow ends before the verifier reads the world again.
            let pending: &PendingWorldReplacement = pending;
            let found = verify_staged_world(world, pending);
            found.err().unwrap_or_default()
        }
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
        if let Some(entity) = staged_entity {
            // ⛔ TAKEN OFF THE PUBLICATION: adopting the staged world means the
            // publication no longer carries a replacement that has already been
            // applied. The publication itself survives until its verdict has
            // been read — see `begin_publication`.
            if let Some(pending) = world.entity_mut(entity).take::<PendingWorldReplacement>() {
                apply_world_replacement(world, pending);
            }
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
        let staged = staged_entity.is_some();
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
    record(world, published);
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations,
        projection_violations,
        staged_violations,
        published,
    });
}
