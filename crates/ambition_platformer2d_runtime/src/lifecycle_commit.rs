//! Confirmed-frame lifecycle commit (Track B, Piece 2).
//!
//! The sim side records a [`PendingLifecycleCommit`] instead of executing a
//! room-lifecycle op on a speculative frame (Piece 1, in `ambition_platformer2d_actor_monolith`).
//! This module is the host-side other half: once the recording frame is
//! confirmed, it executes the reconstruction in the EXCLUSIVE world — outside
//! `GgrsSchedule`, so it is never rolled back — and then **rebases the session**
//! so no earlier snapshot can restore the pre-op room.
//!
//! Placement: `PreUpdate`, `.after(RunGgrsSystems)` (installed by
//! `rollback::session::install_session_bridge`). By that point the whole GGRS
//! advance batch for this rendered frame is done. The committer is an exclusive
//! `fn(&mut World)`, the same shape as `enforce_session_contract`.
//!
//! Ownership gate: only a [`RollbackSessionOwnership::LocalSyncTest`] session may
//! be rebased unilaterally. External / P2P requires a coordinated peer barrier
//! (the documented Matchbox seam), so this is inert there.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::session::lifecycle_commit::{
    LifecycleIntent, PendingIntent, PendingLifecycleCommit,
};
use ambition_platformer2d_actor_monolith::world::rooms::RoomConstructionPlan;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::ConfirmedFrameBoundary;

use crate::rollback::{
    build_sync_test_session, install_rebased_sync_test_session, RollbackSessionOwnership,
};

