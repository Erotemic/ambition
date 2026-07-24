//! **Track B, T1b — the owed RECONSTRUCTION reproduction.**
//!
//! The in-place reset (`rollback_lifecycle_reset.rs`) mutates SURVIVING entities
//! and is rollback-safe. A room TRANSITION is different: it despawns the whole
//! source roster and spawns the target roster via Commands (reconstruction), and
//! the multi-tick transition transaction machinery
//! (`RoomTransitionLoadState` / `RoomTransitionContentEpoch` / `LoadCoordinator`,
//! installed as plain resources in `app/plugins.rs`) is NOT rollback-registered.
//! The entity/room state it drives (`RoomSet`, `RoomGeometry`, `FeatureSimEntity`,
//! `room_transition_cooldown`) IS rollback state. So a GGRS rollback that
//! straddles the transaction's tick span rewinds the world but not the
//! transaction phase/tick barrier — the commit can then fire at a different tick,
//! twice, or fail to reproduce, diverging the sync-test checksum. That is the
//! reconstruction divergence Track B names as still owed.
//!
//! This test walks the controlled body through the east `EdgeExit` of the
//! calibration lab into the adjacent boss room INSIDE a forced sync-test rollback
//! window. It encodes the invariant: the transition actually occurs (the active
//! room flips, the source roster is gone, a fresh target roster is present ⇒ new
//! entity generations) AND the sim stays checksum-clean across and past the
//! commit.
//!
//! **History.** It was written RED (2026-07-23): `GGRS sync-test checksum mismatch
//! at frames [9, 10, 11]` while `active == combat_calibration_lab` — the
//! divergence landed in the transition LOAD phase, as the body overlapped the
//! exit and the multi-tick transaction engaged, BEFORE the room flipped, because
//! the transaction machinery ran eagerly on a speculative frame while its own
//! progress state is not rollback-registered. It is now GREEN under **Track B**:
//! `detect_room_transition_system` records a `LifecycleIntent::Transition` under a
//! rollback host instead of engaging the load machine, and
//! `lifecycle_commit::commit_confirmed_lifecycle` reconstructs the target room in
//! the exclusive world and rebases the session once the frame is confirmed — so
//! the reconstruction never runs on a speculative frame and resim cannot diverge.
//! The same room brawled 2400 frames clean (`rollback_lifecycle_reset`).

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

/// `combat_calibration_lab` is the harness room the in-place reset tests already
/// prove builds+runs; its right-edge `EdgeExit` (`combat_lab_to_boss`, LDtk px
/// [1264,560] w16×h176) leads to `first_system_boss`.
const SOURCE_ROOM: &str = "combat_calibration_lab";
const TARGET_ROOM: &str = "first_system_boss";

fn repro_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room(SOURCE_ROOM)
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Every room-scoped feature/actor entity currently spawned. A transition
/// despawns this whole set and spawns a fresh one; because despawn bumps the
/// generation, no source `Entity` value can equal a post-transition one.
fn feature_roster(sim: &mut SandboxSim) -> HashSet<Entity> {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<Entity, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    q.iter(world).collect()
}

fn player_y(sim: &mut SandboxSim) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition::platformer::body::BodyKinematics, With<ambition::platformer::markers::PrimaryPlayer>>();
    q.single(world).map(|k| k.pos.y).unwrap_or(0.0)
}

/// The rollback session generation — bumped once per session rebase, so a Track B
/// confirmed lifecycle commit advances it exactly once.
fn session_generation(sim: &SandboxSim) -> u64 {
    sim.world()
        .get_resource::<ambition::engine_core::ConfirmedFrameBoundary>()
        .map(|boundary| boundary.session)
        .unwrap_or(0)
}

/// Whether a deferred lifecycle intent is currently recorded (rollback state).
fn intent_pending(sim: &SandboxSim) -> bool {
    sim.world()
        .get_resource::<ambition::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .is_some_and(|slot| slot.pending.is_some())
}

