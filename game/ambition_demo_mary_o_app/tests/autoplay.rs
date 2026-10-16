//! `AMBITION_AUTOPLAY=mary_o`: Mary-O plays herself to the pole of a level,
//! in the shipped demo. Her policy takes the cosmic form and names the goal;
//! the engine's autopilot takes her there by the room's surface graph for her
//! own body, through the player's seat.

use bevy::prelude::*;

use ambition_demo_mary_o::flag::FlagSequence;
use ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID;
use ambition_platformer2d::autopilot::{Autopilot, AutopilotStatus};

/// The most frames the test course may take (30 s).
const FRAMES: usize = 1800;

#[test]
fn mary_o_plays_the_test_course_to_its_pole_by_herself() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(TEST_COURSE_ROOM_ID.to_string()));
    ambition_platformer2d::scripted_input::install_autoplay(&mut app, ambition_demo_mary_o::autoplay::AUTOPLAY_GAME);
    let mut went = false;
    for frame in 0..FRAMES {
        app.update();
        let world = app.world_mut();
        went |= world.resource::<Autopilot>().status() == AutopilotStatus::Going;
        let grabbed = world.query::<&FlagSequence>().iter(world).any(FlagSequence::active);
        if grabbed {
            // Not by chance: the autopilot had a graph and followed it.
            assert!(went, "she reached the pole with the autopilot never going");
            assert!(world.resource::<Autopilot>().graph().is_some(), "she reached the pole with no graph");
            eprintln!("the pole on frame {frame}");
            return;
        }
    }
    let world = app.world_mut();
    let at = world
        .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .iter(world)
        .next()
        .map(|kin| kin.pos);
    panic!(
        "no pole after {FRAMES} frames: the autopilot is {:?}, she is at {at:?}",
        world.resource::<Autopilot>().status()
    );
}