/// Execute a confirmed deferred lifecycle op in the exclusive world and rebase.
///
/// No-op unless (a) a rollback host is installed (`ConfirmedFrameBoundary`
/// present), (b) it is a `LocalSyncTest` session we may rebase, and (c) a pending
/// intent exists whose recording frame is confirmed.
pub fn commit_confirmed_lifecycle(world: &mut World) {
    let Some(boundary) = world.get_resource::<ConfirmedFrameBoundary>().copied() else {
        return;
    };
    let Some(RollbackSessionOwnership::LocalSyncTest { settings, owner }) =
        world.get_resource::<RollbackSessionOwnership>().copied()
    else {
        return;
    };

    let Some(PendingIntent { kind, .. }) = world
        .get_resource::<PendingLifecycleCommit>()
        .and_then(|pending| pending.confirmed(boundary.confirmed).cloned())
    else {
        return;
    };

    // Never rebase over an already-diverged session. `start_sync_test_session`
    // installs a fresh `RollbackSessionStatus`, which would ERASE a
    // `SyncTestMismatch` reported during THIS same update — and the confirmation
    // and the mismatch both fire at the check horizon, so they coincide. If the
    // old timeline is unhealthy, leave the diagnostic visible and do not commit;
    // a rebase must never launder a divergence into a clean baseline.
    //
    // ⛔ **BUT IT MUST NOT BE SILENT** (GPT 5.6 via Jon, 2026-08-12). This is the
    // veto that decides whether a CONFIRMED room transition happens, so while a
    // session is unhealthy every door and every loading zone in the game is
    // inert — detection fired, the intent was recorded, and nothing moves. A bare
    // `return` here reports that as *the room simply did not change*, which is
    // indistinguishable from the content being wrong and is what sent an
    // investigation into the input layer.
    //
    // ⚠ `_once`: an unhealthy timeline stays unhealthy, and this runs per frame.
    // The line names the transition it is holding, because "which one" is the
    // first thing anybody reading this asks.
    if let Err(error) = crate::rollback::session_health(world) {
        bevy::log::error_once!(
            "a confirmed lifecycle commit is being HELD because the rollback \
             session is unhealthy ({error:?}) — until this clears, doors and \
             loading zones will detect and never move. The room is not the \
             problem; the desync is."
        );
        return;
    }

    // ⭐⭐ **A ROOM CHANGE WAITS FOR ITS ROOM** (D71). A transition intent is
    // handed to the SAME readiness transaction an eager host uses, and may only
    // commit once that transaction authorizes it: the destination prepared, its
    // assets accounted for, the opaque cover proven up. Until 2026-08-14 this
    // committed the instant the frame confirmed — no transaction ever opened on
    // this route — so the SHIPPED game rebuilt every destination room the moment
    // it was confirmed, uncovered, with the theme's parallax still lazy-loading
    // behind it and no way to report a failure.
    //
    // ⚠ **returning is not dropping.** The intent stays pending and this runs
    // again next frame; the transaction is progressing in `Update` in between.
    // The other lifecycle variants open no transaction and are unaffected.
    //
    // ⛔⛔ **AND IT COMMITS THE PLAN THE TRANSACTION AUTHORIZED, NOT A FRESH
    // ONE.** Waiting for `CommitAuthorized` and then calling
    // `RoomConstructionPlan::prepare` again would use the transaction as a
    // permission BIT and start construction over — so the readiness could
    // authorize a plan built under content epoch E while the world got built from
    // E+1, with the assets accounted for on the wrong one. `authorized_plan`
    // hands over the exact `Arc<RoomConstructionPlan>` the transaction prepared,
    // or refuses.
    let authorized = match &kind {
        LifecycleIntent::Transition(intent) => match authorized_plan(world, intent) {
            AuthorizedPlan::Ready(plan) => Some(plan),
            // Not yet, or no longer valid. Returning is not DROPPING: the intent
            // stays pending and this runs again next frame while the transaction
            // progresses (or is superseded) in `Update`.
            AuthorizedPlan::Wait => return,
        },
        _ => None,
    };

    // ATOMICITY: build the replacement session — the ONLY fallible step of the
    // whole commit — BEFORE any destructive mutation. It touches no world and
    // depends only on `settings`, so if it fails the room is never reconstructed,
    // the intent stays pending, and the timeline is untouched (it retries on a
    // later confirmed frame). Constructing after the room mutation, as this once
    // did, could leave a reconstructed room with the old session still installed,
    // the clock reset, and no rebase — a half-committed state.
    let session = match build_sync_test_session(settings) {
        Ok(session) => session,
        Err(error) => {
            error!(
                "Track B: failed to BUILD the rebase session; leaving the room and the pending intent untouched: {error}"
            );
            return;
        }
    };

    // Now the fallible work is done. Reconstruct the room (also atomic: it
    // prepares + preflights before its own infallible `apply_to_world`).
    //
    // ⚠ **timed, because this is the expensive half and nothing was measuring
    // it.** The eager commit records `commit_duration` around its own
    // application; this route never did, so the one number that describes what a
    // room change costs on the SHIPPED host did not exist. It is not the same
    // number as the eager one either: this path flushes and drains the plan's
    // spawn requests synchronously (see `commit_transition`'s exclusive-world
    // tail), so the confirmed commit legitimately bills work the eager commit
    // defers to the frame's flush.
    //
    // AMBITION_REVIEW(determinism): wall clock into a write-only diagnostic on
    // `RoomTransitionLoadState`, which is not rollback-registered and which no
    // sim decision reads — the same standing exemption `construction_preflight_duration`
    // carries. ⛔ `bevy::platform::time::Instant` rather than `std`: it is
    // `web-time` on wasm, and a `#[cfg(not(wasm32))]` clock leaves the one
    // platform whose numbers would explain a slow transition recording none.
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
        // would fail forever) and NOT substituted with another body (GPT review
        // #1). Dropping the built session is free; no world was touched.
        CommitOutcome::Cancelled => {
            if let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>() {
                pending.take();
            }
            // ⛔⛔ **AND THE TRANSACTION THE INTENT OPENED, or the crossing is
            // only half-cancelled.** Dropping the intent alone left the
            // authorized transaction resident in `CommitAuthorized` with its
            // load barrier never retired and nothing left that could ever
            // commit it. `begin_room_transition_load_system` returns early
            // whenever no intent is pending, so nothing else would ever come
            // back for it — and the next crossing to the SAME destination
            // matches `same_destination`, returns early against the orphan, and
            // commits under a plan prepared for a crossing that was cancelled.
            //
            // ⚠ safe from here, `PreUpdate`, unlike the intent: the transaction
            // state is NOT rollback-registered (that is the whole reason
            // readiness moved host-side), so writing it outside the rewound
            // schedule is ordinary. The rollback-registered `PendingLifecycleCommit`
            // above is cleared from the exclusive world on a CONFIRMED frame,
            // which is the one place that is legal.
            if let LifecycleIntent::Transition(intent) = kind {
                retire_cancelled_room_transition(world, &intent);
            }
            return;
        }
        CommitOutcome::Committed => {}
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
    // ⭐ **a rebase KEEPS its owner.** This commit rebuilds the session under a
    // new world; it does not change whose session it is, and inferring an owner
    // here would quietly hand a match-activation session to the local
    // maintainer.
    // The transaction this crossing waited on has done its job: close it out the
    // way the eager commit closes out its own — which, when a cover is up, means
    // handing it to the presentation adapter rather than dropping it here.
    retire_committed_room_transition(world, commit_started.elapsed());

    install_rebased_sync_test_session(world, session, settings, owner);
}

