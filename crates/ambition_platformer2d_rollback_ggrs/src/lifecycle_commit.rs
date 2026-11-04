//! Peer input-driven crossings and confirmed lifecycle barriers.
//!
//! Ordinary peer travel applies inside the simulation from that frame's input
//! verdicts and retained plans. Presentation receives a confirmed receipt.
//! Checkpoint restores, room rebuilds, and local sync-test operations retain
//! the confirmed host barrier and timeline rebase.

use bevy::prelude::*;

use ambition_platformer2d_session::lifecycle_commit::{
    LifecycleIntent, PendingIntent, PendingLifecycleCommit,
};
use ambition_platformer2d_publication::RoomConstructionPlan;
#[cfg(test)]
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::ConfirmedFrameBoundary;

use crate::session::{SyncTestOwner, SyncTestSettings};
use crate::{build_sync_test_session, install_rebased_sync_test_session, RollbackSessionOwnership};

/// `W`: the frames the simulation still runs after the frame that recorded a
/// lifecycle operation, under a peer session. [`freeze_frame`] is `C = R + W`.
///
/// One is the smallest value, because the recording frame has run when the
/// operation exists. A larger value gives no more agreement: the freeze makes
/// the state equal on each peer, and the rollback window does not.
pub const PEER_COMMIT_FREEZE_DELAY: i32 = 1;

/// `C`: the first frame that does not simulate while `intent` waits under a
/// peer session. The state saved at frame `C - 1` is the state of each later
/// frame, on each peer.
pub fn freeze_frame(intent: &PendingIntent) -> i32 {
    intent.frame.saturating_add(PEER_COMMIT_FREEZE_DELAY)
}

/// Hold peer simulation only for an operation that requires a timeline rebase.
/// Ordinary travel commits from frame inputs and does not enter this barrier.
pub fn a_peer_commit_holds_the_simulation(
    ownership: Option<Res<RollbackSessionOwnership>>,
    pending: Option<Res<PendingLifecycleCommit>>,
    frame: Option<Res<crate::RollbackFrameCount>>,
) -> bool {
    let (Some(ownership), Some(pending), Some(frame)) = (ownership, pending, frame) else {
        return false;
    };
    holds_the_simulation(*ownership, pending.peek(), frame.0)
}

fn holds_the_simulation(
    ownership: RollbackSessionOwnership,
    pending: Option<&PendingIntent>,
    frame: i32,
) -> bool {
    matches!(ownership, RollbackSessionOwnership::Peer)
        && pending.is_some_and(|intent| !ordinary_crossing(intent) && freeze_frame(intent) <= frame)
}

/// Who commits, which decides when the operation is confirmed and which
/// session the rebase installs. The operation itself is one body.
#[derive(Clone, Copy)]
enum Committer {
    /// A sync test: this host alone, when the recording frame is confirmed.
    Alone {
        settings: SyncTestSettings,
        owner: SyncTestOwner,
    },
    /// A peer session: each peer, on the frozen world, when its confirmed
    /// frame reaches the freeze frame.
    BehindThePeerBarrier,
}

/// The operation that may commit now, by the rule of `committer`.
fn operation_to_commit(
    world: &World,
    committer: Committer,
    boundary: ConfirmedFrameBoundary,
) -> Option<PendingIntent> {
    let pending = world.get_resource::<PendingLifecycleCommit>()?;
    match committer {
        Committer::Alone { .. } => pending.confirmed(boundary.confirmed).cloned(),
        Committer::BehindThePeerBarrier => {
            let intent = pending.peek().filter(|intent| !ordinary_crossing(intent))?;
            let frozen_from = freeze_frame(intent);
            // The confirmed frame says that no input can change the frozen
            // state: each input of a frame that simulated is in, and the
            // advance that published this boundary ran after the rollback
            // that applied them. The frame counter says that the live world
            // IS a frozen frame. A peer can know of inputs past its own frame.
            let world_is_frozen = world
                .get_resource::<crate::RollbackFrameCount>()
                .is_some_and(|frame| frozen_from <= frame.0);
            (frozen_from <= boundary.confirmed && world_is_frozen && each_peer_prepared(world, intent))
                .then(|| intent.clone())
        }
    }
}

/// Each handle of the peer session said, in a confirmed input, that its peer
/// prepared `intent`.
///
/// Measured 2026-10-04 (`two_peers`): without this, a peer whose plan was
/// authorized committed while the other peer could not prepare the room. It
/// started its next session alone, and the other peer stayed in the old one,
/// so neither session ran again.
///
/// A failed preparation does not commit, and it is reported. The operation
/// then ends on each peer ([`end_an_operation_a_peer_could_not_prepare`]).
fn each_peer_prepared(world: &World, intent: &PendingIntent) -> bool {
    let Some(verdicts) = world.get_resource::<crate::PeerVerdicts>() else {
        return false;
    };
    let Some(operation) = selected_preparation(world, intent).as_ref().map(crate::PeerOperation::from) else {
        return false;
    };
    let failed = verdicts.failed(operation);
    if !failed.is_empty() {
        bevy::log::error_once!(
            "the peer(s) of handle(s) {failed:?} could not prepare the lifecycle \
             operation recorded on frame {}, so no peer commits it (Q156)",
            intent.frame
        );
    }
    verdicts.prepared_plan(operation).is_some()
}

/// A PEER THAT COULD NOT PREPARE THE OPERATION ENDS IT ON EACH PEER.
///
/// Q156 is not ruled; this is the default in force until it is
/// (`awaiting-maintainer-decision.md`). A failed preparation ends the
/// operation, as a failed respawn ends on one machine, and a door that a
/// peer did not prepare does not open. Measured before: each peer stayed
/// frozen with no end.
///
/// When a handle's input of THIS frame says `Failed` for the operation that
/// waits, the operation leaves the slot on this frame. It reads the frame's
/// inputs, not the confirmed record: each peer reads the same input for the
/// frame (a prediction is corrected by a rollback), so each peer ends the
/// operation on the same frame, and the simulation runs again from it. A
/// confirmed record would arrive at a different frame on each peer, and the
/// simulation would start again on two different frames. What follows from
/// the slot is the same as for any intent that leaves it: the transaction is
/// cancelled, and a checkpoint restore publishes `Cancelled`.
/// A disconnected required handle also refuses the pending operation.
pub fn end_an_operation_a_peer_could_not_prepare(
    ownership: Option<Res<RollbackSessionOwnership>>,
    inputs: Option<Res<bevy_ggrs::PlayerInputs<crate::AmbitionGgrsConfig>>>,
    pending: Option<ResMut<PendingLifecycleCommit>>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    captured: Query<&ambition_platformer2d_session::room_preparation::CapturedRoomPreparation, With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>>,
) {
    use crate::peer_input::PreparationVerdict;
    if !matches!(ownership.as_deref(), Some(RollbackSessionOwnership::Peer)) {
        return;
    }
    let (Some(inputs), Some(mut pending)) = (inputs, pending) else {
        return;
    };
    let Some(operation) = pending.peek()
        .and_then(|intent| captured.single().ok()?.for_operation(intent))
        .map(crate::PeerOperation::from) else {
        return;
    };
    let failed: Vec<usize> = inputs
        .iter()
        .enumerate()
        .filter(|(_, (input, status))| {
            *status == bevy_ggrs::ggrs::InputStatus::Disconnected
                || (input.verdict.operation == operation && input.verdict.said == PreparationVerdict::Failed)
        })
        .map(|(handle, _)| handle)
        .collect();
    let recorded_on = operation.recorded_on;
    let now = boundary.map_or(recorded_on, |boundary| boundary.current);
    if !failed.is_empty() && pending.retract_recorded_on(recorded_on, now) {
        bevy::log::warn!(
            "the peer(s) of handle(s) {failed:?} could not prepare the lifecycle \
             operation recorded on frame {recorded_on}, so it ends on each peer (Q156)"
        );
    }
}

