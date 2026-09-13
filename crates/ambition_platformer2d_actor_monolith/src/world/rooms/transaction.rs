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
    BaselineCaptureError, RosterViolation, TransactionBaseline,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;

/// The baseline captured at the head of a construction transaction, waiting for
/// the verification pass at its tail.
///
/// A resource because the two ends are separate commands in one queue and nothing else can
/// carry a value between them.
#[derive(Resource)]
pub(crate) struct PendingConstructionBaseline(Result<TransactionBaseline, OpenRefused>);

/// ⛔⛤ **A10'S BRACKET FLAG, NAMED — IT USED TO BE A BARE `false` AT ONE CALL
/// SITE.** `true` builds every root in every lane as an `InactiveCandidate`, so
/// a room that fails verification is DROPPED rather than left standing
/// half-built. See `RoomConstructionPlan::spawn_contents` for the measured hold
/// (`Q124`) that keeps it off.
///
/// ⚠ It is a constant rather than a literal because it is read TWICE — the
/// bracket's opening check and the spawn itself — and two spellings of one
/// decision is how the two ends come to disagree about whether a room is a
/// candidate.
pub(crate) const ROOM_CANDIDATE_BRACKET: bool = false;

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
    plan: &crate::features::RoomFeatureConstructionPlan,
    candidate_bracket: bool,
) {
    let reconstructing = plan.planned_sim_ids();
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
                    baseline.reconstructing(reconstructing.iter().cloned())
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
) {
    let plan = plan.clone();
    let receipt = receipt.clone();
    commands.queue(move |world: &mut World| {
        verify_and_publish(world, &plan, &receipt, room_id, session);
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
) {
    let refuse = |world: &mut World, room_id: String| {
        world.insert_resource(LastConstructionVerification {
            room_id,
            violations: Vec::new(),
            published: false,
        });
    };

    let baseline = match world.remove_resource::<PendingConstructionBaseline>() {
        Some(PendingConstructionBaseline(Ok(baseline))) => baseline,
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
    if let Some(live) = world.get_resource::<ActiveContentBinding>() {
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
    // ⚠ **THIS IS NOT YET THE FULL LAST-GOOD-WORLD GUARANTEE, and the difference
    // is one ordering.** `replace_live_world` still retires the OUTGOING room
    // before this transaction opens, so a refusal leaves the session with NO
    // room rather than with its previous one. What it buys today is that a
    // refused room leaves no debris — no half-built scene standing beside a
    // world that never accepted it. Moving the retirement after this point is
    // the remaining step, and it lands in `replace_live_world` alone.
    let published = violations.is_empty();
    // ⛔ EVERY LANE'S TRANSACTION, not just the actor lane's — see
    // `construction_transactions`, and the measurement that corrected me.
    let transactions = plan.construction_transactions(session);
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
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-loaded {room_id} ({admitted} roots admitted)"
        ));
        world.write_message(ambition_platformer2d_world::rooms::RoomLoaded {
            room_id: room_id.clone(),
        });
    } else {
        let failure_count = violations.len();
        let dropped: usize = transactions
            .iter()
            .map(|transaction| {
                ambition_platformer2d_shared_tangle::construction::retire_candidate(
                    world,
                    transaction,
                )
            })
            .sum();
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` was NOT published: {failure_count} construction violation(s). \
             The candidate was never visible and its {dropped} roots are dropped."
        );
    }
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations,
        published,
    });
}
