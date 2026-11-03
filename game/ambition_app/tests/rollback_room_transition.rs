//! Room crossings through confirmed host commits and reversible simulation transactions.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

type RoomCensus = std::collections::BTreeMap<&'static str, (usize, u64)>;

#[derive(bevy::prelude::Resource)]
struct ScheduledRoomCrossing {
    plan: std::sync::Arc<ambition_platformer2d::publication::RoomConstructionPlan>,
    subject: ambition_platformer2d::platformer::lifecycle::LiveBodyId,
    applications: usize,
    source_census: Option<RoomCensus>,
    settled_census: std::collections::BTreeMap<i32, RoomCensus>,
}

fn room_census(world: &mut bevy::prelude::World) -> RoomCensus {
    let probes = world.resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>().clone();
    probes.census_all_as_peers_compare(world).into_iter()
        .map(|(row, reading)| (row, (reading.count, reading.xor))).collect()
}

fn assert_room_census(expected: &RoomCensus, actual: &RoomCensus, phase: &str) {
    for row in expected.keys().chain(actual.keys()) {
        assert_eq!(expected.get(row), actual.get(row), "{phase}: {row}");
    }
}

fn record_replayed_room_frame(world: &mut bevy::prelude::World) {
    let frame = world.resource::<ambition_platformer2d::rollback::RollbackFrameCount>().0;
    if !(20..=24).contains(&frame) { return; }
    let actual = room_census(world);
    let mut crossing = world.resource_mut::<ScheduledRoomCrossing>();
    if let Some(expected) = crossing.settled_census.get(&frame) {
        assert_room_census(expected, &actual, "replayed room frame");
    } else {
        crossing.settled_census.insert(frame, actual);
    }
}

fn cross_inside_the_rollback_schedule(world: &mut bevy::prelude::World) {
    use ambition_platformer2d::runtime::room_transition::{
        finalize_room_transition, RoomTransitionApplication,
    };
    if world.resource::<ambition_platformer2d::rollback::RollbackFrameCount>().0 != 20 {
        return;
    }
    let stored: std::collections::BTreeSet<_> = world.resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .descriptors().filter(|row| row.kind.carries_state()
            && row.kind != ambition_platformer2d::rollback::RollbackEntryKind::Derived)
        .map(|row| row.type_name.clone()).collect();
    let mut census = room_census(world);
    // Derived values can wait for their normal phase. Each root needs its overlay buffer now.
    census.retain(|row, _| stored.contains(*row)
        || *row == std::any::type_name::<ambition_platformer2d::platformer::feature_overlay::FeatureEcsWorldOverlay>());
    let (plan, subject) = {
        let mut crossing = world.resource_mut::<ScheduledRoomCrossing>();
        if let Some(expected) = &crossing.source_census {
            assert_room_census(expected, &census, "restored source row");
        } else {
            crossing.source_census = Some(census);
        }
        crossing.applications += 1;
        (crossing.plan.clone(), crossing.subject.clone())
    };
    let mut state = bevy::ecs::system::SystemState::<RoomTransitionApplication>::new(world);
    let staged = {
        let mut application = state.get_mut(world).expect("the room application is installed");
        let body = application.subject_entity(&subject).expect("the source body is restored");
        application.stage(
            &plan, Some(body), Some(ambition_platformer2d::characters::control::PlayerSlot(0)),
            plan.target_index(), Some(plan.spec().world.spawn), false, None, false, None,
        ).expect("the room crossing stages")
    };
    state.apply(world);
    assert!(finalize_room_transition(world, &staged), "the room crossing publishes");
}