fn ordinary_crossing(operation: &PendingIntent) -> bool {
    operation.checkpoint_operation.is_none()
        && matches!(&operation.kind, LifecycleIntent::Transition(intent)
            if intent.disposition == ambition_platformer2d_session::lifecycle_commit::RoomTransitionDisposition::Travel)
}

#[derive(Message, Clone)]
pub(crate) struct PeerRoomCommit {
    applied: ambition_platformer2d_session::lifecycle_commit::AppliedIntent,
    duration: std::time::Duration,
}

/// Apply only the agreement carried by this simulation frame's inputs.
pub(crate) fn commit_peer_crossing(world: &mut World) {
    if !matches!(world.get_resource::<RollbackSessionOwnership>(), Some(RollbackSessionOwnership::Peer)) {
        return;
    }
    let Some(operation) = world.get_resource::<PendingLifecycleCommit>().and_then(|pending| pending.peek())
        .filter(|operation| ordinary_crossing(operation)).cloned() else { return };
    let Some(preparation) = selected_preparation(world, &operation) else { return };
    let peer_operation = crate::PeerOperation::from(&preparation);
    let Some(plan_identity) = world.get_resource::<bevy_ggrs::PlayerInputs<crate::AmbitionGgrsConfig>>()
        .and_then(|inputs| crate::peer_input::agreed_plan(peer_operation,
            inputs.iter().map(|(input, status)| (*status != bevy_ggrs::ggrs::InputStatus::Disconnected).then_some(input.verdict)))) else { return };
    let authorization = world.resource::<crate::room_promises::PromisedRoomPlans>()
        .get(&preparation).cloned().expect("a prepared input must retain its exact room plan through replay");
    assert_eq!(authorization.peer_identity(), Some(plan_identity), "the retained plan differs from the input agreement");
    assert!(authorization.matches_world(world), "the retained room promise no longer matches its admission");
    let frame = world.resource::<crate::RollbackFrameCount>().0;
    let started = bevy::platform::time::Instant::now();
    match commit_transition(world, &authorization.plan, None, &operation.kind) {
        CommitOutcome::Committed(arrived_in) => {
            let applied = world.resource_mut::<PendingLifecycleCommit>()
                .complete(frame, preparation.checksum(), arrived_in).expect("the applied admission is pending");
            world.write_message(PeerRoomCommit { applied, duration: started.elapsed() });
        }
        CommitOutcome::Cancelled => {
            world.resource_mut::<PendingLifecycleCommit>().retract_recorded_on(operation.frame, frame);
        }
        CommitOutcome::Retry => panic!("a retained and agreed room plan cannot require host preparation during simulation"),
    }
}

/// Retire presentation only after the crossing's frame is confirmed.
pub(crate) fn observe_peer_room_commits(world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<PeerRoomCommit>>,
) {
    let commits: Vec<_> = cursor.read(world.resource::<Messages<PeerRoomCommit>>()).cloned().collect();
    for commit in commits {
        let matches = world.get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
            .and_then(|state| state.active.as_ref()).is_some_and(|active|
                active.describes(&commit.applied.operation)
                && active.preparation.as_ref().is_some_and(|preparation| preparation.checksum() == commit.applied.preparation_checksum));
        if matches {
            world.resource_mut::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
                .active.as_mut().unwrap().arrived_in = commit.applied.arrived_in;
            retire_committed_room_transition(world, commit.duration);
        }
    }
}

/// Decide what this peer says, in its next input, of the operation it waits
/// on. In `ReadInputs`, before the local inputs are published.
///
/// `Prepared` is the same test the commit makes ([`authorized_plan`]), so a
/// peer says it only when it can commit.
pub fn decide_this_peers_verdict(world: &mut World) {
    use crate::peer_input::{PeerVerdict, PreparationVerdict};
    let peer = matches!(
        world.get_resource::<RollbackSessionOwnership>(),
        Some(RollbackSessionOwnership::Peer)
    );
    let pending = world
        .get_resource::<PendingLifecycleCommit>()
        .and_then(PendingLifecycleCommit::peek)
        .cloned();
    let verdict = match pending.filter(|_| peer) {
        None => PeerVerdict::default(),
        Some(intent) => {
            let Some(operation) = selected_preparation(world, &intent).as_ref().map(crate::PeerOperation::from) else {
                world.insert_resource(crate::ThisPeersVerdict::default());
                return;
            };
            let (plan, said) = match authorized_plan(world, &intent) {
                AuthorizedPlan::Ready(authorization) => match authorization.peer_identity() {
                    Some(plan) => {
                        let mismatches = world.get_resource::<crate::PeerVerdicts>()
                            .map(|verdicts| verdicts.mismatched_plans(operation, plan)).unwrap_or_default();
                        if !mismatches.is_empty() {
                            bevy::log::error_once!("peer room preparation differs for {operation:?}: handles {mismatches:?}, expected {plan:?}");
                        }
                        if mismatches.is_empty() {
                            world.resource_mut::<crate::room_promises::PromisedRoomPlans>().retain(authorization);
                        }
                        (plan, if mismatches.is_empty() { PreparationVerdict::Prepared } else { PreparationVerdict::Failed })
                    }
                    None => {
                        bevy::log::error_once!("a peer room plan has no stated content identity; refusing preparation");
                        (Default::default(), PreparationVerdict::Failed)
                    }
                },
                AuthorizedPlan::InvalidPromise => (Default::default(), PreparationVerdict::Failed),
                AuthorizedPlan::Wait => {
                    let failed = world.get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
                        .is_some_and(|state| state.failed_to_prepare(&intent));
                    (Default::default(), if failed { PreparationVerdict::Failed } else { PreparationVerdict::NotYet })
                }
            };
            PeerVerdict { operation, plan, said }
        }
    };
    world.insert_resource(crate::ThisPeersVerdict(verdict));
}

