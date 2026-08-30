//! `BodyMode::Submerged`: travel under the stage, and the surface it is under.

use super::super::*;
use super::step_scratch;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, LocalAxes, Vec2, World};

/// One platform with a RIGHT LEDGE at x = 600 and nothing past it.
fn platform_with_a_right_ledge() -> World {
    let h = 900.0;
    World {
        name: "submerged test world".to_string(),
        size: Vec2::new(1600.0, h),
        spawn: Vec2::new(560.0, h - 48.0 - 24.0),
        blocks: vec![crate::world::Block::solid(
            "platform",
            Vec2::new(0.0, h - 48.0),
            Vec2::new(600.0, 48.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
    }
}

fn submerged_at(spawn: Vec2) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    scratch.body_mode.body_mode = crate::player_state::BodyMode::Submerged;
    scratch
}

fn stick(x: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(x, 0.0),
        ..Default::default()
    }
}

/// ⛔⛔ THE TRAPDOOR STAYS ON THE BOARDS IT OPENED. Jon, 2026-08-28: the door
/// *"can only move along a ground surface (i.e. it can't go over a ledge)."*
///
/// A submerged body is passable against every block in the world, so nothing
/// else in this pipeline can stop it: before this rule she travelled under open
/// air, off the end of the stage, and surfaced into nothing.
///
/// ⛔ THE ARMS STRADDLE THE LEDGE, and the second one is what makes the first
/// mean something. A rule that refused every submerged step would pass an
/// "off the edge" arm and delete the move; travelling INWARD along the same
/// platform must still work.
#[test]
fn a_submerged_body_travels_along_its_surface_and_stops_at_the_ledge() {
    let world = platform_with_a_right_ledge();
    let feet_on_the_platform = world.spawn;

    // Toward the ledge: she reaches it and stops short of leaving it.
    let mut outward = submerged_at(Vec2::new(560.0, feet_on_the_platform.y));
    for _ in 0..240 {
        let _ = step_scratch(&world, &mut outward, stick(1.0));
    }
    let half_width = outward.kinematics.size.x * 0.5;
    assert!(
        outward.kinematics.pos.x + half_width <= 600.0 + 4.0,
        "she travelled to x = {}, which is past the platform's right edge at 600 — \
         a body under the stage went out over open air",
        outward.kinematics.pos.x,
    );
    assert!(
        outward.kinematics.pos.x > 560.0,
        "she never moved at all (x = {}); the rule refused travel rather than \
         refusing to LEAVE, which deletes the move",
        outward.kinematics.pos.x,
    );

    // Back along the same platform: unobstructed.
    let mut inward = submerged_at(Vec2::new(560.0, feet_on_the_platform.y));
    for _ in 0..60 {
        let _ = step_scratch(&world, &mut inward, stick(-1.0));
    }
    assert!(
        inward.kinematics.pos.x < 500.0,
        "travelling INWARD along the platform stopped at x = {}, and there is \
         nothing there to stop it",
        inward.kinematics.pos.x,
    );
}

/// ⭐⭐ ONE TICK IS ONE STEP, AND THE MOVEMENT SPINE IS THE ONLY PLACE THAT
/// MOVES A BODY.
///
/// ⛔⛔ `integrate_submerged_clusters` WROTE BOTH `vel` AND `pos`, and the shared
/// sweep below it then advanced by that same velocity — so every submerged tick
/// travelled TWICE. Measured before the fix: **10.8px against an authored 5.4 at
/// 60Hz, ratio exactly 2.00**.
///
/// ⛔ AND THE SPEED IS THE SMALLER HALF. `stays_over_its_surface` validates ONE
/// prospective step; the sweep added a second, equal step nobody asked about, so
/// a body approved for a supported step landed a step further on — past the lip
/// the ledge rule exists to refuse. The neighbouring ledge test could not see
/// it: it asserts WHERE she stops, and she stops at the lip either way.
///
/// ⭐ THE SECOND ASSERTION IS THE CONTRACT, NOT THE NUMBER. `pos` moved by
/// exactly `vel * dt` says "one displacement authority" in a form that catches
/// the next mode to write a position, whatever its authored speed is.
#[test]
fn a_submerged_tick_travels_exactly_the_step_it_authored() {
    let world = platform_with_a_right_ledge();
    // Mid-platform, so the surface rule approves the step and the measurement is
    // about integration rather than about the ledge.
    let mut body = submerged_at(Vec2::new(300.0, world.spawn.y));
    let before = body.kinematics.pos;

    let _ = step_scratch(&world, &mut body, stick(1.0));

    let moved = body.kinematics.pos.x - before.x;
    let authored = TEST_TUNING.base.max_run_speed * super::super::integration::SUBMERGED_SPEED_FRAC / 60.0;
    assert!(
        (moved - authored).abs() < 0.01,
        "one submerged tick moved {moved}px against an authored {authored}px \
         ({:.2}x) — some layer other than the movement spine is advancing the \
         position as well",
        moved / authored,
    );

    // ⛔ NON-VACUITY: a mode that refused to move would satisfy nothing above
    // if `authored` were also zero, and would satisfy the ratio test trivially.
    assert!(
        authored > 1.0,
        "the fixture authors a {authored}px step, which is too small to tell a \
         double integration from rounding"
    );

    assert!(
        (moved - body.kinematics.vel.x / 60.0).abs() < 0.01,
        "the body moved {moved}px while publishing {} px/s = {}px of travel — a \
         position written outside the sweep makes those two disagree, and the \
         swept sample every consumer reads is the second one",
        body.kinematics.vel.x,
        body.kinematics.vel.x / 60.0,
    );
}

/// ⭐⭐ AN EXCLUSIVE MODE ENDS THE GROUND MANEUVERS IT TAKES THE BODY AWAY FROM.
///
/// ⛔⛔ CLEARING `dash_timer` ALONE WAS NOT ENOUGH, and the half that was missed
/// is the one nothing ticks down: `initial_dash_timer` is decayed by NORMAL
/// movement, which is exactly the branch an exclusive mode REPLACES. Measured on
/// the shipped code — armed at 0.2333s, still **exactly 0.2333s after 30
/// submerged ticks holding nothing**, when 14 frames of normal movement spend
/// it. It then resumed on the way out, so a fighter surfaced still owed a dash
/// window bought half a second earlier.
///
/// ⚠ THE ARM MEASURES DECAY, NOT PRESENCE. Asserting the timer is zero while
/// submerged would also pass on a body that never armed one, which is what the
/// first version of this probe did — `TEST_TUNING` authors no initial dash, so
/// it read 0 before AND after and proved nothing.
#[test]
fn an_exclusive_mode_ends_an_initial_dash_instead_of_freezing_it() {
    let world = super::test_world();
    let mut tuning = TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    tuning.initial_dash_speed = 0.0;
    let hold = |x: f32| InputState {
        axes: LocalAxes::new(x, 0.0),
        ..InputState::default()
    };

    let mut body = BodyClusterScratch::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Land first: an airborne body has no dash phase at all, which reads exactly
    // like the fix working.
    for _ in 0..40 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut body,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(body.ground.on_ground, "the fixture never reached the floor");
    super::update_player_with_tuning_scratch(&world, &mut body, hold(1.0), 1.0 / 60.0, tuning);
    let armed = body.axis().initial_dash_timer;
    assert!(
        armed > 0.0,
        "the dash window never opened, so nothing below observes anything"
    );

    body.body_mode.body_mode = crate::player_state::BodyMode::Submerged;
    super::update_player_with_tuning_scratch(
        &world,
        &mut body,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );

    assert_eq!(
        body.axis().initial_dash_timer,
        0.0,
        "the initial dash window survived into the trapdoor (armed at {armed}) — \
         normal movement is what spends it, and this mode replaces normal \
         movement, so it freezes and resumes on the way out"
    );
}
