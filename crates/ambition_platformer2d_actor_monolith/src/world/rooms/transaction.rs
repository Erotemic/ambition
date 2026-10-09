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
use ambition_platformer2d_shared_tangle::lifecycle::{live_room_root_for, SessionSpawnScope};

/// The baseline captured at the head of a construction transaction, waiting for
/// the verification pass at its tail.
///
/// A COMPONENT ON THE PUBLICATION, because the two ends are separate commands in
/// one queue and nothing else can carry a value between them. ⚠ THIS SENTENCE
/// READ "a resource" UNTIL 2026-09-18, three lines above the `#[derive(Component)]`
/// that contradicts it and directly above the paragraph explaining why a resource
/// was wrong — the stale half of the very change it describes.
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

/// What one room's candidate construction actually built, recorded by the
/// construction boundary for the verdict to read.
///
/// ⛔⛤ **IT IS ON THE PUBLICATION BECAUSE CONSTRUCTION IS NOW DEFERRED —
/// 2026-09-15 AUDIT, FINDING 4.** `close` used to take the receipt as a
/// parameter, which was only possible because `spawn_contents_for` called the
/// spawn at QUEUE time and got its receipt immediately. That is exactly what made
/// the prerequisite unenforceable: the construction commands were already in the
/// queue behind `open`, so an opening refusal could not stop them. Construction
/// is one exclusive-world command now, and its receipt travels the way every
/// other fact about a publication travels — on the publication.
///
/// ⚠ ABSENT MEANS NOTHING WAS BUILT. That is a refusal, not an empty room.
#[derive(Component)]
pub(crate) struct PendingConstructionReceipt(
    pub(crate) crate::features::RoomFeatureConstructionReceipt,
);

/// Did this publication's OPENING refuse?
///
/// ⛔⛤ **ONE DECISION, READ ONCE — 2026-09-15 AUDIT, FINDING 4.** The room
/// construction boundary asks this before it builds anything, rather than
/// re-testing the world fact `open` already tested. Two spellings of one
/// prerequisite is how the two ends of a bracket come to disagree, and this
/// version also covers every OTHER reason an opening can refuse — a duplicate
/// identity in the baseline included — instead of only the missing filter.
///
/// ⚠ FAIL-CLOSED. A publication with no opening record at all is treated as
/// refused: the only way to reach that is a publication nothing opened, and
/// building a candidate population for one of those is the outcome this exists
/// to prevent.
pub(crate) fn opening_refused(world: &World, publication: PublicationHandle) -> bool {
    world
        .get::<PendingConstructionBaseline>(publication.0)
        .is_none_or(|opened| opened.0.is_err())
}

/// A caller's exact reference to the publication it started.
///
/// ⚠ **HOST-LOCAL AND CONTROL-PLANE ONLY.** It is an `Entity`, never canonical
/// state, never in a snapshot, never mixed into a `SimId` or a `TransactionId`.
/// The peer-stable identity campaign owns canonical provenance; this is the
/// engine's own bookkeeping and must not become a second identity workaround.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationHandle(pub bevy::ecs::entity::Entity);

impl PublicationHandle {
    /// The receipt entity this publication's own state hangs off — its verdict,
    /// its staged world, and its custody handoffs.
    pub fn entity(self) -> bevy::ecs::entity::Entity {
        self.0
    }
}

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

/// Settle a publication this caller owns: read its verdict ONCE, run the arm
/// that verdict selects, then retire the receipt. Returns whether it published.
///
/// The owner roads (room transition, New Game, the LDtk dev reload) each wrote
/// this skeleton by hand, and each had to remember two orderings its own
/// comments warned about: retirement comes after EVERY reader, and it happens
/// on the refusal branch too, so an arm that returns early cannot leak the
/// receipt. Here both are structure. Call it where the verdict exists — from a
/// command queued behind the commit, or an exclusive system after the flush.
pub fn settle_publication(
    world: &mut World,
    publication: PublicationHandle,
    on_published: impl FnOnce(&mut World),
    on_refused: impl FnOnce(&mut World),
) -> bool {
    let published = publication_succeeded(world, publication);
    if published {
        on_published(world);
    } else {
        on_refused(world);
    }
    retire_publication(world, publication);
    published
}

/// Everything a VERIFIED room publication owes the world outside its own
/// candidate population, frozen at the verdict and consumed at finalization.
///
/// ⛔⛤ **A10 NESTED ENTITY VISIBILITY WITHOUT NESTING PUBLICATION EFFECTS —
/// 2026-09-15 REVIEW, FINDING 1.** The barrier count made a room publishable
/// INSIDE a still-hidden candidate session: the room lowers its own barrier and
/// the session's keeps the population invisible, which is correct and stays.
/// What did not nest is everything the same success arm did NEXT —
/// `retire_superseded` despawning live bodies, `apply_custody_handoffs`
/// stripping items off live hands, `apply_world_replacement` writing the
/// world-defining state, and a `RoomLoaded` message that carries only a room id.
///
/// ⚠ **`RoomLoaded` ALONE IS ENOUGH TO BREAK THE INVARIANT, AND NOT
/// HYPOTHETICALLY.** `FreshAttempt` treats any `RoomLoaded` as a fresh attempt;
/// production's `void_pending_player_hits_at_lifecycle_boundaries` answers by
/// clearing `PendingPlayerHitEvents`, which is rollback-registered and
/// checksummed. So a candidate B that later REFUSES could still have cleared
/// live session A's staged hits on its way past — A survives, but A is not
/// UNCHANGED, and unchanged is what the last-good-world invariant promises.
/// `a_published_room_inside_a_pending_candidate_session_stays_invisible` already
/// shows that window is multiple frames wide.
///
/// ⇒ **THE RULE, STATED ONCE:** nothing constructed or internally published
/// under candidate B may produce an externally authoritative effect outside B's
/// ownership before B's own publication boundary. The bundle is how that rule is
/// represented rather than remembered — an ordinary live-room transition
/// finalizes it on the spot, and a candidate session's first room hands it to
/// the session, which finalizes it at admission.
#[derive(Component)]
pub(crate) struct FrozenPublicationEffects {
    room_id: String,
    /// ⛔ THE EXACT TARGET, RESOLVED ONCE. See [`apply_world_replacement`].
    target: Option<bevy::ecs::entity::Entity>,
    effects: ambition_platformer2d_shared_tangle::construction::PublicationEffects,
    baseline: ambition_platformer2d_shared_tangle::construction::TransactionBaseline,
    admitted: usize,
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

/// How many publication receipts are still standing.
///
/// ⛔⛤ **FOR ASSERTING THAT NONE OUTLIVES ITS OPERATION.** A
/// `PublicationRetention::UntilOwnerRetires` receipt is an ENTITY that stands
/// until its owner calls [`retire_publication`], and "every owner retires it" was
/// a property held by reading five call sites. Nothing could count them, so
/// nothing could notice a sixth owner that forgot — the same shape as the
/// candidate-slot exits that leaked until they were enumerated.
///
/// ⚠ A count, not a list: the receipts themselves stay `pub(crate)`, because a
/// reader outside this crate has no business holding one.
pub fn outstanding_publications(world: &mut World) -> usize {
    world.query::<&RoomPublication>().iter(world).count()
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

/// Record a verdict on the publication as its retention asks.
fn record_verdict(
    world: &mut World,
    publication: PublicationHandle,
    retention: PublicationRetention,
    published: bool,
) {
    if let Ok(mut entity) = world.get_entity_mut(publication.0) {
        match retention {
            PublicationRetention::UntilTheVerdictIsRecorded => entity.despawn(),
            PublicationRetention::UntilOwnerRetires => {
                entity.insert(PublicationVerdict { published });
            }
        }
    }
}

/// A publication that passed its check and waits for the owner of its
/// sequence. Its candidates are not promoted and its effects are frozen on it
/// ([`FrozenPublicationEffects`]). A sequence of rooms that must publish all
/// or none checks each room first, and then commits each room or refuses each
/// room: no room of it is the world's before every room passed.
#[derive(Component)]
struct HeldForOwner {
    supersessions: usize,
}

/// Whether the publication passed its check and waits for its owner.
pub fn publication_is_held(world: &World, publication: PublicationHandle) -> bool {
    world.get::<HeldForOwner>(publication.0).is_some()
}

/// Promote and finalize a held publication. `false` if it was not held.
pub fn commit_held_publication(world: &mut World, publication: PublicationHandle) -> bool {
    let Some(HeldForOwner { supersessions }) = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<HeldForOwner>())
    else {
        return false;
    };
    let Some((room_id, transactions, retention)) = world
        .get::<RoomPublication>(publication.0)
        .map(|about| (about.room_id.clone(), about.transactions.clone(), about.retention))
    else {
        return false;
    };
    let Some(FrozenPublicationEffects { target, effects, baseline, .. }) = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<FrozenPublicationEffects>())
    else {
        return false;
    };
    let left_to_custodian = promote_and_finalize(
        world,
        publication,
        &room_id,
        &transactions,
        target,
        effects,
        baseline,
    );
    record_verdict(world, publication, retention, true);
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations: Vec::new(),
        projection_violations: Vec::new(),
        staged_violations: Vec::new(),
        published: true,
        left_to_custodian,
        supersessions,
    });
    true
}

