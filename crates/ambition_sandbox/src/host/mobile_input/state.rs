//! Pure touch input state types and the `TouchInputState ->
//! ControlFrame` fold helper.
//!
//! No Bevy resources, no plugin wiring. This module is always built
//! (regardless of the `mobile_touch` feature) so RL agents, tests,
//! and the active-build code path share the same types.

use crate::input::ControlFrame;

/// Edge-vs-held button state. Two flags per button so the sim's
/// "pressed this frame" semantics survive the touch path. The Bevy
/// systems compute these by diffing per-frame against the last
/// frame's pressed mask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TouchButton {
    /// True if the button is currently held.
    pub held: bool,
    /// True if the button was newly pressed this frame.
    pub pressed_this_frame: bool,
    /// True if the button was released this frame.
    pub released_this_frame: bool,
}

impl TouchButton {
    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn off() -> Self {
        Self {
            held: false,
            pressed_this_frame: false,
            released_this_frame: false,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn pressed_now() -> Self {
        Self {
            held: true,
            pressed_this_frame: true,
            released_this_frame: false,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn held_continued() -> Self {
        Self {
            held: true,
            pressed_this_frame: false,
            released_this_frame: false,
        }
    }
}

/// One frame of mobile-touch input: two analog sticks (Move + Aim) plus
/// the gameplay-relevant action buttons. Mirrors the
/// `SandboxAction` set on the desktop side.
#[derive(Clone, Copy, Debug, Default)]
pub struct TouchInputState {
    /// Left stick raw value `[-1, 1]` (pre-deadzone).
    pub move_x: f32,
    pub move_y: f32,
    /// Edge flags: true on the frame the move stick crossed the
    /// up/down threshold (in either direction). The Bevy plugin
    /// computes these by diffing against the previous frame's
    /// `move_y`; tests / RL agents can set them directly. Auto-
    /// deriving from `move_y > 0.5` per frame is incorrect because
    /// `register_down_tap` would count every held frame as a
    /// fresh tap and trigger MorphBall on the second frame.
    pub move_y_just_crossed_up: bool,
    pub move_y_just_crossed_down: bool,
    /// Right stick raw value `[-1, 1]` (pre-deadzone).
    pub aim_x: f32,
    pub aim_y: f32,
    pub jump: TouchButton,
    pub attack: TouchButton,
    pub dash: TouchButton,
    pub blink: TouchButton,
    pub interact: TouchButton,
    pub projectile: TouchButton,
    pub fly_toggle: TouchButton,
    pub shield: TouchButton,
    pub start: TouchButton,
    pub reset: TouchButton,
}

/// Apply a circular deadzone to an analog stick reading. Mirrors the
/// `ControlSettings::apply_deadzone` shape from the desktop input
/// pipeline so touch and stick feel identical at the seam.
pub fn apply_deadzone(x: f32, y: f32, deadzone: f32) -> (f32, f32) {
    let mag = (x * x + y * y).sqrt();
    if mag <= deadzone {
        return (0.0, 0.0);
    }
    // Re-scale so the post-deadzone magnitude reaches 1.0 at full
    // stick deflection rather than a clipped (1 - deadzone). Same
    // approach as the desktop deadzone helper.
    let scaled = (mag - deadzone) / (1.0 - deadzone).max(1e-6);
    let scaled = scaled.clamp(0.0, 1.0);
    let inv_mag = if mag > 1e-6 { 1.0 / mag } else { 0.0 };
    (x * inv_mag * scaled, y * inv_mag * scaled)
}

/// Fold a `TouchInputState` into the engine's `ControlFrame` shape.
///
/// `move_deadzone` and `aim_deadzone` are the per-stick deadzone
/// magnitudes; the desktop pipeline's `ControlSettings` holds the
/// canonical values, but the touch path can pick its own (touch
/// sticks usually have no drift, so a smaller deadzone like 0.05 is
/// enough). Pass 0.0 to disable.
///
/// Pure function — no Bevy / world / globals — so tests can pin every
/// edge case (sign convention, deadzone, button semantics) without
/// touching the rest of the engine.
pub fn fold_touch_into_control_frame(
    state: TouchInputState,
    move_deadzone: f32,
    aim_deadzone: f32,
) -> ControlFrame {
    let (move_x, move_y_raw) = apply_deadzone(state.move_x, state.move_y, move_deadzone);
    let (aim_x, aim_y_raw) = apply_deadzone(state.aim_x, state.aim_y, aim_deadzone);
    // The simulation's +Y is downward (screen-space). Touch joysticks
    // typically follow the same convention if mapped to "drag down =
    // axis_y > 0". Caller is responsible for matching that
    // convention before this function; we don't flip here.
    let move_y = move_y_raw;
    let aim_y = aim_y_raw;

    // Up / Down edge flags come from the caller explicitly (set on
    // the frame the move-Y axis crosses the threshold, cleared
    // next frame). Auto-deriving from "move_y > 0.5" every frame
    // breaks register_down_tap which counts each consecutive
    // true as a fresh tap and double-taps into MorphBall after one
    // held frame -- the same bug class as the AgentAction
    // converter; same fix.
    let up_pressed = state.move_y_just_crossed_up;
    let down_pressed = state.move_y_just_crossed_down;

    ControlFrame {
        axis_x: move_x,
        axis_y: move_y,
        jump_pressed: state.jump.pressed_this_frame,
        jump_held: state.jump.held,
        jump_released: state.jump.released_this_frame,
        dash_pressed: state.dash.pressed_this_frame,
        up_pressed,
        down_pressed,
        fast_fall_pressed: false,
        blink_pressed: state.blink.pressed_this_frame,
        blink_held: state.blink.held,
        blink_released: state.blink.released_this_frame,
        attack_pressed: state.attack.pressed_this_frame,
        pogo_pressed: false,
        fly_toggle_pressed: state.fly_toggle.pressed_this_frame,
        interact_pressed: state.interact.pressed_this_frame,
        reset_pressed: state.reset.pressed_this_frame,
        start_pressed: state.start.pressed_this_frame,
        projectile_pressed: state.projectile.pressed_this_frame,
        projectile_held: state.projectile.held,
        projectile_released: state.projectile.released_this_frame,
        shield_held: state.shield.held,
        aim_x,
        aim_y,
    }
}

/// True if any touch input field has a non-default value. Used to
/// gate the merge so an empty touch state doesn't stomp the
/// keyboard-derived ControlFrame every frame.
///
/// Includes `released_this_frame` flags: without them, the frame
/// after a button release would skip the merge and the release edge
/// would never reach the simulator. Concrete repro: tapping
/// Projectile with a mouse charged the fireball (frame N: pressed)
/// but never released it (frame N+1: held=false, pressed=false,
/// released=true → activity gate skipped the merge without this
/// clause).
pub(crate) fn touch_state_is_active(state: &TouchInputState) -> bool {
    let stick_active = state.move_x.abs() > 1e-3
        || state.move_y.abs() > 1e-3
        || state.aim_x.abs() > 1e-3
        || state.aim_y.abs() > 1e-3;
    let any_button = state.jump.held
        || state.attack.held
        || state.dash.held
        || state.blink.held
        || state.interact.held
        || state.projectile.held
        || state.fly_toggle.held
        || state.shield.held
        || state.start.held
        || state.reset.held;
    let any_edge = state.jump.pressed_this_frame
        || state.attack.pressed_this_frame
        || state.dash.pressed_this_frame
        || state.blink.pressed_this_frame
        || state.interact.pressed_this_frame
        || state.projectile.pressed_this_frame
        || state.fly_toggle.pressed_this_frame
        || state.shield.pressed_this_frame
        || state.start.pressed_this_frame
        || state.reset.pressed_this_frame
        || state.move_y_just_crossed_up
        || state.move_y_just_crossed_down;
    let any_release = state.jump.released_this_frame
        || state.attack.released_this_frame
        || state.dash.released_this_frame
        || state.blink.released_this_frame
        || state.interact.released_this_frame
        || state.projectile.released_this_frame
        || state.fly_toggle.released_this_frame
        || state.shield.released_this_frame
        || state.start.released_this_frame
        || state.reset.released_this_frame;
    stick_active || any_button || any_edge || any_release
}
