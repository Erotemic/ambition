//! Room construction transaction boundary.
//!
//! One room load captures a baseline before construction and verifies the
//! completed room before publishing `RoomLoaded`; feature plans are participants,
//! not owners of that outer transaction. The same open/close bracket serves
//! deferred and exclusive-world execution. Verification can withhold publication
//! but cannot undo already-applied Bevy commands, so this is consistency checking,
//! not atomic rollback.

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
pub(crate) struct PendingConstructionBaseline(Result<TransactionBaseline, BaselineCaptureError>);

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
pub(crate) fn open(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let captured = TransactionBaseline::capture(world);
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

    let published = violations.is_empty();
    if published {
        ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
            "room-loaded {room_id}"
        ));
        world.write_message(ambition_platformer2d_world::rooms::RoomLoaded {
            room_id: room_id.clone(),
        });
    } else {
        let failure_count = violations.len();
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "room `{room_id}` was NOT published: {failure_count} construction violation(s). The \
             world has already been mutated and cannot be rolled back."
        );
    }
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations,
        published,
    });
}
