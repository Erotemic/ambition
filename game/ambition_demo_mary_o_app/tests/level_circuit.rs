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