/// Refuse a held publication because another room of its sequence was
/// refused. Nothing of it was promoted, so this is the refusal road of a room
/// that failed its own check: its candidates are dropped and its frozen
/// effects go with the publication, having done nothing.
pub fn refuse_held_publication(world: &mut World, publication: PublicationHandle) {
    let held = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<HeldForOwner>())
        .is_some();
    let Some((room_id, transactions, retention)) = world
        .get::<RoomPublication>(publication.0)
        .map(|about| (about.room_id.clone(), about.transactions.clone(), about.retention))
    else {
        return;
    };
    if !held {
        return;
    }
    crate::items::pickup::discard_custody_handoffs(world, publication);
    if let Ok(mut entity) = world.get_entity_mut(publication.0) {
        entity.remove::<(PendingWorldReplacement, FrozenPublicationEffects)>();
    }
    let dropped: usize = transactions
        .iter()
        .map(|transaction| {
            ambition_platformer2d_shared_tangle::construction::retire_candidate(world, transaction)
        })
        .sum();
    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
        "room-refused {room_id} (passed its check; another room of its sequence was \
         refused, {dropped} roots dropped, live world kept)"
    ));
    record_verdict(world, publication, retention, false);
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations: Vec::new(),
        projection_violations: Vec::new(),
        staged_violations: Vec::new(),
        published: false,
        left_to_custodian: 0,
        supersessions: 0,
    });
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

// ⛔ A ROOM IS A CANDIDATE UNTIL IT IS ADMITTED — unconditionally. There is no
// flag and no parameter: `open`'s filter check, the supersession partition and
// the projected-roster check below all run for every room.
//
// ⚠ A test that wants an UNHIDDEN population asks `ConstructionPlan::commit`,
// the construction-layer primitive; it must not ask a room API to pretend.

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
/// ⛔⛤ **THE HALF OF A10 THAT IS NOT ENTITIES. EVERYTHING BELOW DESCRIBES
/// THE SHAPE A10 DELETED ON 2026-09-14 — THIS PARAGRAPH DID NOT SAY SO UNTIL
/// 2026-09-19, AND IN THE PRESENT TENSE IT READS AS LIVE ARCHITECTURE.**
/// `RoomConstructionPlan::commit_deferred` <!-- cite-ok: names the method A10 DELETED; that deletion is what this paragraph is about -->
/// is four statements and only the last one builds the
/// candidate; the other three — `rooms.set_active`, `geometry.0 = ..`,
/// `*moving_platforms = ..` — write the LIVE world, and `replace_live_world`
/// retires the OUTGOING room ahead of all of it. Left there, a refusal points
/// the session at a room index with no room in it: the strictly worse outcome
/// A10 exists to delete, reached by the candidate machinery WORKING.
///
/// ⇒ Every one of those four is now staged here and applied by the ONE
/// publication authority in `verify_and_publish` (private to this module), after the candidate has been
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
    /// The live room this publication mints, and what becomes of the room
    /// the crossing leaves, both pinned when it was staged. A replaced room's
    /// root is the sink for the geometry, and publication seats it as the
    /// minted instance; an opened room's root is spawned. The staged
    /// occupants carry the minted instance ([`Self::publishes_as`]).
    succession: Option<LiveRoomSuccession>,
    /// The other live rooms this publication retires whole, root and all,
    /// beside the one it replaces: a whole-session restart (Q151). Their
    /// residents are in `outgoing`. Empty for every other publication.
    retires_beside: Vec<ambition_platformer2d_world::rooms::LiveRoomInstance>,
    /// The checkpoint restore this publication is, if it is one. When the
    /// verdict accepts the room, its consequences run before anything of the
    /// old room is retired (`session::checkpoint::run_restore_consequences`).
    restore: Option<crate::session::checkpoint::CheckpointOperationKey>,
    /// The verdict waits for the owner of a sequence of publications. A room
    /// that passes its check promotes nothing until the owner commits it
    /// ([`commit_held_publication`]) or refuses it
    /// ([`refuse_held_publication`]).
    held: bool,
}

