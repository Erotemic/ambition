//! Census the two room-transition routes over an authored door crossing.
//!
//! Without rollback, a room change opens `RoomTransitionLoadState`. With a
//! confirmed-frame boundary, detection records `PendingLifecycleCommit` instead
//! and the rollback backend commits the room change outside speculative
//! simulation. The test counts room changes, transactions, and deferred intents
//! to ensure exactly one route owns each transition.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

use crate::common::base;

fn interact() -> AgentAction {
    AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    }
}

fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

fn transaction_open(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>()
        .is_some_and(|state| state.active.is_some())
}

fn boundary_present(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
        .is_some()
}

/// Signal for the confirmed-frame transition route used instead of the load
/// transaction while rollback is active.
fn deferred_intent_pending(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .is_some_and(|state| state.pending.is_some())
}

struct Census {
    room_changes: usize,
    transactions: usize,
    deferred_intents: usize,
    boundary: bool,
}

fn census(rollback: bool, frames: usize) -> Census {
    let mut options =
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the harness builds");

    for _ in 0..20 {
        sim.step(base());
    }
    let boundary = boundary_present(&sim);

    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the active room has a RoomSet");
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .expect("the start room authors a Door zone")
    };
    let centre = {
        use ambition_platformer2d::engine_core::AabbExt as _;
        door.aabb.center()
    };
    sim.teleport_player((centre.x, centre.y));

    let mut room = active_room(&mut sim);
    let mut room_changes = 0usize;
    let mut transactions = 0usize;
    let mut deferred_intents = 0usize;
    let mut was_open = transaction_open(&sim);
    let mut was_pending = deferred_intent_pending(&sim);
    for _ in 0..frames {
        sim.step(interact());
        let open = transaction_open(&sim);
        if open && !was_open {
            transactions += 1;
        }
        was_open = open;
        let pending = deferred_intent_pending(&sim);
        if pending && !was_pending {
            deferred_intents += 1;
        }
        was_pending = pending;
        let now = active_room(&mut sim);
        if now != room {
            room_changes += 1;
            room = now;
        }
    }
    Census {
        room_changes,
        transactions,
        deferred_intents,
        boundary,
    }
}

/// The other half: is the SHIPPED app's host the rollback one? The harness above
/// asks for a sync-test session explicitly; this asks the real composition.
#[test]
#[ignore = "PROBE, print-only: panics to report its census. Preserved mid-investigation \
           (D71) when the run was paused; run explicitly with --ignored."]
fn d71_probe_shipped_app_host() {
    use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
    use ambition_platformer2d::game_shell::ShellCommand;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(16),
    ));
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut boundary = false;
    let mut ownership = String::from("<none>");
    for _ in 0..900 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(2));
        boundary = app
            .world()
            .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
            .is_some();
        if boundary {
            ownership = format!(
                "{:?}",
                app.world()
                    .get_resource::<ambition_platformer2d::rollback::RollbackSessionOwnership>()
            );
            break;
        }
    }
    panic!("D71 SHIPPED HOST: ConfirmedFrameBoundary present={boundary} ownership={ownership}");
}

#[test]
#[ignore = "PROBE, print-only: panics to report its census. Preserved mid-investigation \
           (D71) when the run was paused; run explicitly with --ignored."]
fn d71_probe_counts_room_changes_against_transactions() {
    let eager = census(false, 120);
    let roll = census(true, 360);
    panic!(
        "D71 CENSUS\n  \
         fixed-tick host (ConfirmedFrameBoundary={}): \
         room changes={} transactions={} deferred intents={}\n  \
         ROLLBACK host  (ConfirmedFrameBoundary={}): \
         room changes={} transactions={} deferred intents={}",
        eager.boundary,
        eager.room_changes,
        eager.transactions,
        eager.deferred_intents,
        roll.boundary,
        roll.room_changes,
        roll.transactions,
        roll.deferred_intents,
    );
}

