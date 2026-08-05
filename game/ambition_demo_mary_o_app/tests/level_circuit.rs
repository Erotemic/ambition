//! **1-1 leads to 1-2, and 1-2 leads back — on the real schedule.**
//!
//! ⛔ **the unit tests for this were green while the return leg was broken.**
//! `exit_for_room` answered correctly for both rooms and every wiring assertion
//! passed, but the resource carrying that answer was installed ONCE at Startup
//! from `MaryOEntryRoom` — the room the session STARTS in. So after 1-1's goal
//! sent the player to 1-2, the destination was still 1-1's: finishing 1-2 asked
//! to go to 1-2, nothing moved, and nothing said why.
//!
//! ⚠ **`install_goal_pole`'s own comment warns about exactly this** — *"a goal
//! you can reach in a room whose exit belongs to another one"* — and it shipped
//! anyway, because a question answered once is a question whose two halves can
//! drift. This test is the one that rode it.
//!
//! ⭐ **it drives the schedule and reads the ROOM**, rather than asserting on
//! the resource. What was wrong was never the answer; it was who was being
//! asked. A test that checked `exit_for_room` again would have stayed green.

use bevy::prelude::*;

use ambition_demo_mary_o::flag::{FlagPhase, FlagSequence};
use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_platformer2d::actors::actor::PrimaryPlayer;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::rooms::RoomSet;

/// How many frames a transition may take to commit before we call it wedged.
///
/// ⚠ a liveness backstop, not a measurement: it exists so a broken transition
/// fails with a message instead of hanging, and the assertion below is about
/// WHICH room, never about when. Measured at ~180 frames; this is generous.
const COMMIT_CAP: usize = 600;

/// The id of the room that is AUTHORITATIVE right now.
///
/// ⚠ this used to read `RoomGeometry`'s display NAME and match a substring of
/// it, which worked only while 1-2 was a Rust room that could call itself
/// "Mary-O 1-2". Both levels are authored areas now and the composer names a
/// world after its area id (`"Ambition: mary o 1 2"`), so the substring stopped
/// meaning anything. The ROOM ID is the fact the transition changes; ask for it.
fn room_id(app: &mut App) -> Option<String> {
    let mut q = app.world_mut().query::<&RoomSet>();
    q.iter(app.world())
        .next()
        .map(|set| set.rooms[set.active].id.clone())
}

/// Drop a settled tally on the level owner — the state reaching the goal
/// produces — then run until the room changes.
fn finish_the_level(app: &mut App, from: &str) -> String {
    {
        let mut q = app.world_mut().query::<&mut FlagSequence>();
        let world = app.world_mut();
        let mut sequence = q
            .iter_mut(world)
            .next()
            .expect("the mode owner carries a flag sequence");
        sequence.phase = FlagPhase::Tallied { score: 800 };
    }
    for _ in 0..COMMIT_CAP {
        app.update();
        match room_id(app) {
            Some(id) if id != from => return id,
            _ => {}
        }
    }
    panic!("finishing `{from}` never changed the room within {COMMIT_CAP} frames");
}

