//! Blink mechanics: press / hold / release, precision aim, soft-wall
//! pass, post-blink gravity grace, downward-velocity clamp.

use super::super::*;
use super::{step, test_world};
use crate::engine_core::world::BlinkWallTier;
use crate::engine_core::{AbilitySet, Vec2};

#[test]
fn held_blink_arms_when_cooldown_clears_without_new_press() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.blink_cooldown = 0.02;

    // Pressing slightly early should not arm yet.
    let _ = update_player_control_with_tuning(
        &world,
        &mut player,
        InputState {
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(!player.blink_hold_active);

    // Cooldown clears in simulation time.
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        0.03,
        DEFAULT_TUNING,
    );
    assert_eq!(player.blink_cooldown, 0.0);

    // The user is still holding the button, so control time can arm blink
    // without requiring another just-pressed edge.
    let _ = update_player_control_with_tuning(
        &world,
        &mut player,
        InputState {
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(player.blink_hold_active);
}

#[test]
fn blink_ability_gates_teleport() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.blink = false;
    abilities.precision_blink = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    let start = player.pos;
    let input = InputState {
        axis_x: 1.0,
        blink_pressed: true,
        blink_held: true,
        ..Default::default()
    };
    let _ =
        update_player_control_with_tuning(&world, &mut player, input, 1.0 / 60.0, DEFAULT_TUNING);
    let input = InputState {
        axis_x: 1.0,
        blink_released: true,
        ..Default::default()
    };
    let events =
        update_player_control_with_tuning(&world, &mut player, input, 1.0 / 60.0, DEFAULT_TUNING);
    assert_eq!(player.pos, start);
    assert!(events.blinks.is_empty());
}

#[test]
fn quick_blink_moves_on_release() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let start = player.pos;
    step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
    );
    let events = step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
    );
    assert!(player.pos.x > start.x + 20.0);
    assert_eq!(events.blinks.len(), 1);
    assert!(!events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::Blink));
}

#[test]
fn held_blink_enters_precision_aiming() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    for _ in 0..20 {
        let blink_pressed = !player.blink_hold_active;
        step(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_held: true,
                blink_pressed,
                ..Default::default()
            },
        );
    }
    assert!(player.blink_aiming);
    let events = step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
    );
    assert_eq!(events.blinks.len(), 1);
    assert!(events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::PrecisionBlink));
}

#[test]
fn repeated_blinks_clamp_downward_velocity_each_time() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.pos = Vec2::new(420.0, 620.0);

    for _ in 0..2 {
        player.vel = Vec2::new(25.0, 900.0);
        player.blink_cooldown = 0.0;
        player.blink_hold_active = true;
        player.blink_aiming = false;
        let events = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_released: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert_eq!(events.blinks.len(), 1);
        assert!(
            player.vel.y
                <= DEFAULT_TUNING.blink_max_downward_speed + DEFAULT_TUNING.gravity / 60.0 + 1.0,
            "blink should not preserve a large downward fall speed; got {}",
            player.vel.y
        );
        assert!(player.blink_grace_timer > 0.0);
    }
}

#[test]
fn post_blink_grace_suspends_gravity_for_tiny_window() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.pos = Vec2::new(420.0, 620.0);
    player.vel = Vec2::new(0.0, 900.0);
    player.blink_hold_active = true;
    let _events = update_player_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    let after_blink_vy = player.vel.y;
    let _events = update_player_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 240.0,
        DEFAULT_TUNING,
    );
    assert!(
        player.vel.y <= after_blink_vy + 0.1,
        "gravity should be suspended during the short post-blink grace window"
    );
}

#[test]
fn blink_walls_can_be_passed_by_upgrade_without_allowing_solid_walls() {
    let mut world = test_world();
    world.blocks.clear();
    world.blocks.push(crate::engine_core::world::Block::blink_wall(
        "test soft blink membrane",
        Vec2::new(220.0, 0.0),
        Vec2::new(22.0, 300.0),
        BlinkWallTier::Soft,
    ));

    let mut blocked_abilities = AbilitySet::basic();
    blocked_abilities.blink = true;
    let blocked_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), blocked_abilities);
    let blocked = blink_destination_to_point(&world, &blocked_player, Vec2::new(340.0, 140.0));
    assert!(blocked.x < 220.0);

    let mut pass_abilities = blocked_abilities;
    pass_abilities.blink_through_soft_walls = true;
    let pass_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), pass_abilities);
    let passed = blink_destination_to_point(&world, &pass_player, Vec2::new(340.0, 140.0));
    assert!(passed.x > 300.0);
}
