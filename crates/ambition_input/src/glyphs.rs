//! Device-conditional glyph rendering for a seat's bindings.
//!
//! `glyph_for` names the physical control a prompt shows for an action, in the
//! vocabulary of the seat's current device: "Z" on a keyboard preset, "A" on
//! an Xbox pad, "Cross" on a DualShock, nothing on touch (the on-screen button
//! is its own glyph).
//!
//! Two separate questions:
//!
//! * Which control: read from [`crate::ActionBindings`], projected from the
//!   same `InputMap` the router reads. A rebind moves the glyph.
//! * How to draw it: [`GamepadStyle`]. `GamepadButton::South` is "A" on Xbox,
//!   "Cross" on DualShock, and "B" on Switch (Nintendo mirrors positions).

use std::borrow::Cow;

use bevy::prelude::*;

use crate::active_input::{ActiveDevice, GamepadStyle};
use crate::bindings::{action_for_slot, ActionBindings, PhysicalControl, SeatBindings};
use crate::presets::{KeyboardPreset, PresetId};
use crate::Platformer2dInputActionMonolith;

/// Name the glyph that represents `action` on `device`, for a seat whose
/// bindings are `bindings` and whose keyboard preset is `preset`.
///
/// The mouse draws keyboard glyphs, because it is part of the
/// keyboard-and-mouse pair.
pub fn glyph_for(
    action: Platformer2dInputActionMonolith,
    preset: &KeyboardPreset,
    bindings: &ActionBindings,
    device: ActiveDevice,
) -> Cow<'static, str> {
    match device {
        ActiveDevice::Keyboard | ActiveDevice::Mouse => keyboard_glyph(action, preset, bindings),
        ActiveDevice::Gamepad(style) => gamepad_glyph(action, style, bindings),
        ActiveDevice::Touch => Cow::Borrowed(""),
    }
}

/// The physical control of a kind this seat has bound to `action`.
///
/// If nothing of this kind is bound, this returns `None` and the caller draws
/// an empty glyph. A glyph for an unbound button would be wrong; a text label
/// can fall back instead. Each call site states its own miss policy.
fn bound_control(
    bindings: &ActionBindings,
    action: Platformer2dInputActionMonolith,
    want_key: bool,
) -> Option<&PhysicalControl> {
    let device = if want_key {
        ActiveDevice::Keyboard
    } else {
        // Only the class matters here; the caller spells the result with the
        // seat's real style.
        ActiveDevice::Gamepad(GamepadStyle::default())
    };
    bindings.control_for(&action, device)
}

/// Keyboard glyph for an action.
///
/// Movement returns the preset's summary label ("Arrows" / "WASD"), which
/// names four keys at once. Other verbs come from the seat's live binding.
fn keyboard_glyph(
    action: Platformer2dInputActionMonolith,
    preset: &KeyboardPreset,
    bindings: &ActionBindings,
) -> Cow<'static, str> {
    let movement_label = match preset.id {
        PresetId::ArrowsZxc | PresetId::ArrowsQwer => "Arrows",
        PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
    };
    match action {
        Platformer2dInputActionMonolith::Move
        | Platformer2dInputActionMonolith::MoveLeft
        | Platformer2dInputActionMonolith::MoveRight
        | Platformer2dInputActionMonolith::MoveUp
        | Platformer2dInputActionMonolith::MoveDown
        | Platformer2dInputActionMonolith::MenuStick => Cow::Borrowed(movement_label),
        Platformer2dInputActionMonolith::MenuNavigateUp
        | Platformer2dInputActionMonolith::MenuNavigateDown
        | Platformer2dInputActionMonolith::MenuNavigateLeft
        | Platformer2dInputActionMonolith::MenuNavigateRight => Cow::Borrowed(movement_label),
        // Pogo has no dedicated key on every preset. The fallback is a chord,
        // which no single binding can name.
        Platformer2dInputActionMonolith::Pogo
            if bound_control(bindings, action, true).is_none() =>
        {
            Cow::Borrowed("D+X")
        }
        _ => match bound_control(bindings, action, true) {
            Some(control) => Cow::Owned(control.label()),
            None => Cow::Borrowed(""),
        },
    }
}

