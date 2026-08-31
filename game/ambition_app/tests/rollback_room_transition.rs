//! Verify room reconstruction across a forced rollback window.
//!
//! Under rollback, transition detection records a confirmed-frame lifecycle
//! intent rather than running the multi-frame load transaction speculatively.
//! The test crosses an authored `EdgeExit`, requires the active room and entire
//! room-scoped roster to be replaced, requires exactly one session rebase, and
//! keeps the sync-test checksum healthy across the commit.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

/// Authored adjacent rooms connected by the calibration lab's right `EdgeExit`.
const SOURCE_ROOM: &str = "combat_calibration_lab";
const TARGET_ROOM: &str = "first_system_boss";

fn repro_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(SOURCE_ROOM)
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Every room-scoped feature/actor entity currently spawned. A transition
/// despawns this whole set and spawns a fresh one; because despawn bumps the
/// generation, no source `Entity` value can equal a post-transition one.
fn feature_roster(sim: &mut Platformer2dSimHarness) -> HashSet<Entity> {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<Entity, With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>>();
    q.iter(world).collect()
}

fn player_y(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.single(world).map(|k| k.pos.y).unwrap_or(0.0)
}

/// Rollback session generation, incremented by each session rebase.
fn session_generation(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
        .map(|boundary| boundary.session)
        .unwrap_or(0)
}

/// Whether a deferred lifecycle intent is currently recorded (rollback state).
fn intent_pending(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
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
    // a fresh target roster present (despawn+respawn  disjoint entity ids).
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

/// Track B principal timeline oracle (T6). The deferred lifecycle intent must
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
/// `ambition_platformer2d_actor_monolith::session::lifecycle_commit` unit tests cover only
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

/// The committer must instead REFUSE to rebase over an unhealthy session, so the diagnostic
/// survives, no discontinuity is claimed, and the intent stays pending.
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
        .resource_mut::<ambition_platformer2d::rollback::ActiveRollbackAuthority>()
        .record_mismatch([-999]);

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

/// the door still makes a sound on the rollback host.
///
/// it did not. The eager commit plays the zone's cue from
/// `RoomTransitionRequested::zone_sfx`; the deferred committer emitted nothing at
/// all, so on the shipped rollback binary every door and every portal was
/// SILENT. Nothing failed, no test noticed, and the cue was computed at detection
/// and dropped on the floor one branch later.
///
/// The cue now rides the intent — it has to, because the commit runs on a
/// confirmed frame long after the zone that named it is out of reach, and the
/// intent names its room by id rather than carrying a zone.
#[test]
fn a_deferred_transition_still_plays_the_zone_cue() {
    use ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent;

    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    // Walk into the exit and catch the intent while it is still pending.
    let mut recorded_cue: Option<Option<String>> = None;
    for _ in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        if let Some(intent) = sim
            .world()
            .get_resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
            .and_then(|slot| slot.pending.clone())
        {
            // ⛔ A CROSSING, not a rebuild. `LifecycleIntent` grew a bodyless
            // `ReconstituteRoom` in v146, and a door walked through by a body
            // must never record one — it carries no cue and places nobody.
            let LifecycleIntent::Transition(transition) = intent.kind else {
                panic!(
                    "walking through a door recorded a bodyless room \
                     reconstitution instead of a crossing"
                );
            };
            recorded_cue = Some(transition.zone_sfx);
        }
        if obs.active_room.as_str() == TARGET_ROOM {
            break;
        }
    }

    let cue = recorded_cue
        .expect("the crossing recorded a transition intent")
        .expect(
            "⛔ the intent carried NO cue, so the deferred commit has nothing to play and \
             the door is silent — which is exactly the bug this pins",
        );
    // The east exit is an `EdgeExit`, which the detection rule maps to the
    // portal-enter cue. Asserting the VALUE, not merely presence: a `Some("")`
    // would satisfy an is_some check and still play nothing audible.
    assert_eq!(
        cue, "world.portal.enter",
        "an EdgeExit owes the portal cue, the same one the eager path passes"
    );
}

/// The transaction currently open, as `(sequence, content_epoch, is_authorized)`.
fn open_transaction(sim: &Platformer2dSimHarness) -> Option<(u64, u64, bool)> {
    sim.world()
        .get_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>()
        .and_then(|state| state.active.as_ref())
        .map(|active| {
            (
                active.sequence,
                active.content_epoch,
                active.phase
                    == ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase::CommitAuthorized,
            )
        })
}