#[test]
fn a_room_transition_survives_the_rollback_window() {
    let mut sim = repro_sim();

    // Baseline: we start in the source room, checksum-clean.
    let start = sim.step(AgentAction::default());
    assert_eq!(
        start.active_room.as_str(),
        SOURCE_ROOM,
        "the sim starts in the source room"
    );
    sim.rollback_health().expect("clean before staging");

    // Stage the body just west of the east EdgeExit (past the hazard band that
    // sits west of x≈720); teleport auto-rebases, folding this into the baseline.
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    // Record the source roster so we can prove it is gone after the transition.
    let source_roster = feature_roster(&mut sim);
    assert!(!source_roster.is_empty(), "the source room has a roster");

    // Walk right into the EdgeExit. The transition commits ~2 sim ticks after the
    // overlap frame, all inside the forced rollback window.
    let mut transitioned_at = None;
    for frame in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (active={}): {error}", obs.active_room));
        if obs.active_room.as_str() == TARGET_ROOM {
            transitioned_at = Some(frame);
            break;
        }
    }
    let transitioned_at = transitioned_at.expect(
        "the body should have crossed the east EdgeExit into the target room within 240 frames",
    );

    // Prove the reconstruction happened: target room active, source roster gone,
    // a fresh target roster present (despawn+respawn ⇒ disjoint entity ids).
    let target_roster = feature_roster(&mut sim);
    assert!(
        !target_roster.is_empty(),
        "the target room spawned a roster"
    );
    assert!(
        source_roster.is_disjoint(&target_roster),
        "the transition despawned the source roster and spawned fresh entities \
         (source={} target={} shared={})",
        source_roster.len(),
        target_roster.len(),
        source_roster.intersection(&target_roster).count(),
    );

    // Keep running well past the commit, still checksum-clean.
    for frame in 0..180 {
        let obs = sim.step(AgentAction::default());
        sim.rollback_health().unwrap_or_else(|error| {
            panic!(
                "post-transition frame {frame} (t+{transitioned_at}, active={}): {error}",
                obs.active_room
            )
        });
    }
}

/// **Track B principal timeline oracle (T6).** The deferred lifecycle intent must
/// be RECORDED but NOT EXECUTED while its frame is still predicted, then COMMIT
/// EXACTLY ONCE on confirmation — bumping the session generation — after which the
/// slot is empty and no second commit ever fires.
///
/// (Corrected-input cancellation — a mispredicted intent rewinding away with the
/// world — is NOT proven here and is NOT unit-tested. It FOLLOWS from
/// `PendingLifecycleCommit` being rollback-registered state, so a rewind restores
/// its pre-intent value, plus the codec round-trip — but a `LocalSyncTest`
/// re-simulates with identical input and cannot mispredict, so end-to-end
/// cancellation belongs to the External/P2P work. The
/// `ambition_actors::session::lifecycle_commit` unit tests cover only
/// earliest-sticky recording, the confirmation comparison, and `take()`.)
#[test]
fn a_transition_intent_is_recorded_then_committed_exactly_once() {
    let mut sim = repro_sim();
    let start = sim.step(AgentAction::default());
    assert_eq!(start.active_room.as_str(), SOURCE_ROOM);

    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));
    // Captured AFTER the teleport's own rebase, so the transition commit is the
    // only generation bump we are counting.
    let generation_before = session_generation(&sim);

    // Walk into the exit. While predicted, the intent is recorded and the room has
    // NOT reconstructed; on confirmation the committer flips the room.
    let mut recorded_while_still_in_source = false;
    let mut committed_at = None;
    for frame in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (active={}): {error}", obs.active_room));
        if intent_pending(&sim) && obs.active_room.as_str() == SOURCE_ROOM {
            // Deferred, not eager: an intent exists but the room is untouched.
            recorded_while_still_in_source = true;
        }
        if obs.active_room.as_str() == TARGET_ROOM {
            committed_at = Some(frame);
            break;
        }
    }
    committed_at.expect("the deferred transition should have committed within 240 frames");

    assert!(
        recorded_while_still_in_source,
        "the intent must be recorded while its frame is still predicted, without \
         reconstructing the room — deferral, not eager execution"
    );
    let generation_after = session_generation(&sim);
    assert_eq!(
        generation_after,
        generation_before + 1,
        "the confirmed commit rebased the session exactly once"
    );
    assert!(
        !intent_pending(&sim),
        "the committer cleared the slot, so the intent cannot re-fire"
    );

    // No second commit: the generation holds and the room stays put, clean.
    let generation_committed = session_generation(&sim);
    for frame in 0..120 {
        let obs = sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("post-commit frame {frame}: {error}"));
        assert_eq!(
            obs.active_room.as_str(),
            TARGET_ROOM,
            "the room stays committed to the target"
        );
    }
    assert_eq!(
        session_generation(&sim),
        generation_committed,
        "no second rebase — the lifecycle op committed exactly once"
    );
}

