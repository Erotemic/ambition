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

    // Never rebase over an already-diverged session. `start_sync_test_session`
    // installs a fresh `RollbackSessionStatus`, which would ERASE a
    // `SyncTestMismatch` reported during THIS same update — and the confirmation
    // and the mismatch both fire at the check horizon, so they coincide. If the
    // old timeline is unhealthy, leave the diagnostic visible and do not commit;
    // a rebase must never launder a divergence into a clean baseline.
    if crate::rollback::session_health(world).is_err() {
        return;
    }

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
            error!("Track B: failed to BUILD the rebase session; leaving the room and the pending intent untouched: {error}");
            return;
        }
    };

    // Now the fallible work is done. Reconstruct the room (also atomic: it
    // prepares + preflights before its own infallible `apply_to_world`).
    match execute_lifecycle_commit(world, &kind) {
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
    install_rebased_sync_test_session(world, session, settings);
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

fn execute_lifecycle_commit(world: &mut World, kind: &LifecycleIntent) -> CommitOutcome {
    match kind {
        LifecycleIntent::Transition {
            subject,
            target_room,
            arrival,
            edge_exit,
        } => commit_transition(world, subject, target_room, *arrival, *edge_exit),
        // The in-place resets (death / manual / replay) are already rollback-safe
        // executed eagerly, and the full sandbox reset was proven rollback-safe
        // single-tick, so no consumer records these variants. Keep a stray intent
        // pending rather than laundering a rebase for a no-op; the match stays
        // exhaustive if deferral extends.
        LifecycleIntent::DeathReset
        | LifecycleIntent::ManualReset
        | LifecycleIntent::Replay
        | LifecycleIntent::FullReset => CommitOutcome::Retry,
    }
}

/// Resolve the EXACT body a deferred transition recorded, or `None`.
///
/// The recorded [`SimId`] is the body that CROSSED the exit (GPT review #2) —
/// look it up by exact identity and transit THAT body or nothing. It never
/// substitutes another entity:
///
/// * An id that still resolves → that body.
/// * An id that no longer resolves → `None`. The crossing body is gone (it
///   died during the confirmation delay). Substituting the home player — as this
///   once did — teleports a body that never touched the exit into the target
///   room, silently moving the primary into a room the player never walked to
///   (GPT review #1). A possessed body now un-room-scopes on possession (it can
///   navigate rooms as itself), so the only way to reach this arm is genuine
///   death, and a dead crossing is a VOID crossing, not a licence to move
///   someone else.
/// The caller maps `None` to a CANCELLED commit: the intent is dropped and the
/// source room stays authoritative. Deliberately never the live
/// `ControlledSubject`, because possession may have changed since the trigger.
fn resolve_transition_subject(
    world: &mut World,
    subject: &ambition_platformer_primitives::sim_id::SimId,
) -> Option<Entity> {
    let mut ids = world.query::<(Entity, &ambition_platformer_primitives::sim_id::SimId)>();
    ids.iter(world)
        .find(|(_, id)| *id == subject)
        .map(|(entity, _)| entity)
}

/// Reconstruct the target room synchronously and apply the CANONICAL transition
/// body semantics to the TRIGGERING body — faithful to
/// `commit_room_transition_geometry` (`world/rooms/load.rs`) +
/// `apply_room_transition_resets` (`app/world_flow/room_flow.rs`), which this
/// mirrors so a deferred transition behaves like an eager one. Kept in sync with
/// those by the line comments below; a change there without a matching change
/// here is a regression.
fn commit_transition(
    world: &mut World,
    subject: &ambition_platformer_primitives::sim_id::SimId,
    target_room: &str,
    arrival: ae::Vec2,
    edge_exit: bool,
) -> CommitOutcome {
    // Preparation is mutation-free and fallible — every room/content lookup
    // happens here, before any world mutation. A failure commits NOTHING and is
    // treated as TRANSIENT (the target room may become preparable later), so the
    // caller keeps the intent pending.
    let plan = match RoomConstructionPlan::prepare(world, target_room) {
        Ok(plan) => plan,
        Err(error) => {
            error!("Track B: transition commit could not prepare room {target_room:?}: {error:?}");
            return CommitOutcome::Retry;
        }
    };

    // Resolve + PREFLIGHT the subject BEFORE any destructive mutation (GPT review
    // #1/#2): everything that can fail must fail with the world still whole. A
    // subject that no longer resolves is a VOID crossing — the body that crossed
    // is gone — so the intent is CANCELLED (dropped), never substituted with
    // another body and never retried forever.
    let Some(subject) = resolve_transition_subject(world, subject) else {
        error!(
            "Track B: the recorded transition subject is gone; cancelling the crossing \
             (no substitute body is transited)"
        );
        return CommitOutcome::Cancelled;
    };
    // The full body-transit contract, checked NOW: the subject must be a live
    // body carrying the exact components the transit below mutates. A body that
    // fails this is rejected BEFORE `apply_to_world` retires the old room — never
    // logged-and-succeeded after the room is already gone (finding 3). It rides
    // through the reconstruction (carried if room-scoped, otherwise session-
    // scoped), so passing here means the transit after `apply_to_world` succeeds.
    {
        let mut transit = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_actors::features::MotionModel,
        )>();
        if transit.get_mut(world, subject).is_err() {
            error!(
                "Track B: transition subject fails the body-transit contract; \
                 cancelling before any reconstruction"
            );
            return CommitOutcome::Cancelled;
        }
    }
    let carry_body = world
        .get::<ambition_platformer_primitives::lifecycle::RoomScopedEntity>(subject)
        .map(|_| subject);

    // Retire the source roster (exempting the carried body), publish the target
    // geometry, spawn the target roster — synchronously, with `world.flush()`.
    // From here nothing may fail: the subject and its transit contract were
    // validated above.
    plan.apply_to_world(world, carry_body);

    // Tuning snapshots (primitive copies, so no borrow is held across the body
    // mutation below).
    let air_jumps = world
        .get_resource::<ae::ActiveMovementTuning>()
        .map(|tuning| tuning.0.air_jumps)
        .unwrap_or(0);
    let (edge_cd, door_cd, edge_flash, door_flash) = world
        .get_resource::<SandboxFeelTuning>()
        .map(|feel| {
            (
                feel.edge_transition_cooldown,
                feel.door_transition_cooldown,
                feel.edge_transition_flash,
                feel.door_transition_flash,
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    // Validate the authored arrival against the (now target) geometry using the
    // body's size — the same `validated_spawn` guard the canonical path applies,
    // so the body is never placed inside a solid or out of bounds.
    let player_size = world
        .get::<ambition_platformer_primitives::body::BodyKinematics>(subject)
        .map(|kin| kin.size)
        .unwrap_or_else(ae::default_player_body_size);
    let arrival = ambition_platformer_primitives::lifecycle::session_world_component::<
        ae::RoomGeometry,
    >(world)
    .map(|geometry| ambition_world::rooms::validated_spawn(&geometry.0, arrival, player_size))
    .unwrap_or(arrival);

    // Body transit on the CONTROLLED subject (load.rs:55-80): reset clusters to
    // the arrival, refresh jump/dash/flight, and preserve edge-exit momentum.
    {
        let mut query = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_actors::features::MotionModel,
        )>();
        if let Ok((mut cluster_item, mut motion_model)) = query.get_mut(world, subject) {
            let mut clusters = cluster_item.as_clusters_mut();
            let old_velocity = clusters.kinematics.vel;
            let fly_enabled = clusters.flight.fly_enabled;
            ae::reset_body_clusters(&mut motion_model, &mut clusters, arrival);
            ae::refresh_movement_resources_clusters(
                clusters.abilities,
                &mut clusters.dash,
                &mut clusters.jump,
                air_jumps,
            );
            clusters.flight.fly_enabled = fly_enabled && clusters.abilities.abilities.fly;
            if edge_exit {
                clusters.kinematics.vel = old_velocity;
            }
        } else {
            // UNREACHABLE after the preflight validated this exact query on this
            // exact subject. If it ever fires, a carried body lost its transit
            // components during reconstruction — an invariant violation, not a
            // normal partial-failure outcome.
            error!(
                "Track B: BUG — a preflighted transit subject lost its components \
                 during reconstruction"
            );
        }
    }

    // Cross-domain per-transition resets (room_flow.rs:46-68), each a separate
    // borrow so no query aliases. Optional components (safety/blink) are absent
    // for a possessed non-home body, exactly as the canonical path allows.
    if let Some(mut combat) = world.get_mut::<ambition_characters::actor::BodyCombat>(subject) {
        combat.hit_flash = if edge_exit { edge_flash } else { door_flash };
        combat.hitstop_timer = 0.0;
        combat.damage_invuln_timer = 0.0;
        combat.hitstun_timer = 0.0;
        combat.recoil_lock_timer = 0.0;
    }
    if let Some(mut safety) = world.get_mut::<ambition_actors::avatar::PlayerSafetyState>(subject) {
        safety.last_safe_pos = arrival;
    }
    if let Some(mut blink) =
        world.get_mut::<ambition_actors::avatar::PlayerBlinkCameraState>(subject)
    {
        blink.blink_in_timer = 0.0;
        blink.blink_camera_from = arrival;
        blink.blink_camera_to = arrival;
        blink.camera_snap_timer = if edge_exit {
            0.0
        } else {
            ambition_actors::ROOM_DOOR_CAMERA_SNAP_TIME
        };
    }

    // Reset the sim clock (load.rs:81), close any open dialogue (room_flow.rs:68),
    // flash the dev preset marker (load.rs:90), and set the transition cooldown
    // (load.rs:85) so detection does not immediately re-fire.
    if let Some(mut clock) = world
        .get_resource_mut::<bevy::ecs::message::Messages<
            ambition_actors::time::time_control::ClockResetRequest,
        >>()
    {
        clock.write(
            ambition_actors::time::time_control::ClockResetRequest::sim_clock(
                ambition_actors::time::time_control::ClockRequester::Engine,
                "room_transition",
            ),
        );
    }
    if let Some(mut dialogue) = world.get_resource_mut::<ambition_dialog::DialogState>() {
        dialogue.close();
    }
    if let Some(mut dev_state) = world.get_resource_mut::<ambition_dev_tools::SandboxDevState>() {
        dev_state.preset_flash = 1.0;
    }
    if let Some(mut sim_state) = world.get_resource_mut::<SandboxSimState>() {
        sim_state.room_transition_cooldown = if edge_exit { edge_cd } else { door_cd };
    }

    // NOTE (bounded gap): the canonical path also emits the transition Reset
    // SFX/VFX. Presentation effects on the confirmed-commit host path go through
    // the external-effect quarantine with different timing, so they are
    // deliberately NOT emitted here; this is a presentation-only difference, not a
    // state divergence. Tracked in the campaign doc.
    CommitOutcome::Committed
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_platformer_primitives::sim_id::SimId;

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
    #[test]
    fn a_missing_transition_subject_resolves_to_none_never_a_substitute() {
        let mut world = World::new();
        let triggerer = world.spawn(SimId::placement("triggerer")).id();
        let primary = world
            .spawn((SimId::player_slot(0), PlayerEntity, PrimaryPlayer))
            .id();
        assert_ne!(triggerer, primary);

        assert_eq!(
            resolve_transition_subject(&mut world, &SimId::placement("triggerer")),
            Some(triggerer),
            "the recorded triggering SimId is transported, not the current primary"
        );
        assert_eq!(
            resolve_transition_subject(&mut world, &SimId::placement("gone")),
            None,
            "a recorded body that despawned before commit is a void crossing, \
             not a licence to teleport the home player"
        );
    }
}
