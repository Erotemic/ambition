//! Device-conditional glyph rendering for a seat's bindings.
//!
//! `glyph_for` names the physical control a prompt should show for an action,
//! in the vocabulary of the device the seat is actually using: "Z" on the
//! keyboard preset, "A" on an Xbox pad, "Cross" on a DualShock, nothing on
//! touch (the on-screen button IS its own glyph).
//!
//! Two questions, deliberately separated:
//!
//! * WHICH control — read from [`crate::ActionBindings`], projected from
//!   the very `InputMap` the router reads. Nothing to keep in step: a rebind
//!   moves the glyph because it moved the binding.
//! * HOW TO DRAW IT — [`GamepadStyle`]: the same `GamepadButton::South`
//!   is "A" on an Xbox pad, "Cross" on a DualShock and "B" on a Switch pad,
//!   because Nintendo mirrors the positions.
//!
//! This lived in the actor monolith's `affordances::devices` (with its own
//! private device detector); it is input-layer vocabulary through and
//! through — every input it takes is defined in this crate — and moving it
//! here removed the touch overlay's only reason to name the actor crate for
//! glyphs.

use std::borrow::Cow;

use bevy::prelude::*;

use crate::active_input::{ActiveDevice, GamepadStyle};
use crate::bindings::{ActionBindings, PhysicalControl};
use crate::presets::{KeyboardPreset, PresetId};
use crate::Platformer2dInputActionMonolith;

/// Name the glyph that represents `action` on `device`, for a seat whose
/// bindings are `bindings` and whose keyboard preset is `preset`.
///
/// The mouse draws keyboard glyphs — it is half of the keyboard-and-mouse
/// bundle, and a click does not move the player's other hand off the keys.
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
/// an action bound to nothing of this kind yields nothing, and the caller
/// renders an empty glyph. That is the honest answer for a GLYPH — a picture of
/// a button nobody has bound is a lie, where the text label can fall back and be
/// understood. The two miss policies differ on purpose and are stated at each
/// call site rather than buried in the selection.
fn bound_control(
    bindings: &ActionBindings,
    action: Platformer2dInputActionMonolith,
    want_key: bool,
) -> Option<&PhysicalControl> {
    let device = if want_key {
        ActiveDevice::Keyboard
    } else {
        // Selection only cares about the CLASS; the caller re-spells with the
        // seat's real style, so any pad style resolves the same control here.
        ActiveDevice::Gamepad(GamepadStyle::default())
    };
    bindings.control_for(&action, device)
}

/// Keyboard glyph for an action.
///
/// Movement returns the preset's SUMMARY label ("Arrows" / "WASD"), which no
/// single binding can produce — it names four keys at once. Every other verb
/// comes from the seat's live binding.
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
        // Pogo has no dedicated key on every preset; the chord is the fallback
        // and it is a CHORD, which no single binding can name.
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
/// WHICH control now comes from the binding projection; only the vendor's SPELLING of it is a
/// table, and that is a real presentation fact.
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
        // Bound to no gamepad control. Empty — and it stops being empty the
        // day somebody binds it, which is exactly what `Special` is waiting
        // for.
        _ => Cow::Borrowed(""),
    }
}

/// How this pad's vendor draws a button. Presentation only — WHICH button is
/// pressed is the binding's answer, not this function's.
///
/// THE one gamepad-button label table.
pub(crate) fn button_label(button: GamepadButton, style: GamepadStyle) -> &'static str {
    match button {
        GamepadButton::South => match style {
            GamepadStyle::PlayStation => "Cross",
            // Switch mirrors the A/B positions: the button in the SOUTH
            // position is physically labelled "B".
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
        // A Bevy upgrade adding a variant must print something odd, never
        // panic a HUD.
        _ => "Button",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seat's bindings, projected from the preset's own `InputMap` — the
    /// same map the router reads. Building them here rather than hand-writing
    /// expectations is the point of the design these tests cover.
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