/// THE ACCEPTANCE TARGET for readiness convergence, and it is RED by construction until that
/// lands.
///
/// Unlike the two probes above this asserts rather than reports, and it asserts
/// what a PLAYER gets: a room change on the host the shipped binary composes
/// opens a readiness transaction, which is the thing the opaque cover is driven
/// off. `drive_room_transition_presentation` returns immediately while
/// `RoomTransitionLoadState::active` is `None`, so with zero transactions no
/// cover is ever spawned — and `RoomConstructionPlan::prepare_from_parts` asks
/// nothing about assets, so the destination theme's parallax is still loading when the room
/// appears.
///
/// `#[ignore]`d deliberately, and this is the one shape that earns it. The
/// convergence is a real slice in rollback-adjacent code; a target that fails the
/// gate for the days that takes would be removed by whoever it inconveniences,
/// which is how a known gap becomes an unknown one. Run it with `--ignored`; when
/// it passes, DELETE the attribute rather than the test.
#[test]
fn a_room_change_on_the_shipped_host_opens_a_readiness_transaction() {
    let rollback = census(true, 900);
    assert!(
        rollback.boundary,
        "precondition: this census must run the host the shipped binary composes"
    );
    assert!(
        rollback.room_changes > 0,
        "precondition: the fixture must actually change rooms"
    );
    assert!(
        rollback.transactions > 0,
        "{} room changes opened {} readiness transactions and deferred {} intents. \
         Every one of those room changes was uncovered: the cover is driven off \
         `RoomTransitionLoadState::active`, which stays `None` on this route, and \
         the target room is constructed before its assets exist.",
        rollback.room_changes,
        rollback.transactions,
        rollback.deferred_intents,
    );
}

/// The body that CROSSED is the body that ARRIVES — whoever is driving when
/// the transaction finally commits.
///
/// A room transition is not instantaneous: detection opens a readiness transaction and the
/// authorized commit lands several frames later. It is the same subject only while nothing changes
/// hands in between, and possession, death and control handoff are exactly the things that do.
///
/// makes the richer contract the only one, and this is the behaviour that proves the message
/// now carries it.
#[test]
fn the_recorded_subject_transits_rather_than_whoever_is_controlled() {
    const SUMMONED: &str = "d71_crossing_body";

    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("the harness builds");
    for _ in 0..20 {
        sim.step(base());
    }
    let start_room = active_room(&mut sim);

    // A second body in the room, built the ordinary way, which is emphatically
    // not the body anybody is driving.
    sim.spawn_enemy_character_at(
        SUMMONED,
        "Crossing Body",
        (620.0, 300.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    for _ in 0..8 {
        sim.step(base());
    }

    // the id is READ off the body, never spelled: `SimId::placement(id)` would
    // reproduce the construction site's spelling and agree with itself even if
    // construction changed.
    let subject = {
        let world = sim.world_mut();
        let mut query = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::platformer::sim_id::SimId,
        )>();
        query
            .iter(world)
            .find(|(feature, _)| feature.0 == SUMMONED)
            .map(|(_, id)| id.clone())
            .expect("the summoned body reached the world with an identity")
    };

    let (target_room, arrival) = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the active room has a RoomSet");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .expect("the start room authors a Door zone");
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the authored door resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };
    let avatar_before = primary_pos(&mut sim);

    sim.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room,
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );

    let mut room = start_room.clone();
    for _ in 0..600 {
        sim.step(base());
        room = active_room(&mut sim);
        if room != start_room {
            break;
        }
    }
    assert_ne!(
        room, start_room,
        "the transition never committed, so this test says nothing about WHICH \
         body it moved"
    );

    let summoned_pos = {
        let world = sim.world_mut();
        let mut query = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::engine_core::BodyKinematics,
        )>();
        query
            .iter(world)
            .find(|(feature, _)| feature.0 == SUMMONED)
            .map(|(_, kin)| kin.pos)
            .expect(
                "the recorded subject did not survive its own crossing — a carried \
                 body must arrive, not be swept with the source room",
            )
    };
    assert!(
        summoned_pos.distance(arrival) < 32.0,
        "the transition named the summoned body but landed it at {summoned_pos:?} \
         instead of the arrival {arrival:?}. The commit is still deciding who \
         transits from who is CONTROLLED rather than from who the request recorded"
    );
    let avatar_after = primary_pos(&mut sim);
    assert!(
        avatar_after.distance(arrival) > 32.0,
        "the AVATAR was transited to {arrival:?} by a request that named another \
         body ({avatar_before:?} -> {avatar_after:?}). That is the
         `ControlledSubject`-at-commit-time fallback, and it moves whichever body \
         happens to be driven when readiness finishes rather than the one that \
         crossed"
    );
}

/// The primary avatar's position — read the same way in both halves of the
/// assertion above so a change of frame cannot flatter it.
fn primary_pos(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Vec2 {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<
        &ambition_platformer2d::engine_core::BodyKinematics,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    query
        .iter(world)
        .next()
        .expect("gameplay has a primary avatar")
        .pos
}
