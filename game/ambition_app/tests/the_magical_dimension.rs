//! The Magical dimension (Mary-O's), its first region
//! (`tools/ambition_ldtk_tools/specs/magical/make_specs.py`; Jon, 2026-10-30:
//! "start authoring the maryo-dimension for the ambition game"). The player
//! walks it by keys: a loop from Starlight Square down its old well, along the
//! Starless Hollow, up through the gate garden's hatch and east to the square
//! again; the Dune Road out of town and back; and the square's people talk.

use crate::the_pirate_port::{a_session_in, go, talk};

const GARDEN: &str = "magical_gate_garden";
const SQUARE: &str = "magical_starlight_square";
const HOLLOW: &str = "magical_starless_hollow";
const ROAD: &str = "magical_stardune_road";

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
