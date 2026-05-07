//! Mobile / touch input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + buttons. The Leafwing keyboard/gamepad
//! pipeline is the canonical desktop input surface; this module
//! translates touch joystick + virtual buttons into the same
//! `ControlFrame` resource the simulator already consumes.
//!
//! Two layers:
//!
//! 1. **Pure helper (this module, always built)** —
//!    `fold_touch_into_control_frame` takes a `TouchInputState` plus
//!    a deadzone and returns a `ControlFrame`. Pure data, unit-tested,
//!    no Bevy / `virtual_joystick` dep. This is what RL agents,
//!    tests, and the Bevy systems all share.
//!
//! 2. **Bevy plugin (gated behind `mobile_touch`)** — wires
//!    `virtual_joystick` Move + Aim sticks plus a small button UI to
//!    the helper, then writes `ControlFrame`. Lives in
//!    `mobile_input::bevy::*`.
//!
//! See `TODO.md` → "Android demo touch controls" for the full plan.

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
    pub const fn off() -> Self {
        Self {
            held: false,
            pressed_this_frame: false,
            released_this_frame: false,
        }
    }

    pub const fn pressed_now() -> Self {
        Self {
            held: true,
            pressed_this_frame: true,
            released_this_frame: false,
        }
    }

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

    // Edge-derived flags that the sandbox consumes alongside `axis_*`.
    // Up / Down "pressed this frame" only fires if the move stick
    // crossed the deadzone threshold this frame. We approximate by
    // looking at the stick magnitude vs threshold; the Bevy plugin
    // can replace this with a stricter edge detector if needed.
    let up_pressed = move_y < -0.5;
    let down_pressed = move_y > 0.5;

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
        aim_x,
        aim_y,
    }
}

/// Bevy plugin wiring `virtual_joystick` to the `ControlFrame` seam.
/// Gated behind the `mobile_touch` feature so desktop / gamepad /
/// headless / RL builds don't pull in `virtual_joystick` and don't
/// register the touch systems.
///
/// Today the plugin only wires the two analog sticks (Move + Aim);
/// touch buttons for Jump / Attack / Dash / Blink / Interact /
/// Projectile / Start / Reset are documented as a follow-up. RL
/// agents and tests can still produce a `TouchInputState` directly
/// and call `fold_touch_into_control_frame` from any code path.
#[cfg(feature = "mobile_touch")]
pub mod bevy_plugin {
    use super::{fold_touch_into_control_frame, TouchInputState};
    use crate::input::ControlFrame;
    use bevy::prelude::*;
    use virtual_joystick::*;

    /// Joystick id. The `virtual_joystick` plugin is generic over a
    /// user-supplied id type; this enum picks Move (left stick) and
    /// Aim (right stick).
    #[derive(Default, Debug, Reflect, Hash, Clone, PartialEq, Eq)]
    pub enum MobileStick {
        #[default]
        Move,
        Aim,
    }

    /// Live touch-input state. Updated each frame from the stick
    /// messages + button state. The folder system reads this and
    /// writes the canonical `ControlFrame`.
    #[derive(Resource, Default, Clone, Copy, Debug)]
    pub struct MobileTouchState(pub TouchInputState);

    pub struct MobileTouchPlugin;

    impl Plugin for MobileTouchPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins(VirtualJoystickPlugin::<MobileStick>::default())
                .insert_resource(MobileTouchState::default())
                .add_systems(Update, (read_joystick_messages, fold_to_control_frame).chain());
        }
    }

    /// Read every `VirtualJoystickMessage<MobileStick>` published this
    /// frame and update the `MobileTouchState`. The plugin emits a
    /// stream of axis updates per touch; we keep the latest reading
    /// per stick.
    fn read_joystick_messages(
        mut reader: MessageReader<VirtualJoystickMessage<MobileStick>>,
        mut state: ResMut<MobileTouchState>,
    ) {
        for msg in reader.read() {
            let axis = msg.snap_axis(None);
            match msg.id() {
                MobileStick::Move => {
                    state.0.move_x = axis.x;
                    // Bevy's UI Y increases UPWARD; the simulator's +Y
                    // is downward. Flip so the touch stick matches the
                    // desktop convention (drag down -> axis_y > 0).
                    state.0.move_y = -axis.y;
                }
                MobileStick::Aim => {
                    state.0.aim_x = axis.x;
                    state.0.aim_y = -axis.y;
                }
            }
        }
    }

    /// Write the latest `MobileTouchState` into `ControlFrame`. The
    /// desktop input pipeline does the same via Leafwing; both run
    /// from a presentation-side system. The presentation harness
    /// chooses which one to register based on the feature flag.
    fn fold_to_control_frame(
        state: Res<MobileTouchState>,
        mut frame: ResMut<ControlFrame>,
    ) {
        // Use slightly-tighter deadzones than the desktop defaults --
        // touch sticks rarely have drift, so a smaller deadzone gives
        // a more responsive feel.
        const MOVE_DEADZONE: f32 = 0.05;
        const AIM_DEADZONE: f32 = 0.10;
        *frame = fold_touch_into_control_frame(state.0, MOVE_DEADZONE, AIM_DEADZONE);
    }

    // Re-export the helper so `MobileTouchPlugin` is a one-import seam.
    pub use super::{fold_touch_into_control_frame as _fold_for_doc, TouchButton as _btn_for_doc, TouchInputState as _state_for_doc};
    // Suppress dead-code warnings for the re-export aliases.
    #[allow(dead_code)]
    fn _re_exports_used() {
        let _ = _fold_for_doc;
        let _ = _state_for_doc::default();
        let _ = _btn_for_doc::off();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn fold_y_threshold_fires_up_pressed() {
        // Player drags stick fully UP (move_y = -1.0 in our +Y-down
        // convention). `up_pressed` should fire.
        let mut state = TouchInputState::default();
        state.move_y = -1.0;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.up_pressed);
        assert!(!frame.down_pressed);
    }

    #[test]
    fn fold_y_threshold_fires_down_pressed() {
        let mut state = TouchInputState::default();
        state.move_y = 1.0;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.down_pressed);
        assert!(!frame.up_pressed);
    }

    #[test]
    fn fold_partial_y_does_not_fire_up_or_down_pressed() {
        // Just below the 0.5 threshold.
        let mut state = TouchInputState::default();
        state.move_y = 0.4;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(!frame.up_pressed);
        assert!(!frame.down_pressed);
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
}
