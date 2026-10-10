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

use crate::common::walk::{walk_through, Pad};
use crate::common::{a_save_that_has_seen_the_hub_intro, base, fixed_60hz_room_options};

const HUB: &str = "central_hub_complex";

/// The rooms from the hub to Alice and Bob, each through an exit of the room
/// before: the playthrough's route. Before Bob's survey, the lock wall
/// of Alice's private return stands on the floor between Alice and the exit
/// to Bob (112 px tall). The player gets over it with the air jump.
const ROUTE_TO_BOB: &[&str] = &[
    "intro_wake_room",
    "intro_raid_corridor",
    "intro_escape_shaft",
    "drain_alley",
    "under_town_pipes",
    "alice_relay",
    "bob_relay",
];

/// ⭐ THE PLAYER WALKS FROM THE HUB TO ALICE AND BOB. Each crossing is the shipped
/// room transition, and the body got to each exit by its own movement: the
/// live room changes to each room of the route in turn. No step puts the body
/// anywhere.
#[test]
fn the_player_walks_from_the_hub_to_alice_and_bob() {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("the hub boots");
    sim.step_n(base(), 30);
    for target in ROUTE_TO_BOB {
        let from = sim.observation().active_room.clone();
        let steps = walk_through(&mut Pad::new(&mut sim), target).unwrap_or_else(|why| panic!("`{from}` to `{target}`: {why}"));
        eprintln!("WALKED `{from}` to `{target}` in {steps} steps");
        sim.step_n(base(), 30);
    }
    assert_eq!(sim.observation().active_room, "bob_relay");
}
