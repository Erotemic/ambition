//! The Map page's regions, on the authored LDtk worlds
//! (`ambition_menu::map::view`).
//!
//! Jon (2026-10-30): "the LDTK layout of connected components of loading zones
//! is the base for the exterior maps (or interior of a large dungeon)". Rooms
//! joined by edge exits are one region; a door enters a building, a region of
//! its own, and inside one the map shows the region outside it.

use ambition_platformer2d::ldtk_map::ActiveLdtkProject;
use ambition_platformer2d::menu::map::view::{map_view, region};
use ambition_platformer2d::menu::map::{map_room_node, MapMenuState};

use crate::the_pirate_port::a_session_in;

fn authored_map() -> MapMenuState {
    let app = a_session_in("pirate_port_waterfront");
    let project = app.world().resource::<ActiveLdtkProject>();
    MapMenuState { rooms: project.0.levels.iter().map(map_room_node).collect(), ..Default::default() }
}

fn region_of(map: &MapMenuState, room: &str) -> Vec<String> {
    let start = map.rooms.iter().position(|r| r.id == room).unwrap_or_else(|| panic!("{room} is on the map"));
    let mut ids: Vec<String> = region(&map.rooms, start).iter().map(|&i| map.rooms[i].id.clone()).collect();
    ids.sort();
    ids
}

#[test]
fn the_maps_regions_follow_the_worlds_edge_exits() {
    let map = authored_map();
    // The Pirate Port's shore, plaza to point, joined by side exits.
    let port = region_of(&map, "pirate_wreckers_reef");
    for room in ["pirate_port_gate_plaza", "pirate_port_waterfront", "pirate_port_docks", "pirate_lighthouse_point"] {
        assert!(port.contains(&room.to_string()), "{room} is in the port's region: {port:?}");
    }
    assert!(!port.contains(&"pirate_port_tavern".to_string()), "the tavern is behind a door");
    // The Math town's mine is under the woods, joined by a vertical exit.
    assert!(region_of(&map, "math_mobius_mine").contains(&"math_fractal_woods".to_string()));
    // The evil dimension's road west: the forest, the moor, the allotments,
    // the hamlet; the bakery behind its door is its own.
    let moor = region_of(&map, "evil_searchlight_moor");
    for room in ["evil_gate_forest", "evil_castle_allotments", "evil_lower_godwin"] {
        assert!(moor.contains(&room.to_string()), "{room} is in the moor's region: {moor:?}");
    }
    assert_eq!(region_of(&map, "evil_crumbs_bakery"), ["evil_crumbs_bakery"]);
}

#[test]
fn inside_the_bakery_the_map_shows_lower_godwin_and_its_door() {
    let map = authored_map();
    let view = map_view(&map, "evil_crumbs_bakery", 1.7).expect("the bakery's door is on the map");
    assert!(view.inside.is_some());
    let here: Vec<&str> = view.rooms.iter().filter(|r| r.here).map(|r| r.id.as_str()).collect();
    assert_eq!(here, ["evil_lower_godwin"]);
    assert_eq!(view.marked_door, Some(0));
}
