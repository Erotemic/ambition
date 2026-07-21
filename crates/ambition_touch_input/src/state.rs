//! Pure touch input state types — the raw virtual-device state the Bevy
//! collect systems fill and the leafwing input kinds
//! (`crate::virtual_device`) publish through the participant's bindings.
//!
//! No Bevy resources, no plugin wiring. This module is always built
//! (regardless of the `mobile_touch` feature) so RL agents, tests,
//! and the active-build code path share the same types.

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
    #[allow(dead_code)] // Constructor reserved for the multi-frame touch tests.
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

    #[allow(dead_code)] // Constructor reserved for the multi-frame touch tests.
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
/// `SandboxAction` set on the desktop side. Cardinal direction EDGES are
/// not stored here: the `TouchStickDirection` virtual buttons hold the
/// threshold state and leafwing derives press edges from the held
/// transition, exactly as it does for a gamepad stick direction.
#[derive(Clone, Copy, Debug, Default)]
pub struct TouchInputState {
    /// Left stick raw value `[-1, 1]` (pre-deadzone), +Y down (screen space).
    pub move_x: f32,
    pub move_y: f32,
    /// Right stick raw value `[-1, 1]` (pre-deadzone).
    pub aim_x: f32,
    pub aim_y: f32,
    pub jump: TouchButton,
    pub attack: TouchButton,
    pub special: TouchButton,
    pub dash: TouchButton,
    pub blink: TouchButton,
    pub interact: TouchButton,
    pub projectile: TouchButton,
    pub fly_toggle: TouchButton,
    pub shield: TouchButton,
    /// Sustained-technique slot — held, not tapped.
    pub modifier: TouchButton,
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

#[cfg(test)]
mod touch_state_tests {
    //! The touch input seam's pure half: the radial deadzone matches the
    //! desktop helper's shape so touch and stick feel identical.
    use super::*;

    #[test]
    fn deadzone_zeros_below_and_rescales_above() {
        assert_eq!(apply_deadzone(0.05, 0.0, 0.1), (0.0, 0.0));
        // Full deflection reaches magnitude ~1 regardless of the deadzone.
        let (x, y) = apply_deadzone(1.0, 0.0, 0.2);
        assert!((x - 1.0).abs() < 1e-5 && y.abs() < 1e-5, "({x},{y})");
        // Direction preserved, magnitude between 0 and 1 in the band.
        let (x, _) = apply_deadzone(0.6, 0.0, 0.1);
        assert!(x > 0.0 && x < 1.0);
        // Zero deadzone is a pass-through.
        let (x, _) = apply_deadzone(0.5, 0.0, 0.0);
        assert!((x - 0.5).abs() < 1e-5);
    }
}
