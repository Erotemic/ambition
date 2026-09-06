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
//! - genuine touch activity marks the primary seat [`ActiveDevice::Touch`], the symmetric
//!   counterpart of the keyboard/mouse/gamepad detector.

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::bevy_plugin::{MenuTouchGestureState, MobileTouchState};
use ambition_input::{ActiveDevice, MenuControlFrame, SeatActiveDevices};

/// Fold non-control touch drags into menu scroll, and mark touch as the
/// active input source while the overlay is genuinely driving the game.
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
    placement: Res<crate::placement::TouchControlPlacement>,
    mut gesture: ResMut<MenuTouchGestureState>,
    mut frame: ResMut<MenuControlFrame>,
    mut devices: ResMut<SeatActiveDevices>,
) {
    // The on-screen joystick / touch buttons are a FIRST-CLASS input source:
    // any genuine overlay input this frame marks the primary seat's device
    // `Touch`, which keeps the mouse hover-gate from being the active source
    // while a finger drives a menu. The central detector already marks raw
    // `Touches`; this covers the overlay's virtual controls, which a MOUSE
    // can drive without any finger existing. A motionless stick + no buttons
    // leaves the marker untouched (last-writer-wins), so it does not stomp
    // keyboard/gamepad.
    let touch = state.0;
    let stick_mag = (touch.move_x * touch.move_x + touch.move_y * touch.move_y).sqrt();
    let any_button_active = [
        touch.jump,
        touch.attack,
        touch.special,
        touch.burst,
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
        devices.mark_primary(ActiveDevice::Touch);
    }

    let Ok(window) = windows.single() else {
        gesture.drag_scroll.reset();
        return;
    };

    let occupied = |pos: &Vec2| touch_control_area_contains(*pos, &placement);
    let touch_pos = touches
        .iter()
        .map(|touch| touch.position())
        .find(|pos| !occupied(pos));
    let mouse_pos = if mouse_buttons.pressed(MouseButton::Left) {
        window.cursor_position().filter(|pos| !occupied(pos))
    } else {
        None
    };
    let menu_pos = touch_pos.or(mouse_pos);

    frame.scroll_y += gesture.drag_scroll.update(menu_pos, 30.0, 3.0, 5.0);
}

/// Should `pos` count as occupied by an on-screen touch control?
///
/// Used by the menu drag-scroll path so dragging the move stick or tapping an
/// action button doesn't accidentally trigger menu scroll. Reads the resolved
/// placement, so it follows the controls wherever they were actually put.
pub(super) fn touch_control_area_contains(
    pos: Vec2,
    placement: &crate::placement::TouchControlPlacement,
) -> bool {
    [
        placement.movement,
        placement.action_bezel,
        placement.menu_row,
    ]
    .into_iter()
    .flatten()
    .any(|rect| rect.contains(pos))
}