/// The authorized plan, or a reason to wait.
enum AuthorizedPlan {
    Ready(std::sync::Arc<RoomConstructionPlan>),
    Wait,
}

/// **Does an AUTHORIZED transaction still describe THIS crossing, and what plan
/// did it authorize?**
///
/// ⛔⛔ **the transaction is not a permission bit.** Checking only
/// `phase == CommitAuthorized` and then preparing a fresh plan means readiness
/// authorized one construction and the world got another — the assets were
/// accounted for on a plan nobody applied. Every term below is a way that can go
/// wrong between the frame the transaction authorized and the frame this commits:
///
/// * a DIFFERENT intent — the transaction was superseded, or belongs to another
///   crossing entirely;
/// * a stale CONTENT EPOCH — the room set or a construction registry changed, so
///   the prepared plan describes content that no longer exists (a hot reload is
///   the ordinary way this happens, and `same_destination` already treats the
///   epoch as identity for exactly this reason);
/// * a different SESSION SCOPE — the session was torn down and rebuilt under the
///   transaction;
/// * a different SOURCE ROOM — something else moved the world since preparation,
///   so the retirement half of the plan targets the wrong roster.
///
/// ⚠ **a mismatch WAITS rather than failing.** The pending intent is untouched
/// and the readiness transaction is left to its own supersession path in
/// `Update`: `begin` sees a still-pending intent whose destination no longer
/// matches the active transaction, opens a fresh one, and this commits that.
/// Failing here would need a cancellation contract for a crossing that is still
/// perfectly wanted.
fn authorized_plan(
    world: &mut World,
    intent: &ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomTransitionIntent,
) -> AuthorizedPlan {
    // Everything the active transaction is compared against, copied out FIRST:
    // the checks below query the world, and a live borrow of the load state
    // would make that impossible.
    let epoch = world
        .get_resource::<crate::room_transition::RoomTransitionContentEpoch>()
        .map(|epoch| epoch.get());
    let session_scope = world
        .get_resource::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>()
        .and_then(|scope| scope.current());
    let source_room = {
        let mut rooms = world.query::<&ambition_platformer2d_actor_monolith::rooms::RoomSet>();
        rooms.iter(world).next().map(|set| set.active)
    };
    let Some(state) = world.get_resource::<crate::room_transition::RoomTransitionLoadState>()
    else {
        return AuthorizedPlan::Wait;
    };
    let Some(active) = state.active.as_ref() else {
        return AuthorizedPlan::Wait;
    };
    if active.phase != crate::room_transition::RoomTransitionLoadPhase::CommitAuthorized {
        return AuthorizedPlan::Wait;
    }
    if &active.intent != intent {
        bevy::log::warn_once!(
            "the authorized room transition describes a different crossing than the \
             pending one ({:?} vs {:?}); waiting for a transaction that matches",
            active.intent,
            intent
        );
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
    if epoch != Some(active.content_epoch) {
        bevy::log::warn_once!(
            "the authorized room transition was prepared under content epoch {} and \
             the world is now at {:?}; waiting for a transaction prepared against \
             the current content",
            active.content_epoch,
            epoch
        );
        return AuthorizedPlan::Wait;
    }
    if session_scope != active.session_scope {
        return AuthorizedPlan::Wait;
    }
    if source_room != Some(active.source_room) {
        bevy::log::warn_once!(
            "the authorized room transition was prepared from room index {} and the \
             world is now in {:?}; waiting",
            active.source_room,
            source_room
        );
        return AuthorizedPlan::Wait;
    }
    AuthorizedPlan::Ready(plan)
}

/// Close out the readiness transaction a just-committed confirmed transition
/// used, **exactly as the eager commit closes out its own** — which is not the
/// same thing as dropping it.
///
/// ⛔⛔ **THIS DROPPED THE TRANSACTION UNCONDITIONALLY, AND THAT SILENCED THE
/// COVER AND THE ONLY LATENCY INSTRUMENT THE GAME HAS** (measured by reading,
/// 2026-08-15). `RoomTransitionLoadPhase::Committed` has exactly ONE writer (the
/// eager `commit_ready_room_transition_system`) and exactly ONE reader
/// (`drive_room_transition_presentation`'s retirement gate). This route never
/// set it: it nulled `active` here in `PreUpdate`, so the adapter's next `Update`
/// took its *"no active transition"* teardown branch instead. Three things live
/// behind the gate it skipped, and every one of them is why slice 2 put the
/// transaction on the shipped host in the first place:
///
/// * the `UnclaimedFeatureViews` SETTLE WAIT — the cover comes down when the
///   target room has actually been DRAWN, not one frame after it was built. Its
///   absence is precisely Jon's 2026-07-30 report (*"It flashes squares, which
///   then flash the placeholder sprite, and then it flashes to the characters"*),
///   whose 2026-08-09 fix was therefore protecting only the fixed-tick host;
/// * `minimum_visible`, the floor that stops a loading foreground strobing on a
///   fast transition;
/// * `RoomTransitionTelemetry::record` — the ONLY site that computes
///   `request_to_ready`, `asset_wait`, `commit_to_first_target_frame`,
///   `prefetch_hit` and the over-budget warning. On the shipped host it produced
///   zero samples, so *"measure what a transition costs"* had no instrument on
///   the route players take. ⭐ this is the same class of silence the
///   unconditional BEGIN log in `loading.rs` was added to break: an instrument
///   that reports nothing exactly when the interesting thing happens reads as
///   *"nothing happened"*.
///
/// ⇒ so the fork is deleted rather than duplicated. `cover_required` IS
/// *"a presentation adapter is installed"* — it is set from
/// `RoomTransitionPresentationAvailable`, the marker that adapter inserts — so
/// handing the transaction over cannot strand it: the adapter that will retire it
/// is exactly the one whose presence made a cover required. When no cover is
/// required there is no adapter, and this route retires it itself, as before.
///
/// ⚠ **not rollback state.** `RoomTransitionLoadState` is deliberately not
/// rollback-registered (that is why readiness moved host-side), and this runs in
/// `PreUpdate` outside the rewound schedule — the same standing that let the
/// unconditional drop live here.
///
/// ⚠ **`committed_at` is REQUIRED, not telemetry garnish**, and getting it wrong
/// hangs the cover: the adapter's give-up deadline is
/// `now - committed_at >= presentation_settle_deadline`, and a `None` there reads
/// as *zero elapsed* forever, so a room with a genuinely unclaimable feature view
/// would hold a black screen instead of revealing with the warning.
fn retire_committed_room_transition(world: &mut World, commit_duration: std::time::Duration) {
    let Some((barrier, cover_required)) = world
        .get_resource::<crate::room_transition::RoomTransitionLoadState>()
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
            world.get_resource_mut::<crate::room_transition::RoomTransitionLoadState>()
        {
            if let Some(active) = state.active.as_mut() {
                active.commit_duration = Some(commit_duration);
                active.committed_at = now;
                active.phase = crate::room_transition::RoomTransitionLoadPhase::Committed;
            }
        }
        // ⚠ **no `GameMode` restore here, and none is owed.** A rollback host is
        // the only caller (`commit_confirmed_lifecycle` is installed by
        // `rollback::session`), and it never entered `GameMode::RoomTransition`:
        // `begin_room_transition_load_system` guards that request on
        // `!is_rollback_host` because the mode gates SIM systems and desynced the
        // checksum. The adapter sets `Playing` at its own retirement, which is
        // where the eager host's mode genuinely comes back.
        return;
    }

    if let Some(mut loads) = world.get_resource_mut::<ambition_load::LoadCoordinator>() {
        loads.retire(&barrier);
    }
    if let Some(mut state) =
        world.get_resource_mut::<crate::room_transition::RoomTransitionLoadState>()
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
/// ⛔ **only the transaction this exact intent opened.** A crossing is cancelled
/// because its body is gone, not because room transitions in general are off;
/// retiring whatever happens to be active would take out a newer, unrelated
/// transaction opened by somebody else in the same frame. Matching the intent is
/// the same key `same_destination` uses to decide the transaction OWNS the
/// crossing, asked in the other direction.
///
/// ⚠ **no `GameMode` restore, deliberately.** Only a rollback host reaches here,
/// and the rollback host never entered `GameMode::RoomTransition` in the first
/// place — `begin_room_transition_load_system` guards that on
/// `!pending.is_rollback_host()`, because setting it gates the sim systems and
/// desynced the checksum. Restoring a mode that was never set would be the
/// symmetry looking right rather than being right.
fn retire_cancelled_room_transition(
    world: &mut World,
    intent: &ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomTransitionIntent,
) {
    let Some(barrier) = world
        .get_resource::<crate::room_transition::RoomTransitionLoadState>()
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
        world.get_resource_mut::<crate::room_transition::RoomTransitionLoadState>()
    {
        state.active = None;
    }
}

/// What a commit attempt resolved to. The three outcomes differ in what happens
/// to the pending intent and the session, so a bool cannot express them:
///
/// * [`Committed`](CommitOutcome::Committed) — reconstruction happened; clear the
///   intent and rebase the session.
/// * [`Retry`](CommitOutcome::Retry) — a TRANSIENT failure (the target room could
///   not prepare yet); keep the intent pending and try again on a later confirmed
///   frame. Nothing was mutated.
/// * [`Cancelled`](CommitOutcome::Cancelled) — the intent is VOID and can never
///   succeed (the crossing body is gone or fails the transit contract); DROP
///   the intent
///   without reconstructing or rebasing, leaving the source room authoritative.
///   The distinction from `Retry` is what stops a dead-subject transition from
///   either retrying forever or laundering itself into a home-player teleport
///   (GPT review #1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitOutcome {
    Committed,
    Retry,
    Cancelled,
}

