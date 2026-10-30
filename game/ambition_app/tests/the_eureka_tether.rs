//! Eureka, the science dimension's institute town, and the space elevator
//! from it to the orbital station (`tools/ambition_ldtk_tools/specs/eureka/`).
//!
//! Jon (2026-10-29): "the science dimension should have a space elevator that
//! brings you up to the orbital station". The player walks by keys
//! (`common::walk`) from the gate court through the quad to the tether's
//! ground station, steps onto the car and does nothing: the car carries it
//! up through the roof into the climb, the climb's car carries it out of the
//! top, and it stands in the terminus on the station. Then it walks into the
//! docking bay and back, and takes the down car home.

use ambition_platformer2d::engine_core as ae;

use crate::the_pirate_port::{a_session_in, go, go_to, live_room};

const COURT: &str = "eureka_gate_court";
const QUAD: &str = "eureka_quad";
const PATENT: &str = "eureka_patent_office";
const LAB: &str = "eureka_coil_lab";
const DOME: &str = "eureka_observatory";
const STATION: &str = "eureka_elevator_station";
const CLIMB: &str = "eureka_elevator_climb";
const TERMINUS: &str = "orbital_station_tether_terminus";
const BAY: &str = "orbital_station_docking_bay";

/// The middle of the ground station's car, over its top (room px).
const STATION_CAR: ae::Vec2 = ae::Vec2::new(1024.0, 1300.0);

#[test]
fn the_tether_carries_a_rider_from_the_ground_to_the_station() {
    let mut app = a_session_in(COURT);
    go(&mut app, QUAD, Some("court_east_low"));
    for (room, door) in [(PATENT, "quad_patent_door"), (LAB, "quad_lab_door"), (DOME, "quad_dome_door")] {
        go(&mut app, room, Some(door));
        go(&mut app, QUAD, None);
    }
    go(&mut app, STATION, Some("quad_east"));

    // Onto the car, then hands off: the tether does the rest.
    go_to(&mut app, STATION_CAR, "the tether's car");
    let mut rooms = vec![live_room(&mut app)];
    for _ in 0..60 * 30 {
        app.update();
        let room = live_room(&mut app);
        if rooms.last() != Some(&room) {
            rooms.push(room);
        }
        if rooms.last().map(String::as_str) == Some(TERMINUS) {
            break;
        }
    }
    assert_eq!(rooms, [STATION, CLIMB, TERMINUS], "the rider's rooms on the way up");
    // It stands there: the terminus's floor holds it.
    for _ in 0..90 {
        app.update();
    }
    assert_eq!(live_room(&mut app), TERMINUS, "the rider fell back down the tether");

    // The station, and the way home.
    go(&mut app, BAY, Some("terminus_bay_door"));
    go(&mut app, TERMINUS, Some("bay_tether_door"));
    go(&mut app, STATION, Some("terminus_down_door"));
}