/// A TRANSACTION AUTHORIZED UNDER EPOCH E MUST NOT BUILD THE WORLD UNDER
/// EPOCH E+1.
///
/// `authorized_plan` compares `active.content_epoch` against the live epoch and returns `Wait`;
/// the eager side discards the transaction outright.
///
/// the invariant is not "the room never changes". Bumping the epoch does
/// not cancel the crossing: readiness re-opens a transaction under the NEW epoch
/// and that one commits, which is the desired behaviour. What must never happen
/// is the room changing under the transaction that was authorized before the
/// bump — so this identifies that transaction by `sequence` and watches which
/// one is live when the room flips.
#[test]
fn a_transaction_authorized_under_a_stale_content_epoch_never_commits() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    // That is the exact state the poison targets: readiness has finished, the plan is prepared,
    // and the only thing left is to build the world from it.
    let mut poisoned = None;
    for _ in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        if let Some((sequence, epoch, authorized)) = open_transaction(&sim) {
            if authorized {
                poisoned = Some((sequence, epoch));
                break;
            }
        }
        assert_ne!(
            obs.active_room.as_str(),
            TARGET_ROOM,
            "the room changed before any transaction was ever authorized, so this test \
             never reached the state it exists to poison"
        );
    }
    let (stale_sequence, stale_epoch) = poisoned.expect(
        "no room transition reached CommitAuthorized within 240 frames; the shipped host \
         opens a readiness transaction on every room change (D71: 21/21), so this is a \
         regression in the transaction path rather than a slow test",
    );

    // THE POISON. The world's content moves out from under the authorization.
    {
        let world = sim.world_mut();
        let mut epoch = world
            .resource_mut::<ambition_platformer2d::runtime::room_transition::RoomTransitionContentEpoch>();
        epoch.bump();
        assert_ne!(
            epoch.get(),
            stale_epoch,
            "bump() did not move the epoch, so nothing was poisoned"
        );
    }

    // The transaction authorized on the frame the room changes is the one that changed it, and
    // it is only visible from the near side of that step.
    let mut committing = None;
    let mut flipped = false;
    for _ in 0..480 {
        let authorized_now = open_transaction(&sim)
            .filter(|(_, _, authorized)| *authorized)
            .map(|(sequence, _, _)| sequence);
        let obs = sim.step(AgentAction::move_x(1.0));
        if obs.active_room.as_str() == TARGET_ROOM {
            committing = authorized_now;
            flipped = true;
            break;
        }
    }

    assert!(
        flipped,
        "the crossing never completed. Bumping the epoch must INVALIDATE the stale \
         authorization, not wedge the transition — readiness owes a fresh transaction \
         under the current epoch, and a test that passed by never transitioning would \
         be pinning a deadlock instead of the invariant."
    );
    // BOTH TERMS, OR THIS ASSERTS NOTHING. `assert_ne!(None, Some(n))`
    // passes for free, so a run where no transaction was ever authorized after
    // the bump would report success while having observed nothing at all. The
    // room changed, so a REPLACEMENT authorization must exist — name it first,
    // then say it is not the poisoned one.
    let committing = committing.expect(
        "the room changed with NO transaction authorized on that frame, so the crossing \
         committed outside the readiness path entirely — the D71 bypass, back",
    );
    assert_ne!(
        committing, stale_sequence,
        "the room changed under transaction {stale_sequence}, which was authorized at \
         content epoch {stale_epoch} and never re-checked after the epoch moved. The \
         source roster is despawned and the target world was built from a plan whose \
         assets nobody proved were still there."
    );
}

/// The ambient gravity direction the whole room simulates under.
fn base_gravity_dir(sim: &Platformer2dSimHarness) -> Option<bevy::prelude::Vec2> {
    sim.world()
        .get_resource::<ambition_platformer2d::world::BaseGravity>()
        .map(|gravity| gravity.dir)
}

