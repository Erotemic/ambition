//! Ledge grab sandbox presentation helpers.
//!
//! Gameplay ownership has moved into `ambition_engine`: `ae::Player::ledge_grab`
//! is advanced by `ae::update_player_simulation_with_tuning` alongside gravity,
//! wall contact, moving platforms, and water. The sandbox keeps this module only
//! as a stable place for presentation code/tests that want the public timing
//! constants.

pub use ambition_engine::{LEDGE_CLIMB_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_TOWARD_CLIMB_DELAY};

#[cfg(test)]
mod tests {
    use ambition_engine as ae;

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
        let mut player = ae::Player::new_with_abilities(world.spawn, abilities);
        player.pos = ae::Vec2::new(86.0, 110.0);
        player.vel = ae::Vec2::new(30.0, 20.0);
        player.wall_clinging = true;
        player.on_wall = true;
        player.wall_normal_x = -1.0;

        let mut input = ae::InputState {
            axis_x: 1.0,
            control_dt: 0.016,
            ..Default::default()
        };
        let events = ae::update_player_simulation(&world, &mut player, input, 0.016);
        assert!(
            player.ledge_grab.is_some(),
            "engine tick should latch ledge state"
        );
        assert!(events.operations.contains(&ae::MovementOp::LedgeGrab));

        input.axis_y = -1.0;
        let mut saw_start = false;
        for _ in 0..16 {
            let events = ae::update_player_simulation(&world, &mut player, input, 0.016);
            if events.operations.contains(&ae::MovementOp::LedgeClimbStart) {
                saw_start = true;
                break;
            }
        }
        assert!(
            saw_start,
            "engine should start the climb after the hang delay"
        );
        assert!(player.ledge_grab.map(|s| s.climbing).unwrap_or(false));

        let mut saw_finish = false;
        for _ in 0..32 {
            let events = ae::update_player_simulation(&world, &mut player, input, 0.016);
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
        assert!(player.ledge_grab.is_none());
        assert!(player.on_ground);
    }
}
