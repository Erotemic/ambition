//! The houses of the Math town one can go into (Jon, 2026-10-22: "Enterable
//! interiors"): the cafe, the Surveyor's Office and the Observatory, each by
//! the door its house front paints in `math_town_street`, and back out of it
//! by the room's own door to the same house.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::AabbExt as _;

use crate::common::{base, door_to, fixed_60hz_room_options, walk_through_the_door_to};

const STREET: &str = "math_town_street";
const ROOMS: [&str; 3] = ["math_cafe", "math_surveyors_office", "math_observatory"];

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
