//! The Map page's regions, on the authored LDtk worlds
//! (`ambition_menu::map::view`).
//!
//! Jon (2026-10-30): "the LDTK layout of connected components of loading zones
//! is the base for the exterior maps (or interior of a large dungeon)". Rooms
//! joined by edge exits are one region; a door enters a building, a region of
//! its own, and inside one the map shows the region outside it.

use ambition_platformer2d::ldtk_map::ActiveLdtkProject;
use ambition_platformer2d::menu::map::browser::{MapAccess, MapAction, MapAtlas, MapBrowser, MapEntryKind, MapHierarchy};
use ambition_platformer2d::menu::map::view::{map_view, region};
use ambition_platformer2d::menu::map::{map_room_node, MapMenuState};

use crate::the_pirate_port::a_session_in;

fn authored_map() -> MapMenuState {
    let app = a_session_in("pirate_port_waterfront");
    let project = app.world().resource::<ActiveLdtkProject>();
    // The player's map, with its fog of war (the developer map shows all).
    MapMenuState { rooms: project.0.levels.iter().map(map_room_node).collect(), reveal_all: false, ..Default::default() }
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

#[test]
fn the_fantasy_overworld_opens_the_barrows_as_a_separate_map() {
    use ambition_platformer2d::world::world_manifest::WorldManifest;

    let app = a_session_in("intro_moor");
    let rooms: Vec<_> = app.world().resource::<ActiveLdtkProject>().0.levels.iter().map(map_room_node).collect();
    let hierarchy = MapHierarchy {
        world_titles: app.world().resource::<WorldManifest>().worlds.iter()
            .filter_map(|world| Some((world.id.as_str().to_owned(), world.map_title.clone()?)))
            .collect(),
        ..Default::default()
    };
    let atlas = MapAtlas::new(&rooms, Some(&hierarchy));
    let visits = Default::default();
    let access = MapAccess { current_room: Some("intro_moor"), visited: &visits, reveal_all: true, can_travel: false };
    let mut browser = MapBrowser::default();
    browser.apply(&atlas, access, 1.7, MapAction::Fit);
    let overworld = browser.view(&atlas, access, 1.7);
    assert_eq!(overworld.title, "Fantasy Dimension");
    let entrance = overworld.entries.iter().find(|entry| entry.title == "The Barrows").unwrap();
    assert_eq!(entrance.kind, MapEntryKind::Landmark);
    assert_eq!(entrance.pieces.len(), 3);
    assert!(overworld.entries.iter().all(|entry| !entry.title.contains("WHEELWORKS")));
    browser.apply(&atlas, access, 1.7, MapAction::Open(entrance.id));
    let dungeon = browser.view(&atlas, access, 1.7);
    assert_eq!(dungeon.title, "The Barrows");
    assert_eq!(dungeon.parent_title.as_deref(), Some("Fantasy Dimension"));
    assert_eq!(dungeon.entries.len(), 7);
    assert!(dungeon.entries.iter().any(|entry| entry.title == "THE WHEELWORKS"));
    assert!(dungeon.links.len() >= 6);

    browser.apply(&atlas, access, 1.7, MapAction::Up);
    let returned = browser.view(&atlas, access, 1.7);
    let focused = returned.entries.iter().find(|entry| entry.id == entrance.id).unwrap();
    assert!(focused.selected);
    assert!(focused.pieces.iter().any(|piece| piece.width() > entrance.pieces[0].width()));
}

#[test]
fn a_named_room_travel_request_reaches_another_dimension() {
    use ambition_platformer2d::platformer::host_intents::write_host_intent;
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::session::rooms::RoomTravelRequested;
    use crate::neighbor_prefetch_prepares_rooms::{alice, room_of};

    let mut app = crate::alices_reading_reaches_bob::the_shipped_session();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(17),
    ));
    let body = alice(&mut app);
    let initial = LiveBodyId::of_entity(app.world(), body).expect("the controlled body has an ID");
    let destination = "pirate_port_gate_plaza";
    assert_ne!(room_of(&app, body).as_deref(), Some(destination));
    write_host_intent(app.world_mut(), RoomTravelRequested {
        body: initial.clone(),
        destination: destination.into(),
    });

    for _ in 0..120 {
        app.update();
        let body = alice(&mut app);
        if room_of(&app, body).as_deref() == Some(destination) {
            assert_eq!(LiveBodyId::of_entity(app.world(), body).unwrap().sim_id, initial.sim_id);
            return;
        }
    }
    panic!("the travel request did not reach {destination}");
}
