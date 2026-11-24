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

    /// A control's picture is named by the renderer's rule (`input_glyphs`):
    /// the pad family and Bevy's own button word, or the key's label.
    #[test]
    fn a_control_names_its_published_picture_by_the_renderers_rule() {
        use crate::GamepadStyle as S;
        let path = |control: PhysicalControl, style| control.glyph_path(style).unwrap();
        assert_eq!(path(PhysicalControl::Button(GamepadButton::South), S::XboxLike), "sprites/input_glyphs/input_glyph_xbox_south.png");
        assert_eq!(path(PhysicalControl::Button(GamepadButton::South), S::PlayStation), "sprites/input_glyphs/input_glyph_playstation_south.png");
        assert_eq!(path(PhysicalControl::Button(GamepadButton::DPadUp), S::Switch), "sprites/input_glyphs/input_glyph_switch_d_pad_up.png");
        assert_eq!(path(PhysicalControl::Button(GamepadButton::LeftTrigger2), S::Generic), "sprites/input_glyphs/input_glyph_generic_left_trigger2.png");
        for (key, name) in [(KeyCode::KeyF, "f"), (KeyCode::Escape, "esc"), (KeyCode::ShiftRight, "shift"), (KeyCode::BracketLeft, "bracketleft"), (KeyCode::Digit3, "digit3")] {
            assert_eq!(path(PhysicalControl::Key(key), S::PlayStation), format!("sprites/input_glyphs/input_glyph_key_{name}.png"));
        }
        assert_eq!(PhysicalControl::Other("Chord".into()).glyph_path(S::XboxLike), None);
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

/// A key's label: [`crate::presets::key_name`], the one table, or for a key it
/// does not list the `KeyCode` debug form (`F13`, `NumpadAdd`), as with
/// [`PhysicalControl::Other`].
fn key_label(key: KeyCode) -> String {
    match crate::presets::key_name(key) {
        "?" => format!("{key:?}"),
        named => named.to_string(),
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

    /// The published picture of this control on a pad of `style` (keys and
    /// the mouse draw their own): the file the renderer's `input_glyphs`
    /// target names by rule from the game's own vocabulary (its docstring),
    /// under [`GLYPH_FOLDER`]. `None` for a control that rule cannot name.
    ///
    /// A name is not a promise that the file was drawn (an `F13` key has none):
    /// the caller asks its asset catalog before loading, and draws
    /// [`Self::label_for`] when the picture is not there.
    pub fn glyph_path(&self, style: crate::GamepadStyle) -> Option<String> {
        let name = match self {
            Self::Key(key) => format!("key_{}", key_label(*key).to_lowercase()),
            Self::Button(button) => format!("{}_{}", style_word(style), snake_case(&format!("{button:?}"))),
            Self::Other(_) => return None,
        };
        Some(format!("{GLYPH_FOLDER}/input_glyph_{name}.png"))
    }

}

/// Where the renderer's `input_glyphs` target publishes its pictures.
pub const GLYPH_FOLDER: &str = "sprites/input_glyphs";

impl PhysicalControl {
    /// The name the published device layouts key this control by: Bevy's own
    /// `KeyCode` / `GamepadButton` name (`KeyF`, `South`). `None` for a
    /// control no layout places.
    pub fn layout_name(&self) -> Option<String> {
        match self {
            Self::Key(key) => Some(format!("{key:?}")),
            Self::Button(button) => Some(format!("{button:?}")),
            Self::Other(_) => None,
        }
    }
}

/// The published picture of the whole device a seat holds, and the layout
/// that places each of its controls in that picture at full size
/// (`input_device_{keyboard|xbox|playstation|switch|generic}[_layout.ron]`,
/// the renderer's `input_glyphs` target).
pub fn device_picture(device: crate::ActiveDevice) -> (String, String) {
    let word = if device.draws_keyboard_glyphs() {
        "keyboard"
    } else {
        style_word(device.gamepad_style())
    };
    (
        format!("{GLYPH_FOLDER}/input_device_{word}.png"),
        format!("{GLYPH_FOLDER}/input_device_{word}_layout.ron"),
    )
}

/// The word a glyph file names a pad family by.
fn style_word(style: crate::GamepadStyle) -> &'static str {
    match style {
        crate::GamepadStyle::XboxLike => "xbox",
        crate::GamepadStyle::PlayStation => "playstation",
        crate::GamepadStyle::Switch => "switch",
        crate::GamepadStyle::Generic => "generic",
    }
}

/// `DPadUp` -> `d_pad_up`, `LeftTrigger2` -> `left_trigger2` (the `heck`
/// rule the renderer names its files by).
fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A control as a prompt shows it on one device: its word, its published
/// picture, and the control itself, so a drawn picture can press when it does.
/// One value, so the word and the picture always name the same control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedControl {
    /// "Z" on a keyboard preset, "A" on an Xbox pad, "Cross" on a DualShock.
    pub label: String,
    /// The published picture (`sprites/input_glyphs/...`), when the renderer's
    /// rule names one and the control is of the device's class: a picture of a
    /// key is wrong in a pad's hands, where the word can still name it.
    pub glyph: Option<String>,
    pub control: PhysicalControl,
}

