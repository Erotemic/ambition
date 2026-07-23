//! **The room construction transaction boundary.**
//!
//! One room load is one transaction. This module owns its two ends — the
//! baseline captured before anything is built, and the verification that decides
//! whether the room may announce itself — and nothing else may publish
//! [`RoomLoaded`].
//!
//! ## Why this is not in the feature plan
//!
//! It was, and the boundary was in the wrong place. `RoomFeatureConstructionPlan::spawn`
//! queued its own capture and its own verify-and-publish around the FEATURE work,
//! and then `RoomConstructionPlan::spawn_contents` — its caller — queued the
//! moving-platform bodies and the [`LastRoomConstructionCommit`] receipt AFTER
//! it returned. Command queues apply in insertion order, so the publication ran
//! strictly before the room was finished: `RoomLoaded` announced a room with no
//! platforms and no commit receipt, and verification inspected a world the
//! transaction had not stopped building. Every listener that reacted to
//! `RoomLoaded` by reading the room's platforms saw an empty set.
//!
//! A feature plan cannot own this boundary, because a feature plan is not the
//! transaction — it is one participant in it. The outer artifact that knows when
//! the room is COMPLETE is [`RoomConstructionPlan`](super::RoomConstructionPlan),
//! so the bracket lives with it: [`open`] first, every participant's work in
//! between, [`close`] last.
//!
//! ## What "last" buys, and what it does not
//!
//! Because the queue is ordered, [`close`] runs after feature construction,
//! planned roots, planned relationships, moving-platform entities, the
//! last-commit receipt, and — for every lifecycle path — after active room
//! selection, room geometry, moving-platform resource state, and carried-player
//! handling, all of which are applied before `spawn_contents` is called at all.
//! The same two closures serve the deferred path and the exclusive-world
//! `apply_to_world` path, so there is ONE publication route rather than two that
//! can drift.
//!
//! ⚠ **This is detection, not rollback, and withholding publication is not
//! rollback either.** By the time [`close`] runs, every construction command has
//! applied and Bevy commands cannot be undone. A failed verification means the
//! room never announces itself and the world keeps whatever the offending recipe
//! produced. That is better than publishing a room nobody can describe, and it
//! is not atomicity: real atomicity needs a staging world, and there isn't one.

use bevy::ecs::resource::Resource;
use bevy::prelude::{Commands, World};

use ambition_platformer_primitives::construction::{
    verify_committed_roster, AuthoritativeScope, BaselineCaptureError, ConstructionReceipt,
    RosterViolation, Severity, TransactionBaseline,
};
use ambition_platformer_primitives::lifecycle::SessionSpawnScope;

/// The baseline captured at the head of a construction transaction, waiting for
/// the verification pass at its tail.
///
/// A resource because the two ends are separate commands in one queue and
/// nothing else can carry a value between them. Removed by the verification
/// pass, so its presence means a transaction is open.
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
    /// Everything verification found, fatal and un-migrated alike.
    pub violations: Vec<RosterViolation>,
    /// Whether `RoomLoaded` was written.
    pub published: bool,
}

impl LastConstructionVerification {
    /// The findings that withheld publication, as opposed to the ones that only
    /// name a known un-migrated family.
    pub fn fatal(&self) -> impl Iterator<Item = &RosterViolation> {
        self.violations
            .iter()
            .filter(|violation| violation.severity() == Severity::Fatal)
    }
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
    plan: &crate::construction::ActorConstructionPlan,
    receipt: &ConstructionReceipt,
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
pub struct ActiveContentBinding(pub ambition_platformer_primitives::construction::ContentBinding);

impl ActiveContentBinding {
    /// The binding for one exact prepared-content generation — the app-side
    /// spelling for "the session now runs under this epoch".
    pub fn content(epoch: ambition_engine_core::ContentEpoch) -> Self {
        Self(ambition_platformer_primitives::construction::ContentBinding::Content(epoch))
    }
}

fn verify_and_publish(
    world: &mut World,
    plan: &crate::construction::ActorConstructionPlan,
    receipt: &ConstructionReceipt,
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
            // The world was already inconsistent before this transaction began.
            // Publishing a room on top of that would bury the earlier fault.
            bevy::log::error!(
                target: "ambition::construction",
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
                target: "ambition::construction",
                "room `{room_id}` reached verification without an opening baseline"
            );
            refuse(world, room_id);
            return;
        }
    };

    // The ownership token is keyed by content generation, room, AND session, so
    // it must be derived from the session that COMMITTED — the same one the
    // executor stamped its roots with.
    let transaction = plan.scope().transaction(session);
    let scope = AuthoritativeScope::gather(world, &transaction);
    let mut violations = verify_committed_roster(plan, receipt, &baseline, &scope, world)
        .err()
        .unwrap_or_default();
    // The actor-domain composition pass: exact rig equality per planned host.
    // The generic per-relation postconditions above prove each planned limb
    // landed; only the domain that owns `LimbRig` can prove nothing EXTRA did.
    violations.extend(crate::construction::verify_rig_composition(
        plan, receipt, world,
    ));
    // The commit-boundary staleness check (Phase 4g): a plan prepared against
    // one content generation must not publish a room into a session that has
    // moved to another. Enforced HERE because commit cannot yet be prevented
    // (no staging world) — a stale plan's room is refused publication.
    if let Some(live) = world.get_resource::<ActiveContentBinding>() {
        if plan.scope().binding != live.0 {
            violations.push(
                ambition_platformer_primitives::construction::RosterViolation::ContentBindingMismatch {
                    planned: plan.scope().binding,
                    live: live.0,
                },
            );
        }
    }
    violations.sort_by_key(|violation| format!("{violation:?}"));
    violations.dedup();

    let fatal = violations
        .iter()
        .filter(|violation| violation.severity() == Severity::Fatal)
        .count();
    for violation in &violations {
        match violation.severity() {
            Severity::Fatal => bevy::log::error!(
                target: "ambition::construction",
                "room `{room_id}` failed construction verification: {violation}"
            ),
            Severity::Unmigrated => bevy::log::debug!(
                target: "ambition::construction",
                "room `{room_id}`: {violation}"
            ),
        }
    }

    let published = fatal == 0;
    if published {
        world.write_message(crate::rooms::RoomLoaded {
            room_id: room_id.clone(),
        });
    } else {
        // Loud, and NOT a `RoomLoaded`. The world keeps whatever the offending
        // recipe produced — commands do not roll back — so the honest outcome is
        // a room that never announces itself rather than one that lies.
        bevy::log::error!(
            target: "ambition::construction",
            "room `{room_id}` was NOT published: {fatal} fatal construction violation(s). The \
             world has already been mutated and cannot be rolled back."
        );
    }
    world.insert_resource(LastConstructionVerification {
        room_id,
        violations,
        published,
    });
}
