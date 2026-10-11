//! A coconut is a thing to throw, and it hurts the body it strikes.
//!
//! The engine's item step says an item reached a body and how fast. Ambition's
//! rule (`ambition_content::items::thrown_impact`) makes that a blow for the
//! items it lists: one arrival is one blow, and the item bounces off. The rule
//! names no thrower, so a coconut that falls off a palm is the same blow as
//! one a hand threw.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::Entity;

/// A room with a floor, a player, nothing that fights and a high ceiling: a
/// coconut needs room to fall.
const ROOM: &str = "hall_of_biomes";

type GroundItem = ambition_platformer2d::held_items::GroundItem;
type ItemCustody = ambition_platformer2d::held_items::ItemCustody;

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the room has a player");
    (*kin, health.health.current)
}

/// A coconut in the world at `at`, not at rest: it falls from there.
fn loose_coconut(sim: &mut Platformer2dSimHarness, at: ae::Vec2) -> Entity {
    let spec = ambition_platformer2d::characters::brain::held_item_by_id("coconut").expect("the coconut is a held item");
    sim.world_mut().spawn(GroundItem::at_rest(spec, at, ae::Vec2::new(9.0, 7.5))).id()
}

#[test]
fn a_coconut_that_falls_on_the_player_is_one_blow_and_bounces_off() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    // Past any protection of the first moments in a room.
    for _ in 0..180 {
        sim.step(base());
    }
    let (kin, before) = player(&mut sim);
    // A fall of 150 arrives at about 640, which is over the coconut's bar.
    let nut = loose_coconut(&mut sim, kin.pos - ae::Vec2::new(0.0, kin.size.y * 0.5 + 150.0));
    let mut went_up = false;
    for _ in 0..240 {
        sim.step(base());
        went_up |= sim.world().get::<GroundItem>(nut).is_some_and(|item| item.vel.y < -20.0);
    }
    let (_, after) = player(&mut sim);
    assert_eq!(before - after, 2, "a coconut that falls on the player is one blow of 2; health went {before} -> {after}");
    assert!(went_up, "the coconut bounces off the body it strikes");
    assert!(sim.world().get::<GroundItem>(nut).is_some(), "the coconut is still in the world: it can be thrown again");
}

#[test]
fn the_player_picks_a_coconut_up_and_throws_it() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    for _ in 0..60 {
        sim.step(base());
    }
    let (kin, _) = player(&mut sim);
    // At the player's feet, so the fall is too short to be a blow.
    let nut = loose_coconut(&mut sim, kin.pos + ae::Vec2::new(0.0, kin.size.y * 0.5 - 12.0));
    for _ in 0..30 {
        sim.step(base());
    }
    let held = |sim: &Platformer2dSimHarness| matches!(sim.world().get::<ItemCustody>(nut), Some(ItemCustody::Held { .. }));
    sim.step(AgentAction { attack: true, ..base() });
    sim.step(base());
    assert!(held(&sim), "an Attack press over a coconut picks it up");
    let (_, before) = player(&mut sim);
    // A coconut has no swing and no shot: the next Attack throws it.
    sim.step(AgentAction { attack: true, ..base() });
    sim.step(base());
    assert!(!held(&sim), "an Attack press with a coconut in hand throws it");
    let item = sim.world().get::<GroundItem>(nut).expect("the thrown coconut is in the world");
    assert!(item.vel.length() > 200.0, "a thrown coconut leaves the hand fast; it left at {:?}", item.vel);
    for _ in 0..120 {
        sim.step(base());
    }
    let (_, after) = player(&mut sim);
    assert_eq!(before, after, "the thrower's own coconut does not hurt the thrower");
}

/// A body that is not a player's is hurt the same: the rule asks no question
/// of sides. The sandbag of the scroll lab is an enemy that stands still to be
/// hit.
#[test]
fn a_coconut_that_falls_on_a_sandbag_hurts_it() {
    let mut sim = fixed_60hz_room_sim("scroll_lab");
    for _ in 0..120 {
        sim.step(base());
    }
    let sandbag = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut q = world.query::<(&ambition_platformer2d::characters::actor::WornCharacter, &ae::BodyKinematics, &BodyHealth)>();
        q.iter(world)
            .find(|(worn, _, _)| worn.id() == "sandbag")
            .map(|(_, kin, health)| (*kin, health.health.current))
            .expect("the room has a sandbag")
    };
    let (kin, _) = sandbag(&mut sim);
    // Beside it: a body far from every player sleeps.
    sim.teleport_player((kin.pos.x - 90.0, kin.pos.y));
    for _ in 0..60 {
        sim.step(base());
    }
    let (kin, before) = sandbag(&mut sim);
    let nut = loose_coconut(&mut sim, kin.pos - ae::Vec2::new(0.0, kin.size.y * 0.5 + 120.0));
    let mut fastest = 0.0_f32;
    for _ in 0..180 {
        sim.step(base());
        fastest = fastest.max(sim.world().get::<GroundItem>(nut).map_or(0.0, |item| item.vel.length()));
    }
    let (_, after) = sandbag(&mut sim);
    assert!(fastest > 300.0, "premise: the coconut fell; its fastest was {fastest}");
    assert_eq!(before - after, 2, "a coconut that falls on the sandbag is one blow of 2; its health went {before} -> {after}");
}