/// **Finding 1 — edge-exit momentum is preserved.** The canonical transition
/// keeps the body's velocity across an `EdgeExit` (you flow through the seam, you
/// don't stop). The reduced committer zeroed it; the faithful committer restores
/// it. A zeroing regression leaves the arrival velocity at 0.
#[test]
fn an_edge_exit_transition_preserves_the_body_momentum() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    let mut vel_in_source = 0.0f32;
    let mut committed = false;
    for _ in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame (active={}): {error}", obs.active_room));
        if obs.active_room.as_str() == SOURCE_ROOM {
            // The last rightward speed carried into the EdgeExit.
            vel_in_source = obs.player_vel.0;
        } else if obs.active_room.as_str() == TARGET_ROOM {
            assert!(
                vel_in_source > 1.0,
                "sanity: the body was actually moving into the exit ({vel_in_source})"
            );
            assert!(
                obs.player_vel.0 > vel_in_source * 0.5,
                "edge-exit momentum must survive the transition \
                 (into-exit={vel_in_source}, arrival={}) — a zeroing bug leaves 0",
                obs.player_vel.0
            );
            committed = true;
            break;
        }
    }
    assert!(committed, "the edge-exit transition committed");
}

/// **Finding 2 poison test.** A lifecycle rebase installs a fresh
/// `RollbackSessionStatus`, which would ERASE a `SyncTestMismatch` reported on
/// the same update the intent confirms — laundering a diverged session into a
/// clean baseline. The committer must instead REFUSE to rebase over an unhealthy
/// session, so the diagnostic survives, no discontinuity is claimed, and the
/// intent stays pending.
#[test]
fn a_confirmed_commit_refuses_to_rebase_over_a_diverged_session() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));
    let generation_before = session_generation(&sim);

    // Walk until a transition intent is recorded, but poison BEFORE it commits.
    let mut recorded = false;
    for _ in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        if obs.active_room.as_str() == TARGET_ROOM {
            panic!("the intent committed before the session could be poisoned");
        }
        if intent_pending(&sim) {
            recorded = true;
            break;
        }
    }
    assert!(recorded, "a transition intent was recorded while predicted");

    // Poison: as if the sim diverged this window.
    sim.world_mut()
        .resource_mut::<ambition::runtime::rollback::RollbackSessionStatus>()
        .mismatch_frames
        .push(-999);

    // Step past the confirmation horizon: the committer sees the unhealthy
    // session and must NOT rebase (a rebase would erase the mismatch).
    for _ in 0..40 {
        let _ = sim.step(AgentAction::default());
    }

    assert!(
        sim.rollback_health().is_err(),
        "the injected mismatch must survive — a lifecycle rebase must not launder \
         a diverged session clean"
    );
    assert_eq!(
        session_generation(&sim),
        generation_before,
        "no rebase happened over the diverged session (generation unchanged)"
    );
    assert!(
        intent_pending(&sim),
        "the refused commit leaves the intent pending rather than losing it"
    );
}
