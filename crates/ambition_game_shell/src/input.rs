//! Narrow neutral shell action adapter shared by startup, launcher, loading,
//! and gameplay-to-home presentation.
//!
//! Devices reach the shell through TWO sources, folded here into one edge set:
//!
//! 1. Raw keyboard + gamepad, read directly. The shell must work before any
//!    session exists (startup cards, launcher, loading), where no player entity
//!    carries a leafwing `ActionState` and the semantic seam below is therefore
//!    empty.
//! 2. [`MenuControlFrame`], the engine's device-agnostic menu intent. Touch,
//!    on-screen buttons, mouse wheel, and (eventually) Android system-back all
//!    fold into it — see `ambition_touch_input::fold_to_menu_control_frame`.
//!
//! Source 2 is what makes the shell reachable from a phone. It is OPTIONAL: an
//! app composing `MinimalShellPlugins` without a host input stack simply has no
//! such resource, and the shell stays keyboard/gamepad-driven.
//!
//! Both sources carry one-frame edges and this adapter samples once per frame,
//! so it does not matter whether the fold runs before or after a shell system —
//! a press produces exactly one edge either way, at worst a frame late. That
//! order-independence is deliberate: it keeps the shell from having to name a
//! schedule set owned by a crate above it.

use ambition_input::MenuControlFrame;
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
    menu: Option<&MenuControlFrame>,
    analog: &mut ShellAnalogLatch,
) -> ShellActionEdges {
    let key = |code| keys.is_some_and(|input| input.just_pressed(code));
    let pad = |button| pads.iter().any(|gamepad| gamepad.just_pressed(button));
    // An absent menu frame is the neutral element, so every OR below is a no-op
    // for apps that do not compose a host input stack.
    let menu = menu.copied().unwrap_or_default();
    let vertical = pads
        .iter()
        .map(|gamepad| gamepad.left_stick().y)
        .max_by(|lhs, rhs| lhs.abs().total_cmp(&rhs.abs()))
        .unwrap_or(0.0);
    let (analog_up, analog_down) = analog.vertical_edges(vertical);
    let previous = key(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) || analog_up || menu.up;
    let next = key(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) || analog_down || menu.down;
    let confirm =
        key(KeyCode::Enter) || key(KeyCode::Space) || pad(GamepadButton::South) || menu.select;
    let back = key(KeyCode::Escape) || pad(GamepadButton::East) || menu.back;
    ShellActionEdges {
        previous,
        next,
        confirm,
        back,
        // The touch HUD's "Menu" button folds to `MenuControlFrame::start`, so
        // this is the binding that lets a phone open the pause menu at all.
        pause: key(KeyCode::Escape) || pad(GamepadButton::Start) || menu.start,
        startup_acknowledge: confirm,
        loading_continue: confirm,
        retry: key(KeyCode::KeyR) || pad(GamepadButton::West),
    }
}

#[cfg(test)]
mod tests {
    use super::{shell_action_edges, ShellAnalogLatch};
    use ambition_input::MenuControlFrame;
    use bevy::ecs::system::SystemState;
    use bevy::input::gamepad::Gamepad;
    use bevy::prelude::{Query, World};

    /// Every shell surface below is reachable with NO keyboard and NO gamepad —
    /// the state a phone is actually in. Each assertion names the device-neutral
    /// menu intent that has to carry it.
    ///
    /// The pause case is the regression this guards: the shell menu used to read
    /// only `ButtonInput<KeyCode>` + `Query<&Gamepad>`, so the touch HUD's "Menu"
    /// button folded into `MenuControlFrame::start` and was then dropped on the
    /// floor. On Android that meant a live session with no way back to the title
    /// screen.
    #[test]
    fn the_menu_frame_alone_drives_every_shell_action() {
        let mut world = World::new();
        let mut state: SystemState<Query<&Gamepad>> = SystemState::new(&mut world);
        let pads = state.get(&world);
        let mut latch = ShellAnalogLatch::default();

        // Pre-poison: with no device and no menu frame, nothing may fire. A
        // permissive adapter would make every assertion below vacuous.
        let idle = shell_action_edges(None, &pads, None, &mut latch);
        assert_eq!(
            idle,
            Default::default(),
            "no keyboard, no pad, no menu frame -> no edges"
        );

        let from = |frame: MenuControlFrame, latch: &mut ShellAnalogLatch| {
            shell_action_edges(None, &pads, Some(&frame), latch)
        };
        let start = MenuControlFrame {
            start: true,
            ..Default::default()
        };
        assert!(
            from(start, &mut latch).pause,
            "the touch HUD's Menu button (-> start) must open the pause menu"
        );
        let back = MenuControlFrame {
            back: true,
            ..Default::default()
        };
        assert!(
            from(back, &mut latch).back,
            "the touch HUD's Back button must close an open menu"
        );
        let select = MenuControlFrame {
            select: true,
            ..Default::default()
        };
        let confirmed = from(select, &mut latch);
        assert!(
            confirmed.confirm && confirmed.startup_acknowledge && confirmed.loading_continue,
            "one touch confirm must dismiss startup cards, pick launcher rows, \
             and release a loading ready-hold"
        );
        let down = MenuControlFrame {
            down: true,
            ..Default::default()
        };
        assert!(
            from(down, &mut latch).next,
            "the touch stick's menu-nav fold must move the cursor"
        );
        let up = MenuControlFrame {
            up: true,
            ..Default::default()
        };
        assert!(from(up, &mut latch).previous);
    }

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