/// A publication's live rooms: the one it mints, and what becomes of the
/// live room it is published from.
///
/// The minted one comes from the session's counter
/// ([`ambition_platformer2d_world::rooms::RoomSet::next_live_room`]), not from
/// `replaces.next()`. With two live rooms, `replaces.next()` can be the other
/// room's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveRoomSuccession {
    /// The publication re-seats live room `replaces` as `mints`. The room is
    /// retired with its residents, and the crossing body and what it carries
    /// move into the new one.
    Replace {
        replaces: ambition_platformer2d_world::rooms::LiveRoomInstance,
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// The publication opens `mints` as a new live room beside `leaves`,
    /// which stays whole, because another player's body is still in it (OW1
    /// cut 6c). Nothing is retired. Only the crossing body and what it
    /// carries move into the new room.
    Open {
        leaves: ambition_platformer2d_world::rooms::LiveRoomInstance,
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// The crossing body enters `joins`, a live room of the target room that
    /// another player already holds (OW1 cut 6e). Nothing is built and
    /// nothing is written onto that room: it is live, and its state is the
    /// other player's world. When `retires`, no other player stays in
    /// `leaves`, so it is retired, as a replaced room is, and its root is
    /// despawned; otherwise only the crossing body and what it carries move.
    Join {
        leaves: ambition_platformer2d_world::rooms::LiveRoomInstance,
        joins: ambition_platformer2d_world::rooms::LiveRoomInstance,
        retires: bool,
    },
}

impl LiveRoomSuccession {
    pub fn replacing(
        replaces: ambition_platformer2d_world::rooms::LiveRoomInstance,
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
    ) -> Self {
        Self::Replace { replaces, mints }
    }

    pub fn opening(
        leaves: ambition_platformer2d_world::rooms::LiveRoomInstance,
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
    ) -> Self {
        Self::Open { leaves, mints }
    }

    pub fn joining(
        leaves: ambition_platformer2d_world::rooms::LiveRoomInstance,
        joins: ambition_platformer2d_world::rooms::LiveRoomInstance,
        retires: bool,
    ) -> Self {
        Self::Join { leaves, joins, retires }
    }

    /// The succession of a crossing out of live room `leaves`.
    ///
    /// A live room stays live while a player's body is in it, and a room a
    /// player holds is the one live room of it that a crossing enters. So a
    /// crossing into a room another player holds (`joins`) joins it. Else a
    /// crossing that leaves another player's body behind opens a new live
    /// room, and the room it leaves stays whole. Otherwise the crossing
    /// replaces the room it leaves, which is the one-room profile: one
    /// player, one live room.
    pub fn for_crossing(
        leaves: ambition_platformer2d_world::rooms::LiveRoomInstance,
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
        another_player_stays: bool,
        joins: Option<ambition_platformer2d_world::rooms::LiveRoomInstance>,
    ) -> Self {
        match joins {
            Some(joins) => Self::joining(leaves, joins, !another_player_stays),
            None if another_player_stays => Self::opening(leaves, mints),
            None => Self::replacing(leaves, mints),
        }
    }

    /// The live room this publication mints. `None` for a crossing that
    /// joins a live room.
    pub fn mints(self) -> Option<ambition_platformer2d_world::rooms::LiveRoomInstance> {
        match self {
            Self::Replace { mints, .. } | Self::Open { mints, .. } => Some(mints),
            Self::Join { .. } => None,
        }
    }

    /// The live room the crossing body is in after the publication: the one
    /// it mints, or the one it joins.
    pub fn enters(self) -> ambition_platformer2d_world::rooms::LiveRoomInstance {
        match self {
            Self::Replace { mints, .. } | Self::Open { mints, .. } => mints,
            Self::Join { joins, .. } => joins,
        }
    }

    /// The live room this publication retires and re-seats. `None` for a
    /// publication that opens or joins a room.
    pub fn replaces(self) -> Option<ambition_platformer2d_world::rooms::LiveRoomInstance> {
        match self {
            Self::Replace { replaces, .. } => Some(replaces),
            Self::Open { .. } | Self::Join { .. } => None,
        }
    }

    /// The live room the crossing body is published from.
    pub fn leaves(self) -> ambition_platformer2d_world::rooms::LiveRoomInstance {
        match self {
            Self::Replace { replaces, .. } => replaces,
            Self::Open { leaves, .. } | Self::Join { leaves, .. } => leaves,
        }
    }

    /// Whether the room the crossing leaves stays live: then only the
    /// crossing body and what it carries move out of it.
    pub fn keeps_left(self) -> bool {
        match self {
            Self::Replace { .. } => false,
            Self::Open { .. } => true,
            Self::Join { retires, .. } => !retires,
        }
    }

    /// The live rooms the transaction's world is made of: the replaced room
    /// and the minted one; for a room it opens, the minted one alone; for a
    /// join, which builds nothing, the room it retires, or else the room it
    /// joins.
    pub fn transaction_rooms(self) -> ambition_platformer2d_shared_tangle::lifecycle::TransactionRooms {
        use ambition_platformer2d_shared_tangle::lifecycle::TransactionRooms;
        match self {
            Self::Replace { replaces, mints } => TransactionRooms::replacing(replaces, mints),
            Self::Open { mints, .. } => TransactionRooms::opening(mints),
            Self::Join { leaves, retires: true, .. } => TransactionRooms::only(leaves),
            Self::Join { joins, retires: false, .. } => TransactionRooms::only(joins),
        }
    }
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
            succession: None,
            retires_beside: Vec::new(),
            restore: None,
            held: false,
        }
    }

    /// Hold the verdict for the owner of a sequence of publications.
    pub(crate) fn holding(mut self, held: bool) -> Self {
        self.held = held;
        self
    }

    /// State the other live rooms this publication retires whole.
    pub(crate) fn retiring_beside(
        mut self,
        rooms: Vec<ambition_platformer2d_world::rooms::LiveRoomInstance>,
    ) -> Self {
        self.retires_beside = rooms;
        self
    }

    /// This publication is checkpoint restore `restore`.
    pub(crate) fn restoring(mut self, restore: Option<crate::session::checkpoint::CheckpointOperationKey>) -> Self {
        self.restore = restore;
        self
    }

    /// State the live room this publication replaces and the one it mints.
    pub(crate) fn replacing(mut self, succession: Option<LiveRoomSuccession>) -> Self {
        self.succession = succession;
        self
    }

    /// The live room this publication mints or joins. The staged occupants
    /// carry it.
    pub(crate) fn publishes_as(&self) -> Option<ambition_platformer2d_world::rooms::LiveRoomInstance> {
        self.succession.map(LiveRoomSuccession::enters)
    }

    /// The live room this publication replaces.
    fn replaces(&self) -> Option<ambition_platformer2d_world::rooms::LiveRoomInstance> {
        self.succession.and_then(LiveRoomSuccession::replaces)
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
    /// ⛔⛤ **THE CONSEQUENCE WAS SILENT, WHICH IS WHY THIS EXISTS.**
    /// `RoomSet::set_active` was `self.active = index.min(len - 1)` — an
    /// out-of-range index did not panic, it CLAMPED, and the session woke up in
    /// the last room of the set with the geometry of the one it was told to
    /// build. Measured by accident 2026-09-14: a poison that staged
    /// `usize::MAX` moved the active room rather than failing.
    ///
    /// ⭐ **THE SET REFUSES TOO, AND THIS CHECK IS STILL THE ONE THAT
    /// MATTERS.** `RoomSet::definition` returns `None` for an index out of
    /// range, so no caller can be talked into the clamp. But it is reached from
    /// `apply_world_replacement`, AFTER the outgoing room has been torn down — a
    /// refusal there has no good answer left, and the publication asserts rather
    /// than limps. This violation is the one that can still say no while the old
    /// world is standing, which is the whole reason a preflight exists.
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
    /// `apply_world_replacement` writes onto the root the transaction's own scope
    /// resolved and hands it in (`session_world_component_mut_at`), and every
    /// sink there answers `None` when that entity carries no such component —
    /// including when there is no session root to have resolved. ⚠ THIS SAID
    /// `session_world_component_mut` UNTIL 2026-09-18, which is the *"which root
    /// is LIVE right now"* question the 2026-09-15 review deliberately moved it
    /// OFF. The refusal is unchanged; the sentence describing it had gone stale
    /// against the very change that motivated it. So without this the room would
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
    /// The exact publication target carries no `RoomGeometry` (in
    /// `ambition_platformer2d_core::world`) to publish the
    /// staged room's geometry into.
    ///
    /// ⛔⛤ **THE SINK NOTHING PREFLIGHTED — 2026-09-15 REVIEW, FINDING 5.**
    /// `apply_world_replacement` wrote the geometry through an `if let Some(..)`
    /// and carried on, so a publication could validate a coherent staged world,
    /// report success, and leave the session colliding against the geometry of
    /// the room it just left. The room set and the platform state were checked;
    /// this one was not asked about at all.
    NoRoomGeometryToPublishInto,
    /// The session has no root for the live room this publication replaces:
    /// another publication replaced it after this one was staged, or it never
    /// existed. The staged occupants carry the instance after `replaces`, so
    /// publishing would put them in a live room that is not the session's.
    StaleRoomInstance {
        replaces: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// The session has no root for the live room this crossing was staged to
    /// join, or that root is not the target room: the other player's room
    /// was retired or replaced after this one was staged.
    StaleJoinedRoom {
        joins: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// Another publication minted the live room this one was staged to mint.
    /// Its occupants carry that identity, so publishing would put them in a
    /// live room that is already another room's.
    StaleMint {
        mints: ambition_platformer2d_world::rooms::LiveRoomInstance,
        next: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// The publication builds a live room of a room that another live room,
    /// which it does not replace, already instantiates. The durable rows
    /// (`AuthoredOccurrences`, `WorldTimeSchedule`) name a place by its
    /// room id, so two live rooms of one room would write one row each and
    /// build each other's occurrences. No shipped road gets here: a crossing
    /// into a held room joins it, and a replay or a reset replaces.
    DefinitionAlreadyLive {
        live: ambition_platformer2d_world::rooms::LiveRoomInstance,
    },
    /// The publication brings a room set that has no room for a live room it
    /// does not replace or retire. That room's definition is an index into
    /// the set, so after the replacement it would name no room, or another
    /// room. A hot reload that deletes the room another player is in gets
    /// here.
    LiveRoomNotInNextSet {
        live: ambition_platformer2d_world::rooms::LiveRoomInstance,
        room: String,
    },
}

impl std::fmt::Display for StagedWorldViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetRoomOutOfRange { target, rooms } => write!(
                f,
                "this room would become active room {target} of a set holding \
                 {rooms}; there is no such room, and by the time the setter could \
                 say so the outgoing world would already be gone"
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
            Self::NoRoomSetToPublishInto => write!(
                f,
                "this room staged a world and the session root it publishes into \
                 carries no `RoomSet`, so publishing would write the geometry and \
                 leave the active room where it was"
            ),
            Self::NoRoomGeometryToPublishInto => write!(
                f,
                "this room staged a world and the session root it publishes into \
                 carries no `RoomGeometry`, so publishing would seat the session \
                 in a room whose geometry is still the old one"
            ),
            Self::StaleJoinedRoom { joins } => write!(
                f,
                "this crossing was staged to join live room {joins}, which the \
                 session no longer has as the target room, so the body would \
                 arrive in a room that is not there"
            ),
            Self::StaleRoomInstance { replaces } => write!(
                f,
                "this room was staged to replace live room {replaces}, which the \
                 session no longer has, so its occupants would belong to a room \
                 the session is not in"
            ),
            Self::DefinitionAlreadyLive { live } => write!(
                f,
                "this publication builds a second live room of the room that \
                 live room {live} already is, and the durable rows name a place \
                 by its room id, so the two would share them"
            ),
            Self::LiveRoomNotInNextSet { live, room } => write!(
                f,
                "this publication replaces the room set with one that has no \
                 room `{room}`, and live room {live} is that room and stays live, \
                 so it would be left in a room that is not in the set"
            ),
            Self::StaleMint { mints, next } => write!(
                f,
                "this room was staged to mint live room {mints}, but the session                  mints {next} next: another publication took it, so its occupants                  would share another live room's identity"
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
pub(crate) fn verify_staged_world(
    world: &World,
    pending: &PendingWorldReplacement,
    // ⛔⛤ **THE ROOT THIS TRANSACTION IS PUBLISHING INTO, RESOLVED BY ITS
    // CALLER — 2026-09-15.** This asked `session_world_entity`, which answers
    // *"which root is LIVE right now"*, and refused any staged replacement into a
    // session that is itself still a candidate. `verify_and_publish` already
    // resolves the right root by the transaction's own session scope, so the
    // answer is handed in rather than asked for a second time — and the two ends
    // cannot disagree about which session this publication belongs to.
    publishing_into: Option<bevy::ecs::entity::Entity>,
) -> Result<(), Vec<StagedWorldViolation>> {
    use ambition_platformer2d_world::rooms::RoomSet;

    let mut violations = Vec::new();
    // The set that would be live: the staged replacement's, or the one already
    // on the session root when this transaction replaces only the active room.
    let staged_rooms = pending.next_rooms.as_ref();
    let live_rooms = publishing_into.and_then(|root| world.get::<RoomSet>(root));
    let rooms = match (staged_rooms, live_rooms) {
        (Some(next), _) => Some(&next.rooms),
        (None, Some(live)) => Some(&live.rooms),
        (None, None) => None,
    };
    // ⛔ ASKED SEPARATELY FROM THE SET, because a replacement that BRINGS its own
    // room set still needs a root to put it on.
    if publishing_into.is_none() {
        violations.push(StagedWorldViolation::NoSessionRootToPublishInto);
    }
    // The moving platforms need no sink check: they are inserted on the live
    // room root, which the geometry check below requires.
    // ⛔⛤ **THE SINKS ARE ASKED ABOUT ON THE EXACT TARGET — 2026-09-15 REVIEW,
    // FINDING 5.** This used to accept a staged `next_rooms` as evidence that a
    // room set existed, which answers a question about the VALUE rather than
    // about the place it is going. `apply_world_replacement` then wrote through
    // `if let Some(..)` and published a success verdict having skipped whatever
    // was missing. A publication may not validate a value without proving the
    // exact authoritative sink for it exists.
    match publishing_into {
        Some(root) => {
            match world.get::<RoomSet>(root) {
                None => violations.push(StagedWorldViolation::NoRoomSetToPublishInto),
                Some(rooms) => {
                    // A publication that brings a set mints against the counter
                    // that the application holds after it takes the set
                    // (`inherit_live_room_counter`, the larger of the two). A
                    // later room of a held sequence is checked against a set
                    // whose counter has moved past each mint before it.
                    let next = match staged_rooms {
                        Some(staged) => rooms.next_live_room().max(staged.next_live_room()),
                        None => rooms.next_live_room(),
                    };
                    if let Some(mints) = pending.succession.and_then(LiveRoomSuccession::mints) {
                        if mints != next {
                            violations.push(StagedWorldViolation::StaleMint { mints, next });
                        }
                    }
                }
            }
            // One live room per room: see `DefinitionAlreadyLive`.
            let built = match pending.succession {
                Some(LiveRoomSuccession::Open { .. }) => Some(None),
                Some(LiveRoomSuccession::Replace { replaces, .. }) => Some(Some(replaces)),
                _ => None,
            };
            let target = world.get::<RoomSet>(root).and_then(|rooms| target_in_live_set(rooms, pending));
            if let (Some(replaces), Some(target), Some(scope)) = (built, target, scope_of_root(world, root)) {
                // A room this publication retires beside the replaced one does
                // not stay live, so it does not hold the definition.
                if let Some(live) = live_room_of_definition(world, scope, target, replaces)
                    .filter(|live| !pending.retires_beside.contains(live))
                {
                    violations.push(StagedWorldViolation::DefinitionAlreadyLive { live });
                }
            }
            // A live room that this publication keeps must have its room in
            // the set the publication brings: see `LiveRoomNotInNextSet`.
            if let (Some(next), Some(rooms), Some(scope)) =
                (pending.next_rooms.as_ref(), world.get::<RoomSet>(root), scope_of_root(world, root))
            {
                let replaced = match pending.succession {
                    Some(LiveRoomSuccession::Replace { replaces, .. }) => Some(replaces),
                    _ => None,
                };
                for (_, live, definition) in live_rooms_of_scope(world, scope) {
                    if Some(live) == replaced || pending.retires_beside.contains(&live) {
                        continue;
                    }
                    let room = &rooms.spec(definition).id;
                    if next.definition_by_id(room).is_none() {
                        violations.push(StagedWorldViolation::LiveRoomNotInNextSet { live, room: room.clone() });
                    }
                }
            }
            // The geometry's sink is the root of the live room this replaces,
            // not the session root. A publication that opens a room spawns
            // that room's root, so its sink is made at application.
            match (pending.succession, scope_of_root(world, root)) {
                (Some(LiveRoomSuccession::Open { .. }), Some(_)) => {}
                // A joined room is the sink of nothing, but it must still be
                // the live room of the target room that the crossing enters.
                (Some(LiveRoomSuccession::Join { joins, .. }), Some(scope)) => {
                    let target = world
                        .get::<RoomSet>(root)
                        .and_then(|rooms| target_in_live_set(rooms, pending));
                    let joined = live_room_root_for(world, scope, joins).and_then(|room_root| {
                        world.get::<ambition_platformer2d_world::rooms::LiveRoomDefinition>(room_root)
                    });
                    if target.is_none() || joined.copied() != target {
                        violations.push(StagedWorldViolation::StaleJoinedRoom { joins });
                    }
                }
                (Some(LiveRoomSuccession::Replace { replaces, .. }), Some(scope)) => {
                    match live_room_root_for(world, scope, replaces) {
                        Some(room_root) => {
                            if world
                                .get::<ambition_platformer2d_core::RoomGeometry>(room_root)
                                .is_none()
                            {
                                violations.push(StagedWorldViolation::NoRoomGeometryToPublishInto);
                            }
                        }
                        None => violations.push(StagedWorldViolation::StaleRoomInstance { replaces }),
                    }
                }
                _ => violations.push(StagedWorldViolation::NoRoomGeometryToPublishInto),
            }
        }
        // `NoSessionRootToPublishInto` above already says this, and naming the
        // sinks of a root that does not exist would say it twice.
        None => {}
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
/// ⛔⛤ **THE TARGET IS HANDED IN — 2026-09-15 REVIEW, FINDING 5.** Every write
/// below used `session_world_component_mut`, which asks *"which root is LIVE
/// right now"* — a different question from *"which root did this publication
/// verify against"* the moment a hidden candidate session exists.
/// `verify_and_publish` resolves the target by the transaction's own scope and
/// `verify_staged_world` validates against THAT entity; re-asking here meant a
/// publication could be validated for hidden B and applied to live A.
///
/// ⚠ **AND A MISSING SINK IS NO LONGER SILENT.** These were `if let Some(..)`
/// branches that published a success verdict having skipped part of the
/// application. `verify_staged_world` now preflights every sink on this exact
/// entity, so reaching one of the error arms below means the world changed
/// between verification and application — an invariant violation, and it says so
/// rather than writing nothing.
pub(crate) fn apply_world_replacement(
    world: &mut World,
    mut pending: PendingWorldReplacement,
    target: Option<bevy::ecs::entity::Entity>,
) {
    // ⛔⛤ **REVALIDATE BEFORE THE FIRST DESTRUCTIVE ACT, NOT AFTER IT.** Every
    // arm below this block used to be discovered AFTER the outgoing roster had
    // already been despawned: a missing sink logged an error and carried on,
    // having destroyed the old world and installed none of the new one. That is
    // the same shape `install_rebased_sync_test_session` was corrected for on
    // 2026-09-17 and `maintain_local_session` on 2026-09-18 — *"everything that
    // can decline runs while the old session is still alive; the stop is the
    // first irreversible act and nothing after it may fail"*.
    //
    // ⚠ These are the same sinks `verify_staged_world` preflighted on this exact
    // entity, so reaching a refusal here still means the world changed between
    // verification and application. What changes is the COST of that: the
    // outgoing roster is still standing, so the session keeps the room it has
    // instead of keeping neither.
    //
    // ⭐ Read-only, and cheap: three `get`s against one entity, once per
    // publication.
    let refusal = match target {
        None => Some("no publication target — `verify_staged_world` refuses that with \
                      `NoSessionRootToPublishInto`".to_string()),
        Some(root) if world.get_entity(root).is_err() => {
            Some(format!("publication target {root:?} no longer exists"))
        }
        // ⛔ EITHER MARKER, BECAUSE A CANDIDATE PUBLISHES ITS FIRST ROOM BEFORE
        // IT IS ADOPTED. `Q132` gave a prepared candidate its own root identity
        // (`CandidateSessionRoot`), and this revalidation is about *"is the
        // target a session's root"*, not *"is it the LIVE one"* — the sink was
        // chosen by `session_root_for_scope`, which already answered that.
        Some(root)
            if world
                .get::<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>(root)
                .is_none()
                && world
                    .get::<ambition_platformer2d_shared_tangle::lifecycle::CandidateSessionRoot>(
                        root,
                    )
                    .is_none() =>
        {
            Some(format!(
                "publication target {root:?} carries neither `SessionRoot` nor \
                 `CandidateSessionRoot`"
            ))
        }
        Some(root)
            if world
                .get::<ambition_platformer2d_world::rooms::RoomSet>(root)
                .is_none() =>
        {
            Some(format!("publication target {root:?} carries no `RoomSet`"))
        }
        Some(root)
            if pending.succession.is_some_and(|succession| {
                world
                    .get::<ambition_platformer2d_world::rooms::RoomSet>(root)
                    .is_some_and(|rooms| {
                        succession.mints().is_some_and(|mints| rooms.next_live_room() != mints)
                    })
            }) =>
        {
            Some(format!(
                "publication target {root:?} mints another live room than {:?}, \
                 the one this room was staged for",
                pending.publishes_as()
            ))
        }
        Some(root)
            if pending.replaces().is_some_and(|replaces| {
                scope_of_root(world, root)
                    .and_then(|scope| live_room_root_for(world, scope, replaces))
                    .and_then(|room_root| world.get::<ambition_platformer2d_core::RoomGeometry>(room_root))
                    .is_none()
            }) =>
        {
            Some(format!(
                "publication target {root:?} has no root for live room {:?} carrying \
                 `RoomGeometry`",
                pending.replaces()
            ))
        }
        Some(_) => None,
    };
    if let Some(why) = refusal {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "staged world REFUSED at application, nothing was destroyed: {why}, though \
             its preflight found one. The session keeps the room it is standing in"
        );
        return;
    }

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
    let Some(root) = target else {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "a staged world reached application with no publication target.              `verify_staged_world` refuses that with `NoSessionRootToPublishInto`,              so the world-defining state has silently gone nowhere"
        );
        return;
    };
    use ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut_at;
    if let Some(mut next) = pending.next_rooms.take() {
        // Each live room's room, by id. A definition is an index into the
        // set, and the same room can have another index in the next one, so
        // the index is carried across the replacement by the id.
        let carried: Vec<(bevy::ecs::entity::Entity, String)> =
            match (world.get::<ambition_platformer2d_world::rooms::RoomSet>(root), scope_of_root(world, root)) {
                (Some(rooms), Some(scope)) => live_rooms_of_scope(world, scope)
                    .into_iter()
                    .map(|(entity, _, definition)| (entity, rooms.spec(definition).id.clone()))
                    .collect(),
                _ => Vec::new(),
            };
        match session_world_component_mut_at::<ambition_platformer2d_world::rooms::RoomSet>(world, root) {
            Some(mut rooms) => {
                next.inherit_live_room_counter(&rooms);
                *rooms = next;
            }
            None => bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "publication target {root:?} carries no `RoomSet` at application,                  though its preflight found one"
            ),
        }
        let remapped: Vec<_> = match world.get::<ambition_platformer2d_world::rooms::RoomSet>(root) {
            // The verifier refused a set that has no room for a live room
            // this publication keeps. The room it replaces gets its
            // definition below, and a room it retires goes.
            Some(rooms) => carried
                .into_iter()
                .filter_map(|(entity, room)| Some((entity, rooms.definition_by_id(&room)?)))
                .collect(),
            None => Vec::new(),
        };
        for (entity, definition) in remapped {
            if let Some(mut live) = world.get_mut::<ambition_platformer2d_world::rooms::LiveRoomDefinition>(entity) {
                if *live != definition {
                    *live = definition;
                }
            }
        }
    }
    // Which definition the published room is, resolved against the set the
    // session holds now (a hot reload replaced it above). It is written onto
    // the root this publication replaces, beside that root's geometry, and not
    // onto the session: two live rooms can be two rooms (OW1 cut 5e).
    let definition = match session_world_component_mut_at::<ambition_platformer2d_world::rooms::RoomSet>(world, root) {
        Some(rooms) => {
            // ⛔⛤ **A HARD FAILURE, BECAUSE THE PUBLICATION IS ALREADY
            // DESTRUCTIVE BY THE TIME WE ARE HERE.** The staged verifier refuses
            // `TargetRoomOutOfRange` before anything is torn down, so reaching
            // this arm means the verifier and the set disagree about the same
            // set. Logging and carrying on would leave the live room seated in
            // the OLD room while the geometry, platforms and staged population
            // below become the NEW one -- the exact silent split the violation
            // exists to prevent, only now with the old world already gone.
            let rooms_len = rooms.rooms.len();
            let definition = rooms.definition(pending.target_index);
            assert!(
                definition.is_some(),
                "published room {} of a set holding {rooms_len}: the staged \
                 verifier passed a target the room set refuses, so the live room \
                 would keep its old definition under the new geometry",
                pending.target_index,
            );
            definition
        }
        None => {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "publication target {root:?} carries no `RoomSet` at application, \
                 so the live room keeps the definition it had"
            );
            None
        }
    };
    // ⭐ **THE LIVE ROOM GETS AN IDENTITY THE ROOM DEFINITION DOES NOT HAVE.**
    // One line above, the session was seated in a room DEFINITION by index — the
    // same index every time it stands there. This mints the instance: leaving
    // `blink_run` and coming back is two live rooms, and nothing else in this
    // world can tell them apart. See `LiveRoomInstance`; it is OW1's
    // precondition and it is minted HERE because this is the one road that
    // seats a session in a published room.
    //
    // The instance lives on the root of the live room this publication
    // replaces (OW1 cut 3). The verifier refused a publication whose room root
    // is gone, so the advance mints exactly the value the staged occupants
    // carry.
    // The minted identity is the session's next one, pinned at staging and
    // checked by the verifier (`StaleMint`); the counter moves past it here.
    // A crossing that joins a live room mints nothing.
    let minted = pending.succession.filter(|succession| match succession.mints() {
        Some(mints) => {
            session_world_component_mut_at::<ambition_platformer2d_world::rooms::RoomSet>(world, root)
                .is_some_and(|mut rooms| rooms.mint_live_room(mints))
        }
        None => true,
    });
    // The minted room's root: the replaced room's, re-seated, or for a room
    // this publication opens, a new root beside the rooms that stay live.
    let scope = scope_of_root(world, root);
    let room_root = match minted {
        Some(LiveRoomSuccession::Open { mints, .. }) => scope.map(|scope| {
            world
                .spawn(ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope))
                .insert((mints, ambition_platformer2d_core::RoomGeometry(pending.geometry.clone())))
                .id()
        }),
        Some(LiveRoomSuccession::Join { joins, .. }) => {
            scope.and_then(|scope| live_room_root_for(world, scope, joins))
        }
        _ => pending
            .replaces()
            .zip(scope)
            .and_then(|(replaces, scope)| live_room_root_for(world, scope, replaces)),
    };
    let replaced = match minted {
        Some(LiveRoomSuccession::Open { leaves, mints }) => room_root.map(|_| (leaves, mints)),
        Some(LiveRoomSuccession::Join { leaves, joins, .. }) => room_root.map(|_| (leaves, joins)),
        _ => room_root.zip(minted).and_then(|(room_root, succession)| {
            let mut live_room =
                world.get_mut::<ambition_platformer2d_world::rooms::LiveRoomInstance>(room_root)?;
            let replaced = *live_room;
            *live_room = succession.enters();
            Some((replaced, *live_room))
        }),
    };
    // What moves with the crossing out of a room that STAYS LIVE: the
    // crossing body and its custody closure (what it holds, rides or wears).
    // The rest of the room it leaves stays there.
    let moving = minted.is_some_and(LiveRoomSuccession::keeps_left).then(|| {
        use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
        let custody: Vec<(bevy::ecs::entity::Entity, bevy::ecs::entity::Entity)> = world
            .query::<(bevy::ecs::entity::Entity, &InCustodyOf)>()
            .iter(world)
            .map(|(entity, custody)| (entity, custody.custodian))
            .collect();
        ambition_platformer2d_shared_tangle::lifecycle::custody_closure(
            pending.arrival.as_ref().map(|arrival| arrival.subject),
            &custody,
        )
    });
    // ⭐ WHAT THE SWEEP LEFT STANDING IN THE REPLACED ROOM IS NOW IN THE NEW
    // ONE. The outgoing roster is gone; what still carries the replaced
    // instance is what crosses with the publication: the carried body, what it
    // holds, and a body a hot reload re-seats. One rule, so no crossing road
    // has to remember to move its body.
    //
    // ⚠ ONLY THIS SESSION'S. The ordinal counts one session's publications, so a
    // candidate session prepared beside this one has its own #0.
    if let Some((replaced, minted)) = replaced {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, SessionRoot, SessionScopedEntity,
        };
        let session = world.get::<SessionRoot>(root).map(|root| root.0);
        let mut residents = world.query::<(
            bevy::ecs::entity::Entity,
            &mut InRoomInstance,
            Option<&SessionScopedEntity>,
        )>();
        for (entity, mut room, owner) in residents.iter_mut(world) {
            let ours = owner.map(|owner| owner.0) == session || owner.is_none();
            let moves = moving.as_ref().is_none_or(|moving| moving.contains(&entity));
            if ours && room.0 == replaced && moves {
                room.0 = minted;
            }
        }
    }
    // A whole-session restart retires the other live rooms, root and all.
    // Their residents were retired with the outgoing roster.
    for retired in &pending.retires_beside {
        if let Some(retired_root) = scope.and_then(|scope| live_room_root_for(world, scope, *retired)) {
            world.despawn(retired_root);
        }
    }
    // A room the crossing joins is the other player's live world: its
    // definition, geometry, platforms and collision are not written. A room
    // it leaves and retires goes, root and all.
    if let Some(LiveRoomSuccession::Join { leaves, retires, .. }) = minted {
        if retires {
            if let Some(left_root) = scope.and_then(|scope| live_room_root_for(world, scope, leaves)) {
                world.despawn(left_root);
            }
        }
        if let Some(arrival) = pending.arrival {
            apply_staged_arrival(world, arrival);
        }
        return;
    }
    if let Some((room_root, definition)) = room_root.zip(definition) {
        world.entity_mut(room_root).insert(definition);
    }
    match room_root.and_then(|room_root| world.get_mut::<ambition_platformer2d_core::RoomGeometry>(room_root)) {
        Some(mut geometry) => geometry.0 = pending.geometry,
        None => bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "publication target {root:?} has no live room root carrying `RoomGeometry` \
             at application, so the published room set names a world whose geometry \
             is the old one"
        ),
    }
    // The published room's platforms, on the same root as its geometry, and
    // the collision contributions of the room just left retracted there: the
    // contributors rebuild them for this room on the next tick, and until then
    // `CollisionWorld` must not compose this room with the old room's walls.
    if let Some(room_root) = room_root {
        let mut root = world.entity_mut(room_root);
        root.insert(ambition_platformer2d_world::collision::MovingPlatformSet(
            pending.moving_platforms,
        ));
        match root.get_mut::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>() {
            Some(mut overlay) => overlay.retract_for_room_change(),
            None => {
                root.insert(ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay::default());
            }
        }
    }
    // ⛔ LAST, AND AFTER THE GEOMETRY. The arrival was validated against the
    // plan's world by the caller, and the body is placed into a world that is
    // already the one it was validated against.
    if let Some(arrival) = pending.arrival {
        apply_staged_arrival(world, arrival);
    }
}

