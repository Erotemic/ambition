//! Pure ability-flag sanity: a flag set to `false` must keep the
//! corresponding op out of the FrameEvents / state.

use super::super::*;
use super::{step, test_world};
use crate::engine_core::AbilitySet;

#[test]
fn double_jump_ability_controls_air_jump() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.double_jump = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.air_jumps_available = 0;
    let events = step(
        &world,
        &mut player,
        InputState {
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert!(!events.operations.contains(&MovementOp::DoubleJump));

    abilities.double_jump = true;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.air_jumps_available = 1;
    let events = step(
        &world,
        &mut player,
        InputState {
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert!(events.operations.contains(&MovementOp::DoubleJump));
}

#[test]
fn double_dash_ability_controls_dash_charges() {
    let world = test_world();
    let mut single_dash = AbilitySet::sandbox_all();
    single_dash.double_dash = false;
    let player = Player::new_with_abilities(world.spawn, single_dash);
    assert_eq!(player.dash_charges_available, 1);

    let player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    assert_eq!(player.dash_charges_available, 2);
}

#[test]
fn wall_climb_requires_wall_cling() {
    let mut abilities = AbilitySet::sandbox_all();
    abilities.wall_cling = false;
    assert!(abilities
        .compatibility_warnings()
        .iter()
        .any(|w| w.contains("wall_climb")));
}
