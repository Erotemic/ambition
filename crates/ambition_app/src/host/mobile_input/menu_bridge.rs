//! Bridge touch / mouse / joystick input into both the gameplay
//! `ControlFrame` and the menu-side `MenuControlFrame`.
//!
//! Two systems live here:
//!
//! - [`fold_to_control_frame`] — gameplay merge (axis exclusive,
//!   buttons OR-merge, activity-gated against keyboard).
//! - [`fold_to_menu_control_frame`] — menu/dialog merge (touch
//!   buttons + analog stick + drag-scroll).
//!
//! The systems read [`super::bevy_plugin::MobileTouchState`] (a
//! Bevy `Resource` wrapping the pure [`super::state::TouchInputState`])
//! plus [`super::bevy_plugin::TouchControlsVisible`] /
//! [`super::bevy_plugin::MenuTouchGestureState`], and write
//! [`ambition_sandbox::input::ControlFrame`] / [`ambition_sandbox::input::MenuControlFrame`].
//! They are scheduled by [`super::bevy_plugin::TouchControlsPlugin`].

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::bevy_plugin::{MenuTouchGestureState, MobileTouchState};
use super::layout::{touch_action_at_position, TOUCH_SCALE};
use super::state::{fold_touch_into_control_frame, touch_state_is_active, TouchInputState};
use ambition_sandbox::input::{ControlFrame, MenuControlFrame, MenuInputState};

/// Merge the latest [`MobileTouchState`] into gameplay
/// [`ControlFrame`]. The desktop input pipeline (Leafwing) writes its
/// own version of the frame upstream; this system MERGES rather than
/// replaces:
///
/// - **Movement axis** is mutually exclusive between keyboard and
///   touch. If the touch stick is past its deadzone, touch wins
///   (keyboard's axis is overwritten). Otherwise the keyboard
///   contribution is preserved. This matches Jon's "disable the
///   touch dpad when I'm using the keyboard arrows, and disable the
///   keyboard arrows when I'm using the touch dpad" intent.
/// - **Action buttons** OR-merge. A held touch button OR a held
///   keyboard button counts as held. Edge flags are similarly
///   merged so a touch tap + keyboard tap on the same frame both
///   register. Per Jon's "the held/release buttons for actions I
///   think should be independent."
///
/// When the touch UI is hidden, inactive, or the game is in a UI
/// mode, the merge is a no-op so the keyboard-derived/suppressed
/// frame passes through unchanged. UI modes consume touch
/// stick/button intent via [`fold_to_menu_control_frame`] instead.
pub fn fold_to_control_frame(
    mode: Res<State<ambition_sandbox::game_mode::GameMode>>,
    cutscene: Res<ambition_sandbox::presentation::cutscene::ActiveCutscene>,
    state: Res<MobileTouchState>,
    mut frame: ResMut<ControlFrame>,
) {
    // Touch input is always live while the plugin is installed (enablement is
    // plugin-gated, not a runtime boolean). The `touch_controls_visible` setting
    // only hides/shows the on-screen overlay (see `sync_touch_ui_visibility`); it
    // no longer gates the input fold. The activity gate below (`touch_state_is_active`)
    // already keeps an untouched overlay from stomping keyboard input.
    if !mode.get().allows_gameplay() {
        return;
    }
    // Cutscenes don't change GameMode (they overlay `Playing`), so the
    // mode gate above doesn't catch them. Without this check, touch
    // joystick + buttons would steer the character through a scripted
    // beat even though `populate_control_frame_from_actions` already
    // zeroed the keyboard-derived gameplay frame for the cutscene.
    // Cutscene advance/skip from touch lives on the menu frame via
    // `apply_menu_frame_to_cutscene_request`.
    if cutscene.is_playing() {
        return;
    }
    if !touch_state_is_active(&state.0) {
        return;
    }
    const MOVE_DEADZONE: f32 = 0.05;
    const AIM_DEADZONE: f32 = 0.10;
    let touch_frame = fold_touch_into_control_frame(state.0, MOVE_DEADZONE, AIM_DEADZONE);
    // Mutually-exclusive axis: touch wins iff its post-deadzone
    // magnitude beats threshold 0.05. Otherwise leave keyboard
    // axis alone.
    let touch_move_mag =
        (touch_frame.axis_x * touch_frame.axis_x + touch_frame.axis_y * touch_frame.axis_y).sqrt();
    if touch_move_mag > 0.05 {
        frame.axis_x = touch_frame.axis_x;
        frame.axis_y = touch_frame.axis_y;
        // Also forward the up/down edge flags from touch, since
        // an axis source switch can be the gesture that fires
        // a Door tap or ladder entry.
        frame.up_pressed = frame.up_pressed || touch_frame.up_pressed;
        frame.down_pressed = frame.down_pressed || touch_frame.down_pressed;
    }
    let touch_aim_mag =
        (touch_frame.aim_x * touch_frame.aim_x + touch_frame.aim_y * touch_frame.aim_y).sqrt();
    if touch_aim_mag > 0.10 {
        frame.aim_x = touch_frame.aim_x;
        frame.aim_y = touch_frame.aim_y;
    }
    // OR-merge action buttons. A keyboard JUMP plus a touch
    // JUMP on the same frame should still register as a single
    // press.
    frame.jump_pressed |= touch_frame.jump_pressed;
    frame.jump_held |= touch_frame.jump_held;
    frame.jump_released |= touch_frame.jump_released;
    frame.dash_pressed |= touch_frame.dash_pressed;
    frame.attack_pressed |= touch_frame.attack_pressed;
    frame.blink_pressed |= touch_frame.blink_pressed;
    frame.blink_held |= touch_frame.blink_held;
    frame.blink_released |= touch_frame.blink_released;
    frame.interact_pressed |= touch_frame.interact_pressed;
    frame.projectile_pressed |= touch_frame.projectile_pressed;
    frame.projectile_held |= touch_frame.projectile_held;
    frame.projectile_released |= touch_frame.projectile_released;
    frame.fly_toggle_pressed |= touch_frame.fly_toggle_pressed;
    frame.shield_held |= touch_frame.shield_held;
    frame.reset_pressed |= touch_frame.reset_pressed;
    frame.start_pressed |= touch_frame.start_pressed;
    frame.pogo_pressed |= touch_frame.pogo_pressed;
}

