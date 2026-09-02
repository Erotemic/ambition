//! Hazard contact consumes the tick's travelled path, not its endpoint alone.
//!
//! A body fast enough to step over a thin hazard between two samples used to
//! survive it: the shared gate tested `BodyKinematics::aabb()`, which is where
//! the body ENDED, and a hazard it passed through mid-step was never anywhere
//! in that test. The gate now reads the kernel's canonical `SweepSample`.
//!
//! Every case here drives a real simulation step through the one shared gate,
//! so the sample under test is the one production writes.

use super::super::*;
use super::test_world;
use crate::body_clusters::{BodyClusterScratch, SweepSample};
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, Vec2, World};

/// x-range of the thin hazard every case in this file crosses. Full-height, so
/// a case never has to reason about where gravity put the body vertically.
const HAZARD_MIN_X: f32 = 700.0;
const HAZARD_WIDTH: f32 = 4.0;
const HAZARD_MAX_X: f32 = HAZARD_MIN_X + HAZARD_WIDTH;

/// `test_world` plus one thin full-height hazard column.
fn hazard_world() -> World {
    let mut world = test_world();
    let h = world.size.y;
    world.blocks.push(crate::world::Block::hazard(
        "thin spike column",
        Vec2::new(HAZARD_MIN_X, 0.0),
        Vec2::new(HAZARD_WIDTH, h),
    ));
    world
}

/// The same rig with the hazard removed. A hazard block is not a collision
/// surface, so a body moves through this world exactly as it moves through
/// `hazard_world` — which is what lets a case establish its premise here and
/// assert the verdict there.
///
/// ⛔ THE PREMISE CANNOT BE READ FROM THE HAZARD RUN. `update_player_simulation_
/// with_clusters` respawns the body when a reset fires, and the respawn
/// rewrites the `SweepSample` — so the sample observable after a hit describes
/// the reset, not the step that caused it. Reading it there asserts nothing.
fn clear_world() -> World {
    test_world()
}

/// One step at `vel` from `start_x`, with a sample attached the way production
/// attaches one. Returns the reset cause and the sample the kernel wrote.
fn step_with_sample(
    world: &World,
    spec: MotionModelSpec,
    start_x: f32,
    vel_x: f32,
) -> (Option<ResetCause>, SweepSample, Vec2) {
    let spawn = Vec2::new(start_x, world.spawn.y);
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    scratch.kinematics.vel = Vec2::new(vel_x, 0.0);
    let mut sample = SweepSample::default();
    let (model, mut clusters) = scratch.parts();
    switch_motion_model(model, spec);
    clusters.sweep = Some(&mut sample);
    let events = update_player_simulation_with_clusters(
        world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );
    let half = clusters.kinematics.size * 0.5;
    (events.reset, sample, half)
}