/// The session scope a publication target root belongs to, candidate or
/// live.
/// A live room of `scope` that instantiates `definition`, other than
/// `except`.
fn live_room_of_definition(
    world: &World,
    scope: ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId,
    definition: ambition_platformer2d_world::rooms::LiveRoomDefinition,
    except: Option<ambition_platformer2d_world::rooms::LiveRoomInstance>,
) -> Option<ambition_platformer2d_world::rooms::LiveRoomInstance> {
    use ambition_platformer2d_shared_tangle::lifecycle::{RoomInstanceRoot, SessionScopedEntity};
    let mut roots = world.try_query_filtered::<(
        &ambition_platformer2d_world::rooms::LiveRoomInstance,
        &ambition_platformer2d_world::rooms::LiveRoomDefinition,
        Option<&SessionScopedEntity>,
    ), bevy::prelude::With<RoomInstanceRoot>>()?;
    let mut live: Vec<_> = roots
        .iter(world)
        .filter(|(instance, of, owner)| {
            **of == definition && Some(**instance) != except && owner.is_none_or(|owner| owner.0 == scope)
        })
        .map(|(instance, ..)| *instance)
        .collect();
    live.sort();
    live.into_iter().next()
}

/// The publication's target in the set the session holds now, which is the
/// set that each live room's definition indexes. A publication that brings
/// its own set names the target in that set, and the same room has the same
/// id in both.
fn target_in_live_set(
    rooms: &ambition_platformer2d_world::rooms::RoomSet,
    pending: &PendingWorldReplacement,
) -> Option<ambition_platformer2d_world::rooms::LiveRoomDefinition> {
    match &pending.next_rooms {
        Some(next) => rooms.definition_by_id(&next.rooms.get(pending.target_index)?.id),
        None => rooms.definition(pending.target_index),
    }
}

