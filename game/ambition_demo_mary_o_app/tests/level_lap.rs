//! Runs the authored 1-1 -> 1-2 -> 1-3 -> 1-1 circuit in one session.
//!
//! Each leg reaches that room's authored pole and lets the real end-of-level
//! transition run. The expected chain is written explicitly so the transition
//! implementation cannot define its own oracle. Keeping one session also covers
//! destination state that must change as rooms advance.

use bevy::prelude::*;

use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;

const LEVEL_1_3_ROOM_ID: &str = "mary_o_1_3";

/// Frames a leg may take before we call the transition wedged. A liveness
/// backstop, not a measurement: the assertions are about WHICH room.
const COMMIT_CAP: usize = 900;

fn room_id(app: &mut App) -> Option<String> {
    let mut q = app.world_mut().query::<&RoomSet>();
    q.iter(app.world())
        .next()
        .map(|set| set.active_spec().id.clone())
}

/// Where the controlled body is, if she exists at all.
fn body_pos(app: &mut App) -> Option<ae::Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let world = app.world_mut();
    q.iter(world).next().map(|kin| kin.pos)
}

/// Finish `from` by touching its OWN authored pole, and report where she
/// landed.
///
/// The pole comes off the level rather than from a constant this test carries:
/// a destination that lives in the level file cannot be one level behind it.
fn finish_at_the_pole(app: &mut App, from: &str) -> String {
    let pole = ambition_demo_mary_o::pole_for_room(from);
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let world = app.world_mut();
        let mut body = q
            .iter_mut(world)
            .next()
            .unwrap_or_else(|| panic!("no controlled body in '{from}' to carry to its pole"));
        body.pos = ae::Vec2::new(pole.x, pole.base_y - 8.0);
        body.vel = ae::Vec2::ZERO;
    }
    for _ in 0..COMMIT_CAP {
        app.update();
        if let Some(id) = room_id(app) {
            if id != from {
                // Let the arrival settle so the body has been placed in the new
                // room before anybody asks where she is.
                for _ in 0..30 {
                    app.update();
                }
                return id;
            }
        }
    }
    panic!(
        "touched the authored pole of '{from}' and the room never changed within \
         {COMMIT_CAP} frames. ⚠ a room that never changes and a room that changes \
         to ITSELF are different failures: `LevelDestination::Replay` restarts \
         this level, which reads to a player as 'finishing sends me back to the \
         start' rather than as a wedge."
    );
}

/// One leg: finish `from`, land in `expected`, and be standing in it.
fn leg(app: &mut App, from: &str, expected: &str) {
    assert_eq!(
        room_id(app).as_deref(),
        Some(from),
        "the lap expected to be standing in '{from}' before finishing it",
    );
    let landed = finish_at_the_pole(app, from);
    assert_eq!(
        landed, expected,
        "finishing '{from}' put her in '{landed}'. ⛔ if that is '{from}' itself, \
         `exit_for_room` answered `Replay` -- the level's authored `next_room` \
         did not resolve, which is exactly 'finishing 1-1 sends you back to 1-1'.",
    );
    // and she is actually IN it. A transition that swaps the room set
    // while leaving no controlled body is still a room change, and it is not
    // arriving anywhere.
    let pos = body_pos(app).unwrap_or_else(|| {
        panic!("she arrived in '{expected}' and there is no controlled body in it")
    });
    assert!(
        pos.is_finite(),
        "she arrived in '{expected}' at a non-finite position {pos:?}",
    );
}

/// The full lap, each leg naming its own destination.
#[test]
fn finishing_each_level_lands_in_the_next_one_and_the_lap_closes() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..300 {
        app.update();
    }
    assert_eq!(
        room_id(&mut app).as_deref(),
        Some(LEVEL_1_1_ROOM_ID),
        "the session did not open in 1-1",
    );

    leg(&mut app, LEVEL_1_1_ROOM_ID, LEVEL_1_2_ROOM_ID);
    leg(&mut app, LEVEL_1_2_ROOM_ID, LEVEL_1_3_ROOM_ID);
    // the return leg. `mary_o_1_3` authors `next_room = mary_o_1_1`, so the
    // lap closes rather than dead-ending -- and a destination installed once at
    // Startup would have sent this one to 1-2.
    leg(&mut app, LEVEL_1_3_ROOM_ID, LEVEL_1_1_ROOM_ID);
}

