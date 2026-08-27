//! Verify the authored Mary-O level circuit through the real schedule: 1-1
//! leads to 1-2 and 1-2 leads back. The tests assert the authoritative active
//! room rather than only checking the exit lookup that feeds the transition.

use bevy::prelude::*;

use ambition_demo_mary_o::flag::{FlagPhase, FlagSequence};
use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;

/// Liveness cap only; transition timing is not part of the assertion.
const COMMIT_CAP: usize = 600;

/// Authoritative active room id.
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

/// Place the body at the level's authored pole and verify that contact exits
/// the level. The pole position comes from content so geometry changes do not
/// invalidate the mechanism test.
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
        // the ROOM is read first, and that ordering is the test. Arriving rearms the sequence
        // to `Idle` on purpose — that is the next lap being armed, not a release. Checking `Idle`
        // before the room flagged the arrival frame itself and failed against the CORRECT
        // behaviour.
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
                // The transition must remain settled after arrival; destination tracking
                // is tied to the requested room, not re-derived from the new room.
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

/// The circuit: finishing every authored level eventually comes home.
///
/// this asserted the LENGTH of the chain, and a third level broke it.
/// It hard-coded *"finishing 1-2 returns to 1-1"* — true only while 1-2 was the
/// last level. Authoring `mary_o_1_3` in LDtk cost no Rust to describe and still
/// reddened this file, because the test had pinned the shape of the world
/// instead of the property being claimed.
///
/// the property is "a circuit, not a dead end", and it does not mention a
/// count. So walk until the entry comes back around, and let the roster say
/// how long that takes: a fourth level authored tomorrow extends the walk rather
/// than failing it.
///
/// and the walk is still bounded. A dead end or a short loop that never
/// reaches the entry must FAIL rather than hang, so the cap is one hop per
/// authored area plus one — enough for the real circuit, never enough to hide a
/// broken one.
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

    let authored = ambition_demo_mary_o::authored_area_ids();
    assert!(
        authored.len() >= 2,
        "a circuit needs at least two levels to be a claim about anything; the \
         world authors {authored:?}"
    );

    // the leg that was broken once: without a second hop this test would have
    // passed over a session that could reach 1-2 and never leave it.
    let second = finish_the_level(&mut app, &first);
    assert_eq!(second, LEVEL_1_2_ROOM_ID, "finishing 1-1 goes to 1-2");

    let mut visited = vec![first.clone(), second.clone()];
    let mut here = second;
    for _ in 0..=authored.len() {
        let next = finish_the_level(&mut app, &here);
        if next == first {
            assert_eq!(
                visited.len(),
                authored.len(),
                "the circuit closed after visiting {visited:?}, but the world \
                 authors {authored:?} — a level nobody can reach by playing is \
                 as good as unauthored"
            );
            return;
        }
        assert!(
            !visited.contains(&next),
            "finishing '{here}' led to '{next}', which is already on this walk \
             ({visited:?}) — that is a SHORT LOOP that never returns to the \
             entry, not a circuit"
        );
        visited.push(next.clone());
        here = next;
    }
    panic!(
        "walked {visited:?} without ever returning to '{first}'; the authored \
         areas are {authored:?}. Some level's exit is a dead end."
    );
}