/// Execute a confirmed deferred lifecycle op in the exclusive world and rebase.
///
/// No-op unless (a) a rollback host is installed (`ConfirmedFrameBoundary`
/// present), (b) the session is a `LocalSyncTest` we may rebase or a `Peer`
/// session behind its barrier, and (c) a pending intent exists that is
/// confirmed by the rule of that session (see [`Committer`]).
pub fn commit_confirmed_lifecycle(world: &mut World) {
    let Some(boundary) = world.get_resource::<ConfirmedFrameBoundary>().copied() else {
        return;
    };
    let committer = match world.get_resource::<RollbackSessionOwnership>().copied() {
        Some(RollbackSessionOwnership::LocalSyncTest { settings, owner }) => {
            Committer::Alone { settings, owner }
        }
        Some(RollbackSessionOwnership::Peer) => Committer::BehindThePeerBarrier,
        Some(RollbackSessionOwnership::External) | None => return,
    };

    // ⛔⛔ **AFTER THE OWNERSHIP GATE AND BEFORE THE PENDING-INTENT GATE, and
    // both placements are load-bearing.**
    //
    // AFTER ownership, because a preparation failure is a HOST fact — this peer
    // could not build the room; another peer may have built it fine — and
    // spending the rollback-registered lifecycle slot on it is a lifecycle
    // DECISION. Only a host that owns that decision may make it. Placed above
    // this gate, as it first was, a P2P peer would unilaterally cancel
    // deterministic checkpoint state on a local asset failure.
    //
    // BEFORE the pending-intent gate, because a checkpoint operation whose room
    // preparation failed still holds a pending intent — that is the wedge this
    // ends. Below the gate it would reach `authorized_plan`, get `Wait` against
    // the failed transaction, and return without ever answering.
    //
    // `boundary.confirmed` is the whole authorization: an operation admitted on a
    // frame a rewind can still revisit is not this call's to end.
    //
    // ⛔ NOT UNDER A PEER SESSION, for the reason above: the failure is a fact
    // of this peer, and the slot is state each peer must hold the same. So a
    // checkpoint restore whose preparation fails on a peer holds the freeze
    // with no end. That is a named remainder of the peer barrier, not a
    // decision that it is right.
    if matches!(committer, Committer::Alone { .. }) {
        let _ = ambition_platformer2d_session::checkpoint::terminalize_abandoned_checkpoint_restore(
            world,
            Some(boundary.confirmed),
        );
    }

    let Some(operation) = operation_to_commit(world, committer, boundary) else {
        return;
    };

    // Never rebase over an unhealthy session. Log once because this veto holds
    // confirmed room transitions while the pending intent remains queued.
    if let Err(error) = crate::session_health(world) {
        bevy::log::error_once!(
            "a confirmed lifecycle commit is being HELD because the rollback \
             session is unhealthy ({error:?}) — until this clears, doors and \
             loading zones will detect and never move. The room is not the \
             problem; the desync is."
        );
        return;
    }

    // A confirmed room transition still waits for the readiness transaction.
    // Commit the exact construction plan that transaction authorized; re-preparing
    // here could build against a different content epoch than the assets checked.
    // ⭐ BOTH SHAPES NEED AN AUTHORIZED PLAN, and for the same reason: each ends
    // with a room built from its authored spec, and re-preparing here could
    // build against a different content epoch than the transaction checked. A
    // rebuild with nobody in it is not a cheaper operation, only a bodyless one.
    let authorized = match authorized_plan(world, &operation) {
        AuthorizedPlan::Ready(authorization) => {
            if matches!(committer, Committer::BehindThePeerBarrier) {
                let peer_operation = crate::PeerOperation::from(&authorization.preparation);
                let Some(agreed) = world.get_resource::<crate::PeerVerdicts>()
                    .and_then(|verdicts| verdicts.prepared_plan(peer_operation)) else {
                    return;
                };
                if authorization.peer_identity() != Some(agreed) {
                    return;
                }
            }
            let checkpoint_operation = authorization.operation().checkpoint_operation;
            Some((authorization.plan, checkpoint_operation))
        },
        // Not yet, or no longer valid. Returning is not DROPPING: the intent
        // stays pending and this runs again next frame while the transaction
        // progresses (or is superseded) in `Update`.
        AuthorizedPlan::Wait | AuthorizedPlan::InvalidPromise => return,
    };
    let kind = operation.kind.clone();

    // It touches no world and depends only on `settings`, so if it fails the room is never
    // reconstructed, the intent stays pending, and the timeline is untouched (it retries on a
    // later confirmed frame).
    let built = match committer {
        Committer::Alone { settings, .. } => build_sync_test_session(settings),
        // The next generation of this peer's session. Building it changes
        // nothing: not the world, not the link, not the generation count.
        Committer::BehindThePeerBarrier => match world.get_resource::<crate::peer::PeerLineage>() {
            Some(lineage) => lineage.build_next(),
            None => {
                bevy::log::error_once!(
                    "a peer session has no `PeerLineage`, so it cannot start its \
                     next timeline and its lifecycle operation is held. A peer \
                     session starts through `start_peer_session`."
                );
                return;
            }
        },
    };
    let session = match built {
        Ok(session) => session,
        Err(error) => {
            error!(
                "Track B: failed to BUILD the rebase session; leaving the room and the pending intent untouched: {error}"
            );
            return;
        }
    };

    // ⛔⛤ **THE SECOND FALLIBLE-BUT-WORLD-UNTOUCHED CHECK, AND IT HAS TO BE HERE
    // RATHER THAN AT THE INSTALL.** `install_rebased_sync_test_session` refuses
    // to declare frame zero while a construction candidate holds a rollback
    // carrier an ordinary query cannot see. Below this line the comment says
    // *"From here NOTHING may fail"* and means it: the op has run, the room is
    // reconstructed, and a refusal at the install would leave the commit
    // half-complete — the old ring history still restorable and no new baseline.
    // ⇒ Asked here, a refusal costs nothing: the room is untouched, the intent
    // stays pending, and the crossing retries on a later confirmed frame, which
    // is exactly what `CommitOutcome::Retry` does for a room that is not
    // preparable yet.
    let eligibility = match crate::session::FrameZeroEligibility::check(world) {
        Ok(eligibility) => eligibility,
        Err(refusal) => {
            error!(
                "Track B: NOT committing this crossing yet — {refusal}. The room and \
                 the pending intent are untouched and this retries on a later \
                 confirmed frame."
            );
            return;
        }
    };

    // Reconstruct atomically after fallible preparation. Wall-clock duration is
    // written only to non-rollback diagnostics; `bevy::platform::time::Instant`
    // keeps the measurement available on wasm as well.
    let commit_started = bevy::platform::time::Instant::now();
    match execute_lifecycle_commit(world, &kind, authorized) {
        // A transient failure (target room not preparable yet) changed nothing —
        // leave the intent pending to retry on a later confirmed frame and DROP
        // the already-built session (installing it without the op would rebase
        // over a room that never changed).
        CommitOutcome::Retry => return,
        // A void crossing (the recorded body is gone / had no identity): the
        // intent can never succeed, so DROP it — without reconstructing or
        // rebasing — and leave the source room authoritative. Not retried (it
        // would fail forever) and NOT substituted with another body (
        // #1). Dropping the built session is free; no world was touched.
        CommitOutcome::Cancelled => {
            if let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>() {
                pending.take();
            }
            // `begin_room_transition_load_system` returns early whenever no intent is pending,
            // so nothing else would ever come back for it — and the next crossing to the SAME
            // destination matches `same_destination`, returns early against the orphan, and
            // commits under a plan prepared for a crossing that was cancelled.
            //
            //  safe from here, `PreUpdate`, unlike the intent: the transaction
            // state is NOT rollback-registered (that is the whole reason
            // readiness moved host-side), so writing it outside the rewound
            // schedule is ordinary. The rollback-registered `PendingLifecycleCommit`
            // above is cleared from the exclusive world on a CONFIRMED frame,
            // which is the one place that is legal.
            retire_cancelled_room_transition(world, &kind);
            // ⛔ A PEER REBASES ALSO WHEN THE OPERATION IS VOID. The slot is
            // rollback state, and this peer cleared it from outside the
            // timeline, at a frame the other peer is not at. The peers hold
            // one world only because it is frozen, so the frame the
            // simulation starts again must be one frame for each of them:
            // frame zero of the next generation. A sync test has one host and
            // keeps its timeline.
            if matches!(committer, Committer::BehindThePeerBarrier) {
                crate::peer::install_next_peer_session(world, session, eligibility);
            }
            return;
        }
        CommitOutcome::Committed(arrived_in) => {
            if let Some(active) = world.get_resource_mut::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
                .and_then(|state| state.into_inner().active.as_mut()).filter(|active| active.describes(&operation)) {
                active.arrived_in = arrived_in;
            }
        }
    }

    // From here NOTHING may fail. Clear the slot so the post-op world (the new
    // baseline) carries no pending intent...
    if let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>() {
        pending.take();
    }

    // ...and install the pre-built session as the new frame-zero baseline. This
    // bumps the session generation and the first frame-zero SaveWorld overwrites
    // every ring slot, so no earlier frame can restore the pre-op room. Executing
    // the op WITHOUT rebasing would leave old ring history restorable — the rebase
    // is the load-bearing half of the confirmed authoritative discontinuity, and
    // the install is infallible so the commit cannot half-complete.
    //  a rebase KEEPS its owner. This commit rebuilds the session under a
    // new world; it does not change whose session it is, and inferring an owner
    // here would quietly hand a match-activation session to the local
    // maintainer.
    // The transaction this crossing waited on has done its job: close it out the
    // way the eager commit closes out its own — which, when a cover is up, means
    // handing it to the presentation adapter rather than dropping it here.
    retire_committed_room_transition(world, commit_started.elapsed());

    // ⭐ INFALLIBLE, and that is a type-level fact rather than a comment: the
    // token was taken above, before anything destructive, and this function
    // cannot return a refusal. There used to be an `error!` branch here for a
    // refusal arriving after the commit — the room authoritative with the
    // previous timeline's order history installed — and nothing useful could be
    // done in it. It is deleted because it can no longer be written.
    match committer {
        Committer::Alone { settings, owner } => {
            install_rebased_sync_test_session(world, session, settings, owner, eligibility);
        }
        // The handshake of the next generation is the last part of the peer
        // barrier: this peer does not confirm a frame until the other one is
        // there.
        Committer::BehindThePeerBarrier => {
            crate::peer::install_next_peer_session(world, session, eligibility);
        }
    }
}

