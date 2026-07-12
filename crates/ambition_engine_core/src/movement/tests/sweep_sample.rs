//! §3.1 `SweepSample` — the kernel-written motion record.
//!
//! The contract under test: the sample is the SIMULATION PHASE's own
//! integration segment (both endpoints captured inside the kernel), so any
//! position change outside that window — blink in the control phase, the
//! player respawn wrapper after the sim returns — is excluded from the
//! record BY CONSTRUCTION, with no reset protocol.

use super::super::*;
use super::test_world;
use crate::body_clusters::{BodyClusterScratch, SweepSample};
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, Vec2};

fn scratch_at(spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

/// Run one simulation step with a sample attached to the view.
fn sim_step_sampled(
    world: &crate::World,
    scratch: &mut BodyClusterScratch,
    sample: &mut SweepSample,
    input: InputState,
    dt: f32,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    clusters.sweep = Some(sample);
    update_player_simulation_with_clusters(world, &mut clusters, input, dt, TEST_TUNING)
}

#[test]
fn the_sample_records_the_integration_segment() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.vel = Vec2::new(120.0, 0.0);
    let start = scratch.kinematics.pos;
    let mut sample = SweepSample::default();

    sim_step_sampled(
        &world,
        &mut scratch,
        &mut sample,
        InputState::default(),
        1.0 / 60.0,
    );

    assert_eq!(sample.prev, start, "prev = position at sim-phase entry");
    assert_eq!(
        sample.curr, scratch.kinematics.pos,
        "curr = position at sim-phase exit"
    );
    assert!(
        (sample.curr - sample.prev).length() > 0.0,
        "a moving body records a non-zero segment"
    );
    assert_eq!(sample.vel, Vec2::new(120.0, 0.0), "vel = velocity at prev");
    assert_eq!(sample.half, scratch.kinematics.size * 0.5);
}

#[test]
fn a_zero_dt_tick_records_a_zero_length_segment_not_a_stale_one() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.vel = Vec2::new(120.0, 0.0);
    let mut sample = SweepSample::default();

    sim_step_sampled(
        &world,
        &mut scratch,
        &mut sample,
        InputState::default(),
        1.0 / 60.0,
    );
    assert!((sample.curr - sample.prev).length() > 0.0);

    // Paused frame: dt = 0 takes the early-return path — the record must
    // collapse to the current position, never persist last tick's segment.
    sim_step_sampled(
        &world,
        &mut scratch,
        &mut sample,
        InputState::default(),
        0.0,
    );
    assert_eq!(
        sample.prev, sample.curr,
        "zero-dt tick = zero-length record"
    );
    assert_eq!(sample.curr, scratch.kinematics.pos);
}

#[test]
fn a_control_phase_blink_is_never_path() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    let before_blink = scratch.kinematics.pos;

    // Blink in the CONTROL phase (a teleport: the body does not traverse
    // the gap — that is blink's design identity). The blink model is
    // press-to-arm / release-to-fire, so arm first, then release.
    let _ = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    let events = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            blink_quick_dir: crate::WorldVec2(Vec2::new(1.0, 0.0)),
            blink_released: true,
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(
        !events.blinks.is_empty(),
        "fixture: the blink must actually fire"
    );
    let after_blink = scratch.kinematics.pos;
    assert_ne!(
        before_blink, after_blink,
        "fixture: the blink moved the body"
    );

    // The following SIM phase's record starts at the post-blink position:
    // the jump is excluded from the path by construction.
    let mut sample = SweepSample::default();
    sim_step_sampled(
        &world,
        &mut scratch,
        &mut sample,
        InputState::default(),
        1.0 / 60.0,
    );
    assert_eq!(
        sample.prev, after_blink,
        "the segment starts AFTER the blink — a swept hazard reader can \
         never graze the blinked-over gap"
    );
}

#[test]
fn the_respawn_wrapper_leaves_a_zero_length_record_at_spawn() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    // Drop the body far below the world so the sim flags the
    // out-of-bounds reset and the player wrapper respawns it.
    scratch.kinematics.pos = Vec2::new(world.spawn.x, world.size.y + 500.0);
    let mut sample = SweepSample::default();

    let mut clusters = scratch.as_mut();
    clusters.sweep = Some(&mut sample);
    let events = update_player_simulation_with_clusters(
        &world,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );
    drop(clusters);

    assert!(events.reset, "fixture: the fall must trigger the reset");
    assert_eq!(scratch.kinematics.pos, world.spawn, "fixture: respawned");
    assert_eq!(
        sample.prev, world.spawn,
        "the respawn teleport is not path — the record is zero-length at spawn"
    );
    assert_eq!(sample.prev, sample.curr);
}
