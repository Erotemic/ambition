//! Leaving a ladder. Jon, 2026-11-04: "Something that also feels bad is
//! ladder movement if you have to horizontally move off of it. It seems so
//! slow and there isn't a natural way to 'jump' off. Maybe if you jump but
//! don't hold up you do jump off the ladder so you aren't constrained by it?"
//!
//! These tests climb the ladder of the sandbox's ladder lab in the whole
//! game, so the body-mode driver and the movement step run in their real
//! order. The lab is the room kept for ladders; the History dimension uses
//! platforms in their place.

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AgentObservation, AmbitionSim as _, Platformer2dSimHarness};

const ROOM: &str = "ladder_lab";
/// The middle of the lab's ladder (Climbable cells 35 and 36 of 16 px).
const LADDER_X: f32 = 576.0;

fn on_the_ladder() -> (Platformer2dSimHarness, AgentObservation) {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room builds");
    for _ in 0..30 {
        sim.step(base());
    }
    sim.teleport_player((LADDER_X, 600.0));
    let mut obs = sim.step(AgentAction { move_y: -1.0, up_pressed: true, ..base() });
    for _ in 0..8 {
        obs = sim.step(AgentAction { move_y: -1.0, ..base() });
    }
    assert_eq!(obs.body_mode, "Climbing", "Up on the ladder climbs it: {obs:?}");
    // Stop on the ladder: no input.
    for _ in 0..4 {
        obs = sim.step(base());
    }
    assert_eq!(obs.body_mode, "Climbing", "with no input she hangs on the ladder");
    (sim, obs)
}

/// Jump with the stick to the side and not up: she leaves the ladder with a
/// real jump, and the side input carries her away at the speed of a jump
/// from the ground, not the slow strafe of the ladder.
#[test]
fn a_jump_without_up_leaves_the_ladder_with_a_real_jump() {
    let (mut sim, start) = on_the_ladder();
    let mut obs = sim.step(AgentAction { move_x: 1.0, jump: true, jump_held: true, ..base() });
    let mut highest = obs.player_pos.1;
    for _ in 0..20 {
        obs = sim.step(AgentAction { move_x: 1.0, jump_held: true, ..base() });
        highest = highest.min(obs.player_pos.1);
    }
    assert_ne!(obs.body_mode, "Climbing", "the jump lets go of the ladder: {obs:?}");
    assert!(start.player_pos.1 - highest > 40.0, "she rises off the ladder, as from a jump: from y {} to {}", start.player_pos.1, highest);
    assert!(obs.player_pos.0 - start.player_pos.0 > 80.0, "the side input carries her clear of the ladder: x {} to {}", start.player_pos.0, obs.player_pos.0);
}

/// A jump with Up held stays the climb boost: she keeps the ladder.
#[test]
fn a_jump_with_up_held_keeps_the_ladder() {
    let (mut sim, _) = on_the_ladder();
    let mut obs = sim.step(AgentAction { move_y: -1.0, jump: true, jump_held: true, ..base() });
    for _ in 0..4 {
        obs = sim.step(AgentAction { move_y: -1.0, jump_held: true, ..base() });
    }
    assert_eq!(obs.body_mode, "Climbing", "Up + Jump boosts the climb and keeps the ladder: {obs:?}");
}