fn replay_room_crossing(
    mut sim: Platformer2dSimHarness,
    source: &str,
    destination: &str,
    expected_rooms: &[&str],
) {
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::runtime::room_transition::{
        begin_room_transition_load_system, RoomTransitionLoadState,
    };
    use ambition_platformer2d::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_eq!(sim.observation().active_room, source);
    crate::common::strengthen_the_float_rows(sim.world_mut());
    assert!(sim.world_mut().resource_mut::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .strengthen_with::<ambition_platformer2d::engine_core::RoomGeometry>(|geometry| {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", geometry.0).hash(&mut hash);
            hash.finish()
        }));
    let world = sim.world_mut();
    let target = world.query::<&ambition_platformer2d::world::rooms::RoomSet>()
        .single(world).unwrap().rooms.iter().find(|room| room.id == destination)
        .unwrap().clone();
    let body = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world).unwrap();
    let subject = LiveBodyId::of_entity(world, body).unwrap();
    // Prepare outside the saved timeline. Apply the crossing inside it.
    let confirmed = world.resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>().confirmed;
    assert!(world.resource_mut::<PendingLifecycleCommit>().record(confirmed,
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: subject.clone(), target_room: destination.into(),
            arrival: target.world.spawn, edge_exit: false, zone_sfx: None,
            participant: Some(ambition_platformer2d::characters::control::PlayerSlot(0)),
        })).admitted());
    world.run_system_once(begin_room_transition_load_system).expect("the target prepares");
    let active = world.resource_mut::<RoomTransitionLoadState>().active.take()
        .expect("the room preparation starts");
    let plan = active.construction_plan.expect("the destination plan is ready");
    world.resource_mut::<ambition_platformer2d::load::LoadCoordinator>()
        .retire(&active.barrier.load_id);
    world.resource_mut::<PendingLifecycleCommit>().take();
    world.insert_resource(ScheduledRoomCrossing {
        plan, subject, applications: 0, source_census: None, settled_census: Default::default(),
    });
    sim.app_mut().add_systems(ambition_platformer2d::rollback::SaveWorld, record_replayed_room_frame);
    sim.app_mut().add_systems(
        ambition_platformer2d::rollback::GgrsSchedule,
        cross_inside_the_rollback_schedule
            .before(ambition_platformer2d::sim::Platformer2dSimulationPhase::CoreSimulation),
    );
    sim.rebase_rollback_history().expect("the prepared source is the baseline");
    let generation = session_generation(&sim);
    let loads = sim.world().resource::<ambition_platformer2d::rollback::RollbackExecutionStats>()
        .lifetime_load_runs;
    for frame in 0..40 {
        sim.step(AgentAction::default());
        sim.rollback_health().unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
    assert_eq!(sim.observation().active_room.as_str(), destination);
    let mut rooms: Vec<_> = crate::two_players_two_live_rooms::live_rooms(&mut sim)
        .into_iter().map(|(_, room)| room).collect();
    rooms.sort();
    let mut expected = expected_rooms.to_vec();
    expected.sort();
    assert_eq!(rooms, expected, "the crossing keeps the correct live rooms");
    assert_eq!(session_generation(&sim), generation, "the crossing did not rebase");
    assert!(sim.world().resource::<ScheduledRoomCrossing>().applications > 1,
        "rollback replays the crossing frame");
    assert!(sim.world().resource::<ambition_platformer2d::rollback::RollbackExecutionStats>()
        .lifetime_load_runs > loads, "the real rollback load ran");
}

#[test]
fn a_room_publication_can_be_replayed_without_rebasing_the_session() {
    let sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_sync_test_rollback_settings(4, 10),
    ).expect("the source room boots");
    replay_room_crossing(sim, "switch_lab", "central_hub_complex", &["central_hub_complex"]);
}

#[test]
fn a_room_publication_can_be_replayed_while_another_body_keeps_the_source() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_sync_test_rollback_settings(4, 10).with_rollback_players(2),
    ).expect("the source room boots");
    crate::two_players_two_live_rooms::bob_beside_alice(&mut sim, "switch_lab",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    replay_room_crossing(sim, "switch_lab", "central_hub_complex", &["switch_lab", "central_hub_complex"]);
}

