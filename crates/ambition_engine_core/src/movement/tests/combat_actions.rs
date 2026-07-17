//! Dodge roll trigger / cooldown / ability gate, and shield/parry
//! activation, deactivation, dash conflict, parry window reset.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;
use crate::Vec2;

fn scratch_at(spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn dodge_roll_triggers_on_ground_with_ability() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.0;
    assert!(
        scratch.abilities.abilities.dodge,
        "sandbox_all enables dodge"
    );
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(
        events.operations.contains(&MovementOp::DodgeRoll),
        "dash on ground with dodge ability should trigger DodgeRoll"
    );
    assert!(
        scratch.axis().dodge_roll_timer > 0.0,
        "dodge_roll_timer should be set"
    );
    assert!(
        scratch.kinematics.vel.x.abs() > 100.0,
        "should have lateral velocity from dodge"
    );
}

#[test]
fn dodge_roll_blocked_by_cooldown() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.3;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
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
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.0;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
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
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.active, "shield should be active while held");
    assert!(
        scratch.shield.parry_window_timer > 0.0,
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
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.active);
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield should drop when button released"
    );
}

#[test]
fn shield_blocked_during_dash() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    scratch.axis_mut().dash_timer = 0.10; // force active dash
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield cannot be raised during a dash"
    );
}

#[test]
fn shield_gives_fresh_parry_on_each_activation() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.parry_window_timer > 0.0);
    // Expire the parry window and drop shield.
    scratch.shield.parry_window_timer = 0.0;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    // Re-raise: fresh parry window.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        scratch.shield.parry_window_timer > 0.0,
        "raising shield again should reset the parry window"
    );
}

#[test]
fn shield_disabled_when_ability_off() {
    let world = test_world();
    let abilities = AbilitySet::basic(); // basic() has shield: false
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield should not activate without the ability"
    );
    assert!(
        !events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp should not fire without the ability"
    );
}
