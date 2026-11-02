//! NAVIGATION, MISSING 2: a route over rooms at run time.
//!
//! `RoomSet::route` gives the fewest-rooms chain of zones from one room to
//! another. Each hop is held against the authority a crossing uses: the zone
//! is a zone of its room, and `transition_for_player` on that zone takes the
//! body to the hop's room.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::rooms::RoomSet;

use crate::common::{base, fixed_60hz_room_sim};

#[test]
#[ignore = "full Castle Godwin playthrough through the gauntlet and into the boss fight"]
fn autoplay_fights_through_castle_godwin_and_reaches_wolfenstein() {
    use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
    use ambition_platformer2d::scripted_input::install_autoplay;
    use crate::common::fixed_60hz_room_options;

    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("central_hub_complex")).expect("the hub builds");
    install_autoplay(sim.app_mut(), ambition_content::autoplay::CASTLE_AUTOPLAY);
    let mut visits = Vec::new();
    for frame in 0..60 * 600 {
        sim.app_mut().update();
        let seen = sim.observation();
        if visits.last() != Some(&seen.active_room) {
            eprintln!("[castle autoplay] frame={frame} room={}", seen.active_room);
            visits.push(seen.active_room.clone());
        }
        if frame % 600 == 0 {
            eprintln!("[castle autoplay] frame={frame} room={} pos={:?} hp={} mana={} resets={}",
                seen.active_room, seen.player_pos, seen.hp, seen.mana, seen.resets);
        }
        let fighting_wolfenstein = if seen.active_room == "castle_godwin_throne" && frame % 30 == 0 {
            let world = sim.world_mut();
            world.query::<(&ambition_platformer2d::boss_encounter::BossConfig,
                &ambition_platformer2d::boss_encounter::BossEncounter,
                &ambition_platformer2d::characters::actor::BodyHealth)>().iter(world)
                .any(|(config, encounter, health)| config.behavior.id == "wolfenstein_boss"
                    && encounter.encounter_phase() != ambition_platformer2d::boss_encounter::BossEncounterPhase::Dormant
                    && health.current() < health.max())
        } else { false };
        if fighting_wolfenstein {
            for room in ["evil_gate_forest", "castle_godwin_gatehouse", "castle_godwin_barracks",
                "castle_godwin_tower", "castle_godwin_great_hall", "castle_godwin_throne"] {
                assert!(visits.iter().any(|visited| visited == room), "the run skipped {room}: {visits:?}");
            }
            return;
        }
        std::thread::yield_now();
    }
    panic!("Castle Godwin autoplay did not finish: {visits:?}; last={:?}", sim.observation());
}

fn shipped_rooms() -> RoomSet {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 2);
    ambition_platformer2d::platformer::lifecycle::session_world_component::<RoomSet>(sim.world())
        .expect("the session keeps its room set")
        .clone()
}

/// Where the zone `zone` of room `room` takes a body, by the crossing rule.
fn crossing(rooms: &RoomSet, room: usize, zone: &str) -> Option<usize> {
    let definition = rooms.definition(room)?;
    let zone = rooms.spec(definition).loading_zones.iter().find(|z| z.id == zone)?;
    rooms
        .transition_for_player(definition, zone.aabb, ae::Vec2::ZERO, true)
        .map(|transition| transition.target_room)
}

#[test]
fn a_route_over_rooms_goes_by_zones_that_cross_where_it_says() {
    let rooms = shipped_rooms();
    let id = |name: &str| rooms.room_index_by_id(name).unwrap_or_else(|| panic!("no room `{name}`"));
    let (hub, alice) = (id("central_hub_complex"), id("alice_relay"));

    let route = rooms.route(hub, alice).expect("the hub reaches Alice");
    assert!(!route.is_empty());
    assert_eq!(route.first().map(|hop| hop.from), Some(hub));
    assert_eq!(route.last().map(|hop| hop.to), Some(alice));
    for pair in route.windows(2) {
        assert_eq!(pair[0].to, pair[1].from, "the route is not a chain: {route:?}");
    }
    for hop in &route {
        assert_eq!(
            crossing(&rooms, hop.from, &hop.zone),
            Some(hop.to),
            "the zone `{}` of `{}` does not cross to `{}`",
            hop.zone,
            rooms.rooms[hop.from].id,
            rooms.rooms[hop.to].id
        );
    }
    // Fewest rooms, measured by the crossing rule itself: the rooms each zone
    // of the last layer crosses to, layer by layer from the hub.
    let mut layer = vec![hub];
    let mut seen = vec![hub];
    let mut distance = 0;
    while !layer.contains(&alice) {
        layer = layer
            .iter()
            .flat_map(|room| rooms.rooms[*room].loading_zones.iter().filter_map(|z| crossing(&rooms, *room, &z.id)))
            .collect();
        layer.retain(|room| !seen.contains(room));
        layer.sort_unstable();
        layer.dedup();
        seen.extend(&layer);
        distance += 1;
        assert!(!layer.is_empty() && distance < 100, "the crossings never reach Alice");
    }
    assert_eq!(route.len(), distance, "the route is not the fewest rooms: {route:?}");
    assert_eq!(rooms.route(alice, alice), Some(Vec::new()));
    assert_eq!(rooms.route(hub, rooms.rooms.len()), None);
}

