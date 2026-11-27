//! Walk the high road and fight for the grave goods with player inputs.

use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::PersistedEncounterState;
use ambition_platformer2d::platformer::lifecycle::sole_live_room_component;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use bevy::prelude::World;

use crate::common::walk::{Pad, Walker as _, fight, walk_through, walk_to};
use crate::common::{base, fixed_60hz_room_sim};

fn gate_stands(world: &mut World, gate: &str) -> bool {
    sole_live_room_component::<FeatureEcsWorldOverlay>(world)
        .expect("the live room has a collision overlay")
        .gate_solids
        .iter()
        .any(|solid| solid.name == format!("gated_lock:{gate}"))
}

#[test]
fn the_high_road_goes_to_brackenford_and_back_after_the_stair_opens() {
    let mut sim = fixed_60hz_room_sim("intro_hobford_lane");
    sim.step_n(base(), 30);
    assert!(gate_stands(sim.world_mut(), "fantasy_high_trail_gate"));

    sim.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("fantasy_high_trail_open", true);
    sim.step_n(base(), 60);
    assert!(!gate_stands(sim.world_mut(), "fantasy_high_trail_gate"));

    let mut pad = Pad::new(&mut sim);
    for room in [
        "fantasy_mountain_ascent",
        "fantasy_mountain_pass",
        "fantasy_mountain_shelter",
        "fantasy_mountain_pass",
        "fantasy_mountain_descent",
        "fantasy_brackenford",
        "fantasy_old_watch",
        "fantasy_brackenford",
        "fantasy_mountain_descent",
        "fantasy_mountain_pass",
        "fantasy_mountain_ascent",
        "intro_hobford_lane",
    ] {
        walk_through(&mut pad, room).unwrap_or_else(|why| panic!("walk to {room}: {why}"));
    }
}

#[test]
fn the_small_mounds_link_the_moor_and_the_burial_vaults() {
    let mut sim = fixed_60hz_room_sim("intro_moor");
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    for room in [
        "intro_barrow_cairnfield",
        "intro_barrow_votive",
        "intro_barrow_cairnfield",
        "intro_barrow_watch_crypt",
        "intro_barrow_tally_vault",
        "intro_barrow",
        "intro_moor",
    ] {
        walk_through(&mut pad, room).unwrap_or_else(|why| panic!("walk to {room}: {why}"));
    }
}

#[test]
fn defeating_the_barrow_king_opens_the_grave_goods_grille() {
    const TOMB: &str = "intro_barrow_royal_tomb";
    const GRILLE: &str = "royal_treasury_grille";
    const VIGIL: &str = "intro_barrow_king_vigil";

    let mut sim = fixed_60hz_room_sim(TOMB);
    sim.step_n(base(), 30);
    assert!(gate_stands(sim.world_mut(), GRILLE));

    let mut pad = Pad::new(&mut sim);
    let approach = Vec2::new(340.0, 432.0);
    walk_to(&mut pad, approach).expect("reach the vigil trigger");
    fight(
        &mut pad,
        |world, body| world.get::<CharacterForm>(body).is_some_and(|form| form.id() == "barrow_king"),
        |world| world.resource::<AmbitionGameSave>().data().encounter(VIGIL) == PersistedEncounterState::Cleared,
        approach,
        2400,
    )
    .expect("defeat the king with the opening kit");
    for _ in 0..60 {
        pad.step(Default::default());
    }
    assert!(!gate_stands(pad.world_mut(), GRILLE));
    walk_to(&mut pad, Vec2::new(1380.0, 432.0)).expect("reach the grave goods");
    walk_through(&mut pad, "intro_barrow_tally_vault").expect("return from the royal tomb");
}