impl NamedControl {
    pub fn of(control: &PhysicalControl, device: crate::ActiveDevice) -> Self {
        let style = device.gamepad_style();
        let of_class = matches!(
            (control, device.draws_keyboard_glyphs()),
            (PhysicalControl::Key(_), true) | (PhysicalControl::Button(_), false)
        );
        Self {
            label: control.label_for(style),
            glyph: control.glyph_path(style).filter(|_| of_class),
            control: control.clone(),
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

    /// The published picture of the control [`Self::label_for`] names, by
    /// the same choice of control. `None` when nothing binds the action or
    /// the renderer's rule names no file for the control.
    pub fn glyph_for(
        &self,
        action: &Platformer2dInputActionMonolith,
        device: crate::ActiveDevice,
    ) -> Option<String> {
        self.control_for(action, device)
            .or_else(|| self.controls(action).first())
            .and_then(|control| control.glyph_path(device.gamepad_style()))
    }

    /// The control of the device's class that is also bound to a `prefer`
    /// action (in that order), else the first not bound to an `avoid` action,
    /// else the first of the class. No fallback to the other class here; the
    /// caller decides.
    fn control_preferring(
        &self,
        action: &Platformer2dInputActionMonolith,
        prefer: &[Platformer2dInputActionMonolith],
        avoid: &[Platformer2dInputActionMonolith],
        device: crate::ActiveDevice,
    ) -> Option<&PhysicalControl> {
        let want_key = device.draws_keyboard_glyphs();
        let of_class: Vec<&PhysicalControl> = self
            .controls(action)
            .iter()
            .filter(|control| matches!((control, want_key), (PhysicalControl::Key(_), true) | (PhysicalControl::Button(_), false)))
            .collect();
        let bound_to = |other: &Platformer2dInputActionMonolith, control: &PhysicalControl| self.controls(other).contains(control);
        prefer
            .iter()
            .find_map(|other| of_class.iter().find(|control| bound_to(other, control)))
            .or_else(|| of_class.iter().find(|control| !avoid.iter().any(|other| bound_to(other, control))))
            .or_else(|| of_class.first())
            .copied()
    }

    /// [`Self::label_for`]'s control, named for `device`.
    pub fn named_for(&self, action: &Platformer2dInputActionMonolith, device: crate::ActiveDevice) -> Option<NamedControl> {
        self.control_for(action, device)
            .or_else(|| self.controls(action).first())
            .map(|control| NamedControl::of(control, device))
    }

    /// The control a prompt names for confirm: a `MenuSelect` control also
    /// bound to Jump (the pad's A and its keyboard stand-in), else the first.
    pub fn confirm_named(&self, device: crate::ActiveDevice) -> Option<NamedControl> {
        use Platformer2dInputActionMonolith as A;
        self.named_preferring(&A::MenuSelect, &[A::Jump], &[], device)
    }

    /// The control a prompt names for back: a `MenuBack` control that is not
    /// also Start, because Start opens the pause menu over the dialogue.
    pub fn back_named(&self, device: crate::ActiveDevice) -> Option<NamedControl> {
        use Platformer2dInputActionMonolith as A;
        self.named_preferring(&A::MenuBack, &[], &[A::Start], device)
    }

    fn named_preferring(
        &self,
        action: &Platformer2dInputActionMonolith,
        prefer: &[Platformer2dInputActionMonolith],
        avoid: &[Platformer2dInputActionMonolith],
        device: crate::ActiveDevice,
    ) -> Option<NamedControl> {
        self.control_preferring(action, prefer, avoid, device)
            .map(|control| NamedControl::of(control, device))
            .or_else(|| self.named_for(action, device))
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

    /// The picture of the control [`Self::label_for_slot`] names.
    pub fn glyph_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
        devices: Option<&crate::SeatActiveDevices>,
    ) -> Option<String> {
        let device = devices.map_or_else(Default::default, |devices| devices.for_seat(seat));
        self.for_seat(seat).glyph_for(&action_for_slot(slot)?, device)
    }

    /// The control an ability slot's prompt names on the seat's device.
    pub fn named_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
        devices: Option<&crate::SeatActiveDevices>,
    ) -> Option<NamedControl> {
        let device = devices.map_or_else(Default::default, |devices| devices.for_seat(seat));
        self.for_seat(seat).named_for(&action_for_slot(slot)?, device)
    }

}
