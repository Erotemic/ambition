#![cfg(feature = "rl_sim")]
//! RADIANT-NPC-HTN slice 3: two NPCs, one exclusive workbench.
//!
//! From five past ten Nikobolt and Coulomba both plan to work at the one
//! bench. A claim on it is a fact of the simulation, not of either planner's
//! belief: the first to arrive takes it, the other's claim is lost and that
//! one plans again from what it sees (the bench taken: read notes instead),
//! and takes the bench when it is free. Witnesses: never two holders or two
//! users of the bench at once, both do an experiment within the hour, and the
//! claim survives the rewinds of a sync test.

use crate::common::{base, fixed_60hz_room_options, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::life::{Doing, Life, LifeStop, Value};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const SCIENTISTS: [&str; 2] = ["npc_science_coulomba", "npc_science_nikobolt"];
const BENCH: &str = "workbench";
/// Both are at work: Nikobolt from eight, Coulomba from ten. An experiment
/// holds a quarter of an hour, so two in turn end before the next hour's plan.
const START_HOUR: f32 = 10.05;
/// By a quarter to eleven each has had a turn.
const BY_HOUR: f32 = 10.75;

fn scientists(sim: &mut ambition_app::Platformer2dSimHarness) -> [Entity; 2] {
    let world = sim.world_mut();
    let bodies: Vec<(String, Entity)> = world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .map(|(entity, worn)| (worn.id().to_string(), entity))
        .collect();
    SCIENTISTS.map(|who| {
        bodies.iter().find(|(id, _)| id == who).map(|(_, entity)| *entity).unwrap_or_else(|| panic!("the lab stages {who}"))
    })
}

fn hour(sim: &ambition_app::Platformer2dSimHarness) -> f32 {
    let world = sim.world();
    world.resource::<DayClock>().hour(world.resource::<DayCycle>())
}

/// How many hold the bench, and how many are at work on it, this tick.
fn on_the_bench(sim: &ambition_app::Platformer2dSimHarness, bodies: [Entity; 2]) -> (usize, usize) {
    let lives: Vec<&Life> = bodies.iter().filter_map(|body| sim.world().get::<Life>(*body)).collect();
    let holders = lives.iter().filter(|life| life.claim.as_deref() == Some(BENCH)).count();
    let users = lives
        .iter()
        .filter(|life| {
            matches!(life.doing, Doing::Holding { .. })
                && life.task().is_some_and(|step| step.action.verb == "use" && step.action.target.as_deref() == Some(BENCH))
        })
        .count();
    (holders, users)
}

fn experiments(life: Option<&Life>) -> i64 {
    match life.and_then(|life| life.facts.get("experiments")) {
        Some(Value::Int(count)) => *count,
        _ => 0,
    }
}

/// ⭐ ONE BENCH, TWO SCIENTISTS, BOTH GET A TURN. From five past ten no tick
/// has two holders or two users of the bench; one of them finds it taken and
/// plans again (the reason is on its life); and by a quarter to eleven each
/// has done an experiment, so the one who waited took the bench when it was
/// free.
#[test]
fn two_scientists_take_turns_at_one_bench() {
    let mut sim = fixed_60hz_room_sim(LAB);
    sim.step(base());
    {
        let world = sim.world_mut();
        let cycle = *world.resource::<DayCycle>();
        *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, START_HOUR);
    }
    let bodies = scientists(&mut sim);
    let mut ticks = 0;
    let mut held = [0usize; 2];
    let mut turned_away = None;
    let mut done = [None; 2];
    while hour(&sim) < BY_HOUR {
        sim.step(base());
        ticks += 1;
        let (holders, users) = on_the_bench(&sim, bodies);
        assert!(holders <= 1, "{holders} bodies hold the bench at {:.3} h", hour(&sim));
        assert!(users <= 1, "{users} bodies work at the bench at {:.3} h", hour(&sim));
        for (index, body) in bodies.into_iter().enumerate() {
            let life = sim.world().get::<Life>(body);
            if life.and_then(|life| life.claim.as_deref()) == Some(BENCH) {
                held[index] += 1;
            }
            if turned_away.is_none() && life.is_some_and(|life| life.because == Some(LifeStop::Taken(BENCH.into()))) {
                let task = life.and_then(Life::task).map(|step| step.task.clone());
                turned_away = Some((SCIENTISTS[index], hour(&sim), task));
            }
            if done[index].is_none() && experiments(life) >= 1 {
                done[index] = Some(hour(&sim));
            }
        }
    }
    println!("{ticks} ticks; bench held {held:?} ticks; turned away: {turned_away:?}; experiments done at {done:?} h");
    let (who, at, task) = turned_away.expect("neither scientist ever found the bench taken: the two never wanted it at once");
    println!("{who} found the bench taken at {at:.3} h and planned {task:?} instead");
    assert_eq!(task.as_deref(), Some("read_notes"), "{who}, turned away from the bench, did not plan to read notes");
    for (index, at) in done.iter().enumerate() {
        assert!(
            at.is_some(),
            "{} did no experiment by a quarter to eleven (held the bench {} ticks): {:?}",
            SCIENTISTS[index],
            held[index],
            sim.world().get::<Life>(bodies[index])
        );
    }
}

/// ⭐ THE CLAIM SURVIVES A REWIND. The same hour under a sync test: each frame
/// is rewound and replayed and the checksums compared. A claim restored wrong
/// would put both scientists at the bench, or neither, and the replayed world
/// would differ. The premise: the bench passed from one holder to the other.
#[test]
fn the_bench_changes_hands_the_same_under_a_sync_test() {
    let options = fixed_60hz_room_options(LAB)
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let mut sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let mut cycle = *app.world().resource::<DayCycle>();
        cycle.start_hour = START_HOUR;
        app.insert_resource(cycle);
        Ok(())
    })
    .expect("the coil lab boots under a sync test");
    sim.try_step(base()).expect("the first frame steps");
    let bodies = scientists(&mut sim);
    let mut holders_seen: Vec<usize> = Vec::new();
    for _ in 0..60 * 50 {
        if sim.try_step(base()).is_err() {
            break;
        }
        let holder = bodies
            .iter()
            .position(|body| sim.world().get::<Life>(*body).and_then(|life| life.claim.as_deref()) == Some(BENCH));
        if let Some(holder) = holder {
            if holders_seen.last() != Some(&holder) {
                holders_seen.push(holder);
            }
        }
        if holders_seen.len() >= 2 {
            break;
        }
        assert!(on_the_bench(&sim, bodies).0 <= 1, "two holders of the bench under the sync test");
    }
    assert_eq!(
        (
            ambition_platformer2d::rollback::session_is_active(sim.world()),
            ambition_platformer2d::rollback::session_health(sim.world())
        ),
        (true, Ok(())),
        "the replayed world differs from the world that ran first"
    );
    assert_eq!(holders_seen.len(), 2, "premise: the bench passed from one scientist to the other (holders {holders_seen:?})");
}
