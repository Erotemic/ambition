//! Narrow neutral shell action adapter shared by startup, launcher, loading,
//! and gameplay-to-home presentation.

use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::prelude::{ButtonInput, KeyCode, Query};

const ANALOG_PRESS_THRESHOLD: f32 = 0.65;
const ANALOG_RELEASE_THRESHOLD: f32 = 0.35;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShellActionEdges {
    pub previous: bool,
    pub next: bool,
    pub confirm: bool,
    pub back: bool,
    /// Open / toggle the in-session pause menu: Escape or the controller Start
    /// button. This is the conventional pause binding; the pause menu it opens
    /// carries "Quit to Title" and "Quit to Desktop" entries, so Start no longer
    /// retires; quitting to home is a separate semantic developer action.
    pub pause: bool,
    pub startup_acknowledge: bool,
    pub loading_continue: bool,
    pub retry: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShellAnalogLatch {
    up_held: bool,
    down_held: bool,
}

impl ShellAnalogLatch {
    fn vertical_edges(&mut self, vertical: f32) -> (bool, bool) {
        if vertical.abs() <= ANALOG_RELEASE_THRESHOLD {
            self.up_held = false;
            self.down_held = false;
        }
        let up = vertical >= ANALOG_PRESS_THRESHOLD && !self.up_held;
        let down = vertical <= -ANALOG_PRESS_THRESHOLD && !self.down_held;
        if up {
            self.up_held = true;
            self.down_held = false;
        }
        if down {
            self.down_held = true;
            self.up_held = false;
        }
        (up, down)
    }
}

pub fn shell_action_edges(
    keys: Option<&ButtonInput<KeyCode>>,
    pads: &Query<&Gamepad>,
    analog: &mut ShellAnalogLatch,
) -> ShellActionEdges {
    let key = |code| keys.is_some_and(|input| input.just_pressed(code));
    let pad = |button| pads.iter().any(|gamepad| gamepad.just_pressed(button));
    let vertical = pads
        .iter()
        .map(|gamepad| gamepad.left_stick().y)
        .max_by(|lhs, rhs| lhs.abs().total_cmp(&rhs.abs()))
        .unwrap_or(0.0);
    let (analog_up, analog_down) = analog.vertical_edges(vertical);
    let previous = key(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) || analog_up;
    let next = key(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) || analog_down;
    let confirm = key(KeyCode::Enter) || key(KeyCode::Space) || pad(GamepadButton::South);
    let back = key(KeyCode::Escape) || pad(GamepadButton::East);
    ShellActionEdges {
        previous,
        next,
        confirm,
        back,
        pause: key(KeyCode::Escape) || pad(GamepadButton::Start),
        startup_acknowledge: confirm,
        loading_continue: confirm,
        retry: key(KeyCode::KeyR) || pad(GamepadButton::West),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellAnalogLatch;

    #[test]
    fn analog_navigation_is_edge_triggered_with_hysteresis() {
        let mut latch = ShellAnalogLatch::default();
        assert_eq!(latch.vertical_edges(0.64), (false, false));
        assert_eq!(latch.vertical_edges(0.70), (true, false));
        assert_eq!(latch.vertical_edges(0.90), (false, false));
        assert_eq!(latch.vertical_edges(0.20), (false, false));
        assert_eq!(latch.vertical_edges(0.80), (true, false));
        assert_eq!(latch.vertical_edges(-0.80), (false, true));
        assert_eq!(latch.vertical_edges(-0.90), (false, false));
    }
}