fn every_policy() -> [(&'static str, MotionModelSpec); 3] {
    [
        (
            "axis-swept",
            MotionModelSpec::AxisSwept(AxisSweptParams::default()),
        ),
        (
            "surface-momentum",
            MotionModelSpec::SurfaceMomentum(MomentumParams::default()),
        ),
        (
            "adhesive-crawler",
            MotionModelSpec::AdhesiveCrawler(CrawlerParams::default()),
        ),
    ]
}

/// THE CASE THIS EXISTS FOR. Every policy runs the one shared gate, so every
/// policy has to catch the tunnel.
#[test]
fn a_body_that_steps_over_a_thin_hazard_is_hit() {
    for (name, spec) in every_policy() {
        // The premise comes from the hazard-free twin, where no reset fires and
        // the sample therefore still describes the step.
        let (clear_reset, sample, half) = step_with_sample(&clear_world(), spec, 400.0, 42_000.0);
        assert_eq!(
            clear_reset, None,
            "{name}: premise — the twin world has no hazard to hit",
        );

        // ⛔ PREMISE GUARD. If the step did not actually straddle the hazard —
        // a policy clamped the speed, a wall stopped it — then this case is
        // testing nothing and would pass for the wrong reason. Assert the
        // tunnel happened before asserting it was caught.
        assert!(
            sample.prev.x + half.x < HAZARD_MIN_X,
            "{name}: premise — the step must START clear of the hazard \
             (prev {} + half {} vs {HAZARD_MIN_X})",
            sample.prev.x,
            half.x,
        );
        assert!(
            sample.curr.x - half.x > HAZARD_MAX_X,
            "{name}: premise — the step must END clear of the hazard, so this \
             is a genuine tunnel (curr {} - half {} vs {HAZARD_MAX_X})",
            sample.curr.x,
            half.x,
        );

        let (reset, _, _) = step_with_sample(&hazard_world(), spec, 400.0, 42_000.0);
        assert_eq!(
            reset,
            Some(ResetCause::Hazard),
            "{name}: a body that crossed the hazard mid-step is hit",
        );
    }
}

/// The other side of the magnitude: a step that stops short is NOT a hit.
/// Without this arm, a swept test that returned `true` unconditionally would
/// pass the tunnel case above.
#[test]
fn a_body_that_stops_short_of_the_hazard_is_not_hit() {
    for (name, spec) in every_policy() {
        let (reset, sample, half) = step_with_sample(&hazard_world(), spec, 400.0, 600.0);

        assert!(
            sample.curr.x + half.x < HAZARD_MIN_X,
            "{name}: premise — this arm must END short of the hazard \
             (curr {} + half {} vs {HAZARD_MIN_X})",
            sample.curr.x,
            half.x,
        );
        assert_eq!(
            reset, None,
            "{name}: a body that never reached it is not hit"
        );
    }
}

/// PARITY with the discrete test this replaced: standing in it still kills.
#[test]
fn a_body_standing_in_the_hazard_is_still_hit() {
    for (name, spec) in every_policy() {
        let (reset, _, _) = step_with_sample(
            &hazard_world(),
            spec,
            HAZARD_MIN_X + HAZARD_WIDTH * 0.5,
            0.0,
        );
        assert_eq!(
            reset,
            Some(ResetCause::Hazard),
            "{name}: overlap is a hit whether or not anything moved",
        );
    }
}

/// A position change made OUTSIDE the simulation phase is not travelled motion.
/// The sample spans phase entry to phase exit, so a teleport across the hazard
/// is not in `prev -> curr` and must not read as having crossed it.
#[test]
fn a_teleport_across_the_hazard_is_not_traversal() {
    let world = hazard_world();
    let spawn = Vec2::new(400.0, world.spawn.y);
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    let mut sample = SweepSample::default();

    // Land the body well past the hazard the way a blink or a room transfer
    // does: by writing the position between phases, not by integrating.
    scratch.kinematics.pos = Vec2::new(1100.0, world.spawn.y);
    scratch.kinematics.vel = Vec2::ZERO;

    let (model, mut clusters) = scratch.parts();
    clusters.sweep = Some(&mut sample);
    let events = update_player_simulation_with_clusters(
        &world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );

    assert!(
        sample.prev.x > HAZARD_MAX_X,
        "premise — the sample must open on the far side, so the jump is \
         outside the segment (prev {})",
        sample.prev.x,
    );
    assert_eq!(
        events.reset, None,
        "a teleport over a hazard did not travel through it",
    );
}

/// THE COMPATIBILITY ARM, pinned deliberately. A body with no `SweepSample` is
/// tested at its endpoint and nothing else — the gate does NOT rebuild a
/// segment from `vel * dt`, because a second motion model beside the kernel's
/// is free to disagree with it. `SweepSample`'s `TODO(compat-remove)` is the
/// plan to delete this arm; when it goes, this case should go red and be
/// removed with it rather than quietly re-tuned.
#[test]
fn a_body_with_no_sample_is_tested_at_its_endpoint_only() {
    let world = hazard_world();
    let spawn = Vec2::new(400.0, world.spawn.y);
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    scratch.kinematics.vel = Vec2::new(42_000.0, 0.0);

    let (model, mut clusters) = scratch.parts();
    assert!(
        clusters.sweep.is_none(),
        "premise — this arm is about the body that has no sample",
    );
    let events = update_player_simulation_with_clusters(
        &world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );

    assert!(
        clusters.kinematics.pos.x > HAZARD_MAX_X,
        "premise — it did cross the hazard's x-range this step",
    );
    assert_eq!(
        events.reset, None,
        "no sample means the endpoint is the only thing known about the body",
    );
}
