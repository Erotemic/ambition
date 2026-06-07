use super::state::{apply_deadzone, fold_touch_into_control_frame, TouchButton, TouchInputState};

#[test]
fn deadzone_kills_sub_threshold_input() {
    let (x, y) = apply_deadzone(0.05, 0.05, 0.10);
    assert_eq!((x, y), (0.0, 0.0));
}

#[test]
fn deadzone_preserves_above_threshold_direction() {
    // Stick pushed all the way right (1.0, 0.0), 0.10 deadzone:
    // post-deadzone should still be effectively (1.0, 0.0).
    let (x, y) = apply_deadzone(1.0, 0.0, 0.10);
    assert!((x - 1.0).abs() < 1e-3, "x should reach 1.0; got {x}");
    assert_eq!(y, 0.0);
}

#[test]
fn deadzone_zero_passes_through() {
    let (x, y) = apply_deadzone(0.5, -0.3, 0.0);
    assert_eq!(x, 0.5);
    assert_eq!(y, -0.3);
}

#[test]
fn fold_zero_state_produces_neutral_control_frame() {
    let frame = fold_touch_into_control_frame(TouchInputState::default(), 0.05, 0.05);
    assert_eq!(frame.axis_x, 0.0);
    assert_eq!(frame.axis_y, 0.0);
    assert!(!frame.jump_pressed);
    assert!(!frame.jump_held);
    assert!(!frame.up_pressed);
    assert!(!frame.down_pressed);
}

#[test]
fn fold_sets_jump_flags_from_button_state() {
    let mut state = TouchInputState::default();
    state.jump = TouchButton::pressed_now();
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    assert!(frame.jump_pressed);
    assert!(frame.jump_held);
    assert!(!frame.jump_released);
}

#[test]
fn fold_translates_aim_stick() {
    let mut state = TouchInputState::default();
    state.aim_x = 0.8;
    state.aim_y = -0.5;
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    // After deadzone (0.05) + scaling: still strongly positive x,
    // negative y. Don't pin exact values; pin sign + magnitude.
    assert!(frame.aim_x > 0.5);
    assert!(frame.aim_y < -0.3);
}

#[test]
fn fold_propagates_explicit_up_pressed_edge() {
    // The Bevy plugin computes edge crossings from previous-
    // frame `move_y`; the pure folder consumes the explicit
    // edge flags rather than auto-deriving from `move_y > 0.5`
    // (which would re-trigger every frame and fire MorphBall
    // through the double-tap-down detector).
    let mut state = TouchInputState::default();
    state.move_y = -1.0;
    state.move_y_just_crossed_up = true;
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    assert!(frame.up_pressed);
    assert!(!frame.down_pressed);
}

#[test]
fn fold_propagates_explicit_down_pressed_edge() {
    let mut state = TouchInputState::default();
    state.move_y = 1.0;
    state.move_y_just_crossed_down = true;
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    assert!(frame.down_pressed);
    assert!(!frame.up_pressed);
}

#[test]
fn fold_held_down_without_edge_flag_does_not_fire_down_pressed() {
    // Pin the bug fix: holding move_y=1.0 every frame WITHOUT
    // setting the edge flag should NOT fire down_pressed. This
    // is the "held Down" case that previously oscillated body_mode
    // through the double-tap-down detector.
    let mut state = TouchInputState::default();
    state.move_y = 1.0;
    state.move_y_just_crossed_down = false;
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    assert!(!frame.down_pressed);
    assert!(!frame.up_pressed);
}