/// A ROOM YOU LEFT MUST NOT KEEP SIMULATING THE ROOM YOU ENTERED.
///
/// The eager commit path calls `RoomTransitionCombatReset::clear_carryover` —
/// despawn every in-flight enemy projectile, return `BaseGravity` to its default
/// — because a fresh room must not inherit hostile shots or a gravity frame from
/// the one just left. `commit_transition`, the CONFIRMED path the shipped
/// rollback host actually runs, calls neither: neither
/// `clear_carryover` nor `BaseGravity` appears anywhere in `lifecycle_commit.rs`.
///
/// Two hosts, one game, two rules — which is what a "mirrors X" fork buys, and
/// what the ONE-application-operation convergence exists to end.
///
/// so do not satisfy it in future by pasting a reset into one host. What
/// makes it hold is that there is nowhere to paste: one operation, two callers.
/// A fix that touches only `commit_transition` has re-forked the thing this
/// test exists to prove is not forked.
#[test]
fn a_confirmed_room_transition_leaves_the_old_room_s_gravity_behind() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let default_dir =
        base_gravity_dir(&sim).expect("the sim publishes ambient gravity as BaseGravity");

    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    // A room that flipped ambient gravity, exactly as an authored gravity room
    // does. The value only has to DIFFER — what is asserted is that crossing a
    // door puts it back, not what it was.
    let flipped_dir = -default_dir;
    {
        let world = sim.world_mut();
        world
            .resource_mut::<ambition_platformer2d::world::BaseGravity>()
            .dir = flipped_dir;
    }
    assert_eq!(
        base_gravity_dir(&sim),
        Some(flipped_dir),
        "the test could not establish the precondition it measures: ambient gravity \
         did not take the flipped value"
    );

    let mut crossed = false;
    for _ in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        if obs.active_room.as_str() == TARGET_ROOM {
            crossed = true;
            break;
        }
    }
    assert!(
        crossed,
        "the body never reached the target room, so nothing about the transition was \
         measured"
    );

    assert_eq!(
        base_gravity_dir(&sim),
        Some(default_dir),
        "the shipped rollback host carried the previous room's ambient gravity across a \
         door. The eager path resets it in `clear_carryover`; the confirmed path never \
         calls that, so the two hosts disagree about what a room transition IS."
    );
}

/// ⭐⭐ THE POSITIVE CONTROL FOR THE SESSION CHECKSUM'S COVERAGE OF THE SAVE.
///
/// This reproduces the recorded hazard exactly — a `Local` edge-detector that
/// writes rollback-REGISTERED save state once — and requires the sync test to
/// CATCH it. The `Local` is not rollback state, so it does not rewind: the first
/// simulation of a frame writes the flag, and the resimulation of that same
/// frame takes the other branch and does not. That is a genuine divergence.
///
/// ⛔ IT WAS INVISIBLE UNTIL 2026-08-29. `resource.sandbox_save` was registered
/// with `rollback_resource_clone`, which saves and restores but installs a
/// PRESENCE-ONLY probe and no checksum projection — so this divergence moved no
/// checksum and the proof pulse that exists to find it reported a clean session.
/// Delete the `checksum` argument from that registration and this test goes
/// green-blind: no mismatch is ever reported.
#[test]
fn a_local_guarded_save_write_diverges_and_the_sync_test_says_so() {
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::persistence::save_data::PersistedFlag;
    use bevy::prelude::{Local, ResMut};

    let mut sim = repro_sim();

    // The hazard, in four lines. Writes once per SIMULATION of the tick, which
    // is once per tick on the first pass and never on a resimulation.
    sim.app_mut().add_systems(
        ambition_platformer2d::rollback::GgrsSchedule,
        |mut written: Local<bool>, save: Option<ResMut<AmbitionGameSave>>| {
            let Some(mut save) = save else {
                return;
            };
            if !*written {
                *written = true;
                save.data_mut()
                    .flags
                    .push(PersistedFlag::new("probe_visited", true));
            }
        },
    );

    // check_distance is 4, so every frame past the window is resimulated and
    // compared. The divergence is written on the first tick and detected as
    // soon as that tick is re-run.
    let mut caught_at = None;
    for frame in 0..60 {
        sim.step(AgentAction::default());
        if sim.rollback_health().is_err() {
            caught_at = Some(frame);
            break;
        }
    }

    assert!(
        caught_at.is_some(),
        "a save write guarded by a non-rewinding `Local` diverges on \
         resimulation, and the sync test must report it — a clean session here \
         means the checksum cannot see `AmbitionGameSave` at all"
    );
}
