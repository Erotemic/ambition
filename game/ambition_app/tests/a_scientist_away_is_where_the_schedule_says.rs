#![cfg(feature = "rl_sim")]
//! RADIANT-NPC-HTN slice 5: offscreen and re-entry.
//!
//! The player leaves the coil lab at half past nine and comes back at half
//! past seven in the evening. Nobody simulated the lab in between: the room was
//! not live. Each scientist stands where the schedule puts them at that hour,
//! with the facts their tasks would have set, read against the schedule, not
//! against a recorded run: Nikobolt (8-12-18) has gone home (he stands at the
//! lab door, his way home) and has eaten; Coulomba (10-13-20) is still at the
//! bench, and has eaten too.

use crate::breakable_respawn_across_rooms::cross_to;
use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::life::{Life, Value};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::time::{DayClock, DayCycle};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const QUAD: &str = "eureka_quad";
const NIKOBOLT: &str = "npc_science_nikobolt";
const COULOMBA: &str = "npc_science_coulomba";
const WORKBENCH_X: f32 = 1072.0;

fn body(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> Option<Entity> {
    let world = sim.world_mut();
    world.query::<(Entity, &WornCharacter)>().iter(world).find(|(_, worn)| worn.id() == character).map(|(entity, _)| entity)
}

fn set_the_clock(sim: &mut ambition_app::Platformer2dSimHarness, hour: f32) {
    let world = sim.world_mut();
    let cycle = *world.resource::<DayCycle>();
    *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, hour);
}

fn life_of(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> (Life, ae::Vec2) {
    let body = body(sim, character).unwrap_or_else(|| panic!("the lab stages {character}"));
    let life = sim.world().get::<Life>(body).cloned().unwrap_or_else(|| panic!("{character} has a life"));
    (life, sim.world().get::<ae::BodyKinematics>(body).expect("a body").pos)
}

/// The middle of the lab door, from the room as it is authored.
fn lab_door_x(sim: &mut ambition_app::Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let mut sets = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let set = sets.iter(world).next().expect("a room set");
    let zone = set
        .rooms
        .iter()
        .find(|room| room.id == LAB)
        .and_then(|room| room.loading_zones.iter().find(|zone| zone.id == "lab_door"))
        .expect("the lab has its door");
    (zone.aabb.min.x + zone.aabb.max.x) * 0.5
}

#[test]
fn back_in_the_evening_each_scientist_is_where_the_schedule_says() {
    let mut sim = fixed_60hz_room_sim(LAB);
    sim.step(base());
    set_the_clock(&mut sim, 9.5);
    sim.step_n(base(), 60);
    let (nikobolt, _) = life_of(&mut sim, NIKOBOLT);
    assert_eq!(nikobolt.facts.get("ate"), Some(&Value::Bool(false)), "premise: nobody has eaten at half past nine");
    let door_x = lab_door_x(&mut sim);

    // Leave; the lab is not live while the player is away.
    assert_eq!(cross_to(&mut sim, QUAD), QUAD);
    sim.step_n(base(), 30);
    assert!(body(&mut sim, NIKOBOLT).is_none(), "premise: the lab is not live, and nobody in it is simulated");
    set_the_clock(&mut sim, 19.5);
    sim.step_n(base(), 30);

    // Come back at half past seven in the evening.
    assert_eq!(cross_to(&mut sim, LAB), LAB);
    sim.step_n(base(), 60);
    let (nikobolt, at) = life_of(&mut sim, NIKOBOLT);
    println!("Nikobolt at x {} (the door at {door_x}): {:?}", at.x, nikobolt.facts);
    assert_eq!(nikobolt.facts.get("at"), Some(&Value::Sym("home".into())), "after six Nikobolt has not gone home");
    assert_eq!(nikobolt.facts.get("ate"), Some(&Value::Bool(true)), "his lunch at noon did not happen while he was away");
    assert!(
        matches!(nikobolt.facts.get("experiments"), Some(Value::Int(count)) if *count >= 1),
        "a working day away did no experiment"
    );
    assert!((at.x - door_x).abs() < 64.0, "gone home, Nikobolt stands at x {} and not at the lab door ({door_x})", at.x);

    let (coulomba, at) = life_of(&mut sim, COULOMBA);
    println!("Coulomba at x {} (the bench at {WORKBENCH_X}): {:?}", at.x, coulomba.facts);
    assert_eq!(coulomba.facts.get("at"), Some(&Value::Sym("lab".into())), "before eight Coulomba is not at the lab");
    assert_eq!(coulomba.facts.get("ate"), Some(&Value::Bool(true)), "her lunch at one did not happen while she was away");
    assert!((at.x - WORKBENCH_X).abs() < 64.0, "at work, Coulomba stands at x {} and not at the bench", at.x);
}
