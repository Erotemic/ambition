//! Routes from Euclid Street to the gate hall, the cafe, the Surveyor's Office,
//! and the Observatory.

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::AabbExt as _;

use crate::common::{base, door_to, fixed_60hz_room_options, fixed_60hz_room_sim, walk_through_the_door_to};

const STREET: &str = "math_town_street";
const HALL: &str = "math_gate_hall";
const ROOMS: [&str; 3] = ["math_cafe", "math_surveyors_office", "math_observatory"];

#[test]
fn the_gate_hall_ladder_reaches_the_street_and_back() {
    let mut sim = fixed_60hz_room_sim(HALL);
    let climbed = (0..640).any(|_| {
        let move_x = ((1552.0 - sim.observation().player_pos.0) / 24.0).clamp(-1.0, 1.0);
        sim.step(AgentAction { move_x, move_y: -1.0, ..base() }).active_room == STREET
    });
    assert!(climbed, "climb to Euclid Street: {:?}", sim.observation().player_pos);

    let cleared_the_floor = (0..90).any(|_| {
        sim.step(AgentAction { move_y: -1.0, ..base() }).player_pos.1 < 1228.0
    });
    assert!(cleared_the_floor, "climb past the street floor: {:?}", sim.observation().player_pos);
    let landed = (0..90).any(|_| {
        let observation = sim.step(AgentAction { move_x: 1.0, ..base() });
        observation.on_ground && observation.player_pos.0 > 192.0
    });
    assert!(landed, "step off the ladder in the street: {:?}", sim.observation().player_pos);
    assert_eq!(sim.observation().active_room, STREET);

    let descended = (0..180).any(|_| {
        let move_x = ((128.0 - sim.observation().player_pos.0) / 24.0).clamp(-1.0, 1.0);
        sim.step(AgentAction { move_x, move_y: 1.0, ..base() }).active_room == HALL
    });
    assert!(descended, "take the ladder down to the gate hall: {:?}", sim.observation().player_pos);
    let grounded = (0..420).any(|_| {
        let observation = sim.step(AgentAction { move_y: 1.0, ..base() });
        observation.on_ground && observation.player_pos.1 > 660.0
    });
    assert!(grounded, "reach the hall floor: {:?}", sim.observation().player_pos);
    assert_eq!(sim.observation().active_room, HALL);
}

#[test]
fn each_house_one_can_enter_is_entered_and_left_by_its_door() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(STREET)).expect("the street boots");
    for _ in 0..10 {
        sim.step(base());
    }
    for room in ROOMS {
        let door = door_to(&mut sim, room).aabb;
        assert_eq!(walk_through_the_door_to(&mut sim, room), room, "the street's door into {room}");
        for _ in 0..10 {
            sim.step(base());
        }
        assert_eq!(walk_through_the_door_to(&mut sim, STREET), STREET, "the door out of {room}");
        for _ in 0..10 {
            sim.step(base());
        }
        // Back out at the house's own door, not elsewhere in the street.
        let at = sim.observation().player_pos;
        let centre = door.center();
        assert!((at.0 - centre.x).abs() < 64.0 && (at.1 - centre.y).abs() < 96.0, "out of {room} at {at:?}, its door at {centre:?}");
    }
}
