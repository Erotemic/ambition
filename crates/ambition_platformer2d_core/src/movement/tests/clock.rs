//! Sim-vs-control clock separation and tiny-dt safety: gravity must
//! follow sim time (bullet-time), input/aim must follow control time.

use super::super::*;
use super::test_world;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::Vec2;

fn scratch_at(spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, crate::AbilitySet::sandbox_all())
}

#[test]
fn tiny_dt_preserves_bullet_time_scale() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.kinematics.vel = Vec2::ZERO;
    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );
    let normal_fall_speed = scratch.kinematics.vel.y;

    let mut slow = scratch_at(world.spawn);
    slow.ground.on_ground = false;
    slow.axis_mut().coyote_timer = 0.0;
    slow.kinematics.vel = Vec2::ZERO;
    let _ = update_player_with_tuning_scratch(
        &world,
        &mut slow,
        InputState::default(),
        (1.0 / 60.0) * 0.001,
        TEST_TUNING,
    );

    assert!(slow.kinematics.vel.y > 0.0);
    assert!(
        slow.kinematics.vel.y < normal_fall_speed * 0.01,
        "tiny dt should not be clamped up to normal-ish gravity"
    );
}

#[test]
fn control_clock_can_aim_blink_while_sim_clock_is_nearly_frozen() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.kinematics.vel = Vec2::ZERO;

    // Real-time control crosses the precision-blink threshold.
    for i in 0..8 {
        let _ = update_player_control_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                movement: crate::ActionEdges::EMPTY.with(
                    crate::MovementAction::Blink,
                    crate::Edge {
                        pressed: i == 0,
                        held: true,
                        released: false,
                    },
                ),
                axes: crate::LocalAxes::new(1.0, 0.0),
                ..Default::default()
            },
            1.0 / 60.0,
            TEST_TUNING,
        );
    }
    assert!(
        scratch.axis().blink_aiming,
        "control time should enter precision aim quickly"
    );

    // Game-time simulation is almost frozen, so gravity should barely change.
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        (1.0 / 60.0) * 0.000035,
        TEST_TUNING,
    );
    assert!(
        scratch.kinematics.vel.y < 0.01,
        "player gravity must use scaled game time while control remains real-time; got {}",
        scratch.kinematics.vel.y
    );
}

/// Direct cluster-mut callable: pins that `update_player_clusters`
/// (TEST_TUNING convenience wrapper) can be driven from a
/// `BodyClusterScratch::as_mut()` view, mirroring the production
/// code path that takes a `Query<BodyClusterQueryData>`.
#[test]
fn update_player_clusters_runs_one_frame() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::ZERO;
    {
        let (model, mut clusters) = scratch.parts();
        let _events = update_player_clusters(
            &world,
            model,
            &mut clusters,
            InputState::default(),
            1.0 / 60.0,
        );
    }
    // Idle frame should still produce gravity-driven downward velocity.
    assert!(
        scratch.kinematics.vel.y > 0.0,
        "update_player_clusters should apply gravity; got {}",
        scratch.kinematics.vel.y
    );
}
