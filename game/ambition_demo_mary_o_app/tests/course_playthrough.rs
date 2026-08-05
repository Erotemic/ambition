//! **The end-to-end run, on a course nobody authors.**
//!
//! ⛔ **This replaces two scripted playthroughs that went stale.** Jon,
//! 2026-08-04: *"the plays level1 test might get stale awfully quick if I modify
//! level 1. We should probably ensure there is a fixture for the test that won't
//! change as we modify the level itself."* He was right, and it had already
//! happened: both routes through 1-1 carried numbers measured against one
//! arrangement — *"the wand needs ~2.9 s to reach the pit"* — and moving the
//! enemies into the LDtk file desynchronised them. Both are `#[ignore]`d.
//!
//! ⭐ **the fixture is boring on purpose** ([`test_course`]): flat ground, one
//! ?-block, one snake, a goal. What made the old routes brittle was not the
//! level's complexity but the route needing TIMING — a jump that must clear a
//! pit has a distance, a speed and a launch frame that all have to stay true. A
//! course with no pit has no numbers to go stale, and still exercises every seam
//! the playthrough existed for.

use bevy::prelude::*;

use ambition_demo_mary_o::test_course::{course_block_aabb, TEST_COURSE_ROOM_ID};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// Boot the real host, entering the fixture course rather than 1-1.
///
/// ⭐ **no new host plumbing was needed.** The provider installs its world source
/// as a SYSTEM — its own doc says it *"may read the provider's own resources"* —
/// so the entry room is a resource read on the update that prepares the session.
/// Inserting it after the app is built and before the first `update()` is early
/// enough, which is why this is a test-side choice rather than a host variant.
fn boot_course() -> App {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        TEST_COURSE_ROOM_ID.to_string(),
    ));
    app
}

fn player_pos(app: &mut App) -> Option<ae::Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|kin| kin.pos)
}

fn settle(app: &mut App) -> ae::Vec2 {
    for _ in 0..600 {
        app.update();
        if let Some(pos) = player_pos(app) {
            if pos.y > 0.0 {
                return pos;
            }
        }
    }
    panic!("the course never produced a playable body");
}

/// **She spawns into the course, and the course is the one the fixture built.**
///
/// The first thing worth proving is that the entry-room seam works at all: a
/// resource decides which room a session starts in, and a shipped game that does
/// not insert it must still get 1-1.
#[test]
fn the_session_enters_the_fixture_course_when_asked() {
    let mut app = boot_course();
    let spawn = settle(&mut app);
    let block = course_block_aabb();
    assert!(
        spawn.x < block.min.x,
        "she starts left of the course's ?-block, with room to walk at it: \
         spawn {spawn:?} vs block {block:?}"
    );

    // The course's own geometry reached the running session — not 1-1's.
    let mut worlds = app.world_mut().query::<&ae::RoomGeometry>();
    let named: Vec<String> = worlds
        .iter(app.world())
        .map(|geo| geo.0.name.clone())
        .collect();
    assert!(
        named.iter().any(|n| n.contains("test course")),
        "the live room is the fixture course, not the authored level: {named:?}"
    );
}

/// **The default is still 1-1**, so a shipped game cannot depend on a resource
/// only a test inserts.
#[test]
fn a_host_that_says_nothing_still_enters_level_one() {
    let shipped = ambition_demo_mary_o::mary_o_session_world();
    // ⚠ the AUTHORED world's name, which the LDtk file supplies — not the string
    // the old Rust builder passed to `World::new`.
    assert!(
        !shipped.geometry.0.name.contains("test course"),
        "no entry resource means the real level, not the fixture: {}",
        shipped.geometry.0.name
    );
    let asked = ambition_demo_mary_o::provider::mary_o_session_world_entering(
        TEST_COURSE_ROOM_ID,
    );
    assert!(
        asked.geometry.0.name.contains("test course"),
        "and asking for the course gets the course: {}",
        asked.geometry.0.name
    );
}
