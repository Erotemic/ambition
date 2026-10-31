//! The Magical dimension (Mary-O's), its first region
//! (`tools/ambition_ldtk_tools/specs/magical/make_specs.py`; Jon, 2026-10-30:
//! "start authoring the maryo-dimension for the ambition game"). The player
//! walks it by keys: a loop from Starlight Square down its old well, along the
//! Starless Hollow, up through the gate garden's hatch and east to the square
//! again; the Dune Road out of town and back; and the square's people talk.
//! Past the road, the Void Castle (Jon, 2026-10-30: "add a void lizard
//! dungeon, but he is not an enemy here ... You can talk to them"): down one
//! stair of the keep to the lizard's bridge and up the other, and the lizard
//! talks. Past the bridge, the Void (black and white): the rift, the gallery
//! under it, the falls, the heart, walked round; its void stars talk. Mary-O
//! meets the robot in the gate garden.

use ambition_platformer2d::engine_core as ae;

use crate::the_pirate_port::{a_session_in, break_the_wall, go, talk};

const GARDEN: &str = "magical_gate_garden";
const SQUARE: &str = "magical_starlight_square";
const HOLLOW: &str = "magical_starless_hollow";
const ROAD: &str = "magical_stardune_road";
const KEEP: &str = "magical_void_keep";
const BRIDGE: &str = "magical_void_bridge";
const RIFT: &str = "magical_void_rift";
const GALLERY: &str = "magical_void_gallery";
const FALLS: &str = "magical_void_falls";
const HEART: &str = "magical_void_heart";
const NURSERY: &str = "magical_void_nursery";
const LEDGES: &str = "magical_void_ledges";
const COLISEUM: &str = "magical_void_coliseum";
const WASTES: &str = "magical_void_wastes";

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

/// The baker, the lamplighter and Mary-O's Solid Snake, off duty
/// (`dialogue/sandbox/magical.yarn`).
#[test]
fn the_squares_people_talk() {
    let mut app = a_session_in(SQUARE);
    talk(&mut app, "npc_magical_celeste", "What do you bake?");
    talk(&mut app, "npc_magical_lumen", "Talked into it?");
    talk(&mut app, "solid_snake", "Why does everyone jump on you?");
}

#[test]
fn the_void_castle_is_a_loop_and_its_lizard_talks() {
    let mut app = a_session_in(ROAD);
    go(&mut app, KEEP, Some("road_east"));
    go(&mut app, BRIDGE, Some("keep_west_stair"));
    talk(&mut app, "void_lizard", "Are you going to fight me?");
    go(&mut app, KEEP, Some("bridge_east_shaft"));
    go(&mut app, ROAD, Some("keep_west"));
}

/// Mary-O is in the world: by the gate, where the robot comes through.
#[test]
fn mary_o_meets_the_robot_at_the_gate() {
    let mut app = a_session_in(GARDEN);
    talk(&mut app, "mary_o_tall", "Where's the void?");
}

/// The Void, round its loop: down from the rift into the gallery, east onto
/// the falls' shelf, down the falls to the heart, up the heart's shaft into
/// the gallery again; its curator and its elder talk.
#[test]
fn the_void_is_walked_round_and_its_stars_talk() {
    let mut app = a_session_in(BRIDGE);
    go(&mut app, RIFT, Some("bridge_east"));
    go(&mut app, GALLERY, Some("rift_down_west"));
    talk(&mut app, "void_star", "What is all this?");
    go(&mut app, FALLS, Some("gallery_east"));
    go(&mut app, HEART, Some("falls_foot"));
    talk(&mut app, "void_star", "Who is she?");
    go(&mut app, GALLERY, Some("heart_up"));
}

/// The heart's secret: its west wall breaks, and behind it is the nursery,
/// where a star goes the first time it goes out; its nurse talks.
#[test]
fn the_hearts_west_wall_breaks_into_the_nursery() {
    let mut app = a_session_in(HEART);
    break_the_wall(&mut app, "heart_nursery_wall", ae::Vec2::new(96.0, 560.0), -1.0);
    go(&mut app, NURSERY, Some("heart_west"));
    talk(&mut app, "void_star", "Who are they?");
}

/// The rift's pool: the way east to the falls crosses it, high on a ledge
/// over it or low on shards that crumble under a body and come back.
#[test]
fn the_rift_is_crossed_over_its_pool_to_the_falls() {
    let mut app = a_session_in(RIFT);
    go(&mut app, FALLS, Some("rift_down_east"));
}

/// The Unlit, where the void is hostile, is a loop: east from the rift along
/// the broken ledges, down into the ring, west across the wastes to the
/// falls' foot.
#[test]
fn the_unlit_is_a_loop_from_the_rift_back_to_the_falls() {
    let mut app = a_session_in(RIFT);
    go(&mut app, LEDGES, Some("rift_east"));
    go(&mut app, COLISEUM, Some("ledges_down"));
    go(&mut app, WASTES, Some("coliseum_west"));
    go(&mut app, FALLS, Some("wastes_west"));
}