/// Gamepad glyph for an action under the given vendor style.
///
/// The binding projection gives the control; only the vendor spelling is a
/// table.
fn gamepad_glyph(
    action: Platformer2dInputActionMonolith,
    style: GamepadStyle,
    bindings: &ActionBindings,
) -> Cow<'static, str> {
    // Sticks are dual-axis inputs, so `iter_buttonlike` correctly does not
    // list them and no binding projection can name one.
    if matches!(
        action,
        Platformer2dInputActionMonolith::Move
            | Platformer2dInputActionMonolith::MoveLeft
            | Platformer2dInputActionMonolith::MoveRight
            | Platformer2dInputActionMonolith::MoveUp
            | Platformer2dInputActionMonolith::MoveDown
            | Platformer2dInputActionMonolith::MenuStick
            | Platformer2dInputActionMonolith::MenuNavigateUp
            | Platformer2dInputActionMonolith::MenuNavigateDown
            | Platformer2dInputActionMonolith::MenuNavigateLeft
            | Platformer2dInputActionMonolith::MenuNavigateRight
    ) {
        return Cow::Borrowed("L-Stick");
    }
    if matches!(
        action,
        Platformer2dInputActionMonolith::BurstAnalog | Platformer2dInputActionMonolith::AimStick
    ) {
        return Cow::Borrowed("R-Stick");
    }
    match bound_control(bindings, action, false) {
        Some(PhysicalControl::Button(button)) => Cow::Borrowed(button_label(*button, style)),
        // Bound to no gamepad control: empty until someone binds it (for
        // example `Special`).
        _ => Cow::Borrowed(""),
    }
}

