//! Dodge roll trigger / cooldown / ability gate, and shield/parry
//! activation, deactivation, dash conflict, parry window reset.

use super::super::*;
use super::{step, test_world};
use crate::engine_core::AbilitySet;

#[test]
fn dodge_roll_triggers_on_ground_with_ability() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.0;
    assert!(player.abilities.dodge, "sandbox_all enables dodge");
    let events = step(
        &world,
        &mut player,
        InputState {
            dash_pressed: true,
            ..Default::default()
        },
    );
    assert!(
        events.operations.contains(&MovementOp::DodgeRoll),
        "dash on ground with dodge ability should trigger DodgeRoll"
    );
    assert!(
        player.dodge_roll_timer > 0.0,
        "dodge_roll_timer should be set"
    );
    assert!(
        player.vel.x.abs() > 100.0,
        "should have lateral velocity from dodge"
    );
}

#[test]
fn dodge_roll_blocked_by_cooldown() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.3;
    let events = step(
        &world,
        &mut player,
        InputState {
            dash_pressed: true,
            ..Default::default()
        },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should be blocked when on cooldown"
    );
}

#[test]
fn dodge_roll_disabled_when_ability_off() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.dodge = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.0;
    let events = step(
        &world,
        &mut player,
        InputState {
            dash_pressed: true,
            ..Default::default()
        },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should not trigger when ability is disabled"
    );
}

#[test]
fn shield_activates_when_held_with_ability() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    let events = step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(player.shield_active, "shield should be active while held");
    assert!(
        player.parry_window_timer > 0.0,
        "parry window should start on first activation"
    );
    assert!(
        events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp op should be recorded"
    );
}

#[test]
fn shield_deactivates_when_released() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(player.shield_active);
    step(
        &world,
        &mut player,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    assert!(
        !player.shield_active,
        "shield should drop when button released"
    );
}

#[test]
fn shield_blocked_during_dash() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    player.dash_timer = 0.10; // force active dash
    step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !player.shield_active,
        "shield cannot be raised during a dash"
    );
}

#[test]
fn shield_gives_fresh_parry_on_each_activation() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(player.parry_window_timer > 0.0);
    // Expire the parry window and drop shield.
    player.parry_window_timer = 0.0;
    step(
        &world,
        &mut player,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    // Re-raise: fresh parry window.
    step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        player.parry_window_timer > 0.0,
        "raising shield again should reset the parry window"
    );
}

#[test]
fn shield_disabled_when_ability_off() {
    let world = test_world();
    let abilities = AbilitySet::basic(); // basic() has shield: false
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = true;
    let events = step(
        &world,
        &mut player,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !player.shield_active,
        "shield should not activate without the ability"
    );
    assert!(
        !events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp should not fire without the ability"
    );
}
