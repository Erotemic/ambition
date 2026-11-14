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
