//! The dash's styles (`AbilitySet::surge_dash`, `AbilitySet::phase_dash`),
//! each measured against the plain dash on the same floor.

use super::*;
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;

const DT: f32 = 1.0 / 60.0;

fn press(action: crate::MovementAction) -> crate::ActionEdges<crate::MovementAction> {
    crate::ActionEdges::EMPTY.with(action, crate::Edge { pressed: true, held: true, released: false })
}

/// Holding right, with `action` pressed on this tick.
fn right_and(action: crate::MovementAction) -> InputState {
    InputState { movement: press(action), ..InputState::with_axes(1.0, 0.0) }
}

/// A body standing on the test floor, with `style` switched on, whose burst
/// press dashes on the ground. The plain dash yields the ground burst to a
/// dodge, so its kit has none; a style takes the burst with the dodge in the
/// kit, which is what the styled arms measure.
fn standing(world: &crate::World, style: fn(&mut AbilitySet)) -> BodyClusterScratch {
    let mut kit = AbilitySet::sandbox_all();
    style(&mut kit);
    if crate::movement::DashStyle::of(&kit) == crate::movement::DashStyle::Classic {
        kit.dodge = false;
    }
    let mut scratch = BodyClusterScratch::new_with_abilities(world.spawn, kit);
    for _ in 0..40 {
        update_player_with_tuning_scratch(world, &mut scratch, InputState::default(), DT, TEST_TUNING);
    }
    assert!(scratch.ground.on_ground, "the fixture never landed");
    scratch
}

/// The body's speed along the floor on each tick: a burst pressed while
/// holding right, then right held.
fn run_out(world: &crate::World, style: fn(&mut AbilitySet), ticks: usize) -> Vec<f32> {
    let mut scratch = standing(world, style);
    let mut speeds = Vec::new();
    update_player_with_tuning_scratch(world, &mut scratch, right_and(crate::MovementAction::Burst), DT, TEST_TUNING);
    speeds.push(scratch.kinematics.vel.x);
    for _ in 1..ticks {
        update_player_with_tuning_scratch(world, &mut scratch, InputState::with_axes(1.0, 0.0), DT, TEST_TUNING);
        speeds.push(scratch.kinematics.vel.x);
    }
    speeds
}

fn classic(_: &mut AbilitySet) {}
fn surge(kit: &mut AbilitySet) {
    kit.surge_dash = true;
}
fn phase(kit: &mut AbilitySet) {
    kit.phase_dash = true;
}

/// Dash II leaves faster than the plain dash and comes out of it still past
/// the run, bleeding that speed off while the stick holds it; the plain dash
/// is back at the run by then.
#[test]
fn a_surge_starts_faster_and_keeps_a_speed_past_the_run() {
    let world = test_world();
    let run = TEST_TUNING.params().locomotion.max_run_speed;
    let plain = run_out(&world, classic, 18);
    let surging = run_out(&world, surge, 18);
    assert!(plain[0] > run, "premise: the plain dash did not dash ({:.0})", plain[0]);
    assert!(surging[0] > plain[0] * 1.2, "the surge left at {:.0}, the plain dash at {:.0}", surging[0], plain[0]);
    // A quarter of a second after the press both bursts are over.
    let (plain_after, surge_after) = (plain[15], surging[15]);
    assert!(plain_after <= run + 1.0, "premise: the plain dash still carries {plain_after:.0} past a run of {run:.0}");
    assert!(surge_after > run + 40.0, "the surge came out at {surge_after:.0}, no faster than the run ({run:.0})");
    // And it eases down rather than snapping: never faster than the tick before.
    assert!(surging.windows(2).all(|w| w[1] <= w[0] + 1.0e-3), "the surge sped up: {surging:?}");
}

/// A jump out of Dash II ends the burst there (gravity comes back) and takes
/// the burst's speed into the air.
#[test]
fn a_jump_out_of_a_surge_keeps_its_speed() {
    let world = test_world();
    let run = TEST_TUNING.params().locomotion.max_run_speed;
    let mut scratch = standing(&world, surge);
    update_player_with_tuning_scratch(&world, &mut scratch, right_and(crate::MovementAction::Burst), DT, TEST_TUNING);
    update_player_with_tuning_scratch(&world, &mut scratch, InputState::with_axes(1.0, 0.0), DT, TEST_TUNING);
    assert!(scratch.axis().dash_timer > 0.0, "premise: the burst was already over");
    update_player_with_tuning_scratch(&world, &mut scratch, right_and(crate::MovementAction::Jump), DT, TEST_TUNING);
    assert_eq!(scratch.axis().dash_timer, 0.0, "the jump did not end the burst");
    assert!(!scratch.ground.on_ground, "premise: the jump did not leave the floor");
    for _ in 0..10 {
        update_player_with_tuning_scratch(&world, &mut scratch, InputState::with_axes(1.0, 0.0), DT, TEST_TUNING);
    }
    assert!(
        scratch.kinematics.vel.x > run + 150.0,
        "the jump kept {:.0} of the surge, little more than the run ({run:.0})",
        scratch.kinematics.vel.x
    );
}

/// Dash III is short and much faster, nothing can hit the body while it
/// lasts, and it stops at the run rather than sliding out.
#[test]
fn a_phase_dash_is_unhittable_and_stops_at_the_run() {
    let world = test_world();
    let run = TEST_TUNING.params().locomotion.max_run_speed;
    let mut scratch = standing(&world, phase);
    update_player_with_tuning_scratch(&world, &mut scratch, right_and(crate::MovementAction::Burst), DT, TEST_TUNING);
    let dash = TEST_TUNING.params().abilities.dash_speed;
    assert!(scratch.kinematics.vel.x > dash * 1.5, "the phase left at {:.0}", scratch.kinematics.vel.x);
    let mut ticks = 1;
    while scratch.axis().dash_timer > 0.0 {
        assert!(scratch.axis().evade_invuln_timer > 0.0, "the body could be hit on tick {ticks} of the phase");
        update_player_with_tuning_scratch(&world, &mut scratch, InputState::with_axes(1.0, 0.0), DT, TEST_TUNING);
        ticks += 1;
    }
    assert!(ticks as f32 * DT < TEST_TUNING.params().abilities.dash_time, "the phase ran {ticks} ticks, as long as the plain dash");
    assert!(scratch.kinematics.vel.x <= run + 1.0, "the phase ended at {:.0}, past the run", scratch.kinematics.vel.x);
}
