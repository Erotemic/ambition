//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_engine_core::ControlFrame;

fn input_with<F: FnOnce(&mut ControlFrame)>(mut_fn: F) -> ControlFrame {
    let mut c = ControlFrame::default();
    mut_fn(&mut c);
    c
}

#[test]
fn neutral_input_yields_neutral_frame() {
    let input = ControlFrame::default();
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.melee_pressed = true; // pre-poisoned
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(!out.melee_pressed);
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    // Facing falls back to snapshot when stick neutral.
    assert_eq!(out.facing, 1.0);
}

#[test]
fn tick_player_brain_without_snapshot_input_falls_back_to_neutral() {
    // When snapshot.player_input is None, tick_player_brain
    // emits a neutral frame + facing only (no garbage from
    // uninitialized fields). Pins the safe-default path so
    // an actor with Brain::Player but no input snapshot
    // doesn't fire random actions.
    let mut s = BrainSnapshot::idle();
    s.actor_facing = -1.0;
    s.player_input = None;
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.melee_pressed = true; // pre-poisoned
    out.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        200.0,
    ));
    tick_player_brain(PlayerSlot(0), &s, &mut out);
    assert!(!out.melee_pressed);
    assert!(out.fire.is_none());
    assert_eq!(out.facing, -1.0);
}

#[test]
fn attack_pressed_routes_to_melee_intent() {
    let input = input_with(|c| {
        c.attack_pressed = true;
        c.axis_x = 0.0;
        c.axis_y = -1.0;
    });
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(out.melee_pressed);
    // Up-tilt: attack_axis carries the input direction.
    assert_eq!(out.attack_axis, ae::Vec2::new(0.0, -1.0));
}

#[test]
fn movement_axis_routes_to_locomotion_and_facing() {
    let input = input_with(|c| {
        c.axis_x = -1.0;
        c.axis_y = 0.3;
    });
    let s = BrainSnapshot::idle();
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert_eq!(out.locomotion, ae::Vec2::new(-1.0, 0.3));
    assert_eq!(out.facing, -1.0);
}

#[test]
fn screen_directed_sideways_gravity_maps_directional_verbs_to_local_frame() {
    let mut s = BrainSnapshot::idle();
    // Gravity / acceleration points screen-right, so the controlled body's
    // local-right direction is screen-up. In screen-directed mode, raw
    // screen-down should therefore mean local-left.
    s.control_down = ae::Vec2::new(1.0, 0.0);
    s.movement_frame_mode = ae::InputFrameMode::ScreenRelative;
    s.actor_facing = 1.0;
    let input = input_with(|c| {
        c.axis_x = 0.0;
        c.axis_y = 1.0;
        c.attack_pressed = true;
    });
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert_eq!(out.locomotion, ae::Vec2::new(-1.0, 0.0));
    assert_eq!(out.facing, -1.0);
    assert_eq!(out.attack_axis, ae::Vec2::new(-1.0, 0.0));

    // Raw screen-right maps to local-down under the same frame/mode, which
    // is the crouch / down-attack / morph-ball direction.
    let input = input_with(|c| {
        c.axis_x = 1.0;
        c.axis_y = 0.0;
        c.attack_pressed = true;
    });
    tick_player_brain_from_control(&input, &s, &mut out);
    assert_eq!(out.locomotion, ae::Vec2::new(0.0, 1.0));
    assert_eq!(out.attack_axis, ae::Vec2::new(0.0, 1.0));
}

#[test]
fn jump_edges_pass_through() {
    let input = input_with(|c| {
        c.jump_pressed = true;
        c.jump_held = true;
        c.jump_released = false;
    });
    let s = BrainSnapshot::idle();
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(out.jump_pressed);
    assert!(out.jump_held);
    assert!(!out.jump_released);
}

#[test]
fn player_brain_keeps_body_contact_damage_disabled() {
    let input = input_with(|c| {
        c.axis_x = 1.0;
        c.jump_pressed = true;
        c.attack_pressed = true;
    });
    let s = BrainSnapshot::idle();
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.body_contact_damage_enabled = true; // pre-poisoned
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(!out.body_contact_damage_enabled);
}

#[test]
fn projectile_released_emits_fire_with_resolved_aim_or_facing() {
    // Aim stick -> local aim is preserved for charge projectiles. The
    // fire request carries that same local direction plus an explicit
    // frame policy; consumers decide how to project it into their runtime
    // frame.
    let input = input_with(|c| {
        c.projectile_released = true;
        c.aim_x = 0.0;
        c.aim_y = -1.0;
    });
    let mut s = BrainSnapshot::idle();
    s.actor_facing = -1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    let fire = out.fire.expect("fire request expected");
    // Aim wins over facing, and remains controlled-body-local.
    assert_eq!(
        fire.dir_policy,
        ae::GameplayFramePolicy::ControlledBodyLocal
    );
    assert!((fire.dir.y - (-1.0)).abs() < 0.001);
    assert_eq!(out.aim, ae::Vec2::new(0.0, -1.0));
    // No aim → fire uses facing.
    let input2 = input_with(|c| {
        c.projectile_released = true;
    });
    let mut out2 = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input2, &s, &mut out2);
    let fire2 = out2.fire.expect("fire request expected");
    assert_eq!(
        fire2.dir_policy,
        ae::GameplayFramePolicy::ControlledBodyLocal
    );
    assert!((fire2.dir.x - (-1.0)).abs() < 0.001);
    assert_eq!(out2.aim, ae::Vec2::ZERO);
}

