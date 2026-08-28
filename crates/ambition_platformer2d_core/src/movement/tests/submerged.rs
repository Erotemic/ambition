//! `BodyMode::Submerged`: under the stage, and still driving.
//!
//! ⛔⛔ THE BUG THESE EXIST FOR. Jon, 2026-08-27: *"As the actor, I cannot move
//! when I go underground."* The mode was reading `InputState::local_axis`, which
//! is the stick AFTER a live move's authored motion lock has damped it — and
//! `hitless_special`, which is how this repository writes a commitment, roots
//! the body with `motion_scale: 0.0` for the move's whole timeline. So the
//! trapdoor damped to zero the one thing the mode exists to provide, and the
//! move that most wanted the steering was the move that could not have it.
//!
//! ⭐ A MODE THAT CANNOT BE STEERED IS A PAUSE, and a pause is not what was
//! asked for: *"I do want the player to be able to control where they move."*

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::player_state::BodyMode;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::reference_frame::LocalAxes;
use crate::{AbilitySet, Vec2};

fn submerged_at(spawn: Vec2) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    scratch.body_mode.body_mode = BodyMode::Submerged;
    scratch
}

/// The stick as a ROOTED move leaves it: damped to nothing, with what the player
/// is actually holding preserved beside it. This is the shape the trapdoor
/// produces, and the shape the first version of the mode could not move under.
fn rooted_stick(x: f32) -> InputState {
    let mut input = InputState::default();
    input.axes = LocalAxes::ZERO;
    input.undamped_axes = Some(LocalAxes { x, y: 0.0 });
    input
}

#[test]
fn a_submerged_body_moves_under_a_rooted_move() {
    let world = test_world();
    let start = Vec2::new(400.0, 600.0);
    let mut scratch = submerged_at(start);
    for _ in 0..10 {
        let _ = step_scratch(&world, &mut scratch, rooted_stick(1.0));
    }
    assert!(
        scratch.kinematics.pos.x > start.x + 10.0,
        "a submerged body must travel on the stick the player is holding, and it \
         moved from {} to {}",
        start.x,
        scratch.kinematics.pos.x
    );
}

/// ⛔ THE MIRRORED ARM, because a sign error moves her the wrong way and a
/// one-direction test agrees with it.
#[test]
fn a_submerged_body_travels_the_way_the_stick_points() {
    let world = test_world();
    let start = Vec2::new(400.0, 600.0);
    let mut left = submerged_at(start);
    let mut right = submerged_at(start);
    for _ in 0..10 {
        let _ = step_scratch(&world, &mut left, rooted_stick(-1.0));
        let _ = step_scratch(&world, &mut right, rooted_stick(1.0));
    }
    assert!(left.kinematics.pos.x < start.x, "left stick goes left");
    assert!(right.kinematics.pos.x > start.x, "right stick goes right");
}

/// ⛔ AND A RELEASED STICK STOPS HER, so the travel is steering and not drift.
#[test]
fn a_submerged_body_with_no_stick_stays_put() {
    let world = test_world();
    let start = Vec2::new(400.0, 600.0);
    let mut scratch = submerged_at(start);
    for _ in 0..10 {
        let _ = step_scratch(&world, &mut scratch, rooted_stick(0.0));
    }
    assert!(
        (scratch.kinematics.pos - start).length() < 1e-3,
        "she drifted to {:?} with the stick released",
        scratch.kinematics.pos
    );
}

/// ⛔⛔ GRAVITY DOES NOT REACH HER. Falling under the stage would drop her out of
/// the bottom of the world, and it is the half a horizontal-travel test cannot
/// see.
#[test]
fn a_submerged_body_does_not_fall() {
    let world = test_world();
    let start = Vec2::new(400.0, 600.0);
    let mut scratch = submerged_at(start);
    for _ in 0..30 {
        let _ = step_scratch(&world, &mut scratch, rooted_stick(0.0));
    }
    assert!(
        (scratch.kinematics.pos.y - start.y).abs() < 1e-3,
        "she fell from {} to {}",
        start.y,
        scratch.kinematics.pos.y
    );
}

/// ⛔ AND GEOMETRY DOES NOT STOP HER, which is what "under the stage" means: the
/// floor she went through is above her, and a body that could be blocked by it
/// would be wedged inside it with nothing to push her out.
#[test]
fn a_submerged_body_passes_through_solid_ground() {
    let mut world = test_world();
    // A wall squarely across her path.
    world.blocks.push(crate::world::Block::solid(
        "wall",
        Vec2::new(440.0, 560.0),
        Vec2::new(40.0, 120.0),
    ));
    let start = Vec2::new(400.0, 600.0);
    let mut scratch = submerged_at(start);
    for _ in 0..30 {
        let _ = step_scratch(&world, &mut scratch, rooted_stick(1.0));
    }
    assert!(
        scratch.kinematics.pos.x > 480.0,
        "she stopped at {} — a submerged body is not in the world and nothing \
         solid may block it",
        scratch.kinematics.pos.x
    );
}
