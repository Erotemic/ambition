//! Confirmed-frame lifecycle commit (Track B, Piece 2).
//!
//! The sim side records a [`PendingLifecycleCommit`] instead of executing a
//! room-lifecycle op on a speculative frame (Piece 1, in `ambition_actors`).
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

use ambition_actors::session::lifecycle_commit::{
    LifecycleIntent, PendingIntent, PendingLifecycleCommit,
};
use ambition_actors::time::feel::SandboxFeelTuning;
use ambition_actors::world::rooms::RoomConstructionPlan;
use ambition_actors::SandboxSimState;
use ambition_engine_core as ae;
use ambition_engine_core::ConfirmedFrameBoundary;

use crate::rollback::{start_sync_test_session, RollbackSessionOwnership};

/// Execute a confirmed deferred lifecycle op in the exclusive world and rebase.
///
/// No-op unless (a) a rollback host is installed (`ConfirmedFrameBoundary`
/// present), (b) it is a `LocalSyncTest` session we may rebase, and (c) a pending
/// intent exists whose recording frame is confirmed.
pub fn commit_confirmed_lifecycle(world: &mut World) {
    let Some(boundary) = world.get_resource::<ConfirmedFrameBoundary>().copied() else {
        return;
    };
    let Some(RollbackSessionOwnership::LocalSyncTest(settings)) =
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

    execute_lifecycle_commit(world, &kind);

    // Clear the slot so the post-op world (about to become the new baseline)
    // carries no pending intent.
    if let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>() {
        pending.take();
    }

    // Rebase: the post-op world becomes the new frame-zero baseline. This bumps
    // the session generation and the first frame-zero SaveWorld overwrites every
    // ring slot, so no earlier frame can restore the pre-op room. Executing the
    // op WITHOUT rebasing would leave old ring history restorable — the rebase is
    // the load-bearing half of the confirmed authoritative discontinuity.
    if let Err(error) = start_sync_test_session(world, settings) {
        error!("Track B: failed to rebase the session after a lifecycle commit: {error}");
    }
}

fn execute_lifecycle_commit(world: &mut World, kind: &LifecycleIntent) {
    match kind {
        LifecycleIntent::Transition {
            target_room,
            arrival,
            edge_exit,
        } => commit_transition(world, target_room, *arrival, *edge_exit),
        // The in-place resets (death / manual / replay) are already rollback-safe
        // executed eagerly, and the full sandbox reset is not yet deferred, so no
        // consumer records these variants yet. They are handled here as a no-op so
        // the match stays exhaustive as Track B extends deferral to them.
        LifecycleIntent::DeathReset
        | LifecycleIntent::ManualReset
        | LifecycleIntent::Replay
        | LifecycleIntent::FullReset => {}
    }
}

/// Reconstruct the target room synchronously and relocate the controlled body,
/// mirroring `commit_room_transition_geometry` but in one exclusive-world call.
fn commit_transition(world: &mut World, target_room: &str, arrival: ae::Vec2, edge_exit: bool) {
    // Preparation is mutation-free and fallible — every room/content lookup
    // happens here, before any world mutation.
    let plan = match RoomConstructionPlan::prepare(world, target_room) {
        Ok(plan) => plan,
        Err(error) => {
            error!("Track B: transition commit could not prepare room {target_room:?}: {error:?}");
            return;
        }
    };

    // Retire the source roster, publish the target geometry, spawn the target
    // roster — synchronously, with `world.flush()` between phases.
    plan.apply_to_world(world);

    // Relocate the controlled body to the arrival point in the new room. Without
    // this the body would sit on the source-room exit coordinates and immediately
    // re-trigger a transition.
    {
        let mut query = world.query_filtered::<(
            ae::BodyClusterQueryData,
            &mut ambition_actors::features::MotionModel,
        ), ambition_actors::actor::PrimaryPlayerOnly>();
        if let Ok((mut cluster_item, mut motion_model)) = query.single_mut(world) {
            let mut clusters = cluster_item.as_clusters_mut();
            ae::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                arrival,
                ae::movement::TransitVelocity::Zero,
            );
        }
    }

    // Set the transition cooldown (rollback state, so it folds into the rebased
    // baseline) so detection does not immediately re-fire.
    let cooldown = world
        .get_resource::<SandboxFeelTuning>()
        .map(|feel| {
            if edge_exit {
                feel.edge_transition_cooldown
            } else {
                feel.door_transition_cooldown
            }
        })
        .unwrap_or(0.0);
    if let Some(mut sim_state) = world.get_resource_mut::<SandboxSimState>() {
        sim_state.room_transition_cooldown = cooldown;
    }
}