/// Merge touch buttons, the touch stick in UI modes, and non-control
/// drag gestures into the semantic menu frame.
///
/// This is intentionally separate from [`fold_to_control_frame`]:
/// gameplay axes and UI gestures have different consumers. The touch
/// Start button toggles pause, Reset acts as Back, Jump/Interact can
/// confirm, and the move stick becomes the same repeated up/down/
/// left/right intent as keyboard arrows while a dialog or pause
/// menu is active. One-finger drags outside the fixed touch-control
/// regions still map to menu scroll/navigation, and the same drag
/// path accepts a pressed left mouse button for desktop testing.
#[allow(clippy::too_many_arguments)]
pub fn fold_to_menu_control_frame(
    time: Res<Time>,
    mode: Res<State<ambition_sandbox::game_mode::GameMode>>,
    state: Res<MobileTouchState>,
    touches: Res<Touches>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    user_settings: Res<ambition_sandbox::persistence::settings::UserSettings>,
    mut gesture: ResMut<MenuTouchGestureState>,
    mut frame: ResMut<MenuControlFrame>,
    mut active_input: ResMut<ambition_sandbox::input::ActiveInputKind>,
) {
    // Touch menu input is always live while the plugin is installed; the
    // `touch_controls_visible` setting only hides the overlay, not the input.
    let touch = state.0;
    frame.start |= touch.start.pressed_this_frame;
    frame.back |= touch.reset.pressed_this_frame;
    frame.back_held |= touch.reset.held;
    frame.select |= touch.jump.pressed_this_frame || touch.interact.pressed_this_frame;
    frame.select_held |= touch.jump.held || touch.interact.held;

    // The on-screen joystick / touch buttons are a FIRST-CLASS menu nav source:
    // any genuine touch input this frame marks `ActiveInputKind = Touch`. This
    // keeps the mouse hover-gate from being the active source while a finger is
    // driving the menu, and is the symmetric counterpart to the keyboard /
    // mouse / gamepad detector. A motionless stick + no buttons leaves the
    // marker untouched (last-writer-wins), so it does not stomp keyboard/gamepad.
    let stick_mag = (touch.move_x * touch.move_x + touch.move_y * touch.move_y).sqrt();
    let touch_button_active = touch.start.pressed_this_frame
        || touch.reset.pressed_this_frame
        || touch.reset.held
        || touch.jump.pressed_this_frame
        || touch.jump.held
        || touch.interact.pressed_this_frame
        || touch.interact.held;
    if stick_mag > user_settings.controls.left_stick_deadzone || touch_button_active {
        active_input.mark(ambition_sandbox::input::ActiveInputKind::Touch);
    }

    if menu_move_active(*mode.get()) {
        let analog_dir = touch_move_to_menu_dir(touch, user_settings.controls.left_stick_deadzone);
        let input = gesture.stick_input.step(
            false,
            false,
            false,
            false,
            analog_dir,
            false,
            false,
            false,
            time.delta_secs(),
            user_settings.controls.menu_repeat_initial_delay,
            user_settings.controls.menu_repeat_interval,
        );
        let stick_frame = MenuControlFrame::from_menu_input(input);
        frame.up |= stick_frame.up;
        frame.down |= stick_frame.down;
        frame.left |= stick_frame.left;
        frame.right |= stick_frame.right;
    } else {
        gesture.stick_input = MenuInputState::default();
    }

    let Ok(window) = windows.single() else {
        gesture.drag_scroll.reset();
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    let touch_pos = touches
        .iter()
        .map(|touch| touch.position())
        .find(|pos| !touch_control_area_contains(*pos, window_size));
    let mouse_pos = if mouse_buttons.pressed(MouseButton::Left) {
        window
            .cursor_position()
            .filter(|pos| !touch_control_area_contains(*pos, window_size))
    } else {
        None
    };
    let menu_pos = touch_pos.or(mouse_pos);

    frame.scroll_y += gesture.drag_scroll.update(menu_pos, 30.0, 3.0, 5.0);
}

/// Is a menu open such that the touch joystick should fold into menu
/// Up/Down/Left/Right navigation?
///
/// True in every mode that opens a menu over the sim: `Paused` (the
/// pause menu, the bevy_ui inventory GRID, AND the 3D kaleidoscope
/// CUBE all live in `Paused`) and `Dialogue`. The kaleidoscope cube
/// has no separate game mode — it opens in `Paused` exactly like the
/// grid — so keying on `Paused` here is what lets the on-screen
/// joystick drive the cube's cursor the same way it drives the grid.
pub fn menu_move_active(mode: ambition_sandbox::game_mode::GameMode) -> bool {
    matches!(
        mode,
        ambition_sandbox::game_mode::GameMode::Dialogue
            | ambition_sandbox::game_mode::GameMode::Paused
    )
}

/// Convert the touch move stick into a `MenuDir` for menu navigation.
/// Touch/gameplay stores +Y as down, while the menu analog helper
/// expects +Y as up to match gamepad/keyboard menu convention. Flip
/// here so dragging the visible joystick down selects the next
/// dialog option.
pub fn touch_move_to_menu_dir(
    touch: TouchInputState,
    deadzone: f32,
) -> Option<ambition_sandbox::input::MenuDir> {
    let (x, y_down) = ambition_sandbox::persistence::settings::ControlSettings::apply_deadzone(
        touch.move_x,
        touch.move_y,
        deadzone,
    );
    ambition_sandbox::input::analog_to_dir(x, -y_down, 0.5)
}

/// Should `pos` count as occupied by an on-screen touch control?
/// Used by the menu drag-scroll path so dragging the move stick or
/// tapping an action button doesn't accidentally trigger menu scroll.
pub(super) fn touch_control_area_contains(pos: Vec2, window_size: Vec2) -> bool {
    if touch_action_at_position(pos, window_size).is_some() {
        return true;
    }
    // Approximate virtual joystick footprint in the lower-left corner.
    // The exact nodes are owned by `virtual_joystick`, so a geometric
    // exclusion is the least-coupled way to avoid treating
    // movement-stick drags as menu scroll gestures. The 300px envelope
    // matches the original 1.0-scale stick (120px base + 24px margin +
    // generous slop) and shrinks with `TOUCH_SCALE` so the smaller
    // stick doesn't reserve a too-large dead zone for menu drags.
    let stick_envelope = 300.0 * TOUCH_SCALE;
    pos.x <= stick_envelope && pos.y >= window_size.y - stick_envelope
}