/// Each live room root of `scope`, in instance order.
fn live_rooms_of_scope(
    world: &World,
    scope: ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId,
) -> Vec<(
    bevy::ecs::entity::Entity,
    ambition_platformer2d_world::rooms::LiveRoomInstance,
    ambition_platformer2d_world::rooms::LiveRoomDefinition,
)> {
    use ambition_platformer2d_shared_tangle::lifecycle::{RoomInstanceRoot, SessionScopedEntity};
    let Some(mut roots) = world.try_query_filtered::<(
        bevy::ecs::entity::Entity,
        &ambition_platformer2d_world::rooms::LiveRoomInstance,
        &ambition_platformer2d_world::rooms::LiveRoomDefinition,
        Option<&SessionScopedEntity>,
    ), bevy::prelude::With<RoomInstanceRoot>>() else {
        return Vec::new();
    };
    let mut live: Vec<_> = roots
        .iter(world)
        .filter(|(.., owner)| owner.is_none_or(|owner| owner.0 == scope))
        .map(|(entity, instance, definition, _)| (entity, *instance, *definition))
        .collect();
    live.sort_by_key(|(_, instance, _)| *instance);
    live
}

fn scope_of_root(
    world: &World,
    root: bevy::ecs::entity::Entity,
) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
    use ambition_platformer2d_shared_tangle::lifecycle::{CandidateSessionRoot, SessionRoot};
    world
        .get::<SessionRoot>(root)
        .map(|root| root.0)
        .or_else(|| world.get::<CandidateSessionRoot>(root).map(|root| root.0))
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
    /// distinction A10 is made of. Empty when the transaction staged no
    /// candidate world, because then there is nothing to project.
    pub projection_violations: Vec<ProjectionViolation>,
    /// Every reason the staged non-entity world — which room becomes active, and
    /// what geometry it collides against — would be incoherent. See
    /// [`StagedWorldViolation`].
    pub staged_violations: Vec<StagedWorldViolation>,
    /// Whether `RoomLoaded` was written.
    pub published: bool,
    /// How many declared supersessions this publication left standing for
    /// another authority to remove — `DepartureAuthority::Custodian`, A10's one
    /// permitted duplicate.
    ///
    /// ⛔ **RECORDED BECAUSE THE WINDOW IS OTHERWISE UNASSERTABLE.** It used to
    /// reach only the world log, so a test could not state the PREMISE *"this
    /// publication actually opened a custody window"* — and a custody-window test
    /// that never opens one passes while measuring nothing. `0` on a refusal:
    /// nothing was superseded because nothing published.
    pub left_to_custodian: usize,
    /// How many SUPERSESSIONS this transaction declared — a live body of the same
    /// identity standing beside the candidate that will replace it.
    ///
    /// ⛔⛤ **IT IS THE PREMISE OF A10's SUPERSESSION CLAIM.** *"Validation
    /// succeeds without first destroying A"* is provable from this plus
    /// `published`: `ProjectionViolation::SupersededNotLive` exists precisely to
    /// refuse a declared supersession whose live half is already gone, so a
    /// declared supersession that PUBLISHED is one whose predecessor was still
    /// standing when the projected world was verified. Without a count, a test
    /// asserting that property cannot say a supersession happened at all.
    pub supersessions: usize,
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
/// authored `SimId`. ⚠ **AND NO RULING IS COMING FOR THE HELD ONE — `Q124` WAS
/// WITHDRAWN HOURS AFTER THIS PARAGRAPH WAS WRITTEN, 2026-09-13.** The rule is
/// already shipped and it is TEMPORAL, not item-kind: the reset restores what
/// the checkpoint saw, per object. What this decides is the smaller and real
/// thing — that the verifier can say which of the two bodies is unexpected.
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
    session: SessionSpawnScope,
) {
    let planned = plan.planned_sim_ids();
    commands.queue(move |world: &mut World| {
        // ⛔ ASKED BEFORE THE BASELINE, because a world that cannot hide a
        // candidate must refuse the room rather than build one it will then
        // validate in plain sight. See `OpenRefused`.
        let captured = if !ambition_platformer2d_shared_tangle::construction::inactive_candidate_filter_installed(
            world,
        ) {
            Err(OpenRefused::CandidateFilterNotInstalled)
        } else {
            // ⛔ THIS SESSION'S WORLD, NOT THE PROCESS'S. See
            // `TransactionBaseline::capture_for_session`: a whole-world capture
            // puts ANOTHER session's bodies in this room's baseline, and a
            // planned identity found there is declared SUPERSEDED — so
            // publication despawns a live session's world.
            // ⛔ AND THESE LIVE ROOMS, NOT THE SESSION'S. Another live room
            // holds the same authored identities; see `TransactionRooms`.
            let rooms = world
                .get::<PendingWorldReplacement>(publication.0)
                .and_then(|pending| pending.succession)
                .map_or(
                    ambition_platformer2d_shared_tangle::lifecycle::TransactionRooms::EVERY,
                    LiveRoomSuccession::transaction_rooms,
                );
            TransactionBaseline::capture_for_session(world, session, rooms)
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
                    // ⭐ That is the SHIPPED custody rule implemented rather
                    // than averaged — `Q124` asked for a blanket one and was
                    // withdrawn because this already existed. The checkpoint
                    // baseline decides PER ITEM, and
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
                        .partition(|sim_id| baseline.contains(sim_id));
                    // ⛔⛤ **THE DEPARTURE AUTHORITY IS DECLARED HERE, FROM THE
                    // BASELINE, NOT DISCOVERED AT RETIREMENT.** A predecessor in
                    // another entity's CUSTODY is removed by its custodian —
                    // `restore_custody_to_checkpoint` unequips AND despawns it as
                    // one operation keyed on that entity, so reaching in first
                    // destroys the key the other half is found by. Publication
                    // used to promise it gone and then skip it because of a
                    // component it noticed; now the promise matches what it does.
                    // ⛔⛤ **MODEL A, LANDED 2026-09-15: PUBLICATION DESPAWNS A
                    // PREDECESSOR IN CUSTODY AND TELLS THE CUSTODIAN WHAT IT
                    // NEEDED FROM IT.** The two facts `restore_custody_to_checkpoint`
                    // required from the live entity — WHOSE hand, and WHICH item,
                    // so it strips the right one — are knowable right here, while
                    // the predecessor is still standing. Recorded, this row
                    // departs under `Publication` like any other.
                    //
                    // ⚠ **AND `Custodian` SURVIVES AS AN HONEST FALLBACK.** A
                    // hand this world cannot resolve is one the drain could not
                    // strip either, so a row whose handoff will not record keeps
                    // Model B rather than being despawned into a stale hand.
                    let in_custody: Vec<_> = superseding
                        .iter()
                        .filter_map(|sim_id| {
                            let entry = baseline.entries().get(sim_id)?;
                            world
                                .get::<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>(
                                    entry.entity,
                                )
                                .map(|_| (sim_id.clone(), entry.entity))
                        })
                        .collect();
                    let mut handed_off = std::collections::BTreeSet::new();
                    let mut left_to_custodian = std::collections::BTreeSet::new();
                    for (sim_id, entity) in in_custody {
                        if crate::items::pickup::record_custody_handoff(
                            world,
                            publication,
                            &sim_id,
                            entity,
                        ) {
                            handed_off.insert(sim_id);
                        } else {
                            left_to_custodian.insert(sim_id);
                        }
                    }
                    let _ = &handed_off;
                    let mut effects = superseding.iter().fold(
                        PublicationEffects::new(),
                        |effects, sim_id| {
                            let departs = if left_to_custodian.contains(sim_id) {
                                ambition_platformer2d_shared_tangle::construction::DepartureAuthority::Custodian
                            } else {
                                ambition_platformer2d_shared_tangle::construction::DepartureAuthority::Publication
                            };
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
/// ⛔⛤ **NO `receipt` PARAMETER ANY MORE — 2026-09-15 AUDIT, FINDING 4.** The
/// receipt is written by the construction boundary onto the publication, and read
/// from there. A caller that could still hand one in could hand in a receipt for
/// a construction that never ran.
pub(crate) fn close(
    commands: &mut Commands,
    publication: PublicationHandle,
    plan: &crate::features::RoomFeatureConstructionPlan,
    session: SessionSpawnScope,
) {
    let plan = plan.clone();
    commands.queue(move |world: &mut World| {
        verify_and_publish(world, publication, &plan, session);
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
    /// spelling for "the session now runs under this epoch, prepared from this
    /// content".
    ///
    /// ⭐ BOTH HALVES OR NEITHER. The epoch does the staleness job and the
    /// content identity is the only term in the binding two peers can agree on;
    /// a caller that has one has the other, because `PreparedContent` exposes
    /// `epoch()` and `fingerprint()` side by side. Taking them as separate
    /// arguments is what stops a caller from stating the local half and
    /// forgetting the peer half.
    pub fn content(
        epoch: ambition_platformer2d_core::ContentEpoch,
        content: ambition_platformer2d_core::PeerContentIdentity,
    ) -> Self {
        Self(ambition_platformer2d_shared_tangle::construction::ContentBinding::content(
            epoch, content,
        ))
    }

    /// The LIVE generation for a road that rebuilds a room this session already
    /// defines, or `unbound` when the session publishes no binding at all.
    ///
    /// ⭐⭐ **IT EXISTS SO THE FALLBACK IS A SENTENCE INSTEAD OF A THIRD
    /// PARAMETER.** `ActorConstructionContext::for_live_room_construction` takes
    /// ONE binding, because taking two is how the door transition, the reset and
    /// the neighbour prefetch came to stamp their roots `content-unstated` while
    /// their commit boundaries were correct. Those three roads each hold an
    /// `Option<Res<ActiveContentBinding>>`, so each needs to say what it means
    /// by `None` — and saying it here once means three roads cannot spell it
    /// three ways.
    ///
    /// ⚠ `None` IS AN HONEST GAP, NOT A WAIVER, and this preserves exactly that:
    /// a headless fixture with no content authority has nothing to be stale
    /// against and nothing to name, so `content_unstated` is its true answer.
    /// What is no longer possible is a session that DOES publish a binding
    /// building roots that do not name it.
    pub fn live_or(
        active: Option<&Self>,
        unbound: ambition_platformer2d_shared_tangle::construction::ContentBinding,
    ) -> ambition_platformer2d_shared_tangle::construction::ContentBinding {
        active.map_or(unbound, |active| active.0)
    }
}

/// Consume a verified publication's frozen effects: make them the world's.
///
/// ⛔⛤ **THE OTHER HALF OF `FrozenPublicationEffects` (`pub(crate)`) — 2026-09-15 REVIEW,
/// FINDING 1.** An ordinary live-room transition calls this on the spot, inside
/// the same verdict, so nothing about that road changes. A room built into a
/// candidate session that is still hidden leaves the bundle standing, and the
/// SESSION calls this at its own publication boundary — after
/// `publish_candidate_session` has made the population authoritative, so
/// `RoomLoaded` means what its name says: the room became authoritative to its
/// owning LIVE session.
///
/// ⛔ **THERE IS NO SECOND VERDICT HERE, DELIBERATELY.** Everything below was
/// already validated against the projected world at the inner publication; this
/// is consumption of validated state, not another chance to say no. A candidate
/// session that is DISCARDED instead despawns the publication entity
/// (`retire_publication`), which drops the bundle — and since nothing in it had
/// run, there is nothing to undo.
///
/// Returns how many declared departures were left to their custodian.
/// How many publications are holding verified effects they have not been
/// allowed to apply yet.
///
/// ⛔⛤ **THE PREMISE A NESTED-EFFECTS WITNESS NEEDS.** *"No `RoomLoaded`
/// escaped while B was pending"* is worth nothing unless B's first room actually
/// REACHED its verdict inside that window — an arm that measures a candidate
/// which had not published yet cannot tell the fix from the defect. This counts
/// exactly the state the fix creates, so a witness can assert its own subject
/// exists before asserting what did not happen.
pub fn publications_holding_frozen_effects(world: &mut World) -> usize {
    world
        .query::<&FrozenPublicationEffects>()
        .iter(world)
        .count()
}

pub fn finalize_room_publication(world: &mut World, publication: PublicationHandle) -> usize {
    let Some(frozen) = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<FrozenPublicationEffects>())
    else {
        // Nothing frozen: this publication was refused, already finalized, or
        // never verified. All three are ordinary.
        return 0;
    };
    let FrozenPublicationEffects {
        room_id,
        target,
        effects,
        baseline,
        admitted,
    } = frozen;
    // ⛔ TAKEN OFF THE PUBLICATION: adopting the staged world means the
    // publication no longer carries a replacement that has already been applied.
    // The publication itself survives until its verdict has been read — see
    // `begin_publication`.
    if let Some(pending) = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<PendingWorldReplacement>())
    {
        apply_world_replacement(world, pending, target);
    }
    let superseded =
        ambition_platformer2d_shared_tangle::construction::retire_superseded(
            world, &effects, &baseline,
        );
    // ⛔⛤ **THE DRAIN, AND IT RUNS ON EVERY FINALIZATION — WHICH IS THE WHOLE
    // DESIGN.** `retire_superseded` above has just despawned predecessors that
    // were in a hand (Model A), so those hands are holding a `HeldItem` naming a
    // dead entity until this line. The obvious home for this is inside
    // `restore_custody_to_checkpoint`, and it is WRONG: that runs only when the
    // commit carries a checkpoint operation, while an ordinary room transition
    // can declare a custody supersession too. An undrained ledger is strictly
    // worse than the Model B window it replaces — a permanently stale hand
    // rather than a duplicate that closes itself inside the frame.
    let stripped = crate::items::pickup::apply_custody_handoffs(world, publication);
    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
        "room-loaded {room_id} ({admitted} roots admitted, \
         {} declared departures retired, {} left to their custodian, \
         {stripped} hands released)",
        superseded.retired, superseded.left_to_custodian
    ));
    world.write_message(ambition_platformer2d_world::rooms::RoomLoaded {
        room_id: room_id.clone(),
    });
    // ⚠ DIAGNOSTICS, AND ONLY WHEN THEY ARE STILL THIS ROOM'S. A deferred
    // finalization happens frames after the verdict, by which time another room
    // may own this last-writer-wins record. Correcting a row that is not ours
    // would make the diagnostic lie in a new way.
    if let Some(mut record) = world.get_resource_mut::<LastConstructionVerification>() {
        if record.room_id == room_id {
            record.left_to_custodian = superseded.left_to_custodian;
        }
    }
    superseded.left_to_custodian
}

/// Promote a checked room and finalize it, or leave its effects frozen when
/// it publishes into a candidate session. Returns the bodies left to their
/// custodian.
fn promote_and_finalize(
    world: &mut World,
    publication: PublicationHandle,
    room_id: &str,
    transactions: &[ambition_platformer2d_shared_tangle::construction::TransactionId],
    publishing_into: Option<bevy::ecs::entity::Entity>,
    effects: PublicationEffects,
    baseline: TransactionBaseline,
) -> usize {
    // ⛔⛔ THE RESTORE'S CONSEQUENCES, HERE AND NOWHERE EARLIER: the verdict
    // accepted the room, the candidates are still hidden, and nothing of
    // the old room is retired. Each consequence sees the world the replay
    // was admitted against (the subject goes back to the old spawn, then
    // arrives below), and none touches the room just built. A refused
    // room never reaches this line, so a cancelled restore changed nothing.
    let restore = world
        .get::<PendingWorldReplacement>(publication.0)
        .and_then(|pending| pending.restore);
    if let Some(key) = restore {
        crate::session::checkpoint::run_restore_consequences(world, key);
    }
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
    // ⛔⛤ **AND HERE THE ROOM STOPS DOING THINGS TO THE WORLD AND STATES
    // WHAT IT OWES IT — 2026-09-15 REVIEW, FINDING 1.** Everything that
    // follows the line above reaches OUTSIDE this room's own candidate
    // population: predecessors despawned, hands stripped, the world-defining
    // state replaced, and a `RoomLoaded` the live session's combat reads. A
    // room built into a candidate session that has not been admitted yet may
    // not do any of it. See [`FrozenPublicationEffects`].
    let deferred = publishing_into.is_some_and(|root| {
        ambition_platformer2d_shared_tangle::construction::entity_is_still_a_candidate(
            world, root,
        )
    });
    if let Ok(mut entity) = world.get_entity_mut(publication.0) {
        entity.insert(FrozenPublicationEffects {
            room_id: room_id.to_string(),
            target: publishing_into,
            effects,
            baseline,
            admitted,
        });
    }
    if deferred {
        // ⭐ THE ROOM IS REAL INSIDE ITS SESSION AND INVISIBLE OUTSIDE IT.
        // Said on the same channel as `room-loaded` so the two are readable
        // as the two halves of one lifecycle rather than a missing event.
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-materialized {room_id} ({admitted} roots admitted inside a \
             pending candidate session; its effects wait for the session's \
             publication)"
        ));
        0
    } else {
        finalize_room_publication(world, publication)
    }
}

fn verify_and_publish(
    world: &mut World,
    publication: PublicationHandle,
    plan: &crate::features::RoomFeatureConstructionPlan,
    session: SessionSpawnScope,
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
        record_verdict(world, publication, retention, published);
    };
    let refuse = |world: &mut World, room_id: String| {
        record(world, false);
        // ⛔⛤ **AN EARLY REFUSAL RETIRES WHAT THE ROAD ALREADY QUEUED —
        // 2026-09-15 REVIEW, FINDING 4.** `spawn_contents_for` queues
        // `open`, then the candidate construction commands, then `close`. A
        // refusal discovered inside `open` cannot unqueue the commands behind it,
        // so by the time this closure runs the candidate roots EXIST. This road
        // used to record a failed verdict and walk away, leaving them standing —
        // and in the `CandidateFilterNotInstalled` case they are not hidden at
        // all, because the whole refusal is that this world cannot hide them.
        //
        // ⚠ **THIS IS THE DEFENSIVE HALF, NOT THE FIX.** Cleaning up after
        // constructing a candidate cannot prove the candidate was never visible
        // when the very fault is that the hiding mechanism is absent. The fix for
        // that case is a composition that registers the filter, which
        // `ambition_platformer2d_runtime` does; this makes the refusal settle its
        // own state either way.
        let dropped: usize = transactions
            .iter()
            .map(|transaction| {
                ambition_platformer2d_shared_tangle::construction::retire_candidate(
                    world,
                    transaction,
                )
            })
            .sum();
        // Always said, also with nothing dropped: a refusal before anything was
        // built is still a refusal, and without this line it reached only
        // `bevy::log`, which a bare App does not print.
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-refused {room_id} (early refusal, {dropped} roots dropped)"
        ));
        // ⛔ A REFUSED PUBLICATION DESPAWNED NOTHING, so the hands its recorded
        // handoffs describe are still correct — draining them would strip an item
        // off a body for a world that was never built.
        crate::items::pickup::discard_custody_handoffs(world, publication);
        // ⛔⛤ **THE STAGED WORLD IS DROPPED HERE, EXPLICITLY.** This comment used
        // to say `retire_candidate` took it because the staged world was
        // "candidate-owned state stamped with this room's transaction". MEASURED
        // and FALSE: `begin_publication` spawns the publication entity carrying
        // `RoomPublication` and no `TransactionId` component, so the candidate
        // sweep cannot reach it. A refusal on an `UntilOwnerRetires` publication
        // left the whole replacement standing -- outgoing roster, room set,
        // geometry, platform state -- on an entity that survives its verdict.
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            entity.remove::<PendingWorldReplacement>();
        }
        world.insert_resource(LastConstructionVerification {
            room_id,
            violations: Vec::new(),
            projection_violations: Vec::new(),
            staged_violations: Vec::new(),
            published: false,
            left_to_custodian: 0,
            // ⛔ NOTHING WAS DECLARED. A refusal on this road happens before the
            // opening baseline yielded its `PublicationEffects`, so there is no
            // supersession count to report — which is a different statement from
            // the one this comment used to make, that *"the transactions are not
            // known"*. They are: `transactions` is read off the publication at
            // the top of this function, which is what lets the refusal above
            // retire them.
            supersessions: 0,
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

    // ⛔⛤ **WHAT WAS BUILT IS READ OFF THE PUBLICATION — AUDIT FINDING 4.**
    // Absent means the construction boundary declined to build, which is the
    // outcome an opening refusal now produces. `refuse` above has already fired
    // for every road that gets here that way; this is the backstop for a
    // publication whose construction command never ran at all.
    let Some(PendingConstructionReceipt(receipt)) = world
        .get_entity_mut(publication.0)
        .ok()
        .and_then(|mut entity| entity.take::<PendingConstructionReceipt>())
    else {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` reached verification with an opened baseline and no \
             construction receipt, so nothing was built for it to verify"
        );
        refuse(world, room_id);
        return;
    };
    let receipt = &receipt;
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
        // A root that a room publishes into owes its binding in every
        // composition (C07): a direct host's session setup states one as a
        // shell session's does. Measured 2026-10-08 with a probe on this arm: the
        // SDK host tests verify 29 rooms in direct compositions and the shipped
        // app and the three demo suites 2,302 shell-routed ones, and each root
        // held a binding. Publishing here would admit a room whose staleness
        // nothing checked.
        None if shell_routed || publishing_into.is_some() => {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "room `{room_id}` cannot be verified: `ActiveContentBinding` is a \
                 canonical authority of the session root the room publishes into, \
                 and it is absent"
            );
            refuse(world, room_id);
            return;
        }
        // A direct-entry fixture with no session root states no binding and
        // means it: there is no generation for the room to be stale against.
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
    // 2026-09-14.** They were: `commit_deferred` writing `RoomSet`, <!-- cite-ok: lists the names A10 DELETED; a resolvable one here would mean the deletion did not happen -->
    // `RoomGeometry` and the platform state; `replace_live_world` retiring the
    // OUTGOING room first; the hot-reload caller advancing the session's content
    // generation afterwards; and a verifier with no way to validate N+1 while
    // intentionally RETAINING N. In order: `PendingWorldReplacement` stages all
    // of the first, the sweep moved inside `apply_world_replacement`,
    // `publication_succeeded` gates the third, and `superseding` plus
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
            let found = verify_staged_world(world, pending, publishing_into);
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
    let projection_violations = {
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
        let scope = AuthoritativeScope::gather_for_session(
            world,
            &transactions[0],
            session,
            baseline.rooms(),
        );
        let projection = project_post_publication_roster(&scope, &effects);
        verify_projected_roster(&projection, &effects, &baseline, &scope, world)
            .err()
            .unwrap_or_default()
    };
    for violation in &projection_violations {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` would not publish into a valid world: {violation}"
        );
    }

    let published =
        violations.is_empty() && projection_violations.is_empty() && staged_violations.is_empty();
    let mut left_to_custodian = 0;
    let supersessions = effects.supersessions().count();
    let held = world
        .get::<PendingWorldReplacement>(publication.0)
        .is_some_and(|pending| pending.held);
    if published && held {
        // The room passed, and the owner of its sequence decides. Nothing is
        // promoted and nothing outside the room changes; the effects wait on
        // the publication, as they wait for a candidate session.
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            entity.insert((
                FrozenPublicationEffects {
                    room_id: room_id.clone(),
                    target: publishing_into,
                    effects,
                    baseline,
                    admitted: 0,
                },
                HeldForOwner { supersessions },
            ));
        }
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-checked {room_id} (passed; held for the owner of its sequence, \
             nothing promoted)"
        ));
        return;
    }
    if published {
        left_to_custodian = promote_and_finalize(
            world,
            publication,
            &room_id,
            &transactions,
            publishing_into,
            effects,
            baseline,
        );
    } else {

        // ⛔ AND THE SAME ON THE LATE REFUSAL. Handoffs were recorded when this
        // transaction DECLARED its supersessions; nothing was despawned, so the
        // hands they describe are still holding the right things.
        crate::items::pickup::discard_custody_handoffs(world, publication);
        let failure_count =
            violations.len() + projection_violations.len() + staged_violations.len();
        // ⭐ THE LAST-GOOD-WORLD GUARANTEE, IN ONE STATEMENT: the room the
        // session is playing was never touched, so there is nothing to recover.
        //
        // ⛔⛤ **AND THE STAGED WORLD IS DROPPED HERE RATHER THAN LEFT TO THE
        // CANDIDATE SWEEP.** `retire_candidate` below takes everything stamped
        // with this room's transaction; the staged world is NOT among it. It is a
        // component on the publication entity, which `begin_publication` spawns
        // carrying `RoomPublication` and no `TransactionId`. MEASURED: a refused
        // candidate left the whole replacement standing -- outgoing roster, room
        // set, geometry, platform state -- on an entity that outlives its verdict
        // whenever the caller declared `UntilOwnerRetires`.
        let staged = staged_entity.is_some();
        if let Ok(mut entity) = world.get_entity_mut(publication.0) {
            entity.remove::<PendingWorldReplacement>();
        }
        let dropped: usize = transactions
            .iter()
            .map(|transaction| {
                ambition_platformer2d_shared_tangle::construction::retire_candidate(
                    world,
                    transaction,
                )
            })
            .sum();
        // ⛔ A REFUSAL IS A WORLD EVENT, NOT ONLY A LOG LINE. `bevy::log::error!`
        // alone surfaces NOTHING in a harness without a log plugin, so *"no
        // violations printed"* cannot be told from *"the transaction published"*.
        // A10 cannot be implemented against an invisible refusal — the
        // publication side has said `room-loaded` on this channel since it
        // existed, and the refusal must answer on the same one.
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
        left_to_custodian,
        supersessions,
    });
}
