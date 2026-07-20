//! The touch pointer-GESTURE lane and the touch active-input marker.
//!
//! Touch BUTTONS and the STICK are a virtual device resolved through the
//! participant's bindings (see [`super::virtual_device`]) — they never write
//! the semantic frames directly. What remains here is exactly what is a
//! gesture rather than a bindable control:
//!
//! - one-finger drags outside the on-screen controls fold into
//!   [`MenuControlFrame::scroll_y`], the same lane the mouse wheel uses
//!   (`populate_menu_control_frame_from_actions` adds wheel scroll; this
//!   system adds drag scroll after it);
//! - genuine touch activity marks [`ActiveInputKind::Touch`], the symmetric
//!   counterpart of the keyboard/mouse/gamepad detector.

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::bevy_plugin::{MenuTouchGestureState, MobileTouchState};
use super::exclusion::{touch_exclusion_contains, TouchExclusionZone};
use ambition_input::{ActiveInputKind, MenuControlFrame};

/// Fold non-control touch drags into menu scroll, and mark touch as the
/// active input source while a finger is genuinely driving the game.
///
/// Runs after `populate_menu_control_frame_from_actions` (which rebuilds the
/// frame from the participant's actions each frame) and before
/// `MenuNavConsume`, so the drag contribution lands in the frame the menus
/// read this frame.
#[allow(clippy::too_many_arguments)]
pub fn fold_touch_gestures(
    state: Res<MobileTouchState>,
    touches: Res<Touches>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    exclusion_zones: Query<&TouchExclusionZone>,
    mut gesture: ResMut<MenuTouchGestureState>,
    mut frame: ResMut<MenuControlFrame>,
    mut active_input: ResMut<ActiveInputKind>,
) {
    // The on-screen joystick / touch buttons are a FIRST-CLASS input source:
    // any genuine touch input this frame marks `ActiveInputKind = Touch`,
    // which keeps the mouse hover-gate from being the active source while a
    // finger drives a menu. A motionless stick + no buttons leaves the
    // marker untouched (last-writer-wins), so it does not stomp
    // keyboard/gamepad.
    let touch = state.0;
    let stick_mag = (touch.move_x * touch.move_x + touch.move_y * touch.move_y).sqrt();
    let any_button_active = [
        touch.jump,
        touch.attack,
        touch.special,
        touch.dash,
        touch.blink,
        touch.interact,
        touch.projectile,
        touch.fly_toggle,
        touch.shield,
        touch.start,
        touch.reset,
    ]
    .iter()
    .any(|button| button.held || button.pressed_this_frame);
    if stick_mag > user_settings.controls.left_stick_deadzone || any_button_active {
        active_input.mark(ActiveInputKind::Touch);
    }

    let Ok(window) = windows.single() else {
        gesture.drag_scroll.reset();
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    let touch_pos = touches
        .iter()
        .map(|touch| touch.position())
        .find(|pos| !touch_control_area_contains(*pos, window_size, &exclusion_zones));
    let mouse_pos = if mouse_buttons.pressed(MouseButton::Left) {
        window
            .cursor_position()
            .filter(|pos| !touch_control_area_contains(*pos, window_size, &exclusion_zones))
    } else {
        None
    };
    let menu_pos = touch_pos.or(mouse_pos);

    frame.scroll_y += gesture.drag_scroll.update(menu_pos, 30.0, 3.0, 5.0);
}

/// Should `pos` count as occupied by an on-screen touch control?
/// Used by the menu drag-scroll path so dragging the move stick or
/// tapping an action button doesn't accidentally trigger menu scroll.
pub(super) fn touch_control_area_contains(
    pos: Vec2,
    window_size: Vec2,
    exclusion_zones: &Query<&TouchExclusionZone>,
) -> bool {
    touch_exclusion_contains(exclusion_zones.iter(), pos, window_size)
}