/// ⭐⛤ **THE ROOM YOU COME BACK TO IS NOT THE ROOM YOU LEFT, AND UNTIL
/// 2026-09-20 NOTHING IN THIS WORLD COULD SAY SO.**
///
/// The lap above closes: 1-1 → 1-2 → 1-3 → 1-1. `RoomSet` therefore reports
/// the SAME active index at both ends, because an index selects a
/// DEFINITION — and `RoomConstructionPlanId` reports the same thing for the
/// same reason, being a content hash whose own doc excludes commit-time
/// facts. `LiveRoomInstance` is the identity neither of them has: it is minted
/// by the one road that seats a session in a published room, so the two 1-1s
/// are two live rooms.
///
/// ⚠ THE FIRST ASSERTION IS THE ANTI-VACUITY ONE. If the lap did not return to
/// the room it started in, "the index repeated and the instance did not" would
/// be true of any two different rooms and would prove nothing.
///
/// ⭐ MEASURED, so the shape is on the record rather than only the relation:
/// `opened=#0 midway=#1 closed=#3 index=0`. Three legs, three publications,
/// and the activation room is `#0` because a session's first live room is not
/// published by the road that seats a crossing — it IS the session.
///
/// This is OW1's precondition in
/// `docs/planning/engine/open-world-runtime-and-residency.md`, exercised on the
/// shipped crossing rather than on a hand-built set.
#[test]
fn a_lap_comes_back_to_the_same_room_definition_and_a_different_live_room() {
    use ambition_platformer2d::world::rooms::LiveRoomInstance;

    fn live_room(app: &mut App) -> LiveRoomInstance {
        let mut q = app.world_mut().query::<&LiveRoomInstance>();
        *q.iter(app.world())
            .next()
            .expect("the live session root carries a live-room instance")
    }

    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..300 {
        app.update();
    }

    let opened_in = room_id(&mut app).expect("the session opened somewhere");
    let opening_index = {
        let mut q = app.world_mut().query::<&RoomSet>();
        q.iter(app.world())
            .next()
            .expect("the session has a RoomSet")
            .active()
    };
    let opening_instance = live_room(&mut app);

    leg(&mut app, LEVEL_1_1_ROOM_ID, LEVEL_1_2_ROOM_ID);
    let midway_instance = live_room(&mut app);
    leg(&mut app, LEVEL_1_2_ROOM_ID, LEVEL_1_3_ROOM_ID);
    leg(&mut app, LEVEL_1_3_ROOM_ID, LEVEL_1_1_ROOM_ID);

    let closing_index = {
        let mut q = app.world_mut().query::<&RoomSet>();
        q.iter(app.world())
            .next()
            .expect("the session has a RoomSet")
            .active()
    };
    let closing_instance = live_room(&mut app);

    assert_eq!(
        room_id(&mut app).as_deref(),
        Some(opened_in.as_str()),
        "the lap did not close, so nothing below is about coming BACK to a room"
    );
    assert_eq!(
        closing_index, opening_index,
        "one room definition must keep one index across a lap: the selection is \
         into a list of definitions and the list did not change"
    );
    assert_ne!(
        closing_instance, opening_instance,
        "the 1-1 she came back to carries the identity of the 1-1 she left, so \
         nothing in this world can tell one live room from the other — which is \
         the conflation OW1 exists to unpick"
    );
    assert!(
        opening_instance.ordinal() < midway_instance.ordinal()
            && midway_instance.ordinal() < closing_instance.ordinal(),
        "the instance must advance on every publication, not merely differ at \
         the ends: opened {opening_instance}, midway {midway_instance}, closed \
         {closing_instance}"
    );
}
