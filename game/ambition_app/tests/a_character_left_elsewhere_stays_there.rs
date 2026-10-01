#![cfg(feature = "rl_sim")]
//! Durable whereabouts of an authored character (Q38, OW3).
//!
//! The ruling: a persistent open-world character can be carried anywhere and
//! stays there; its authored room is not a tether. A respawning population
//! occurrence stays where it is carried while it lives, and its replacement
//! comes from its authored room. So the authored home, the durable whereabouts
//! and the live room occurrence are three facts.
//!
//! The fixture is the hub's authored dog, an NPC placement (a unique person:
//! `DeadStaysDead`). The player possesses it, walks it through a door,
//! releases it there, and goes back.

use crate::common::{base, door_to, fixed_60hz_room_sim};
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core::BodyKinematics;
use bevy::prelude::Entity;

const HOME: &str = "central_hub_complex";
const DOG: &str = "npc_companion_dog";

/// The dog's live bodies: entity, `SimId`, and position.
fn dogs(
    sim: &mut Platformer2dSimHarness,
) -> Vec<(Entity, ambition_platformer2d::platformer::sim_id::SimId, ambition_platformer2d::engine_core::Vec2)> {
    let world = sim.world_mut();
    let mut query = world.query::<(
        Entity,
        &WornCharacter,
        &ambition_platformer2d::platformer::sim_id::SimId,
        &BodyKinematics,
    )>();
    let mut found: Vec<_> = query
        .iter(world)
        .filter(|(_, worn, ..)| worn.id() == DOG)
        .map(|(entity, _, id, kin)| (entity, id.clone(), kin.pos))
        .collect();
    found.sort_by_key(|(entity, ..)| *entity);
    found
}

fn possessed(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    sim.world_mut()
        .resource::<ambition_platformer2d::actors::control::possession::PossessionState>()
        .possessed
}

fn down_interact(press: bool) -> AgentAction {
    AgentAction {
        move_y: 1.0,
        interact: press,
        interact_held: true,
        ..base()
    }
}

fn place(sim: &mut Platformer2dSimHarness, body: Entity, at: ambition_platformer2d::engine_core::Vec2) {
    let mut kin = sim
        .world_mut()
        .get_mut::<BodyKinematics>(body)
        .expect("the body has kinematics");
    kin.pos = at;
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
}

/// Drive `body` into the door to `target` and hold interact until the room
/// changes. Returns the room arrived in.
fn take_the_door(sim: &mut Platformer2dSimHarness, body: Entity, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = sim.observation().active_room.clone();
    let door = door_to(sim, target).aabb.center();
    place(sim, body, door);
    for _ in 0..120 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!("held interact in the door of '{before}' to '{target}' for 120 frames and the room never changed");
}

/// The first room a door of the hub leads to.
fn a_neighbour(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let set = query.iter(world).next().expect("a room set");
    let mut neighbours: Vec<String> = set
        .canonical_links()
        .iter()
        .filter(|link| link.from_room == HOME && link.to_room != HOME)
        .map(|link| link.to_room.clone())
        .collect();
    neighbours.sort();
    neighbours.dedup();
    neighbours.into_iter().next().expect("the hub has a door to another room")
}

/// The dog is possessed, walked out of the hub into a neighbour, and released
/// there. The player goes back to the hub: the dog is not rebuilt at home,
/// because its whereabouts say where it is. The player comes back: the dog is
/// rebuilt where it was left, as the same occurrence. Before this, the hub
/// rebuilt the dog at home and the neighbour had none, so a persistent
/// character went home when its room retired. The control is the setup: the
/// hub authors the dog when it was never moved.
#[test]
fn a_character_released_in_another_room_is_there_when_you_come_back() {
    let mut sim = fixed_60hz_room_sim(HOME);
    sim.step_n(base(), 10);
    let found = dogs(&mut sim);
    let [(dog, ref dog_id, at)] = found[..] else {
        panic!("control: the hub stages exactly one authored dog when it was never moved: {found:?}");
    };
    let dog_id = dog_id.clone();
    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
        q.single(world).expect("one primary player")
    };
    place(&mut sim, player, ambition_platformer2d::engine_core::Vec2::new(at.x - 40.0, at.y));
    for i in 0..900 {
        sim.step(down_interact(i == 0));
        if possessed(&mut sim) == Some(dog) {
            break;
        }
    }
    assert_eq!(possessed(&mut sim), Some(dog), "setup: the player never possessed the dog");

    let away = a_neighbour(&mut sim);
    assert_eq!(take_the_door(&mut sim, dog, &away), away, "setup: the dog did not take the door");
    sim.step_n(base(), 30);
    // Release: a fresh Down+Interact press.
    sim.step(base());
    sim.step(down_interact(true));
    sim.step_n(base(), 10);
    assert_eq!(possessed(&mut sim), None, "setup: the release did not take");
    let left_at = match dogs(&mut sim)[..] {
        [(_, ref id, pos)] if *id == dog_id => pos,
        ref other => panic!("setup: the dog is not alone in '{away}' after the release: {other:?}"),
    };

    assert_eq!(take_the_door(&mut sim, player, HOME), HOME, "setup: the player did not go home");
    sim.step_n(base(), 30);
    assert_eq!(
        dogs(&mut sim),
        Vec::new(),
        "the hub rebuilt the dog at home, but it was left in '{away}'"
    );

    // The save keeps it: a fresh process booted from the file this run wrote
    // does not build the dog at home either.
    let file = sim
        .world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .clone();

    assert_eq!(take_the_door(&mut sim, player, &away), away, "setup: the player did not go back");
    sim.step_n(base(), 2);
    let back = dogs(&mut sim);
    assert!(
        matches!(back[..], [(_, ref id, pos)] if *id == dog_id && pos.distance(left_at) < 40.0),
        "the dog is not where it was left in '{away}' ({left_at:?}): {back:?}"
    );

    let mut fresh = fixed_60hz_room_sim(HOME);
    fresh.step_n(base(), 8);
    fresh
        .world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0 = file;
    fresh
        .world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::durable_horizon::SaveRestored>()
        .0 = false;
    fresh.step_n(base(), 90);
    assert!(
        fresh
            .world()
            .resource::<ambition_platformer2d::actors::session::durable_horizon::SaveRestored>()
            .0,
        "setup: the load never landed"
    );
    assert_eq!(dogs(&mut fresh), Vec::new(), "a fresh process built the dog at home from a save that says it was left in '{away}'");
    let player = {
        let world = fresh.world_mut();
        let mut q = world.query_filtered::<Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
        q.single(world).expect("one primary player")
    };
    assert_eq!(take_the_door(&mut fresh, player, &away), away, "setup: the fresh player did not take the door");
    fresh.step_n(base(), 2);
    let loaded = dogs(&mut fresh);
    assert!(
        matches!(loaded[..], [(_, ref id, pos)] if *id == dog_id && pos.distance(left_at) < 40.0),
        "a fresh process did not build the dog where the save says it was left ({left_at:?}): {loaded:?}"
    );
}
