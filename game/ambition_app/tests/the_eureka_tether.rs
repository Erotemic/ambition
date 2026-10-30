//! Eureka, the science dimension's institute town, and the space elevator
//! from it to the orbital station (`tools/ambition_ldtk_tools/specs/eureka/`).
//!
//! Jon (2026-10-29): "the science dimension should have a space elevator that
//! brings you up to the orbital station". The player walks by keys
//! (`common::walk`) from the gate court through the quad to the tether's
//! ground station, hops onto the car and does nothing: the car carries it
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

/// The ground station's floor under its car, and the car's top where it
/// waits a hop over it (room px; `make_specs.py`'s `CAR_HOP`).
const UNDER_THE_CAR: ae::Vec2 = ae::Vec2::new(1024.0, 1320.0);
const CAR_TOP: ae::Vec2 = ae::Vec2::new(1024.0, 1304.0);

/// Set the player down just over the waiting car's top.
fn onto_the_car(app: &mut bevy::prelude::App) {
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    use bevy::prelude::With;
    let half = {
        let mut query = app.world_mut().query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        query.iter(app.world()).next().expect("gameplay has a primary player").size.y * 0.5
    };
    let mut query = app.world_mut().query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query.iter_mut(world).next().expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(&mut motion_model, &mut clusters, ae::Vec2::new(CAR_TOP.x, CAR_TOP.y - half - 1.0), ae::movement::TransitVelocity::Zero);
}

#[test]
fn the_tether_carries_a_rider_from_the_ground_to_the_station() {
    let mut app = a_session_in(COURT);
    go(&mut app, QUAD, Some("court_east_low"));
    for (room, door) in [(PATENT, "quad_patent_door"), (LAB, "quad_lab_door"), (DOME, "quad_dome_door")] {
        go(&mut app, room, Some(door));
        go(&mut app, QUAD, None);
    }
    go(&mut app, STATION, Some("quad_east"));

    // Under the car, then onto it, then hands off: the tether does the rest.
    // The walker's map is the room's still geometry, so it walks to the
    // floor under the car and the hop is a placement onto the car's top (as
    // `mockingbird_fight`'s shark ride is).
    go_to(&mut app, UNDER_THE_CAR, "the floor under the tether's car");
    onto_the_car(&mut app);
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

/// Out of town (Jon, 2026-10-30: "out of town zones with enemies"): the
/// Test Range, west of the gate court, by its side exit and back.
#[test]
fn the_test_range_is_out_of_the_gate_court_and_back() {
    let mut app = a_session_in(COURT);
    go(&mut app, "eureka_test_range", Some("court_west"));
    go(&mut app, COURT, Some("range_east"));
}

/// The coil lab's prototype: the floor lever turns the lab's gravity over and
/// the player falls to the ceiling; its twin there turns it back.
#[test]
fn the_coil_labs_prototype_turns_its_gravity_over_and_back() {
    use bevy::prelude::KeyCode;
    let mut app = a_session_in(LAB);
    let y = |app: &mut bevy::prelude::App| {
        let body = crate::neighbor_prefetch_prepares_rooms::alice(app);
        app.world().get::<ae::BodyKinematics>(body).expect("the player").pos.y
    };
    go_to(&mut app, ae::Vec2::new(1204.0, 490.0), "the prototype");
    crate::the_note_travels_from_alice_to_bob::tap(&mut app, KeyCode::KeyF);
    for _ in 0..180 {
        app.update();
    }
    assert!(y(&mut app) < 150.0, "pressing the prototype did not drop the player on the ceiling (y {})", y(&mut app));
    crate::the_note_travels_from_alice_to_bob::tap(&mut app, KeyCode::KeyF);
    for _ in 0..180 {
        app.update();
    }
    assert!(y(&mut app) > 400.0, "the twin did not bring the player down (y {})", y(&mut app));
    assert_eq!(live_room(&mut app), LAB);
}

/// The Test Range's officer talks (`eureka.yarn` `eureka_pryce`).
#[test]
fn the_range_officer_signs_it_off_as_probably_fine() {
    let mut app = a_session_in("eureka_test_range");
    crate::the_pirate_port::talk(&mut app, "npc_eureka_pryce", "What is tested here?");
}

/// The souvenir seller at the top of the ribbon talks (`orbital.yarn`
/// `station_dobbs`).
#[test]
fn dobbs_sells_genuine_orbit_air_at_the_terminus() {
    let mut app = a_session_in(TERMINUS);
    crate::the_pirate_port::talk(&mut app, "npc_station_dobbs", "What is in the bottles?");
}