/// A room no zone leads into has no route to it. The control is the same
/// room with its zones: it is reached.
#[test]
fn a_room_that_no_zone_leads_into_has_no_route() {
    let rooms = shipped_rooms();
    let hub = rooms.room_index_by_id("central_hub_complex").expect("hub");
    let reached: Vec<usize> = (0..rooms.rooms.len()).filter(|room| rooms.route(hub, *room).is_some()).collect();
    let alice = rooms.room_index_by_id("alice_relay").expect("alice");
    assert!(reached.contains(&alice), "control: the hub reaches Alice");
    // Every room some route reaches is reached by its last hop's zone.
    for room in &reached {
        if let Some(last) = rooms.route(hub, *room).and_then(|route| route.last().cloned()) {
            assert_eq!(crossing(&rooms, last.from, &last.zone), Some(*room));
        }
    }
    let never: Vec<&str> = (0..rooms.rooms.len())
        .filter(|room| !reached.contains(room))
        .map(|room| rooms.rooms[room].id.as_str())
        .collect();
    eprintln!("ROUTE reached {} of {} rooms from the hub; not reached: {never:?}", reached.len(), rooms.rooms.len());
    for room in (0..rooms.rooms.len()).filter(|room| !reached.contains(room)) {
        let entered = (0..rooms.rooms.len()).any(|from| {
            rooms.rooms[from]
                .loading_zones
                .iter()
                .any(|zone| crossing(&rooms, from, &zone.id) == Some(room) && reached.contains(&from))
        });
        assert!(!entered, "`{}` is entered from a reached room and has no route", rooms.rooms[room].id);
    }
}

/// The shipped policy opens terminals, dials, crosses the Trail, and comes
/// home through ordinary simulation input and host intents.
#[test]
fn autoplay_uses_the_gate_network_and_returns_from_the_primeval_trail() {
    use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
    use ambition_platformer2d::scripted_input::install_autoplay;
    use ambition_platformer2d::world::rooms::{GateNetworkCatalog, GateNetworkState};
    use crate::common::fixed_60hz_room_options;

    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("central_hub_complex")).expect("the hub builds");
    install_autoplay(sim.app_mut(), ambition_content::autoplay::AUTOPLAY_GAME);
    let mut visits = Vec::new();
    let mut went_to_trail = false;
    let mut reached_tyrant = false;
    let mut returned = false;
    for _ in 0..12000 {
        // Autoplay owns the seat. Advance its fixed clock without a second
        // participant source that replaces its input with a neutral frame.
        sim.app_mut().update();
        let seen = sim.observation();
        if visits.last() != Some(&seen.active_room) {
            eprintln!("[autoplay trip] room={}", seen.active_room);
            visits.push(seen.active_room.clone());
        }
        went_to_trail |= seen.active_room == "primeval_trailhead";
        reached_tyrant |= seen.active_room == "primeval_tyrant_hall";
        if reached_tyrant && seen.active_room == "central_hub_complex" {
            returned = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(went_to_trail && reached_tyrant && returned, "autoplay did not finish its trip: {visits:?}");
    let world = sim.world();
    let catalog = world.resource::<GateNetworkCatalog>();
    let network = world.resource::<GateNetworkState>();
    for destination in ["GENESIS", "LEGAL"] {
        let address = catalog.spell(destination).expect("the address spells");
        assert!(network.reports.values().any(|report| report.address == address && report.outcome.is_ok()),
            "autoplay never dialed {destination}: {network:?}; visits={visits:?}");
    }
}
