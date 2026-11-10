#![cfg(feature = "rl_sim")]
//! RADIANT-NPC-HTN slice 2: one NPC, one room, a schedule.
//!
//! Nikobolt lives the authored scientist's day (`data/npc_lives.ron`) in the
//! Eureka coil lab. The executor plans on each hour from his facts and carries
//! the plan out with his own body: before noon he walks to the bench and does
//! an experiment, at noon he walks to the lunch crate and eats. Coulomba has
//! no life (yet): she is the control and stays where she stands. The same day
//! under a sync test resimulates to the same world and the same facts.

use crate::common::{base, fixed_60hz_room_options, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::life::{Doing, Life, Value};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const NIKOBOLT: &str = "npc_science_nikobolt";
const COULOMBA: &str = "npc_science_coulomba";
/// The places his plan names, as the lab's signs place them (x).
const WORKBENCH_X: f32 = 1072.0;
const LUNCH_X: f32 = 248.0;
/// The day starts at half past eleven: a working hour, then lunch.
const START_HOUR: f32 = 11.5;

fn body(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == character)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("the coil lab stages {character}"))
}

fn x_of(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world().get::<ae::BodyKinematics>(body).expect("a body").pos.x
}

fn fact(life: &Life, name: &str) -> Option<Value> {
    life.facts.get(name).cloned()
}

fn hour(sim: &ambition_app::Platformer2dSimHarness) -> f32 {
    let world = sim.world();
    world.resource::<DayClock>().hour(world.resource::<DayCycle>())
}

/// ⭐ HE GOES WHERE HIS PLAN SAYS AND DOES WHAT IT SAYS. Before noon the plan
/// is a working one: he walks to the bench and the experiment is done (his
/// facts count it). At noon a new plan: he walks to the lunch crate and eats.
/// Each phase is read off his body and his facts, not off the plan's words.
#[test]
fn nikobolt_works_at_his_bench_and_lunches_at_noon() {
    let mut sim = fixed_60hz_room_sim(LAB);
    sim.step(base());
    {
        let world = sim.world_mut();
        let cycle = *world.resource::<DayCycle>();
        *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, START_HOUR);
    }
    let nikobolt = body(&mut sim, NIKOBOLT);
    let coulomba = body(&mut sim, COULOMBA);
    let (nikobolt_at_start, coulomba_at_start) = (x_of(&sim, nikobolt), x_of(&sim, coulomba));

    // The working hour: to the bench, and one experiment done there.
    let mut at_the_bench = None;
    while hour(&sim) < 12.0 {
        sim.step(base());
        let life = sim.world().get::<Life>(nikobolt).expect("Nikobolt has his life");
        if fact(life, "experiments") == Some(Value::Int(1)) && at_the_bench.is_none() {
            at_the_bench = Some((x_of(&sim, nikobolt), hour(&sim), life.clone()));
        }
    }
    let (x, at, life) = at_the_bench.unwrap_or_else(|| {
        panic!(
            "by noon Nikobolt had done no experiment: he stands at x {} (from {nikobolt_at_start}); his life: {:?}",
            x_of(&sim, nikobolt),
            sim.world().get::<Life>(nikobolt)
        )
    });
    println!("experiment done at {at:.2} h, Nikobolt at x {x} (the bench at {WORKBENCH_X})");
    assert!((x - WORKBENCH_X).abs() < 48.0, "the experiment was done away from the bench: x {x}");
    assert_eq!(fact(&life, "bench_free"), Some(Value::Bool(true)), "the bench was not released after the experiment");
    assert_eq!(life.stopped, None, "the working plan stopped short: {:?}", life.stopped);

    // Noon: a new plan, and lunch at the crate.
    let revision = life.revision;
    let mut lunched = None;
    while hour(&sim) < 12.9 {
        sim.step(base());
        let life = sim.world().get::<Life>(nikobolt).unwrap();
        if fact(life, "ate") == Some(Value::Bool(true)) {
            lunched = Some((x_of(&sim, nikobolt), hour(&sim), life.clone()));
            break;
        }
    }
    let (x, at, life) = lunched.unwrap_or_else(|| {
        panic!(
            "Nikobolt did not eat by 12:54: he stands at x {}; his life: {:?}",
            x_of(&sim, nikobolt),
            sim.world().get::<Life>(nikobolt)
        )
    });
    println!("lunch eaten at {at:.2} h, Nikobolt at x {x} (the crate at {LUNCH_X})");
    assert!(life.revision > revision, "premise: noon is a new plan");
    assert!((x - LUNCH_X).abs() < 48.0, "lunch was eaten away from the crate: x {x}");
    assert_eq!(life.doing, Doing::Idle, "after lunch his plan is spent");

    // The control: a character with no life stands where she stood.
    assert!(sim.world().get::<Life>(coulomba).is_none(), "premise: Coulomba has no life in the book");
    let moved = (x_of(&sim, coulomba) - coulomba_at_start).abs();
    assert!(moved < 4.0, "control: Coulomba, who has no life, moved {moved} px");
}

/// ⭐ HIS DAY RESIMULATES TO THE SAME WORLD. Under a sync test each frame is
/// rewound and replayed and the checksums compared. His plan, its steps and
/// his facts are rollback state; the planner runs again on a replayed hour
/// from the restored facts. The premise: the experiment was done.
#[test]
fn nikobolts_day_resimulates_to_the_same_world() {
    let options = fixed_60hz_room_options(LAB)
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let mut sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        // The session's first day begins at half past eleven.
        let mut cycle = *app.world().resource::<DayCycle>();
        cycle.start_hour = START_HOUR;
        app.insert_resource(cycle);
        Ok(())
    })
    .expect("the coil lab boots under a sync test");
    sim.try_step(base()).expect("the first frame steps");
    let nikobolt = body(&mut sim, NIKOBOLT);
    let mut experiments = None;
    for _ in 0..60 * 40 {
        if sim.try_step(base()).is_err() {
            break;
        }
        experiments = sim.world().get::<Life>(nikobolt).and_then(|life| fact(life, "experiments"));
        if experiments == Some(Value::Int(1)) {
            break;
        }
    }
    assert_eq!(
        (
            ambition_platformer2d::rollback::session_is_active(sim.world()),
            ambition_platformer2d::rollback::session_health(sim.world())
        ),
        (true, Ok(())),
        "the replayed world differs from the world that ran first"
    );
    assert_eq!(experiments, Some(Value::Int(1)), "premise: the experiment was done under the sync test");
    let x = x_of(&sim, nikobolt);
    assert!((x - WORKBENCH_X).abs() < 48.0, "under the sync test the experiment was done away from the bench: x {x}");
}
