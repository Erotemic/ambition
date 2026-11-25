#![cfg(feature = "rl_sim")]
//! WORLD-ACCEPTANCE, the walked route: the player goes from the hub to Alice
//! and on to Bob with its own inputs, room by room, and is never put anywhere.
//!
//! Each room is walked by `common::walk::walk_through` (the surface graph
//! for the player's own body, the input `follow_leg` gives, Interact at a
//! door, the shipped room transition), here through the sim harness with an
//! analog stick, the agent's input. The playthrough walks the same road by
//! keys in the shipped App and asserts its facts
//! (`a_playthrough_of_the_persistent_world`).

use ambition_app::rl_sim::AmbitionSim as _;

use crate::common::walk::{through_the_raid, walk_through, Pad, RaidStart};
use crate::common::{base, fixed_60hz_room_options};

const HUB: &str = "central_hub_complex";

/// The rooms from the hub to Alice and Bob, each through an exit of the room
/// before: the playthrough's route. Alice is in the lane outside the
/// workshop, and Bob in the woods past Hobford. The workshop's way out is shut
/// until its raid is beaten, so the player lives through it first.
const ROUTE_TO_BOB: &[&str] = &[
    "intro_wake_room",
    "intro_hobford_lane",
    "intro_millstream",
    "intro_hobford",
    "intro_woods",
];

/// ⭐ THE PLAYER WALKS FROM THE HUB TO ALICE AND BOB. Each crossing is the shipped
/// room transition, and the body got to each exit by its own movement: the
/// live room changes to each room of the route in turn. No step puts the body
/// anywhere.
#[test]
fn the_player_walks_from_the_hub_to_alice_and_bob() {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB),
    )
    .expect("the hub boots");
    sim.step_n(base(), 30);
    for target in ROUTE_TO_BOB {
        let from = sim.observation().active_room.clone();
        // The workshop's way out is shut until its raid is beaten. The sim's
        // agent action answers no dialogue box, so the raid is started at the
        // barn door; the playthrough starts it in the Creator's conversation.
        if from == "intro_wake_room" {
            through_the_raid(&mut Pad::new(&mut sim), RaidStart::Door).unwrap_or_else(|why| panic!("the raid: {why}"));
        }
        let steps = walk_through(&mut Pad::new(&mut sim), target).unwrap_or_else(|why| panic!("`{from}` to `{target}`: {why}"));
        eprintln!("WALKED `{from}` to `{target}` in {steps} steps");
        sim.step_n(base(), 30);
    }
    assert_eq!(sim.observation().active_room, "intro_woods");
}

/// Has the save recorded `encounter` as won?
fn won(world: &bevy::prelude::World, encounter: &str) -> bool {
    world.get_resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().is_some_and(|save| {
        save.data().encounter(encounter) == ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared
    })
}

/// Is `flag` set in the save?
fn flagged(world: &bevy::prelude::World, flag: &str) -> bool {
    world
        .get_resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .is_some_and(|save| save.data().flag(flag))
}

/// Press Interact where the player stands until `flag` is set.
fn interact_until(pad: &mut Pad, flag: &str) -> Result<(), String> {
    use crate::common::walk::{Press, Walker as _};
    for step in 0..240 {
        if flagged(pad.world(), flag) {
            return Ok(());
        }
        pad.step(Press { interact: step % 20 < 2, confirm: step % 10 == 5, ..Press::default() });
    }
    Err(format!("Interact for 240 frames did not set `{flag}`"))
}

/// ⭐ LEVEL-1, THE FAR HALF OF THE ROUTE: from the woods up the hill path to
/// the gate clearing, where the barrier lever lifts the boom, and down the
/// freight track to the siding, where Scout fights the goblins.
#[test]
fn the_player_walks_from_the_woods_to_the_siding_and_wins_its_fight() {
    use crate::common::walk::{fight, walk_to, FRAMES_FOR_A_ROOM};
    use ambition_platformer2d::engine_core as ae;
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("intro_woods"))
        .expect("the woods boot");
    sim.step_n(base(), 30);
    for target in ["intro_high_meadow", "intro_gate_clearing"] {
        let from = sim.observation().active_room.clone();
        let steps = walk_through(&mut Pad::new(&mut sim), target).unwrap_or_else(|why| panic!("`{from}` to `{target}`: {why}"));
        eprintln!("WALKED `{from}` to `{target}` in {steps} steps");
    }
    // The boom across the road: its lever is on the road's side (x 1504, 24
    // wide, on the hilltop at y 672).
    walk_to(&mut Pad::new(&mut sim), ae::Vec2::new(1516.0, 672.0 - 24.0)).unwrap_or_else(|why| panic!("to the barrier lever: {why}"));
    interact_until(&mut Pad::new(&mut sim), "hobford_barrier_lifted").unwrap_or_else(|why| panic!("the barrier lever: {why}"));
    // The boom comes down on a gate tick after the flag.
    sim.step_n(base(), 30);
    let steps = walk_through(&mut Pad::new(&mut sim), "intro_freight_yard").unwrap_or_else(|why| panic!("the clearing to the siding: {why}"));
    eprintln!("WALKED `intro_gate_clearing` to `intro_freight_yard` in {steps} steps");

    // The yard fight starts at its trigger on the platform (x 440-504, floor
    // y 512); the crew, then the foreman.
    const YARD_FIGHT: &str = "intro_freight_yard_fight";
    // The foreman's cutscene plays on the way in (x 300): the walk takes its
    // beats, as a player does, and the fight's trigger is past it.
    let _ = walk_to(&mut Pad::new(&mut sim), ae::Vec2::new(472.0, 512.0 - 24.0));
    let goblin = |world: &bevy::prelude::World, entity: bevy::prelude::Entity| {
        world
            .get::<ambition_platformer2d::characters::actor::CharacterForm>(entity)
            .is_some_and(|form| matches!(form.id(), "npc_lab_raider" | "npc_goblin_brute"))
    };
    let frames = fight(
        &mut Pad::new(&mut sim),
        goblin,
        |world| won(world, YARD_FIGHT),
        ae::Vec2::new(472.0, 512.0 - 24.0),
        FRAMES_FOR_A_ROOM * 2,
    )
    .unwrap_or_else(|why| panic!("the yard fight: {why}"));
    eprintln!("WON `{YARD_FIGHT}` in {frames} frames");

    assert_eq!(sim.observation().active_room, "intro_freight_yard");
}