/// The exact plan and admission inputs authorized for a crossing.
enum AuthorizedPlan {
    Ready(crate::room_promises::AuthorizedRoomPlan),
    InvalidPromise,
    Wait,
}

fn selected_preparation(
    world: &World,
    operation: &PendingIntent,
) -> Option<ambition_platformer2d_session::room_preparation::RoomPreparation> {
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        ambition_platformer2d_session::room_preparation::CapturedRoomPreparation,
    >(world)?.for_operation(operation).cloned()
}

/// A peer uses its retained promise even after presentation retires the load.
fn authorized_plan(world: &mut World, operation: &PendingIntent) -> AuthorizedPlan {
    if matches!(world.get_resource::<RollbackSessionOwnership>(), Some(RollbackSessionOwnership::Peer)) {
        if let Some(preparation) = selected_preparation(world, operation) {
            if let Some(promise) = world.get_resource::<crate::room_promises::PromisedRoomPlans>()
                .and_then(|plans| plans.get(&preparation)).cloned() {
                return if promise.matches_world(world) {
                    AuthorizedPlan::Ready(promise)
                } else {
                    AuthorizedPlan::InvalidPromise
                };
            }
        }
    }
    authorized_loading_plan(world, operation)
}

fn authorized_loading_plan(world: &mut World, operation: &PendingIntent) -> AuthorizedPlan {
    let epoch = world.get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionContentEpoch>()
        .map(|epoch| epoch.get());
    let session_scope = world.get_resource::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>()
        .and_then(|scope| scope.current());
    let Some(state) = world
        .get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
    else {
        return AuthorizedPlan::Wait;
    };
    let Some(active) = state.active.as_ref() else {
        return AuthorizedPlan::Wait;
    };
    if active.phase
        != ambition_platformer2d_runtime::room_transition::RoomTransitionLoadPhase::CommitAuthorized
    {
        return AuthorizedPlan::Wait;
    }
    if !active.describes(operation) {
        bevy::log::warn_once!(
            "the authorized room transition describes a different crossing than the \
             pending one ({:?} vs {:?}); waiting for a transaction that matches",
            active.intent,
            operation.kind
        );
        return AuthorizedPlan::Wait;
    }
    if epoch != Some(active.content_epoch) || session_scope != active.session_scope {
        return AuthorizedPlan::Wait;
    }
    let Some(plan) = active.construction_plan.clone() else {
        // Authorized with no prepared plan is not a state the transaction should
        // reach; say so rather than silently preparing one here.
        bevy::log::error_once!(
            "a room transition authorized its commit without a prepared construction \
             plan; refusing to construct one at commit time"
        );
        return AuthorizedPlan::Wait;
    };
    let Some(preparation) = active.preparation.clone() else {
        return AuthorizedPlan::Wait;
    };
    if preparation.admission() != operation {
        return AuthorizedPlan::Wait;
    }
    let authorization = crate::room_promises::AuthorizedRoomPlan {
        plan, preparation,
        source_room: active.source_room,
    };
    if !authorization.matches_world(world) {
        return AuthorizedPlan::Wait;
    }
    AuthorizedPlan::Ready(authorization)
}

