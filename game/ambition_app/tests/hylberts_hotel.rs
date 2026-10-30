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
use crate::the_pirate_port::{a_session_in, go, go_to, live_room, talk};

const STREET: &str = "math_town_street";
const LOBBY: &str = "math_hotel_lobby";
const CORRIDOR: &str = "math_hotel_corridor";
const LIFT: &str = "math_hotel_lift";

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

/// The lift only goes up: its car carries the rider out of the top of the
/// shaft, the top comes back in at the bottom, and the car is waiting there.
#[test]
fn hylberts_lift_only_goes_up() {
    let mut app = a_session_in(LOBBY);
    go(&mut app, LIFT, Some("math_hotel_lobby_lift"));
    go(&mut app, LOBBY, Some("math_hotel_lift_door"));
    go(&mut app, LIFT, Some("math_hotel_lobby_lift"));
    // Onto the car (its top 68 over the bottom of the 1600-tall shaft).
    {
        use ambition_platformer2d::platformer::markers::PrimaryPlayer;
        use bevy::prelude::With;
        let half = {
            let mut query = app.world_mut().query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
            query.iter(app.world()).next().expect("a primary player").size.y * 0.5
        };
        let mut query = app.world_mut().query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
        let world = app.world_mut();
        let (mut cluster_item, mut motion_model) = query.iter_mut(world).next().expect("a primary player");
        let mut clusters = cluster_item.as_clusters_mut();
        ae::movement::transit_body(&mut motion_model, &mut clusters, ae::Vec2::new(384.0, 1600.0 - 68.0 - half - 1.0), ae::movement::TransitVelocity::Zero);
    }
    // Rising, then low again (it came round), twice.
    let mut laps = 0;
    let mut was_high = false;
    for _ in 0..60 * 40 {
        app.update();
        assert_eq!(live_room(&mut app), LIFT, "the lift let the rider out");
        let world = app.world_mut();
        let y = world
            .query_filtered::<&ae::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("one primary player")
            .pos
            .y;
        if y < 400.0 {
            was_high = true;
        } else if was_high && y > 1300.0 {
            laps += 1;
            was_high = false;
            if laps == 2 {
                break;
            }
        }
    }
    assert_eq!(laps, 2, "the rider did not come round the shaft twice");
}

/// The corridor's guests talk: the barber, Furrier and Python Goras, each at
/// a door (`math.yarn`'s `hotel_*`).
#[test]
fn the_guests_of_hylberts_hotel_talk() {
    let mut app = a_session_in(CORRIDOR);
    talk(&mut app, "npc_paradox_barber", "Do you shave the guests?");
    talk(&mut app, "npc_joseph_furrier", "Which room are you in?");
    talk(&mut app, "npc_python_goras", "What's wrong with beans?");
}
