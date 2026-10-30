//! Hylbert's Hotel, in the Math dimension's town
//! (`tools/ambition_ldtk_tools/specs/math_town/make_specs.py`).
//!
//! Jon (2026-10-30): "hylberts hotel is something we should build tonight".
//! The player walks by keys (`common::walk`) from Euclid Street through the
//! hotel's door into the lobby, east into the corridor of rooms, on east
//! through its far end (which comes back in at its near end: there is always
//! one more room), then west back to the lobby and out to the street.

use ambition_platformer2d::engine_core as ae;

use crate::common::walk::{Keys, Press, Walker};
use crate::the_pirate_port::{a_session_in, go, go_to, live_room};

const STREET: &str = "math_town_street";
const LOBBY: &str = "math_hotel_lobby";
const CORRIDOR: &str = "math_hotel_corridor";

#[test]
fn the_corridor_of_hylberts_hotel_always_has_one_more_room() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, STREET, Some("math_hotel_lobby_door"));
    go(&mut app, LOBBY, Some("math_hotel_lobby_street"));
    go(&mut app, CORRIDOR, Some("math_hotel_lobby_east"));
    // On east through its far end, twice: each time it comes back in at its
    // near end, and it is still the corridor.
    for lap in 0..2 {
        go_to(&mut app, ae::Vec2::new(1540.0, 360.0), "the corridor's far end");
        let mut came_back_in = false;
        {
            let mut keys = Keys::new(&mut app);
            for _ in 0..240 {
                keys.step(Press { axis: 1.0, ..Press::default() });
                let world = keys.world_mut();
                let body = world
                    .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("one primary player");
                if world.get::<ae::BodyKinematics>(body).is_some_and(|kin| kin.pos.x < 400.0) {
                    came_back_in = true;
                    break;
                }
            }
        }
        assert!(came_back_in, "lap {lap}: walking east out of the corridor did not bring the player back in at its west end");
        assert_eq!(live_room(&mut app), CORRIDOR, "lap {lap}: the far end led out of the corridor");
    }
    go(&mut app, LOBBY, Some("math_hotel_corridor_west"));
}