/// Complete a confirmed room-transition readiness transaction. If presentation owns the
/// cover, publish `Committed` plus `commit_duration` / `committed_at` and let the presentation
/// adapter retire the transaction after its settle barrier. Otherwise retire it immediately.
///
/// This load state is host-side rather than rollback state. `committed_at` is required because the
/// presentation settle deadline is measured from it.
fn retire_committed_room_transition(world: &mut World, commit_duration: std::time::Duration) {
    let Some((barrier, cover_required)) = world
        .get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
        .and_then(|state| state.active.as_ref())
        .map(|active| (active.barrier.load_id.clone(), active.cover_required))
    else {
        return;
    };

    if cover_required {
        let now = world
            .get_resource::<bevy::prelude::Time<bevy::prelude::Real>>()
            .map(|time| time.elapsed());
        if let Some(mut state) =
            world.get_resource_mut::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
        {
            if let Some(active) = state.active.as_mut() {
                active.commit_duration = Some(commit_duration);
                active.committed_at = now;
                active.phase = ambition_platformer2d_runtime::room_transition::RoomTransitionLoadPhase::Committed;
            }
            state.begin_settling();
        }
        // Rollback hosts never enter `GameMode::RoomTransition`; presentation restores its own
        // mode when it retires the cover.
        return;
    }

    if let Some(mut loads) = world.get_resource_mut::<ambition_load::LoadCoordinator>() {
        loads.retire(&barrier);
    }
    if let Some(mut state) =
        world.get_resource_mut::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
    {
        state.active = None;
    }
    ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
        ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
        "room_commit_confirmed",
    );
    if let Some(mut mode) = world.get_resource_mut::<
        bevy::prelude::NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>,
    >() {
        mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
    }
}

/// Close out the readiness transaction a CANCELLED crossing opened.
///
///  only the transaction this exact intent opened. A crossing is cancelled
/// because its body is gone, not because room transitions in general are off;
/// retiring whatever happens to be active would take out a newer, unrelated
/// transaction opened by somebody else in the same frame. Matching the intent is
/// the same key `same_destination` uses to decide the transaction OWNS the
/// crossing, asked in the other direction.
///
///  no `GameMode` restore, deliberately. Only a rollback host reaches here,
/// and the rollback host never entered `GameMode::RoomTransition` in the first
/// place — `begin_room_transition_load_system` guards that on
/// `!pending.is_rollback_host()`, because setting it gates the sim systems and
/// desynced the checksum. Restoring a mode that was never set would be the
/// symmetry looking right rather than being right.
fn retire_cancelled_room_transition(
    world: &mut World,
    intent: &ambition_platformer2d_session::lifecycle_commit::LifecycleIntent,
) {
    let Some(barrier) = world
        .get_resource::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
        .and_then(|state| state.active.as_ref())
        .filter(|active| &active.intent == intent)
        .map(|active| active.barrier.load_id.clone())
    else {
        return;
    };
    if let Some(mut loads) = world.get_resource_mut::<ambition_load::LoadCoordinator>() {
        loads.retire(&barrier);
    }
    if let Some(mut state) =
        world.get_resource_mut::<ambition_platformer2d_runtime::room_transition::RoomTransitionLoadState>()
    {
        state.active = None;
    }
}

/// A successful application carries its arrival room. A retry leaves the
/// operation pending. A cancellation leaves the live rooms intact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitOutcome {
    Committed(Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>),
    Retry,
    Cancelled,
}

fn execute_lifecycle_commit(
    world: &mut World,
    kind: &LifecycleIntent,
    // The plan the readiness transaction AUTHORIZED. `Some` for every transition
    // that reaches here (`authorized_plan` returned it a moment ago); `None` for
    // the variants that open no transaction.
    authorized: Option<(
        std::sync::Arc<RoomConstructionPlan>,
        Option<ambition_platformer2d_session::checkpoint::CheckpointOperationKey>,
    )>,
) -> CommitOutcome {
    match (kind, authorized) {
        (kind, Some((plan, checkpoint_operation))) => commit_transition(
            world,
            &plan,
            checkpoint_operation,
            &kind,
        ),
        // An intent with no authorized plan cannot reach here: the caller
        // returns before building a session. Treated as transient rather than
        // asserted, so a future caller cannot turn a mistake into a panic.
        (_, None) => CommitOutcome::Retry,
    }
}

/// Apply a retained transition through the shared room application.
///
/// Shared room-transition application stays in the common application path. This
/// wrapper bridges `&mut World` through `SystemState`, applies deferred commands
/// synchronously, and drains spawn requests before returning.
#[allow(clippy::too_many_arguments)]
fn commit_transition(
    world: &mut World,
    // Use the exact plan whose readiness and assets were authorized.
    plan: &RoomConstructionPlan,
    // WHICH checkpoint restore this transaction was opened for, resolved when
    // the readiness transaction opened and carried from there. `None` is an
    // ordinary crossing. ⛔ Not re-derived here by intent equality: two crossings
    // to one room with one subject compare equal, and a commit that re-asked
    // could apply a later operation's snapshot to this one's room.
    checkpoint_operation: Option<
        ambition_platformer2d_session::checkpoint::CheckpointOperationKey,
    >,
    intent: &LifecycleIntent,
) -> CommitOutcome {
    let target_room = intent.target_room();
    let Some(target_index) = ({
        let mut rooms = world.query::<&ambition_platformer2d_world::rooms::RoomSet>();
        rooms
            .iter(world)
            .next()
            .and_then(|set| set.rooms.iter().position(|room| room.id == target_room))
    }) else {
        error!(
            "Track B: the recorded transition names room '{target_room}', which the \
             session's RoomSet does not contain; cancelling the crossing"
        );
        return CommitOutcome::Cancelled;
    };

    //  stale spawn requests first. A speculative frame may have enqueued
    // `SpawnActorRequest`s that its rollback never un-enqueued, and this path DRAINS the queue
    // below rather than leaving it to a scheduled system.
    if let Some(mut pending) = world.get_resource_mut::<bevy::ecs::message::Messages<
        ambition_platformer2d_actor_spawn::SpawnActorRequest,
    >>() {
        pending.clear();
    }

    // That escalation is safe for a reason worth stating rather than trusting: reaching here at all
    // means `authorized_plan` found a `CommitAuthorized` transaction, which only exists if
    // `RoomTransitionPlugin` is installed — and that plugin also installs
    // `commit_ready_room_transition_system`, which has taken the same parameters as a plain system
    // all along. A host that could panic here could not have produced the authorization that got
    // here.
    // A fresh checkpoint operation (a New Game) is a whole-session restart.
    let restart = checkpoint_operation
        .zip(ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<ambition_platformer2d_session::checkpoint::AcceptedCheckpointRestore>(world))
        .and_then(|(key, accepted)| accepted.inputs_for_key(key))
        .is_some_and(|accepted| accepted.fresh);
    let mut state: bevy::ecs::system::SystemState<
        ambition_platformer2d_runtime::room_transition::RoomTransitionApplication,
    > = bevy::ecs::system::SystemState::new(world);
    let outcome = {
        // Bevy 0.19 moved SystemParam validation into the fetch, so `get_mut`
        // reports it instead of panicking inside. The paragraph above is the
        // argument that it cannot fail here, so the panic stays where 0.18 put it.
        let mut application = state
            .get_mut(world)
            .expect("RoomTransitionApplication params are the ones RoomTransitionPlugin installs");
        application.stage(plan, intent, target_index, restart, checkpoint_operation)
    };
    //  unconditionally, before any early return. `SystemState` holds the
    // command queue the application wrote into; dropping it without applying
    // would silently discard a half-built room.
    state.apply(world);

    let staged = match outcome {
        Ok(staged) => staged,
        Err(error) => {
            // Every variant is raised BEFORE the first destructive mutation, so
            // the world is still whole and the crossing is simply void.
            error!("Track B: {error}; cancelling the crossing");
            return CommitOutcome::Cancelled;
        }
    };

    // ⛔⛤ **AND NOW THE EXACT PUBLICATION IS ASKED — 2026-09-14 ON REVIEW.**
    // This path treated `Ok` from the application as *"the transition
    // committed"*: it returned `CommitOutcome::Committed` and applied the
    // checkpoint restore without ever asking whether the room the candidate
    // describes actually published. `state.apply(world)` above is what runs the
    // deferred transaction, so the verdict exists by this line.
    //
    // ⭐ THE SAME FUNCTION THE EAGER HOST CALLS, from
    // `finalize_committed_room_transition`. The two hosts differ in WHEN they may
    // mutate the world, never in what publication success means.
    if !ambition_platformer2d_runtime::room_transition::finalize_room_transition(world, &staged) {
        error!(
            "Track B: the room transaction refused its candidate; the crossing is \
             cancelled and the room the session is playing is untouched"
        );
        return CommitOutcome::Cancelled;
    }

    // ── THE EXCLUSIVE-WORLD TAIL ─────────────────────────────────────────────
    // An eager commit's spawns are applied at the frame's flush and its actor
    // requests are drained by a scheduled system. This path runs outside that
    // schedule, so it does both itself, here, synchronously — which is the only
    // thing about a confirmed commit that is not the shared operation.
    world.flush();
    let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
        &mut *world,
        ambition_platformer2d_actor_spawn::apply_spawn_actor_requests,
    );
    world.flush();

    // ⛔⛔ THE DOMAIN RESTORE, AND IT MUST BE HERE — after the room is built and
    // its structural work is applied, and BEFORE the caller rebases. Custody
    // materializes against the identities the build just produced, so running it
    // earlier would restore a hand that does not exist yet; rebasing first would
    // make the session's first restore undo the checkpoint this just put back.
    //
    // ⭐ THE SAME FUNCTION THE EAGER HOST CALLS. The two hosts differ in what
    // authorizes the commit, never in what the commit does.
    if let Some(key) = checkpoint_operation {
        ambition_platformer2d_session::checkpoint::apply_committed_checkpoint_restore(
            world, key,
        );
    }

    CommitOutcome::Committed(staged.arrived_in(world))
}

