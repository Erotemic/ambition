#![cfg(all(feature = "portal", feature = "rl_sim"))]

//! `portal_scale_lab`: two scaling doors, each a wall with a mouth on either
//! face, crossed again and again under the shipped portal tuning.

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

/// The faces of door A's wall and door B's wall.
const A: (f32, f32) = (320.0, 352.0);
const B: (f32, f32) = (608.0, 640.0);

#[test]
fn walking_back_and_forth_through_the_scaling_doors_halves_and_restores_the_upright_body() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_required_start_room("portal_scale_lab")
            .with_sync_test_rollback_settings(4, 10)).unwrap();
    for _ in 0..30 { sim.step(crate::common::base()); }
    let player = { let world = sim.world_mut(); world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap() };
    let body = |sim: &Platformer2dSimHarness| *sim.world().get::<BodyKinematics>(player).unwrap();
    sim.rebase_rollback_history().unwrap();
    let (mut small, mut large) = (0, 0);
    for trip in 0..3 {
        for (heading, goal) in [(1.0, 720.0), (-1.0, 200.0)] {
            let mut frames = 0;
            while (body(&sim).pos.x - goal) * heading < 0.0 {
                frames += 1;
                assert!(frames < 600, "trip {trip}: the walk toward {goal} stalled at {:?}", body(&sim));
                sim.step(AgentAction { move_x: heading, ..crate::common::base() });
                sim.rollback_health().unwrap();
                let pose = body(&sim);
                assert!(pose.material.orthogonal().abs_diff_eq(Mat2::IDENTITY, 1.0e-5),
                    "trip {trip}: the doors face one way, so the body never turns: {pose:?}");
                let half = pose.half_extent().x;
                if pose.pos.x - half > A.1 && pose.pos.x + half < B.0 {
                    assert_eq!(pose.material.scale(), 0.5, "trip {trip}: between the doors the body is half size");
                    small += 1;
                } else if pose.pos.x + half < A.0 || pose.pos.x - half > B.1 {
                    assert_eq!(pose.material.scale(), 1.0, "trip {trip}: outside the doors the body is whole");
                    large += 1;
                }
            }
        }
    }
    assert!(small > 60 && large > 60, "both sizes walked: {small} small, {large} whole");
    assert!(sim.rollback_execution_stats().unwrap().load_runs > 0);
}