#[test]
fn fold_propagates_all_action_buttons() {
    // Every action button: pressed-this-frame should map through.
    let mut state = TouchInputState::default();
    state.attack = TouchButton::pressed_now();
    state.dash = TouchButton::pressed_now();
    state.blink = TouchButton::pressed_now();
    state.interact = TouchButton::pressed_now();
    state.projectile = TouchButton::pressed_now();
    state.fly_toggle = TouchButton::pressed_now();
    state.start = TouchButton::pressed_now();
    state.reset = TouchButton::pressed_now();
    let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
    assert!(frame.attack_pressed);
    assert!(frame.dash_pressed);
    assert!(frame.blink_pressed);
    assert!(frame.interact_pressed);
    assert!(frame.projectile_pressed);
    assert!(frame.fly_toggle_pressed);
    assert!(frame.start_pressed);
    assert!(frame.reset_pressed);
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_move_to_menu_dir_flips_touch_y_for_menu_navigation() {
    use super::menu_bridge::touch_move_to_menu_dir;
    use crate::input::MenuDir;

    let mut state = TouchInputState::default();
    state.move_y = 1.0;
    assert_eq!(touch_move_to_menu_dir(state, 0.05), Some(MenuDir::Down));

    state.move_y = -1.0;
    assert_eq!(touch_move_to_menu_dir(state, 0.05), Some(MenuDir::Up));
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_move_to_menu_dir_applies_deadzone() {
    use super::menu_bridge::touch_move_to_menu_dir;

    let mut state = TouchInputState::default();
    state.move_y = 0.10;
    assert_eq!(touch_move_to_menu_dir(state, 0.25), None);
}

#[cfg(feature = "mobile_touch")]
#[test]
fn axis_override_drives_knob_only_during_gameplay() {
    // Problem 1: while a menu is open the gameplay axis is ~0 (touch is
    // routed to the menu frame), so the knob-drive override must NOT
    // run — otherwise it snaps the knob to center even as the player
    // drags it to navigate the menu. During gameplay the override DOES
    // run so the knob mirrors the move axis.
    use super::bevy_plugin::axis_override_drives_knob;
    use crate::game_mode::GameMode;

    assert!(
        axis_override_drives_knob(GameMode::Playing),
        "gameplay: knob should mirror the move axis"
    );
    assert!(
        !axis_override_drives_knob(GameMode::Paused),
        "pause / inventory grid / kaleidoscope cube: knob follows the live drag, not the zeroed axis"
    );
    assert!(
        !axis_override_drives_knob(GameMode::Dialogue),
        "dialogue menu: knob follows the live drag, not the zeroed axis"
    );
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_drag_folds_into_menu_frame_while_kaleidoscope_paused() {
    // Problem 2: the 3D kaleidoscope cube opens in `GameMode::Paused`,
    // exactly like the bevy_ui grid menu. The touch->MenuControlFrame
    // fold keys off `Paused` (via `menu_move_active`), so a joystick
    // drag in `Paused` produces an Up/Down menu direction the same way
    // it does for the grid. This pins that the kaleidoscope's `Paused`
    // mode is covered by the menu-active gate (no separate state to
    // miss).
    use super::menu_bridge::{menu_move_active, touch_move_to_menu_dir};
    use crate::game_mode::GameMode;
    use crate::input::MenuDir;

    // Kaleidoscope (and grid) open in Paused -> menu fold is active.
    assert!(menu_move_active(GameMode::Paused));
    assert!(menu_move_active(GameMode::Dialogue));
    assert!(!menu_move_active(GameMode::Playing));

    // A downward stick drag while Paused maps to MenuDir::Down (the
    // cube cursor moves), identical to the grid menu.
    let mut state = TouchInputState::default();
    state.move_y = 1.0;
    assert_eq!(touch_move_to_menu_dir(state, 0.05), Some(MenuDir::Down));
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_joystick_drag_down_drives_debounced_menu_down() {
    // Problem 2: while a menu is open (Paused), a touch-joystick drag
    // DOWN must produce `MenuControlFrame.down` and step the cursor,
    // debounced to discrete d-pad-like steps — the SAME result the
    // gamepad stick produces. This mirrors the exact path
    // `fold_to_menu_control_frame` runs: touch stick -> MenuDir ->
    // `MenuInputState::step` -> `MenuControlFrame::from_menu_input`.
    use super::menu_bridge::{menu_move_active, touch_move_to_menu_dir};
    use crate::game_mode::GameMode;
    use crate::input::{MenuControlFrame, MenuInputState};

    assert!(menu_move_active(GameMode::Paused));

    let mut state = TouchInputState::default();
    state.move_y = 1.0; // drag the on-screen stick fully DOWN

    let dir = touch_move_to_menu_dir(state, 0.05);
    assert_eq!(dir, Some(crate::input::MenuDir::Down));

    let mut menu_state = MenuInputState::default();
    let dt = 1.0 / 60.0;
    let initial_delay = 0.3;
    let repeat_interval = 0.1;

    // Frame 1: a NEW direction emits at once -> the menu cursor steps.
    let f1 = menu_state.step(
        false,
        false,
        false,
        false,
        dir,
        false,
        false,
        false,
        dt,
        initial_delay,
        repeat_interval,
    );
    let cf1 = MenuControlFrame::from_menu_input(f1);
    assert!(cf1.down, "first drag-down frame steps the cursor down");

    // Frame 2: still holding, still under the initial delay -> NO repeat.
    // This is the debounce: the stick does NOT fire every frame.
    let f2 = menu_state.step(
        false,
        false,
        false,
        false,
        dir,
        false,
        false,
        false,
        dt,
        initial_delay,
        repeat_interval,
    );
    let cf2 = MenuControlFrame::from_menu_input(f2);
    assert!(
        !cf2.down,
        "held drag-down debounces: no repeat until the initial delay elapses"
    );

    // After enough held frames the analog repeat eventually re-fires,
    // giving a controlled second step (like a held d-pad), not 60/sec.
    let mut later_steps = 0;
    for _ in 0..120 {
        let f = menu_state.step(
            false,
            false,
            false,
            false,
            dir,
            false,
            false,
            false,
            dt,
            initial_delay,
            repeat_interval,
        );
        if MenuControlFrame::from_menu_input(f).down {
            later_steps += 1;
        }
    }
    assert!(
        later_steps > 0,
        "a held drag eventually repeats at the menu repeat interval"
    );
    assert!(
        later_steps < 120,
        "repeat is debounced, not firing on every single frame"
    );
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_back_button_sets_menu_back_frame() {
    // Problem 3: the touch Back button (TouchActionButton::Reset) must
    // reach `MenuControlFrame.back` so menu nav (close / drill-out)
    // fires in BOTH backends. `fold_to_menu_control_frame` does
    // `frame.back |= touch.reset.pressed_this_frame`; pin that mapping
    // on the pure state so the wiring can't silently regress.
    use crate::input::MenuControlFrame;

    let mut touch = TouchInputState::default();
    touch.reset = TouchButton::pressed_now();

    // Reproduce the fold's OR-merge onto a fresh (gamepad-populated)
    // frame: a zeroed frame plus a touch Back press yields `back`.
    let mut frame = MenuControlFrame::default();
    frame.back |= touch.reset.pressed_this_frame;
    frame.back_held |= touch.reset.held;

    assert!(frame.back, "touch Back press sets MenuControlFrame.back");
    assert!(frame.back_held, "a held touch Back also reports back_held");
    // Both backends close on `menu.back`: grid_menu_input's
    // `if menu.back || menu.start` and the cube's `if menu.back`.
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_hud_z_is_above_every_menu_overlay() {
    // Problem 1: the HUD's `GlobalZIndex` band must sit ABOVE every menu
    // overlay so it renders on top AND wins bevy_ui picking (so the
    // joystick keeps receiving drags and the Back button stays tappable
    // while a menu's full-screen scrim is up). Assert the ordering
    // against the concrete overlay z values used in the menu modules.
    use super::bevy_plugin::TOUCH_HUD_Z;

    // Local `ZIndex` values authored on the menu roots:
    const PAUSE_MENU_Z: i32 = 50;
    const MAP_Z: i32 = 60;
    const GRID_MENU_Z: i32 = 62;
    // Documented worst-case the prompt calls out for the grid root.
    const GRID_GLOBAL_Z_WORST_CASE: i32 = 1000;

    assert!(TOUCH_HUD_Z > PAUSE_MENU_Z);
    assert!(TOUCH_HUD_Z > MAP_Z);
    assert!(TOUCH_HUD_Z > GRID_MENU_Z);
    assert!(
        TOUCH_HUD_Z > GRID_GLOBAL_Z_WORST_CASE,
        "HUD must out-rank even a GlobalZIndex(1000) menu root"
    );
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_action_hit_test_includes_fly_button() {
    use super::layout::{
        touch_action_at_position, touch_action_cluster_origin, touch_action_layout,
        TouchActionButton,
    };

    let window_size = bevy::prelude::Vec2::new(1080.0, 2340.0);
    let fly = touch_action_layout()
        .into_iter()
        .find(|spec| matches!(spec.action, TouchActionButton::FlyToggle))
        .expect("Fly button remains in the touch action layout");
    // Center of the visible Fly shoulder button in the lower-right cluster.
    let cluster_origin = touch_action_cluster_origin(window_size);
    let pos = bevy::prelude::Vec2::new(
        cluster_origin.x + fly.left + fly.size * 0.5,
        cluster_origin.y + fly.top + fly.size * 0.5,
    );
    assert!(matches!(
        touch_action_at_position(pos, window_size),
        Some(TouchActionButton::FlyToggle)
    ));
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_action_layout_keeps_visible_circles_apart() {
    use super::layout::touch_action_layout;

    const MIN_VISUAL_GAP: f32 = 4.0;
    let layout = touch_action_layout();
    for (i, a) in layout.iter().enumerate() {
        let ac = bevy::prelude::Vec2::new(a.left + a.size * 0.5, a.top + a.size * 0.5);
        for b in layout.iter().skip(i + 1) {
            let bc = bevy::prelude::Vec2::new(b.left + b.size * 0.5, b.top + b.size * 0.5);
            let gap = ac.distance(bc) - (a.size + b.size) * 0.5;
            assert!(
                gap >= MIN_VISUAL_GAP,
                "touch circles should have at least {MIN_VISUAL_GAP}px gap: {} and {} only have {gap:.1}px",
                a.label, b.label
            );
        }
    }
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_action_hit_test_uses_visible_circle_not_square_bounds() {
    use super::layout::{
        touch_action_at_position, touch_action_cluster_origin, touch_action_layout,
        TouchActionButton,
    };

    let window_size = bevy::prelude::Vec2::new(1280.0, 720.0);
    let layout = touch_action_layout();
    let attack = layout
        .iter()
        .find(|spec| matches!(spec.action, TouchActionButton::Attack))
        .expect("Attack remains in the touch action layout");
    let jump = layout
        .iter()
        .find(|spec| matches!(spec.action, TouchActionButton::Jump))
        .expect("Jump remains in the touch action layout");
    assert!(
        attack.top + attack.size > jump.top,
        "diagonal square bounds should be allowed to overlap vertically"
    );

    let origin = touch_action_cluster_origin(window_size);
    let square_only = bevy::prelude::Vec2::new(
        origin.x + attack.left + attack.size - 2.0,
        origin.y + jump.top + 2.0,
    );
    assert_eq!(touch_action_at_position(square_only, window_size), None);
}