#[cfg(test)]
mod tests {
    fn test_plan_identity() -> crate::PeerPlanIdentity {
        crate::PeerPlanIdentity { incoming_content: [1; 32], expected_content: [2; 32], data_digest: 7 }
    }

    use super::*;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveBodyId, LiveRoomInstance};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// Deferred transitions resolve the recorded live identity of the crossing body.
    /// If that body no longer exists, the crossing is cancelled; another body is
    /// never substituted. The assertion exercises the shared resolver used by
    /// both transition hosts.
    use ambition_platformer2d_session::lifecycle_commit::RoomTransitionIntent;
    use ambition_platformer2d_runtime::room_transition::{
        ActiveRoomTransitionLoad, RoomTransitionLoadPhase, RoomTransitionLoadState,
    };

    fn capture_pending(world: &mut World) -> crate::PeerOperation {
        use bevy::ecs::system::RunSystemOnce;
        use ambition_platformer2d_session::room_preparation::{CapturedRoomPreparation, capture_room_preparation_inputs};
        use ambition_platformer2d_session::checkpoint::AcceptedCheckpointRestore;
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(world,
            (CapturedRoomPreparation::default(), AcceptedCheckpointRestore::default()));
        world.run_system_once(capture_room_preparation_inputs).unwrap();
        let intent = world.resource::<PendingLifecycleCommit>().peek().unwrap();
        crate::PeerOperation::from(&selected_preparation(world, intent).unwrap())
    }

    fn intent_to(target_room: &str, subject: SimId) -> LifecycleIntent {
        LifecycleIntent::Transition(RoomTransitionIntent {
            disposition: Default::default(),
            source_zone: None,
                initiator: ambition_platformer2d_session::lifecycle_commit::RoomBodyTransfer {
                    requirement: ambition_platformer2d_session::lifecycle_commit::BodyTransferRequirement::InPlay,
                    subject: LiveBodyId::new(subject, None),
                    arrival: ambition_platformer2d_session::lifecycle_commit::RoomArrival::At(ae::Vec2::ZERO),
                    participant: None,
                },
                companions: Vec::new(),
            target_room: target_room.to_string(),
            edge_exit: true,
            zone_sfx: None,
        })
    }

    /// The freeze, as a table: who holds the simulation, and from which frame.
    #[test]
    fn only_a_peer_rebuild_holds_the_simulation_from_the_freeze_frame() {
        use crate::session::{SyncTestOwner, SyncTestSettings};
        let recorded_on_30 = PendingIntent {
            frame: 30, checkpoint_operation: None,
            kind: rebuild_of("hub"),
        };
        assert_eq!(freeze_frame(&recorded_on_30), 30 + PEER_COMMIT_FREEZE_DELAY);
        let travel = PendingIntent { frame: 30, checkpoint_operation: None,
            kind: intent_to("hub", SimId::placement("hero")) };
        assert!(!holds_the_simulation(RollbackSessionOwnership::Peer, Some(&travel), 400));
        let sync_test = RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(2),
            owner: SyncTestOwner::Caller,
        };
        let peer = RollbackSessionOwnership::Peer;
        let external = RollbackSessionOwnership::External;
        let c = freeze_frame(&recorded_on_30);
        for (ownership, pending, frame, held, why) in [
            (peer, None, 500, false, "nothing waits"),
            (peer, Some(&recorded_on_30), 30, false, "the recording frame runs again after a rewind"),
            (peer, Some(&recorded_on_30), c - 1, false, "the last frame before the freeze"),
            (peer, Some(&recorded_on_30), c, true, "the freeze frame"),
            (peer, Some(&recorded_on_30), c + 400, true, "the freeze has no end but the commit"),
            (sync_test, Some(&recorded_on_30), c + 2, false, "a sync test commits alone"),
            (external, Some(&recorded_on_30), c + 2, false, "no commit can end a freeze here"),
        ] {
            assert_eq!(
                holds_the_simulation(ownership, pending, frame),
                held,
                "{ownership:?} at frame {frame}: {why}"
            );
        }
    }

    /// WHEN each committer may commit. A sync test: when the recording frame
    /// is confirmed. A peer: when its confirmed frame is at the freeze frame,
    /// its world is at a frozen frame, AND each handle said it prepared the
    /// operation.
    #[test]
    fn a_peer_rebuild_commits_at_the_freeze_frame_and_a_sync_test_at_the_recording_frame() {
        use crate::peer_input::{PeerVerdict, PreparationVerdict::*};
        use crate::session::{SyncTestOwner, SyncTestSettings};
        let alone = Committer::Alone {
            settings: SyncTestSettings::for_players(2),
            owner: SyncTestOwner::Caller,
        };
        let peer = Committer::BehindThePeerBarrier;
        let recorded_on = 30;
        let c = recorded_on + PEER_COMMIT_FREEZE_DELAY;
        let each = [Prepared, Prepared];
        for (committer, confirmed, frame, said, commits, why) in [
            (alone, recorded_on - 1, 34, [NotYet; 2], false, "the recording frame is not confirmed"),
            (alone, recorded_on, 34, [NotYet; 2], true, "the recording frame is confirmed"),
            (peer, recorded_on, c + 5, each, false, "the recording frame is confirmed and the freeze frame is not"),
            (peer, c - 1, c + 5, each, false, "the frame before the freeze frame"),
            (peer, c, c + 5, each, true, "the freeze frame is confirmed and the world is frozen"),
            (peer, c + 9, c - 1, each, false, "this peer knows inputs past its own frame; its world is not at a frozen frame"),
            (peer, c + 9, c, each, true, "the world reached the freeze frame"),
            (peer, c + 9, c + 9, [Prepared, NotYet], false, "the peer of handle 1 has not prepared it"),
            (peer, c + 9, c + 9, [Prepared, Failed], false, "the peer of handle 1 could not prepare it"),
        ] {
            let mut world = World::new();
            let mut pending = PendingLifecycleCommit::default();
            assert!(pending
                .record(recorded_on, rebuild_of("hub"))
                .admitted());
            world.insert_resource(pending);
            world.insert_resource(crate::RollbackFrameCount(frame));
            let mut verdicts = crate::PeerVerdicts::for_handles(2);
            let operation = capture_pending(&mut world);
            for (handle, said) in said.into_iter().enumerate() {
                verdicts.record(handle, confirmed, PeerVerdict { operation, plan: test_plan_identity(), said });
            }
            world.insert_resource(verdicts);
            let boundary = ConfirmedFrameBoundary {
                current: frame,
                confirmed,
                session: 1,
            };
            assert_eq!(
                operation_to_commit(&world, committer, boundary).is_some(),
                commits,
                "confirmed {confirmed}, at frame {frame}: {why}"
            );
        }
    }

    #[test]
    fn a_peer_waits_for_the_replacement_intent_even_when_its_frame_is_unchanged() {
        use crate::peer_input::{PeerOperation, PeerVerdict, PreparationVerdict};
        let old = PendingIntent { frame: 30, checkpoint_operation: None, kind: rebuild_of("old") };
        let mut pending = PendingLifecycleCommit::default();
        assert!(pending.record(30, rebuild_of("new")).admitted());
        let mut world = World::new();
        world.insert_resource(pending);
        let replacement = capture_pending(&mut world);
        world.insert_resource(crate::RollbackFrameCount(40));
        let mut verdicts = crate::PeerVerdicts::for_handles(2);
        for handle in 0..2 {
            verdicts.record(handle, 40, PeerVerdict {
                operation: PeerOperation { recorded_on: old.frame, intent_checksum: old.checksum(), preparation_checksum: 0 }, plan: test_plan_identity(), said: PreparationVerdict::Prepared,
            });
        }
        world.insert_resource(verdicts);
        let boundary = ConfirmedFrameBoundary { current: 40, confirmed: 40, session: 1 };
        assert!(operation_to_commit(&world, Committer::BehindThePeerBarrier, boundary).is_none());
        for handle in 0..2 {
            world.resource_mut::<crate::PeerVerdicts>().record(handle, 41, PeerVerdict {
                operation: replacement, plan: test_plan_identity(), said: PreparationVerdict::Prepared,
            });
        }
        world.insert_resource(crate::RollbackFrameCount(41));
        let boundary = ConfirmedFrameBoundary { current: 41, confirmed: 41, session: 1 };
        assert_eq!(operation_to_commit(&world, Committer::BehindThePeerBarrier, boundary)
            .unwrap().kind, rebuild_of("new"));
    }

    /// The bodyless shape: a room rebuilt with nobody in it (v146).
    fn rebuild_of(target_room: &str) -> LifecycleIntent {
        LifecycleIntent::ReconstituteRoom(
            ambition_platformer2d_session::lifecycle_commit::RoomReconstitutionIntent {
                target_room: target_room.to_string(),
            },
        )
    }

    fn authorized_transaction(intent: LifecycleIntent) -> ActiveRoomTransitionLoad {
        ActiveRoomTransitionLoad {
            checkpoint_operation: None,
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room: 1,
            intent,
            recorded_on: 0,
            arrived_in: None,
            construction_plan: None,
            preparation: None,
            barrier: ambition_load::LoadBarrierRef::new("load", "ready"),
            opened_this_pass: false,
            cover_required: false,
            cover_presented: true,
            phase: RoomTransitionLoadPhase::CommitAuthorized,
            failure: None,
            asset_work_id: ambition_load::LoadWorkId::new("room-transition.assets:b"),
            staged_actor_names: Vec::new(),
            asset_readiness_complete: true,
            last_asset_progress: None,
            asset_progress_since: None,
            asset_stall_report: None,
            prefetch_hit: false,
            construction_preflight_duration: None,
            asset_manifest_duration: None,
            requested_at: None,
            asset_ready_at: None,
            ready_at: None,
            cover_presented_at: None,
            commit_duration: None,
            committed_at: None,
        }
    }

    ///  A CANCELLED CROSSING MUST NOT LEAVE ITS TRANSACTION BEHIND.
    ///
    /// A void crossing — the body that walked through the door died during the confirmation delay —
    /// drops the intent. Nothing would ever come back for it: `begin_room_transition_load_system`
    /// returns early whenever no intent is pending, so the orphan simply sat there — and the next
    /// crossing to the same destination matches `same_destination`, returns early against the
    /// orphan, and commits under a plan prepared for a crossing that was cancelled.
    #[test]
    fn a_cancelled_crossing_retires_the_transaction_it_opened() {
        let mut world = World::new();
        world.insert_resource(ambition_load::LoadCoordinator::default());
        let intent = intent_to("b", SimId::placement("triggerer"));
        let mut state = RoomTransitionLoadState::default();
        state.active = Some(authorized_transaction(intent.clone()));
        world.insert_resource(state);

        retire_cancelled_room_transition(&mut world, &intent);

        assert!(
            world.resource::<RoomTransitionLoadState>().active.is_none(),
            "the cancelled crossing left its authorized transaction resident; the next \
             crossing to the same destination will match it and commit under a plan \
             prepared for a crossing that never happened"
        );
    }

    ///  A COVERED CONFIRMED COMMIT MUST NOT DROP ITS OWN TRANSACTION.
    ///
    /// The cover, the settle wait and the ONLY latency instrument the game has
    /// all hang off `drive_room_transition_presentation`'s
    /// `phase == Committed` gate, and this route used to null `active` in
    /// `PreUpdate` instead — so on the shipped host the adapter took its
    /// "no transaction" teardown branch, the cover came down the frame the room
    /// was built rather than the frame it was DRAWN, and
    /// `RoomTransitionTelemetry` recorded zero samples.
    ///
    ///  and the uncovered half is the poison, in the same test. Without it
    /// this passes just as happily on a route that never retires anything, which
    /// would wedge every later crossing on a headless host.
    #[test]
    fn a_covered_confirmed_commit_hands_its_transaction_to_the_presentation_adapter() {
        let mut covered = World::new();
        covered.insert_resource(ambition_load::LoadCoordinator::default());
        covered.insert_resource(Time::<Real>::default());
        let mut state = RoomTransitionLoadState::default();
        let mut active = authorized_transaction(intent_to("b", SimId::placement("triggerer")));
        active.cover_required = true;
        state.active = Some(active);
        covered.insert_resource(state);

        retire_committed_room_transition(&mut covered, std::time::Duration::from_millis(7));

        assert!(covered.resource::<RoomTransitionLoadState>().active.is_none());
        let handed_over = covered
            .resource::<RoomTransitionLoadState>()
            .presentations().next()
            .expect(
                "the covered confirmed commit dropped its own transaction, so the \
                 presentation adapter never sees a Committed phase: the cover retires \
                 by falling off a cliff instead of waiting for the target room to be \
                 drawn, and no timing sample is ever recorded",
            );
        assert_eq!(handed_over.phase, RoomTransitionLoadPhase::Committed);
        assert!(
            handed_over.committed_at.is_some(),
            "the transaction was handed over without a commit stamp, so the adapter's \
             settle deadline can never expire and an unclaimable feature view holds a \
             black screen forever"
        );
        assert_eq!(
            handed_over.commit_duration,
            Some(std::time::Duration::from_millis(7)),
            "the confirmed commit's own cost — the expensive half, since this route \
             flushes and drains spawns synchronously — must reach the telemetry sample"
        );

        //  THE POISON: no cover means no adapter, so nobody else will ever
        // retire this and it must retire itself.
        let mut uncovered = World::new();
        uncovered.insert_resource(ambition_load::LoadCoordinator::default());
        let mut state = RoomTransitionLoadState::default();
        state.active = Some(authorized_transaction(intent_to(
            "b",
            SimId::placement("triggerer"),
        )));
        uncovered.insert_resource(state);

        retire_committed_room_transition(&mut uncovered, std::time::Duration::ZERO);

        assert!(
            uncovered
                .resource::<RoomTransitionLoadState>()
                .active
                .is_none(),
            "an uncovered confirmed commit left its transaction resident with no adapter \
             to retire it; every later crossing to that destination matches it and \
             commits under a spent plan"
        );
    }

    ///  and ONLY the one it opened. A crossing is cancelled because its body
    /// is gone, not because room transitions are off. Retiring whatever happens
    /// to be active would take out an unrelated transaction — another
    /// participant's crossing, in the same frame.
    #[test]
    fn a_cancelled_crossing_leaves_somebody_else_s_transaction_alone() {
        let mut world = World::new();
        world.insert_resource(ambition_load::LoadCoordinator::default());
        let theirs = intent_to("c", SimId::player_slot(1));
        let mut state = RoomTransitionLoadState::default();
        state.active = Some(authorized_transaction(theirs));
        world.insert_resource(state);

        retire_cancelled_room_transition(&mut world, &intent_to("b", SimId::placement("gone")));

        assert!(
            world.resource::<RoomTransitionLoadState>().active.is_some(),
            "a cancelled crossing retired a transaction that belonged to a different \
             crossing entirely"
        );
    }

    /// A cancelled rebuild retires the transaction it opened, through the same
    /// road a cancelled crossing does. Without this the orphan sits resident and
    /// the next operation to the same room matches it.
    ///
    /// ⛔ A SIBLING TEST WAS DELETED HERE RATHER THAN KEPT GREEN. It asserted
    /// that a bodyless rebuild does not commit under a CROSSING's authorized
    /// plan, by expecting `authorized_plan` to answer `Wait`. Poisoning it with
    /// the matching crossing intent showed it passed either way: this fixture
    /// has no `RoomTransitionContentEpoch` and no construction plan, so
    /// `authorized_plan` waits before it ever compares the intents, and the test
    /// could not fail. The discrimination it meant to pin is real and IS pinned,
    /// one seam earlier, by `repeated_zone_detection_is_one_destination` in
    /// `room_transition::loading` — `same_destination` is what decides whether a
    /// transaction gets reused, and that fixture can tell the two shapes apart.
    #[test]
    fn a_cancelled_rebuild_retires_the_transaction_it_opened() {
        let mut world = World::new();
        world.insert_resource(ambition_load::LoadCoordinator::default());
        let mut state = RoomTransitionLoadState::default();
        state.active = Some(authorized_transaction(rebuild_of("b")));
        world.insert_resource(state);

        retire_cancelled_room_transition(&mut world, &rebuild_of("b"));

        assert!(
            world.resource::<RoomTransitionLoadState>().active.is_none(),
            "a cancelled bodyless rebuild left its authorized transaction resident"
        );
    }

    #[test]
    fn a_missing_transition_subject_resolves_to_none_never_a_substitute() {
        let mut world = World::new();
        let triggerer = world.spawn(SimId::placement("triggerer")).id();
        // Two live instances of one room hold a body with one authored id.
        // The crossing names the one in live room #1.
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let duplicate_in_first = world
            .spawn((SimId::placement("twin"), InRoomInstance(first)))
            .id();
        let twin_in_second = world
            .spawn((SimId::placement("twin"), InRoomInstance(second)))
            .id();
        let primary = world
            .spawn((SimId::player_slot(0), PlayerEntity, PrimaryPlayer))
            .id();
        assert_ne!(triggerer, primary);

        //  `TransitBodies`, not the whole `RoomTransitionApplication`: the
        // resolver is the only thing under test and it is pure queries, so a
        // bare `World` can host it. Building the full application here would
        // demand a dozen unrelated resources — a fixture that models nothing.
        // `RoomTransitionApplication::subject_entity` delegates straight to this.
        let mut state: bevy::ecs::system::SystemState<
            ambition_platformer2d_runtime::room_transition::TransitBodies,
        > = bevy::ecs::system::SystemState::new(&mut world);
        let bodies = state
            .get_mut(&mut world)
            .expect("TransitBodies is pure queries");

        assert_eq!(
            bodies.subject_entity(&LiveBodyId::new(SimId::placement("triggerer"), None)),
            Some(triggerer),
            "the recorded triggering SimId is transported, not the current primary"
        );
        assert_eq!(
            bodies.subject_entity(&LiveBodyId::new(SimId::placement("gone"), None)),
            None,
            "a recorded body that despawned before commit is a void crossing, \
             not a licence to teleport the home player"
        );
        // Control: the duplicate in live room #0 is a real body, and it is
        // found when the crossing names it.
        assert_eq!(
            bodies.subject_entity(&LiveBodyId::new(SimId::placement("twin"), Some(first))),
            Some(duplicate_in_first),
        );
        assert_eq!(
            bodies.subject_entity(&LiveBodyId::new(SimId::placement("twin"), Some(second))),
            Some(twin_in_second),
            "the crossing's subject is in live room #1, and commit resolved the \
             body with the same id in live room #0"
        );
        assert_eq!(
            bodies.subject_entity(&LiveBodyId::new(SimId::placement("twin"), Some(second.next()))),
            None,
            "no body with that id is in live room #2, and commit substituted a \
             duplicate from another live room"
        );
    }
}
