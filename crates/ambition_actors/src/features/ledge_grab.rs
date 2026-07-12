//! Ledge grab sandbox presentation helpers.
//!
//! Gameplay ownership lives in `ambition_engine_core`:
//! `BodyLedgeState::grab` is advanced by
//! `ae::step_motion` alongside gravity,
//! wall contact, moving platforms, and water. The sandbox keeps this
//! module only as a stable place for presentation code/tests that
//! want the public timing constants.

pub use ambition_engine_core::{LEDGE_CLIMB_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_TOWARD_CLIMB_DELAY};

#[cfg(test)]
mod tests {
    use ambition_engine_core as ae;

    fn step_axis(
        world: &ae::World,
        scratch: &mut ae::BodyClusterScratch,
        model: &mut ae::MotionModel,
        input: ae::InputState,
        dt: f32,
    ) -> ae::FrameEvents {
        let frame = ae::MotionFrame::from_direction(ae::DEFAULT_GRAVITY_DIR, ae::GRAVITY);
        let mut clusters = scratch.as_mut();
        ae::step_motion(
            model,
            &mut clusters,
            ae::MotionStepContext {
                world,
                input,
                frame,
                facing_intent: input.axis_x,
                dt,
            },
        )
        .events
    }

    #[test]
    fn engine_owned_ledge_state_hangs_then_climbs() {
        let world = ae::World::new(
            "ledge_test",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(80.0, 160.0),
            vec![ae::Block::solid(
                "ledge",
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(200.0, 200.0),
            )],
        );
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.ledge_grab = true;
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(world.spawn, abilities);
        scratch.kinematics.pos = ae::Vec2::new(86.0, 110.0);
        scratch.kinematics.vel = ae::Vec2::new(30.0, 20.0);
        scratch.wall.wall_clinging = true;
        scratch.wall.on_wall = true;
        scratch.wall.wall_normal_x = -1.0;

        let mut input = ae::InputState {
            axis_x: 1.0,
            control_dt: 0.016,
            ..Default::default()
        };
        let mut model = ae::MotionModel::axis_swept(ae::DEFAULT_AXIS_SWEPT_PARAMS);
        let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
        assert!(
            scratch.ledge.grab.is_some(),
            "engine tick should latch ledge state"
        );
        assert!(events.operations.contains(&ae::MovementOp::LedgeGrab));

        input.axis_y = -1.0;
        let mut saw_start = false;
        for _ in 0..16 {
            let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
            if events.operations.contains(&ae::MovementOp::LedgeClimbStart) {
                saw_start = true;
                break;
            }
        }
        assert!(
            saw_start,
            "engine should start the climb after the hang delay"
        );
        assert!(scratch.ledge.grab.map(|s| s.climbing).unwrap_or(false));

        let mut saw_finish = false;
        for _ in 0..32 {
            let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
            if events
                .operations
                .contains(&ae::MovementOp::LedgeClimbFinish)
            {
                saw_finish = true;
                break;
            }
        }
        assert!(
            saw_finish,
            "engine should finish the climb inside simulation ticks"
        );
        assert!(scratch.ledge.grab.is_none());
        assert!(scratch.ground.on_ground);
    }
}