#[test]
fn a_room_publication_can_be_replayed_when_the_destination_is_already_live() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_sync_test_rollback_settings(4, 10).with_rollback_players(2),
    ).expect("the source room boots");
    crate::common::doors_cross_at_once(&mut sim);
    crate::two_players_two_live_rooms::bob_beside_alice(&mut sim, "switch_lab",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"),
        "central_hub_complex");
    replay_room_crossing(sim, "central_hub_complex", "switch_lab", &["switch_lab"]);
}

#[test]
fn a_room_publication_can_be_replayed_after_retiring_moving_platforms() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("mockingbird_arena")
            .with_sync_test_rollback_settings(4, 10),
    ).expect("the source room boots");
    sim.step(AgentAction::default());
    let platforms = ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::world::collision::MovingPlatformSet,
    >(sim.world()).expect("the source room has a platform set");
    assert!(!platforms.0.is_empty(), "the crossing must retire moving platforms");
    replay_room_crossing(sim, "mockingbird_arena", "central_hub_complex", &["central_hub_complex"]);
}

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
    let mut q = world.query_filtered::<&ambition_platformer2d::actor::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
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
        .get_resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>()
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
/// `ambition_platformer2d_session::lifecycle_commit` unit tests cover only
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

/// The harness refuses to step an unhealthy session (`Q138`): a step fails
/// with the session's own error, and nothing advances.
///
/// ⛔ A sync-test session that diverged or was invalidated accepted every
/// step and returned an observation, so the assertions after it read a world
/// the timeline no longer vouched for, in silence.
#[test]
fn the_harness_refuses_to_step_an_unhealthy_session() {
    type Authority = ambition_platformer2d::rollback::ActiveRollbackAuthority;
    let poisons: [(&str, fn(&mut Authority), &str); 2] = [
        ("a checksum mismatch", |authority| authority.record_mismatch([-999]), "[-999]"),
        (
            "an invalidation",
            |authority| authority.invalidate("poisoned on purpose".to_string()),
            "poisoned on purpose",
        ),
    ];
    for (poison, apply, error) in poisons {
        let mut sim = repro_sim();
        for _ in 0..10 {
            sim.step(AgentAction::default());
        }
        sim.rollback_health()
            .unwrap_or_else(|e| panic!("the premise: the session is healthy before {poison}: {e}"));
        apply(&mut sim.world_mut().resource_mut::<Authority>());
        let sim_tick = |sim: &Platformer2dSimHarness| sim.world().resource::<ambition_platformer2d::time::SimTick>().0;
        let (steps, tick) = (sim.tick_count(), sim_tick(&sim));

        let refusal = sim
            .try_step(AgentAction::default())
            .expect_err("a step over an unhealthy session is refused");
        assert!(refusal.contains(error), "after {poison}, the refusal carries the session's error: {refusal}");
        assert_eq!(
            (sim.tick_count(), sim_tick(&sim)),
            (steps, tick),
            "after {poison}, a refused step advances nothing"
        );
        let stepped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sim.step(AgentAction::default());
        }));
        assert!(stepped.is_err(), "after {poison}, `step` fails rather than returning an observation");
    }
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
        let _ = sim.step_over_an_unhealthy_session(AgentAction::default());
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
    use ambition_platformer2d::session::lifecycle_commit::LifecycleIntent;

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
            .get_resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>()
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

/// The live room the sim stands in: the one live room of this one-player run.
fn sole_live_room(
    world: &bevy::prelude::World,
) -> Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    >(world)
    .copied()
}

/// The ambient gravity direction the live room simulates under.
fn base_gravity_dir(sim: &Platformer2dSimHarness) -> Option<bevy::prelude::Vec2> {
    let world = sim.world();
    let room = sole_live_room(world);
    world
        .get_resource::<ambition_platformer2d::world::BaseGravity>()
        .map(|gravity| gravity.dir_in(room))
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
        let room = sole_live_room(world);
        world
            .resource_mut::<ambition_platformer2d::world::BaseGravity>()
            .turn(room, flipped_dir);
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
                save.data_mut().set_flag("probe_visited", true);
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
