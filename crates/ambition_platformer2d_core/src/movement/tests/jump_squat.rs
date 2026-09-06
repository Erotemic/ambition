//! Jump-squat: the grounded startup a jump owes before the body leaves the
//! floor.
//!
//! this is what makes a jump COMMITTAL — the window in which a platform
//! fighter can be struck out of its own takeoff and an opponent can react to the
//! crouch. It is authored per body rather than globally because a body without a
//! squat is not a badly-tuned fighter, it is a different game: Mary-O's SMB1
//! convergence requires the leap on the press tick, and the default `0.0`
//! preserves that byte-for-byte.

use super::super::*;
use super::{test_world, TEST_TUNING};
use crate::body_clusters::BodyClusterScratch;
use crate::test_support::update_player_with_tuning_scratch;
use crate::{AbilitySet, Vec2};

const DT: f32 = 1.0 / 60.0;

/// A body at REST on the shared test world's floor. the world's spawn hangs a
/// little above the floor, so the body is dropped onto it rather than having
/// `on_ground` set by hand — a hand-set flag survives exactly one tick here and
/// then the integrator corrects it, which silently un-grounds the fixture.
fn grounded_body() -> (crate::World, BodyClusterScratch) {
    let world = test_world();
    let mut scratch =
        BodyClusterScratch::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    for _ in 0..120 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            DT,
            TEST_TUNING,
        );
        if scratch.ground.on_ground {
            break;
        }
    }
    assert!(
        scratch.ground.on_ground,
        "the fixture body must be standing"
    );
    (world, scratch)
}

fn jump_input(pressed: bool, held: bool, released: bool) -> InputState {
    InputState {
        movement: ActionEdges::EMPTY.with(
            MovementAction::Jump,
            Edge {
                pressed,
                held,
                released,
            },
        ),
        ..Default::default()
    }
}

/// The second half is the poison that matters, because EVERY body in the game today is in it —
/// a squat that leaked into the default would change how Mary-O jumps, and nothing else here
/// would notice.
#[test]
fn a_squat_delays_the_leap_and_no_squat_leaves_the_press_tick_untouched() {
    let squat_s = 3.0 * DT;

    // --- authored squat: grounded through the crouch, airborne after it ---
    let (world, mut body) = grounded_body();
    let mut tuning = TEST_TUNING;
    tuning.jump_squat_time = squat_s;
    let start_y = body.kinematics.pos.y;

    update_player_with_tuning_scratch(&world, &mut body, jump_input(true, true, false), DT, tuning);
    assert!(
        body.ground.on_ground,
        "the press tick is the FIRST crouch tick, not the takeoff"
    );
    assert_eq!(
        body.kinematics.pos.y, start_y,
        "a crouching body has not left the floor"
    );

    // Two more crouch ticks, the button merely held — the press is already spent.
    update_player_with_tuning_scratch(
        &world,
        &mut body,
        jump_input(false, true, false),
        DT,
        tuning,
    );
    assert!(body.ground.on_ground, "mid-squat is still grounded");
    update_player_with_tuning_scratch(
        &world,
        &mut body,
        jump_input(false, true, false),
        DT,
        tuning,
    );
    assert!(
        !body.ground.on_ground,
        "the squat expires and the SAME ground leap fires"
    );
    let squat_launch = body.kinematics.vel.y;

    // --- the default: no squat, so the leap is on the press tick ---
    let (world, mut instant) = grounded_body();
    assert_eq!(
        TEST_TUNING.jump_squat_time, 0.0,
        "⛔ the default must stay instant — see the module docs"
    );
    update_player_with_tuning_scratch(
        &world,
        &mut instant,
        jump_input(true, true, false),
        DT,
        TEST_TUNING,
    );
    assert!(
        !instant.ground.on_ground,
        "⛔ POISON: an unauthored body must leave the ground on the press tick"
    );

    // And the delay is the ONLY difference: the leap itself is one rule.
    assert!(
        (squat_launch - instant.kinematics.vel.y).abs() < 1e-3,
        "the squat must not change the launch, only when it happens: \
         squat={squat_launch} instant={}",
        instant.kinematics.vel.y
    );
}

/// a squat is the thing you can be knocked OUT of. Losing the floor mid-crouch
/// voids the leap rather than owing it in the air — without this the startup
/// buys the attacker nothing, which is the entire reason it exists.
#[test]
fn losing_the_floor_mid_squat_voids_the_leap() {
    let (world, mut body) = grounded_body();
    let mut tuning = TEST_TUNING;
    tuning.jump_squat_time = 3.0 * DT;

    update_player_with_tuning_scratch(&world, &mut body, jump_input(true, true, false), DT, tuning);
    assert!(body.ground.on_ground, "crouching");

    // Struck off the floor mid-crouch, launched sideways with no ascent.
    body.ground.on_ground = false;
    body.kinematics.vel = Vec2::new(200.0, 0.0);
    for _ in 0..3 {
        update_player_with_tuning_scratch(
            &world,
            &mut body,
            jump_input(false, true, false),
            DT,
            tuning,
        );
    }
    assert!(
        body.kinematics.vel.y <= 0.0,
        "⛔ the voided squat must not pay out a leap in mid-air, got vy={}",
        body.kinematics.vel.y
    );
}

/// The release edge that shortens a hop lands DURING the crouch, where there is no ascent to
/// cut.
#[test]
fn a_button_released_during_the_squat_still_shortens_the_hop() {
    let squat = 3.0 * DT;

    let mut tap = TEST_TUNING;
    tap.jump_squat_time = squat;
    let (world, mut tapped) = grounded_body();
    update_player_with_tuning_scratch(&world, &mut tapped, jump_input(true, true, false), DT, tap);
    // Button comes up on the very next tick — entirely inside the crouch.
    update_player_with_tuning_scratch(&world, &mut tapped, jump_input(false, false, true), DT, tap);
    update_player_with_tuning_scratch(
        &world,
        &mut tapped,
        jump_input(false, false, false),
        DT,
        tap,
    );

    let (world, mut held) = grounded_body();
    for i in 0..3 {
        update_player_with_tuning_scratch(
            &world,
            &mut held,
            jump_input(i == 0, true, false),
            DT,
            tap,
        );
    }

    assert!(
        !tapped.ground.on_ground && !held.ground.on_ground,
        "both took off"
    );
    // Up is -y in this fixture's frame, so a shorter hop is the LESS negative one.
    assert!(
        tapped.kinematics.vel.y > held.kinematics.vel.y + 1.0,
        "⛔ the release swallowed by the crouch must still shorten the hop: \
         tapped={} held={}",
        tapped.kinematics.vel.y,
        held.kinematics.vel.y
    );
}
