//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod action_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! The AgentAction -> ControlFrame conversion is the RL/scripted-driver
//! seam into the engine. Pin the constructors, the field forwarding,
//! and especially the documented edge-vs-held distinction (a held axis
//! must not synthesize a down/up *edge* — that regressed crouch into
//! MorphBall once).
use super::*;

#[test]
fn default_action_is_a_neutral_control_frame() {
    let cf: ControlFrame = AgentAction::default().into();
    assert_eq!(cf.axis_x, 0.0);
    assert_eq!(cf.axis_y, 0.0);
    assert!(!cf.jump_pressed);
    assert!(!cf.left_pressed);
    assert!(!cf.right_pressed);
    assert!(!cf.down_pressed);
    assert!(!cf.up_pressed);
    assert!(!cf.attack_pressed);
    assert!(!cf.blink_pressed);
}

#[test]
fn move_x_constructor_sets_only_the_horizontal_axis() {
    let cf: ControlFrame = AgentAction::move_x(-1.0).into();
    assert_eq!(cf.axis_x, -1.0);
    assert_eq!(cf.axis_y, 0.0);
    assert!(!cf.jump_pressed);
}

#[test]
fn jump_constructor_presses_and_holds() {
    let cf: ControlFrame = AgentAction::jump().into();
    assert!(cf.jump_pressed);
    assert!(cf.jump_held);
    assert!(!cf.jump_released);
}

#[test]
fn reset_constructor_sets_reset_only() {
    let cf: ControlFrame = AgentAction::reset().into();
    assert!(cf.reset_pressed);
    assert!(!cf.start_pressed);
}

#[test]
fn held_move_y_does_not_synthesize_a_down_edge() {
    let cf: ControlFrame = AgentAction {
        move_y: 1.0,
        ..Default::default()
    }
    .into();
    assert_eq!(cf.axis_y, 1.0, "continuous axis is still forwarded");
    assert!(!cf.down_pressed, "held axis must not fake a down edge");
    assert!(!cf.up_pressed);
}

#[test]
fn explicit_edge_flags_are_forwarded() {
    let cf: ControlFrame = AgentAction {
        left_pressed: true,
        right_pressed: true,
        up_pressed: true,
        down_pressed: true,
        ..Default::default()
    }
    .into();
    assert!(cf.left_pressed);
    assert!(cf.right_pressed);
    assert!(cf.up_pressed);
    assert!(cf.down_pressed);
}

#[test]
fn converter_always_neutralizes_unsourced_fields() {
    // shield_held and fast_fall_pressed have no AgentAction source.
    let cf: ControlFrame = AgentAction {
        jump: true,
        blink: true,
        move_y: -1.0,
        ..Default::default()
    }
    .into();
    assert!(!cf.shield_held);
    assert!(!cf.fast_fall_pressed);
}

#[test]
fn blink_and_projectile_triplets_plus_aim_forward() {
    let cf: ControlFrame = AgentAction {
        blink: true,
        blink_held: true,
        blink_released: true,
        projectile: true,
        projectile_held: true,
        projectile_released: true,
        aim_x: 0.5,
        aim_y: -0.5,
        ..Default::default()
    }
    .into();
    assert!(cf.blink_pressed && cf.blink_held && cf.blink_released);
    assert!(cf.projectile_pressed && cf.projectile_held && cf.projectile_released);
    assert_eq!(cf.aim_x, 0.5);
    assert_eq!(cf.aim_y, -0.5);
}
