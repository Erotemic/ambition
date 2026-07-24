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

    // Never rebase over an already-diverged session. `start_sync_test_session`
    // installs a fresh `RollbackSessionStatus`, which would ERASE a
    // `SyncTestMismatch` reported during THIS same update — and the confirmation
    // and the mismatch both fire at the check horizon, so they coincide. If the
    // old timeline is unhealthy, leave the diagnostic visible and do not commit;
    // a rebase must never launder a divergence into a clean baseline.
    if crate::rollback::session_health(world).is_err() {
        return;
    }

    // Atomic commit: only clear the intent and rebase if the reconstruction
    // ACTUALLY happened. A failed commit (e.g. the target room fails to prepare)
    // must not advertise an authoritative discontinuity that never occurred, nor
    // silently lose the request — leave the intent pending to retry on a later
    // confirmed frame.
    if !execute_lifecycle_commit(world, &kind) {
        return;
    }

    // Clear the slot so the post-op world (the new baseline) carries no pending
    // intent.
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

/// Returns `true` iff the reconstruction actually committed (so the caller may
/// clear the intent and rebase). `false` means the op could not complete and the
/// intent must stay pending.
fn execute_lifecycle_commit(world: &mut World, kind: &LifecycleIntent) -> bool {
    match kind {
        LifecycleIntent::Transition {
            subject,
            target_room,
            arrival,
            edge_exit,
        } => commit_transition(world, subject, target_room, *arrival, *edge_exit),
        // The in-place resets (death / manual / replay) are already rollback-safe
        // executed eagerly, and the full sandbox reset was proven rollback-safe
        // single-tick, so no consumer records these variants. Not committed here
        // (returning `false` keeps a stray intent pending rather than laundering a
        // rebase for a no-op); the match stays exhaustive if deferral extends.
        LifecycleIntent::DeathReset
        | LifecycleIntent::ManualReset
        | LifecycleIntent::Replay
        | LifecycleIntent::FullReset => false,
    }
}

/// Resolve the body a deferred transition should transport.
///
/// The recorded [`SimId`] is the body that CROSSED the exit (finding 2) — look it
/// up by exact identity. If it is gone by commit time (a possessed actor that
/// died, or the trigger despawned during the confirmation delay) or the trigger
/// carried no id, fall back to the PRIMARY player: the persistent home body, and
/// where control returns when a possessed body dies. Deliberately NOT the live
/// `ControlledSubject` — re-resolving current control at commit is exactly the
/// bug finding 2 names, because possession may have changed since the trigger.
fn resolve_transition_subject(
    world: &mut World,
    subject: &Option<ambition_platformer_primitives::sim_id::SimId>,
) -> Option<Entity> {
    if let Some(sim_id) = subject {
        let mut ids = world.query::<(Entity, &ambition_platformer_primitives::sim_id::SimId)>();
        if let Some((entity, _)) = ids.iter(world).find(|(_, id)| *id == sim_id) {
            return Some(entity);
        }
    }
    let mut primary = world.query_filtered::<Entity, ambition_actors::actor::PrimaryPlayerOnly>();
    primary.iter(world).next()
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
    subject: &Option<ambition_platformer_primitives::sim_id::SimId>,
    target_room: &str,
    arrival: ae::Vec2,
    edge_exit: bool,
) -> bool {
    // Preparation is mutation-free and fallible — every room/content lookup
    // happens here, before any world mutation. A failure commits NOTHING (the
    // caller keeps the intent pending).
    let plan = match RoomConstructionPlan::prepare(world, target_room) {
        Ok(plan) => plan,
        Err(error) => {
            error!("Track B: transition commit could not prepare room {target_room:?}: {error:?}");
            return false;
        }
    };

    // Resolve + PREFLIGHT the subject BEFORE any destructive mutation (findings 2
    // and 3): everything that can fail must fail with the world still whole.
    let Some(subject) = resolve_transition_subject(world, subject) else {
        error!("Track B: transition commit found no body to transit");
        return false;
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
            return false;
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
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_platformer_primitives::sim_id::SimId;

    /// GPT review finding 2: the deferred transition transports the body that
    /// CROSSED the exit — resolved by its recorded, rollback-stable `SimId` — and
    /// never re-resolves whatever is controlled at commit time. A recorded id that
    /// has since despawned, or an unstamped trigger, falls back to the primary
    /// player (the persistent home body, where control returns after a possessed
    /// body dies).
    #[test]
    fn the_transition_subject_is_the_recorded_id_not_current_control() {
        let mut world = World::new();
        let triggerer = world.spawn(SimId::placement("triggerer")).id();
        let primary = world
            .spawn((SimId::player_slot(0), PlayerEntity, PrimaryPlayer))
            .id();
        assert_ne!(triggerer, primary);

        assert_eq!(
            resolve_transition_subject(&mut world, &Some(SimId::placement("triggerer"))),
            Some(triggerer),
            "the recorded triggering SimId is transported, not the current primary"
        );
        assert_eq!(
            resolve_transition_subject(&mut world, &Some(SimId::placement("gone"))),
            Some(primary),
            "a recorded body that despawned before commit falls back to the primary"
        );
        assert_eq!(
            resolve_transition_subject(&mut world, &None),
            Some(primary),
            "an unstamped trigger uses the primary player"
        );
    }
}
