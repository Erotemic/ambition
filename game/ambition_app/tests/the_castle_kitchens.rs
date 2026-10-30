//! Castle Godwin as one connected place: its lower floor of staff
//! (`tools/ambition_ldtk_tools/specs/castle_kitchens/make_specs.py`, talk in
//! `dialogue/sandbox/castle_godwin.yarn`) joined to its upper floor of combat
//! (`specs/castle_godwin/make_specs.py`) by their edges on a faithful flat
//! map.
//!
//! Jon (2026-10-30): "It needs to feel like a connected world, not a series
//! of disconnected doors." Inside the castle the rooms touch and join by
//! edge exits: a stair down from the gatehouse to the kitchens, east along
//! the lower floor to the Complaints Department and the cells and into the
//! lab tower partway up; the workshop off the tower's foot; the barracks into
//! the tower above; the tower's top into the great hall. One region on the
//! map; the forest outside is behind the front door.

use ambition_platformer2d::ldtk_map::ActiveLdtkProject;
use ambition_platformer2d::menu::map::view::region;
use ambition_platformer2d::menu::map::{map_room_node, MapMenuState};

use crate::the_pirate_port::{a_session_in, go, talk};

const KITCHENS: &str = "castle_godwin_kitchens";
const GATEHOUSE: &str = "castle_godwin_gatehouse";
const COMPLAINTS: &str = "castle_godwin_complaints";
const CELLS: &str = "castle_godwin_cells";
const TOWER: &str = "castle_godwin_tower";
const WORKSHOP: &str = "castle_godwin_workshop";

/// The staff talk in the kitchens; the stair goes up through the ceiling to
/// the gatehouse, and back down.
#[test]
fn the_castle_staff_talk_in_the_kitchens_under_the_gatehouse() {
    let mut app = a_session_in(KITCHENS);
    talk(&mut app, "npc_castle_osgood", "What do you do here?");
    talk(&mut app, "npc_castle_hargreave", "What are you cooking?");
    talk(&mut app, "npc_castle_dunmore", "What does the Guild want?");
    go(&mut app, GATEHOUSE, Some("kitchens_stair"));
    go(&mut app, KITCHENS, Some("castle_godwin_gatehouse_stair"));
}

/// East along the lower floor: the Complaints Department (Agnes Mudge, ticket
/// 4,117), the cells (Ambrose Quill, the trusty), and into the lab tower
/// partway up, and back.
#[test]
fn the_lower_floor_runs_from_the_kitchens_through_the_cells_into_the_tower() {
    let mut app = a_session_in(KITCHENS);
    go(&mut app, COMPLAINTS, Some("kitchens_east"));
    talk(&mut app, "npc_evil_mudge", "What number are you?");
    go(&mut app, CELLS, Some("complaints_east"));
    talk(&mut app, "npc_castle_quill", "What are you in for?");
    go(&mut app, TOWER, Some("cells_east"));
    go(&mut app, CELLS, Some("castle_godwin_tower_cells"));
    go(&mut app, COMPLAINTS, Some("cells_west"));
}

/// The Prototype Workshop, off the lab tower's foot: Dr. Gideon Sprocket
/// talks, and the tower is back west.
#[test]
fn the_castle_engineer_talks_in_the_prototype_workshop() {
    let mut app = a_session_in(WORKSHOP);
    talk(&mut app, "npc_castle_sprocket", "What do you make here?");
    go(&mut app, TOWER, Some("workshop_west"));
    go(&mut app, WORKSHOP, Some("castle_godwin_tower_foot"));
}

/// The castle is one region of the map: every room inside it joined to the
/// others by edges, the forest outside its front door not.
#[test]
fn castle_godwin_is_one_connected_region() {
    let app = a_session_in(KITCHENS);
    let project = app.world().resource::<ActiveLdtkProject>();
    let map = MapMenuState { rooms: project.0.levels.iter().map(map_room_node).collect(), ..Default::default() };
    let start = map.rooms.iter().position(|r| r.id == GATEHOUSE).expect("the gatehouse is on the map");
    let ids: Vec<&str> = region(&map.rooms, start).iter().map(|&i| map.rooms[i].id.as_str()).collect();
    for room in [GATEHOUSE, "castle_godwin_barracks", TOWER, "castle_godwin_great_hall", KITCHENS, COMPLAINTS, CELLS, WORKSHOP] {
        assert!(ids.contains(&room), "{room} is in the castle's region: {ids:?}");
    }
    assert!(!ids.contains(&"evil_gate_forest"), "the forest is outside the front door: {ids:?}");
}

/// The shortcut: the lever by the jailer's desk opens the cracked trapdoor
/// in the barracks' floor over the cells, and the ladder goes up through it
/// to the barracks and back down. Before the lever, the stone is whole.
#[test]
fn the_lever_in_the_cells_opens_the_trapdoor_up_to_the_barracks() {
    use crate::the_note_travels_from_alice_to_bob::tap;
    use ambition_platformer2d::engine_core as ae;
    use bevy::prelude::KeyCode;

    let lever = |app: &bevy::prelude::App| {
        app.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .switch("castle_godwin_trapdoor_lever")
    };
    let mut app = a_session_in(CELLS);
    assert!(!lever(&app), "control: the lever is not pulled");
    crate::the_pirate_port::go_to(&mut app, ae::Vec2::new(612.0, 560.0), "the trapdoor lever");
    tap(&mut app, KeyCode::KeyF);
    for _ in 0..30 {
        app.update();
    }
    assert!(lever(&app), "the lever is pulled");
    go(&mut app, "castle_godwin_barracks", Some("cells_trapdoor"));
    go(&mut app, CELLS, Some("castle_godwin_barracks_trapdoor"));
}

/// The secret: the kitchens' west wall is false at the floor. Walk into it
/// and you are in the Henchpersons' Guild's room (the strike fund in its
/// chest), and back. The map does not show the room behind it until it is
/// found.
#[test]
fn a_false_wall_in_the_kitchens_hides_the_guild_room() {
    use ambition_platformer2d::menu::map::view::map_view;

    let mut app = a_session_in(KITCHENS);
    let shown = |app: &bevy::prelude::App, visited: &[&str]| -> Vec<String> {
        let project = app.world().resource::<ActiveLdtkProject>();
        let map = MapMenuState {
            rooms: project.0.levels.iter().map(map_room_node).collect(),
            visited: visited.iter().map(|v| v.to_string()).collect(),
            ..Default::default()
        };
        map_view(&map, KITCHENS, 1.7).expect("the kitchens are on the map").rooms.iter().map(|r| r.id.clone()).collect()
    };
    let before = shown(&app, &[KITCHENS]);
    assert!(!before.contains(&"castle_godwin_guild_room".to_string()), "the secret room is not on the map: {before:?}");
    assert!(before.contains(&COMPLAINTS.to_string()), "control: the open way east is: {before:?}");
    go(&mut app, "castle_godwin_guild_room", Some("secret_kitchens_west"));
    go(&mut app, KITCHENS, Some("secret_guild_east"));
    let after = shown(&app, &[KITCHENS, "castle_godwin_guild_room"]);
    assert!(after.contains(&"castle_godwin_guild_room".to_string()), "found, it is on the map: {after:?}");
}