fn execute_lifecycle_commit(
    world: &mut World,
    kind: &LifecycleIntent,
    // The plan the readiness transaction AUTHORIZED. `Some` for every transition
    // that reaches here (`authorized_plan` returned it a moment ago); `None` for
    // the variants that open no transaction.
    authorized: Option<std::sync::Arc<RoomConstructionPlan>>,
) -> CommitOutcome {
    match (kind, authorized) {
        (LifecycleIntent::Transition(intent), Some(plan)) => commit_transition(
            world,
            &plan,
            &intent.subject,
            &intent.target_room,
            intent.arrival,
            intent.edge_exit,
            intent.zone_sfx.as_deref(),
        ),
        // A transition with no authorized plan cannot reach here: the caller
        // returns before building a session. Treated as transient rather than
        // asserted, so a future caller cannot turn a mistake into a panic.
        (LifecycleIntent::Transition(_), None) => CommitOutcome::Retry,
        // The in-place resets (death / manual / replay) are already rollback-safe
        // executed eagerly, and the full sandbox reset was proven rollback-safe
        // single-tick, so no consumer records these variants. Keep a stray intent
        // pending rather than laundering a rebase for a no-op; the match stays
        // exhaustive if deferral extends.
        (
            LifecycleIntent::DeathReset
            | LifecycleIntent::ManualReset
            | LifecycleIntent::Replay
            | LifecycleIntent::FullReset,
            _,
        ) => CommitOutcome::Retry,
    }
}