/// How this pad's vendor draws a button. Presentation only; the binding
/// decides which button. This is the only gamepad-button label table.
pub(crate) fn button_label(button: GamepadButton, style: GamepadStyle) -> &'static str {
    match button {
        GamepadButton::South => match style {
            GamepadStyle::PlayStation => "Cross",
            // Switch mirrors A/B: the south button is labelled "B".
            GamepadStyle::Switch => "B",
            _ => "A",
        },
        GamepadButton::East => match style {
            GamepadStyle::PlayStation => "Circle",
            GamepadStyle::Switch => "A",
            _ => "B",
        },
        GamepadButton::West => match style {
            GamepadStyle::PlayStation => "Square",
            GamepadStyle::Switch => "Y",
            _ => "X",
        },
        GamepadButton::North => match style {
            GamepadStyle::PlayStation => "Triangle",
            GamepadStyle::Switch => "X",
            _ => "Y",
        },
        GamepadButton::LeftTrigger => match style {
            GamepadStyle::PlayStation => "L1",
            _ => "LB",
        },
        GamepadButton::RightTrigger => match style {
            GamepadStyle::PlayStation => "R1",
            _ => "RB",
        },
        GamepadButton::LeftTrigger2 => match style {
            GamepadStyle::PlayStation => "L2",
            _ => "LT",
        },
        GamepadButton::RightTrigger2 => match style {
            GamepadStyle::PlayStation => "R2",
            _ => "RT",
        },
        GamepadButton::Select => match style {
            GamepadStyle::PlayStation => "Share",
            GamepadStyle::Switch => "-",
            _ => "Back",
        },
        GamepadButton::Start => match style {
            GamepadStyle::PlayStation => "Options",
            GamepadStyle::Switch => "+",
            _ => "Start",
        },
        GamepadButton::Mode => "Home",
        GamepadButton::LeftThumb => "L3",
        GamepadButton::RightThumb => "R3",
        GamepadButton::DPadUp => "D-Up",
        GamepadButton::DPadDown => "D-Down",
        GamepadButton::DPadLeft => "D-Left",
        GamepadButton::DPadRight => "D-Right",
        // A new Bevy variant prints a placeholder; the HUD must not panic.
        _ => "Button",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seat's bindings, projected from the preset's own `InputMap` (the
    /// map the router reads), not hand-written expectations.
    fn bindings(preset: &KeyboardPreset) -> ActionBindings {
        ActionBindings::from_map(&preset.input_map())
    }

    #[test]
    fn keyboard_glyph_follows_active_preset() {
        let arrows_zxc = KeyboardPreset::arrows_zxc();
        // Arrows+ZXC: Jump = Z, Attack = X, Burst = C.
        for (action, glyph) in [
            (Platformer2dInputActionMonolith::Jump, "Z"),
            (Platformer2dInputActionMonolith::Attack, "X"),
            (Platformer2dInputActionMonolith::Burst, "C"),
        ] {
            assert_eq!(
                glyph_for(
                    action,
                    &arrows_zxc,
                    &bindings(&arrows_zxc),
                    ActiveDevice::Keyboard
                ),
                glyph
            );
        }

        let wasd = KeyboardPreset::wasd_jkl();
        // WASD: Jump = Space, Attack = J, Burst = K.
        for (action, glyph) in [
            (Platformer2dInputActionMonolith::Jump, "Space"),
            (Platformer2dInputActionMonolith::Attack, "J"),
            (Platformer2dInputActionMonolith::Burst, "K"),
        ] {
            assert_eq!(
                glyph_for(action, &wasd, &bindings(&wasd), ActiveDevice::Keyboard),
                glyph
            );
        }
    }

    #[test]
    fn the_mouse_draws_keyboard_glyphs() {
        // Clicking does not move the player's other hand off the keys.
        let preset = KeyboardPreset::arrows_zxc();
        assert_eq!(
            glyph_for(
                Platformer2dInputActionMonolith::Jump,
                &preset,
                &bindings(&preset),
                ActiveDevice::Mouse
            ),
            "Z"
        );
    }

    #[test]
    fn gamepad_glyph_switches_face_buttons_by_style() {
        let preset = KeyboardPreset::arrows_zxc(); // keyboard preset unused for gamepad path
        let bound = bindings(&preset);
        for (style, jump, attack) in [
            (GamepadStyle::XboxLike, "A", "X"),
            (GamepadStyle::PlayStation, "Cross", "Square"),
        ] {
            assert_eq!(
                glyph_for(
                    Platformer2dInputActionMonolith::Jump,
                    &preset,
                    &bound,
                    ActiveDevice::Gamepad(style)
                ),
                jump
            );
            assert_eq!(
                glyph_for(
                    Platformer2dInputActionMonolith::Attack,
                    &preset,
                    &bound,
                    ActiveDevice::Gamepad(style)
                ),
                attack
            );
        }
    }

    #[test]
    fn touch_glyph_is_empty() {
        let preset = KeyboardPreset::arrows_zxc();
        for action in [
            Platformer2dInputActionMonolith::Jump,
            Platformer2dInputActionMonolith::Attack,
        ] {
            assert_eq!(
                glyph_for(action, &preset, &bindings(&preset), ActiveDevice::Touch),
                ""
            );
        }
    }
}

/// `presets::key_name` returns `"?"` for a key it does not list. For those
/// keys, use the `KeyCode` debug form (`F13`, `NumpadAdd`), as with
/// [`PhysicalControl::Other`].
fn key_label(key: KeyCode) -> String {
    match key {
        KeyCode::Space => "Space".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Escape => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift".to_string(),
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl".to_string(),
        KeyCode::AltLeft | KeyCode::AltRight => "Alt".to_string(),
        KeyCode::ArrowUp => "Up".to_string(),
        KeyCode::ArrowDown => "Down".to_string(),
        KeyCode::ArrowLeft => "Left".to_string(),
        KeyCode::ArrowRight => "Right".to_string(),
        other => match crate::presets::key_name(other) {
            "?" => format!("{other:?}"),
            named => named.to_string(),
        },
    }
}

impl PhysicalControl {
    /// What to print. Short, because it goes on a button. Gamepad buttons use
    /// Xbox names; a caller that knows the seat's pad uses [`Self::label_for`].
    pub fn label(&self) -> String {
        self.label_for(crate::GamepadStyle::XboxLike)
    }

    /// What to print, in the names of the seat's own pad. It uses the same
    /// table as the glyph path, so a prompt and a glyph always agree.
    pub fn label_for(&self, style: crate::GamepadStyle) -> String {
        match self {
            Self::Key(key) => key_label(*key),
            Self::Button(button) => crate::glyphs::button_label(*button, style).to_string(),
            Self::Other(raw) => raw.clone(),
        }
    }

}

impl ActionBindings {

    /// The label a prompt shows: the first binding, which the preset lists as
    /// primary.
    ///
    /// It ignores the device: gamepad buttons use Xbox names and a mixed map
    /// gives its keyboard half. Use it only with no seat (docs generator,
    /// rebind list). A prompt for a seat uses [`Self::label_for`].
    pub fn label(&self, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.controls(action).first().map(PhysicalControl::label)
    }

    /// The label for the device this seat holds. From one map, a DualSense
    /// reads "Cross", an Xbox pad "A", and a keyboard "Z".
    ///
    /// It falls back to the other device class, not `None`: a blank prompt
    /// reads as "this action has no control".
    pub fn label_for(
        &self,
        action: &Platformer2dInputActionMonolith,
        device: crate::ActiveDevice,
    ) -> Option<String> {
        self.control_for(action, device)
            .or_else(|| self.controls(action).first())
            .map(|control| control.label_for(device.gamepad_style()))
    }

}

impl SeatBindings {

    /// The label this seat's prompt should show for this action.
    pub fn label(&self, slot: u8, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.for_seat(slot).label(action)
    }

    /// The label for an ability slot. Uses [`action_for_slot`].
    ///
    /// `devices` is a parameter because the seat's device decides both which
    /// binding to name and how to spell it. `None` (a headless sim with no
    /// device tracking) means the keyboard, the documented cold-start answer.
    pub fn label_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
        devices: Option<&crate::SeatActiveDevices>,
    ) -> Option<String> {
        let device = devices.map_or_else(Default::default, |devices| devices.for_seat(seat));
        self.for_seat(seat)
            .label_for(&action_for_slot(slot)?, device)
    }

}
