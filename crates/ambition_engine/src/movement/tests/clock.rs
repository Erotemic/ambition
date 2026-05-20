//! Sim-vs-control clock separation and tiny-dt safety: gravity must
//! follow sim time (bullet-time), input/aim must follow control time.

use super::super::*;
use super::test_world;
use crate::Vec2;

#[test]
fn tiny_dt_preserves_bullet_time_scale() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.vel = Vec2::ZERO;
    let _ = update_player_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    let normal_fall_speed = player.vel.y;

    let mut slow_player = Player::new(world.spawn);
    slow_player.on_ground = false;
    slow_player.coyote_timer = 0.0;
    slow_player.vel = Vec2::ZERO;
    let _ = update_player_with_tuning(
        &world,
        &mut slow_player,
        InputState::default(),
        (1.0 / 60.0) * 0.001,
        DEFAULT_TUNING,
    );

    assert!(slow_player.vel.y > 0.0);
    assert!(
        slow_player.vel.y < normal_fall_speed * 0.01,
        "tiny dt should not be clamped up to normal-ish gravity"
    );
}

#[test]
fn control_clock_can_aim_blink_while_sim_clock_is_nearly_frozen() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.vel = Vec2::ZERO;

    // Real-time control crosses the precision-blink threshold.
    for i in 0..8 {
        let _ = update_player_control_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_pressed: i == 0,
                blink_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
    }
    assert!(
        player.blink_aiming,
        "control time should enter precision aim quickly"
    );

    // Game-time simulation is almost frozen, so gravity should barely change.
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        (1.0 / 60.0) * 0.000035,
        DEFAULT_TUNING,
    );
    assert!(
        player.vel.y < 0.01,
        "player gravity must use scaled game time while control remains real-time; got {}",
        player.vel.y
    );
}
