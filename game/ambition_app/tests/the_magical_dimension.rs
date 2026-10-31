//! The Magical dimension (Mary-O's), its first region
//! (`tools/ambition_ldtk_tools/specs/magical/make_specs.py`; Jon, 2026-10-30:
//! "start authoring the maryo-dimension for the ambition game"). The player
//! walks it by keys: a loop from Starlight Square down its old well, along the
//! Starless Hollow, up through the gate garden's hatch and east to the square
//! again; the Dune Road out of town and back; and the square's people talk.
//! Past the road, the Void Castle (Jon, 2026-10-30: "add a void lizard
//! dungeon, but he is not an enemy here ... You can talk to them"): down one
//! stair of the keep to the lizard's bridge and up the other, and Mary-O and
//! the lizard talk.

use crate::the_pirate_port::{a_session_in, go, talk};

const GARDEN: &str = "magical_gate_garden";
const SQUARE: &str = "magical_starlight_square";
const HOLLOW: &str = "magical_starless_hollow";
const ROAD: &str = "magical_stardune_road";
const KEEP: &str = "magical_void_keep";
const BRIDGE: &str = "magical_void_bridge";

#[test]
fn the_square_the_hollow_and_the_garden_are_a_loop() {
    let mut app = a_session_in(SQUARE);
    go(&mut app, HOLLOW, Some("square_well"));
    go(&mut app, GARDEN, Some("hollow_hatch_shaft"));
    go(&mut app, SQUARE, Some("garden_east"));
}

#[test]
fn the_dune_road_lies_east_of_the_square() {
    let mut app = a_session_in(SQUARE);
    go(&mut app, ROAD, Some("square_east"));
    go(&mut app, SQUARE, Some("road_west"));
}

/// The baker and the lamplighter (`dialogue/sandbox/magical.yarn`).
#[test]
fn the_squares_people_talk() {
    let mut app = a_session_in(SQUARE);
    talk(&mut app, "npc_magical_celeste", "What do you bake?");
    talk(&mut app, "npc_magical_lumen", "Talked into it?");
}

#[test]
fn the_void_castle_is_a_loop_and_its_lizard_talks() {
    let mut app = a_session_in(ROAD);
    go(&mut app, KEEP, Some("road_east"));
    talk(&mut app, "mary_o", "A truce? With the void?");
    go(&mut app, BRIDGE, Some("keep_west_stair"));
    talk(&mut app, "void_lizard", "Are you going to fight me?");
    go(&mut app, KEEP, Some("bridge_east_shaft"));
    go(&mut app, ROAD, Some("keep_west"));
}