/// Apply an authorized transition on a CONFIRMED frame, from the exclusive
/// world.
///
/// ⭐⭐ **THE APPLICATION ITSELF IS NOT HERE, AND THAT IS THE WHOLE POINT**
/// (D71, 2026-08-14). This function used to be a second implementation of
/// `commit_room_transition_geometry` + the cross-domain resets, declaring itself
/// a mirror *"kept in sync by the line comments below"*. It was not in sync:
/// measured the day it was replaced, the copy never called `clear_carryover`, so
/// on the shipped rollback host a door carried every in-flight enemy projectile
/// into the next room and left a room-modified ambient gravity in force — and it
/// never recorded the Class-B transit either. Nobody wrote those omissions. They
/// are what a second implementation becomes.
///
/// What remains here is the only thing that is genuinely different about a
/// confirmed commit: it runs OUTSIDE the rewound schedule, so its commands must
/// be applied synchronously rather than at the frame's flush, and the spawn
/// requests the plan enqueues must be drained before this returns.
///
/// ⚠ **`SystemState`, deliberately, and not a callback or a context bag.** The
/// eager side is a system with `SystemParam`s and this side is `&mut World`;
/// `SystemState` is Bevy's bridge between exactly those two, and it keeps the
/// shared operation a plain `SystemParam` that the eager system can take
/// directly. A callback would have inverted control to hide a borrow, and a
/// context bag would have re-listed every param — which is the thing being
/// deleted.
fn commit_transition(
    world: &mut World,
    // ⭐⭐ **THE PLAN THE READINESS TRANSACTION AUTHORIZED** (D71, 2026-08-14).
    // This function used to call `RoomConstructionPlan::prepare` ITSELF, which
    // made the transaction a permission bit: readiness accounted for the assets
    // of one plan and the world was built from another, prepared a frame later
    // and possibly under different content. The transaction's own plan is the
    // only one whose assets were proven present.
    plan: &RoomConstructionPlan,
    subject: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    target_room: &str,
    arrival: ae::Vec2,
    edge_exit: bool,
    zone_sfx: Option<&str>,
) -> CommitOutcome {
    let Some(target_index) = ({
        let mut rooms = world.query::<&ambition_platformer2d_actor_monolith::rooms::RoomSet>();
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

    // ⛔ **stale spawn requests first.** A speculative frame may have enqueued
    // `SpawnActorRequest`s that its rollback never un-enqueued, and this path
    // DRAINS the queue below rather than leaving it to a scheduled system. An
    // ordinary frame has no such backlog; an exclusive commit after a rewind can.
    if let Some(mut pending) = world.get_resource_mut::<bevy::ecs::message::Messages<
        ambition_platformer2d_actor_monolith::features::SpawnActorRequest,
    >>() {
        pending.clear();
    }

    // ⚠ **`SystemState::get_mut` PANICS on a missing resource**, and the shared
    // application requires a dozen — where this function used to reach each one
    // through an `Option`-guarded `get_resource_mut`. That escalation is safe for
    // a reason worth stating rather than trusting: reaching here at all means
    // `authorized_plan` found a `CommitAuthorized` transaction, which only exists
    // if `RoomTransitionPlugin` is installed — and that plugin also installs
    // `commit_ready_room_transition_system`, which has taken the same parameters
    // as a plain system all along. A host that could panic here could not have
    // produced the authorization that got here.
    let mut state: bevy::ecs::system::SystemState<
        crate::room_transition::RoomTransitionApplication,
    > = bevy::ecs::system::SystemState::new(world);
    let outcome = {
        let mut application = state.get_mut(world);
        match application.subject_entity(subject) {
            None => Err(crate::room_transition::RoomTransitionApplyError::SubjectGone),
            Some(entity) => {
                application.apply(plan, entity, target_index, arrival, edge_exit, zone_sfx)
            }
        }
    };
    // ⛔ **unconditionally, before any early return.** `SystemState` holds the
    // command queue the application wrote into; dropping it without applying
    // would silently discard a half-built room.
    state.apply(world);

    match outcome {
        Ok(_) => {}
        Err(error) => {
            // Every variant is raised BEFORE the first destructive mutation, so
            // the world is still whole and the crossing is simply void.
            error!("Track B: {error}; cancelling the crossing");
            return CommitOutcome::Cancelled;
        }
    }

    // ── THE EXCLUSIVE-WORLD TAIL ─────────────────────────────────────────────
    // An eager commit's spawns are applied at the frame's flush and its actor
    // requests are drained by a scheduled system. This path runs outside that
    // schedule, so it does both itself, here, synchronously — which is the only
    // thing about a confirmed commit that is not the shared operation.
    world.flush();
    let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
        &mut *world,
        ambition_platformer2d_actor_monolith::features::apply_spawn_actor_requests,
    );
    world.flush();

    CommitOutcome::Committed
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    use crate::room_transition::{
        ActiveRoomTransitionLoad, RoomTransitionLoadPhase, RoomTransitionLoadState,
    };
    /// GPT review #1/#2: the deferred transition transports the body that CROSSED
    /// the exit — resolved by its recorded, rollback-stable `SimId` — and NEVER
    /// substitutes another body. A recorded id that has since despawned resolves
    /// to `None` (a cancelled crossing), NOT the home player: substituting the
    /// primary teleports a body that never touched the exit into the target room.
    /// The home player right next to the triggerer is the tempting wrong answer
    /// this pins against.
    ///
    /// The third historical case — an *unstamped* trigger — is no longer
    /// representable: `LifecycleIntent::Transition.subject` is a `SimId`, not an
    /// `Option<SimId>`, so a body without stable identity cannot produce a
    /// deferred intent at all. The type is the proof; there is nothing left to
    /// assert.
    ///
    /// ⭐ **asked of the ONE resolver, which is the point** (D71, 2026-08-14).
    /// This used to call `resolve_transition_subject`, a private twin of
    /// `TransitBodies::subject_entity` that documented itself as mirroring it.
    /// Both are gone into `RoomTransitionApplication::subject_entity`, so this
    /// invariant is now asserted about the resolver BOTH hosts use rather than
    /// about one host's copy of it.
    use ambition_platformer2d_actor_monolith::session::lifecycle_commit::RoomTransitionIntent;

    fn intent_to(target_room: &str, subject: SimId) -> RoomTransitionIntent {
        RoomTransitionIntent {
            subject,
            target_room: target_room.to_string(),
            arrival: ae::Vec2::ZERO,
            edge_exit: true,
            zone_sfx: None,
        }
    }

    fn authorized_transaction(intent: RoomTransitionIntent) -> ActiveRoomTransitionLoad {
        ActiveRoomTransitionLoad {
            sequence: 1,
            content_epoch: 1,
            session_scope: None,
            source_room: 0,
            source_room_id: "a".to_string(),
            target_room_id: intent.target_room.clone(),
            target_room: 1,
            intent,
            construction_plan: None,
            barrier: ambition_load::LoadBarrierRef::new("load", "ready"),
            commit_not_before_tick: 0,
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

    /// ⛔⛔ **A CANCELLED CROSSING MUST NOT LEAVE ITS TRANSACTION BEHIND.**
    ///
    /// A void crossing — the body that walked through the door died during the
    /// confirmation delay — drops the intent. It used to drop ONLY the intent,
    /// leaving the authorized transaction resident in `CommitAuthorized` with its
    /// load barrier never retired. Nothing would ever come back for it:
    /// `begin_room_transition_load_system` returns early whenever no intent is
    /// pending, so the orphan simply sat there — and the next crossing to the
    /// same destination matches `same_destination`, returns early against the
    /// orphan, and commits under a plan prepared for a crossing that was
    /// cancelled.
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

    /// ⛔⛔ **A COVERED CONFIRMED COMMIT MUST NOT DROP ITS OWN TRANSACTION.**
    ///
    /// The cover, the settle wait and the ONLY latency instrument the game has
    /// all hang off `drive_room_transition_presentation`'s
    /// `phase == Committed` gate, and this route used to null `active` in
    /// `PreUpdate` instead — so on the shipped host the adapter took its
    /// "no transaction" teardown branch, the cover came down the frame the room
    /// was built rather than the frame it was DRAWN, and
    /// `RoomTransitionTelemetry` recorded zero samples.
    ///
    /// ⚠ `committed_at` is asserted because it is not decoration: the adapter's
    /// give-up deadline is measured from it, and a `None` reads as zero elapsed
    /// forever — a black screen instead of a reveal with a warning.
    ///
    /// ⛔ **and the uncovered half is the poison, in the same test.** Without it
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

        let handed_over = covered
            .resource::<RoomTransitionLoadState>()
            .active
            .as_ref()
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

        // ⛔ THE POISON: no cover means no adapter, so nobody else will ever
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

    /// ⛔ **and ONLY the one it opened.** A crossing is cancelled because its body
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

    #[test]
    fn a_missing_transition_subject_resolves_to_none_never_a_substitute() {
        let mut world = World::new();
        let triggerer = world.spawn(SimId::placement("triggerer")).id();
        let primary = world
            .spawn((SimId::player_slot(0), PlayerEntity, PrimaryPlayer))
            .id();
        assert_ne!(triggerer, primary);

        // ⚠ `TransitBodies`, not the whole `RoomTransitionApplication`: the
        // resolver is the only thing under test and it is pure queries, so a
        // bare `World` can host it. Building the full application here would
        // demand a dozen unrelated resources — a fixture that models nothing.
        // `RoomTransitionApplication::subject_entity` delegates straight to this.
        let mut state: bevy::ecs::system::SystemState<crate::room_transition::TransitBodies> =
            bevy::ecs::system::SystemState::new(&mut world);
        let bodies = state.get_mut(&mut world);

        assert_eq!(
            bodies.subject_entity(&SimId::placement("triggerer")),
            Some(triggerer),
            "the recorded triggering SimId is transported, not the current primary"
        );
        assert_eq!(
            bodies.subject_entity(&SimId::placement("gone")),
            None,
            "a recorded body that despawned before commit is a void crossing, \
             not a licence to teleport the home player"
        );
    }
}
