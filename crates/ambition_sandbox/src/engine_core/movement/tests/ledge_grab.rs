//! Ledge grab latch (blink-wall surface, one-way surface) and the
//! "climb finishes inside one simulation tick" invariant.

use super::super::*;
use super::test_world;
use crate::engine_core::world::{BlinkWallTier, Block};
use crate::engine_core::{AbilitySet, Vec2};

#[test]
fn simulation_latches_ledge_grab_on_blink_wall_surface() {
    let mut world = test_world();
    world.blocks.push(Block::blink_wall(
        "soft blink ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
        BlinkWallTier::Soft,
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(86.0, 110.0), abilities);
    player.vel = Vec2::new(120.0, 20.0);
    player.wall_clinging = true;
    player.on_wall = true;
    player.wall_normal_x = -1.0;

    let events = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(player.ledge_grab.is_some(), "blink wall ledge should latch");
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn simulation_latches_ledge_grab_on_one_way_surface_without_wall_collision() {
    let mut world = test_world();
    world.blocks.push(Block::one_way(
        "one-way ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 16.0),
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(86.0, 110.0), abilities);
    player.on_ground = false;
    player.vel = Vec2::new(20.0, 40.0);

    let events = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        player.ledge_grab.is_some(),
        "pressing toward a one-way edge should allow a pull-up even though one-way platforms do not collide on X"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn attack_press_from_hang_starts_getup_attack_and_fires_slash() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(87.0, 119.0), abilities);
    let contact = crate::engine_core::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::engine_core::LedgeGrabState::hanging(contact);
    // Skip past the hang debounce so the input is accepted this tick.
    state.elapsed = crate::engine_core::LEDGE_MIN_CLIMB_DELAY;
    player.ledge_grab = Some(state);

    let events = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            attack_pressed: true,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
        events.operations.contains(&MovementOp::LedgeGetupAttack),
        "attack from hang should emit LedgeGetupAttack"
    );
    assert!(
        events.operations.contains(&MovementOp::Slash),
        "getup attack should also emit a Slash op so the hitbox fires"
    );
    let new_state = player.ledge_grab.expect("getup-attack keeps ledge state");
    assert!(new_state.climbing, "state should be in getup transition");
    assert_eq!(new_state.getup_kind, crate::engine_core::LedgeGetupKind::Attack);
    assert!(
        player.dodge_roll_timer > 0.0,
        "getup attack grants invuln frames via dodge_roll_timer"
    );
}

#[test]
fn active_ledge_grab_climb_finishes_inside_simulation_tick() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(87.0, 119.0), abilities);
    let contact = crate::engine_core::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::engine_core::LedgeGrabState::hanging(contact);
    state.elapsed = crate::engine_core::LEDGE_MIN_CLIMB_DELAY;
    state.climbing = true;
    state.climb_elapsed = crate::engine_core::LEDGE_CLIMB_TIME;
    player.ledge_grab = Some(state);

    let events = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        player.ledge_grab.is_none(),
        "completed climb clears ledge state"
    );
    assert_eq!(player.pos, contact.climb_target);
    assert!(player.on_ground);
    assert!(events.operations.contains(&MovementOp::LedgeClimbFinish));
}
