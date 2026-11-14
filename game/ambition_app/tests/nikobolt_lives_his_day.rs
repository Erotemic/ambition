#![cfg(feature = "rl_sim")]
//! RADIANT-NPC-HTN slice 2: one NPC, one room, a schedule.
//!
//! Nikobolt lives the authored scientist's day (`data/npc_lives.ron`) in the
//! Eureka coil lab. The executor plans on each hour from his facts and carries
//! the plan out with his own body: at half past nine, before Coulomba comes in,
//! he walks to the bench and does an experiment; at noon he walks to the
//! lunch crate and eats. Dmitri has no life: the control, who stays where
//! they stand. His morning under a sync test is witnessed by the bench
//! hand-over (`two_scientists_share_one_bench`): the first holder walks,
//! claims, experiments and lets go before the second takes it.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::life::{Doing, Life, Value};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const NIKOBOLT: &str = "npc_science_nikobolt";
const DMITRI: &str = "npc_eureka_dmitri";
/// The places his plan names, as the lab's signs place them (x).
const WORKBENCH_X: f32 = 1072.0;
const LUNCH_X: f32 = 248.0;
/// The day starts at half past nine: Nikobolt's working hour, before Coulomba
/// (in at ten) is in the lab to want the bench.
const START_HOUR: f32 = 9.5;

fn body(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &CharacterForm)>()
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

/// ⭐ HE GOES WHERE HIS PLAN SAYS AND DOES WHAT IT SAYS. At half past nine
/// the plan is a working one: he walks to the bench and the experiment is
/// done (his facts count it). The clock is then put on to just before noon,
/// and at noon a new plan: he walks to the lunch crate and eats. Each phase
/// is read off his body and his facts, not off the plan's words. The next
/// day he is unfed again and lunches again.
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
    let dmitri = body(&mut sim, DMITRI);
    let (nikobolt_at_start, dmitri_at_start) = (x_of(&sim, nikobolt), x_of(&sim, dmitri));

    // The working hour: to the bench, and one experiment done there.
    let mut at_the_bench = None;
    while hour(&sim) < 10.0 {
        sim.step(base());
        let life = sim.world().get::<Life>(nikobolt).expect("Nikobolt has his life");
        if fact(life, "experiments") == Some(Value::Int(1)) && at_the_bench.is_none() {
            at_the_bench = Some((x_of(&sim, nikobolt), hour(&sim), life.clone()));
        }
    }
    let (x, at, life) = at_the_bench.unwrap_or_else(|| {
        panic!(
            "by ten Nikobolt had done no experiment: he stands at x {} (from {nikobolt_at_start}); his life: {:?}",
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
    {
        let world = sim.world_mut();
        let cycle = *world.resource::<DayCycle>();
        *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, 11.95);
    }
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

    // The next day (review 2026-11-10: `ate` was never reset, so he never
    // ate again): just before noon of day 2 he is unfed again, and eats.
    {
        let world = sim.world_mut();
        let cycle = *world.resource::<DayCycle>();
        let mut clock = DayClock::at_hour(&cycle, 11.95);
        clock.day = world.resource::<DayClock>().day + 1;
        *world.resource_mut::<DayClock>() = clock;
    }
    let mut unfed = false;
    let mut ate_again = None;
    while hour(&sim) < 12.9 {
        sim.step(base());
        let life = sim.world().get::<Life>(nikobolt).unwrap();
        unfed |= fact(life, "ate") == Some(Value::Bool(false));
        if unfed && fact(life, "ate") == Some(Value::Bool(true)) {
            ate_again = Some(x_of(&sim, nikobolt));
            break;
        }
    }
    assert!(unfed, "a new day did not start him unfed: {:?}", sim.world().get::<Life>(nikobolt));
    let x = ate_again.unwrap_or_else(|| panic!("on day 2 Nikobolt did not eat by 12:54: {:?}", sim.world().get::<Life>(nikobolt)));
    assert!((x - LUNCH_X).abs() < 48.0, "day 2's lunch was eaten away from the crate: x {x}");

    // The control: a character with no life stands where they stood.
    assert!(sim.world().get::<Life>(dmitri).is_none(), "premise: Dmitri has no life in the book");
    let moved = (x_of(&sim, dmitri) - dmitri_at_start).abs();
    assert!(moved < 4.0, "control: Dmitri, who has no life, moved {moved} px");
}