/// **Walk into the pole the LEVEL authors, and leave the level.**
///
/// ⛔ **this seam had no test at all, and it is the seam that broke.** Jon,
/// 2026-08-05: *"you can keep playing after you hit the flag instead of
/// transitioning to the next level."* Coverage was two tests either side of the
/// defect: `course_playthrough` drives a REAL grab, but on the fixture course
/// whose destination is `Replay`; `finishing_each_level_carries_you_to_the_other_one`
/// below drives a REAL transition, but PLANTS `FlagPhase::Tallied`. Both halves
/// green, the join untested, and the join is where a one-shot request met a
/// transaction that assumes its producer re-asks.
///
/// ⚠ **it PLACES the body at the pole rather than walking to it.** The two tests
/// that walked 1-1 for real are `#[ignore]`d — *"route tuned to 1-1's old
/// arrangement"* — which is the honest fate of a route test: it rots every time
/// the level moves, and then it is switched off and covers nothing. What must
/// not rot is that touching the authored pole ends the level, so the pole's
/// position is read from the level and only the arrival is faked.
#[test]
fn grabbing_the_authored_pole_carries_you_out_of_the_level() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..300 {
        app.update();
    }
    let from = room_id(&mut app).expect("the session opens in a room");
    assert_eq!(from, LEVEL_1_1_ROOM_ID);

    // The pole 1-1 actually authors — not a constant this test carries.
    let pole = ambition_demo_mary_o::pole_for_room(&from);
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let world = app.world_mut();
        let mut body = q.iter_mut(world).next().expect("she is in the level");
        body.pos = ae::Vec2::new(pole.x, pole.base_y - 8.0);
    }

    let mut grabbed = false;
    let mut released_before_leaving = false;
    for _ in 0..COMMIT_CAP {
        app.update();
        // ⚠ **the ROOM is read first, and that ordering is the test.** Arriving
        // rearms the sequence to `Idle` on purpose — that is the next lap being
        // armed, not a release. Checking `Idle` before the room flagged the
        // arrival frame itself and failed against the CORRECT behaviour. Only an
        // Idle sequence while still standing in the old room is the defect.
        if let Some(id) = room_id(&mut app) {
            if id != from {
                assert!(
                    grabbed,
                    "the room changed without the flag sequence ever leaving Idle — \
                     something other than the goal moved her"
                );
                assert!(
                    !released_before_leaving,
                    "she got control back BEFORE the level ended: the flag sequence \
                     returned to Idle while still standing in `{from}`. That is Jon's \
                     'you can keep playing after you hit the flag' -- the room \
                     changing afterwards only hides it when nothing drops the request."
                );
                assert_eq!(id, LEVEL_1_2_ROOM_ID, "1-1's goal names 1-2");
                // ⛔ **AND IT HAS TO STAY THERE.** The first version of the
                // keep-asking fix remembered only THAT it had asked, not WHERE,
                // so the arrival test compared the active room against a
                // destination re-derived this tick — which, on arriving in 1-2,
                // is already 1-2's own exit back to 1-1. It never saw itself
                // arrive and asked to leave again immediately: Jon's log caught
                // three transitions in 45ms, 1-1→1-2→1-1→1-2, which is a level
                // that never settles and a screen that never stops covering.
                for _ in 0..240 {
                    app.update();
                    assert_eq!(
                        room_id(&mut app).as_deref(),
                        Some(LEVEL_1_2_ROOM_ID),
                        "arriving in 1-2 must SETTLE there; the level bounced \
                         straight back out"
                    );
                }
                return;
            }
        }
        let idle = {
            let mut q = app.world_mut().query::<&FlagSequence>();
            q.iter(app.world())
                .next()
                .is_some_and(|sequence| matches!(sequence.phase, FlagPhase::Idle))
        };
        // ⭐ **THE DISCRIMINATING OBSERVATION.** Reaching 1-2 is not enough: with
        // a host present the transition commits either way, so "the room changed"
        // passes against the defect too. What was actually wrong is that the
        // sequence rearmed to `Idle` the instant it asked to leave — and an Idle
        // sequence is `run_flag_sequence` releasing `ScriptedControl`, which is
        // the player being handed a level they already finished. Watch for that
        // window, not for the destination.
        if grabbed && idle {
            released_before_leaving = true;
        }
        grabbed |= !idle;
    }
    panic!(
        "touching 1-1's authored pole never left the level within {COMMIT_CAP} \
         frames (flag sequence started: {grabbed}). If the sequence started and \
         the room never changed, the level-end transition was dropped."
    );
}

/// **The circuit.** Finish 1-1, arrive in 1-2; finish 1-2, arrive back in 1-1.
#[test]
fn finishing_each_level_carries_you_to_the_other_one() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    for _ in 0..300 {
        app.update();
    }
    let first = room_id(&mut app).expect("the session opens in a room");
    assert_eq!(
        first, LEVEL_1_1_ROOM_ID,
        "the shipped entry is 1-1, or this test is about something else"
    );

    let second = finish_the_level(&mut app, &first);
    assert_eq!(second, LEVEL_1_2_ROOM_ID, "finishing 1-1 goes to 1-2");

    // ⭐ the leg that was broken. Without it this test would have passed over a
    // session that could reach 1-2 and never leave it.
    let third = finish_the_level(&mut app, &second);
    assert_eq!(
        third, LEVEL_1_1_ROOM_ID,
        "finishing 1-2 comes back to 1-1 — a circuit, not a dead end"
    );
}