#[test]
fn projectile_aim_crosses_screen_input_seam_once() {
    let input = input_with(|c| {
        c.projectile_released = true;
        c.aim_x = 0.0;
        c.aim_y = -1.0; // screen-up on the right stick
    });
    let mut s = BrainSnapshot::idle();
    s.control_down = ae::Vec2::new(1.0, 0.0);
    s.aim_frame_mode = ae::InputFrameMode::ScreenRelative;
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert_eq!(out.aim, ae::Vec2::new(1.0, 0.0));
    let fire = out.fire.expect("fire request expected");
    assert_eq!(
        fire.dir_policy,
        ae::GameplayFramePolicy::ControlledBodyLocal
    );
    assert_eq!(fire.dir, out.aim);
    assert_eq!(
        fire.dir_to_world(ae::AccelerationFrame::new(s.control_down)),
        ae::Vec2::new(0.0, -1.0)
    );
}

#[test]
fn blink_both_forms_screen_relative_by_default_quick_rotates_under_body_relative_movement() {
    // The two blink forms steer different sticks and so resolve through the two
    // independent default policies: quick blink follows the LOCOMOTION mode,
    // precision blink follows the AIM mode. BOTH default to ScreenRelative
    // (`InputFrameMode::DEFAULT_MOVEMENT` / `DEFAULT_AIM`), which is exactly why
    // in-game blink always points where the stick points on screen at any
    // gravity. The seam still EXISTS — flipping the locomotion mode to a
    // body-relative policy rotates quick blink with gravity while precision
    // blink stays screen-directed — so this pins both the default and the seam.
    let input = input_with(|c| {
        c.axis_y = -1.0; // screen-up on the locomotion stick
    });
    let mut s = BrainSnapshot::idle(); // movement = Screen, aim = Screen (defaults)
    s.control_down = ae::Vec2::new(1.0, 0.0); // sideways gravity (feet point screen-right)
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);

    // DEFAULT: both forms screen-relative — screen-up stays screen-up in WORLD.
    assert!(
        (out.blink_aim_step - ae::Vec2::new(0.0, -1.0)).length() < 1e-5,
        "precision blink must be screen-relative by default; got {:?}",
        out.blink_aim_step
    );
    assert!(
        (out.blink_quick_dir - ae::Vec2::new(0.0, -1.0)).length() < 1e-5,
        "quick blink is screen-relative under the default movement mode; got {:?}",
        out.blink_quick_dir
    );

    // SEAM: switch ONLY the locomotion mode to strict body-relative. Quick blink
    // now rotates with gravity (screen-up → the body's local up = screen-left at
    // this orientation); precision blink, still on the aim mode, stays screen-up.
    s.movement_frame_mode = ae::InputFrameMode::BodyRelativeStrict;
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(
        (out.blink_aim_step - ae::Vec2::new(0.0, -1.0)).length() < 1e-5,
        "precision blink stays screen-relative regardless of movement mode; got {:?}",
        out.blink_aim_step
    );
    assert!(
        (out.blink_quick_dir - ae::Vec2::new(-1.0, 0.0)).length() < 1e-5,
        "quick blink should be locomotion-framed under body-relative movement; got {:?}",
        out.blink_quick_dir
    );
}

#[test]
fn shield_and_special_pass_through() {
    let input = input_with(|c| {
        c.shield_held = true;
        // Special now comes from its OWN dedicated slot (was aliased to blink).
        c.special_pressed = true;
        c.dash_pressed = true;
        c.interact_pressed = true;
    });
    let s = BrainSnapshot::idle();
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&input, &s, &mut out);
    assert!(out.shield_held);
    assert!(out.special_pressed);
    assert!(out.dash_pressed);
    assert!(out.interact_pressed);
}

#[test]
fn blink_and_special_are_separate_actions() {
    // The retired `special_pressed = blink_pressed` alias, asserted surgically
    // and positively (review): the Special slot fires special and NOT blink;
    // the Blink slot fires blink and NOT special.
    let s = BrainSnapshot::idle();

    let special_only = input_with(|c| c.special_pressed = true);
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&special_only, &s, &mut out);
    assert!(out.special_pressed, "Special slot fires special");
    assert!(!out.blink_pressed, "Special slot does not fire blink");

    let blink_only = input_with(|c| c.blink_pressed = true);
    let mut out = crate::actor::control::ActorControlFrame::default();
    tick_player_brain_from_control(&blink_only, &s, &mut out);
    assert!(out.blink_pressed, "Blink slot fires blink");
    assert!(
        !out.special_pressed,
        "Blink slot no longer fires special (alias retired)"
    );
}

#[test]
fn tick_player_brain_no_input_emits_neutral_with_facing() {
    // The signature-matching variant doesn't have input access
    // yet (Chunk 4 plumbing); it emits neutral + facing from the
    // snapshot. Keep this test honest to that contract.
    let mut s = BrainSnapshot::idle();
    s.actor_facing = -1.0;
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.melee_pressed = true; // pre-poisoned
    tick_player_brain(PlayerSlot(0), &s, &mut out);
    assert!(!out.melee_pressed);
    assert_eq!(out.facing, -1.0);
}
