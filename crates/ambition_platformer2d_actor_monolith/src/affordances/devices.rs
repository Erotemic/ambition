//! Active input device + per-device glyph rendering.
//!
//! Phase 2 of the control-HUD work. The HUD already shows what
//! pressing each button *would do* via [`super::PlayerAffordances`].
//! This module answers the second half of the user's brainstorm: show
//! *which key/button* the player would press, in glyphs appropriate to
//! their current input device — "Z" on keyboard, "A" on Xbox, "✕" on
//! PlayStation, etc.
//!
//! ## Shape
//!
//! - [`InputMethod`] — closed enum naming the kinds of input device we
//!   render for (keyboard, gamepad with sub-kind, touch).
//! - [`ActiveInputMethod`] — resource holding the most-recently-used
//!   device, updated each frame by [`detect_active_input_method`]
//!   (last-input-wins, the de-facto pattern most multi-device games
//!   use). HUD systems read this to decide which glyph style to show.
//! - [`GamepadKind`] — sub-classification used to pick "A/B/X/Y" vs
//!   "Cross/Circle/Square/Triangle" vs Switch glyphs. Detection runs
//!   on `GamepadConnected` events (today returning [`GamepadKind::Generic`]
//!   until we add name-based vendor inference).
//! - [`glyph_for`] — pure adapter `(Platformer2dInputActionMonolith, &KeyboardPreset,
//!   InputMethod) -> Cow<'static, str>`. Keyboard glyphs come from
//!   the active [`KeyboardPreset`]; gamepad glyphs are hardcoded per
//!   `GamepadKind`; touch returns an empty string (the on-screen
//!   button IS its own glyph, no subtitle needed).

use std::borrow::Cow;

use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::prelude::*;

use ambition_input::{KeyboardPreset, PresetId, Platformer2dInputActionMonolith};

/// Which input modality the player is currently using. Updated each
/// frame by [`detect_active_input_method`] — last device that
/// produced input wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum InputMethod {
    /// Keyboard (with mouse). Default cold-start choice on desktop;
    /// `detect_active_input_method` switches to other methods as
    /// soon as they produce input.
    #[default]
    Keyboard,
    /// Gamepad (Xbox-like / PlayStation / Switch / generic).
    Gamepad(GamepadKind),
    /// Touchscreen. Set when any active touch is present.
    Touch,
}

/// Vendor-style classification used to pick the right face-button
/// glyphs. Today every gamepad reads as `Generic`; the future
/// Phase-3 polish parses `GamepadInfo.name` for vendor strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GamepadKind {
    /// Xbox 360 / One / Series and most generic XInput pads. Face
    /// buttons rendered as "A/B/X/Y" with shoulders "LB/RB/LT/RT".
    #[default]
    XboxLike,
    /// PlayStation DualShock / DualSense. Face buttons render as the
    /// shape glyphs; shoulders as "L1/R1/L2/R2".
    PlayStation,
    /// Nintendo Switch Pro Controller / Joy-Con. Face buttons keep
    /// their physical labels (B is bottom on Switch, A on the right).
    Switch,
    /// Anything we couldn't classify. Falls back to Xbox-style glyphs.
    Generic,
}

/// Resource: which input method was used most recently. Defaults to
/// [`InputMethod::Keyboard`] so cold-start desktop builds render
/// keyboard glyphs immediately, even before the player touches any
/// input.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActiveInputMethod(pub InputMethod);

/// Per-frame "what did the player touch this frame" detector.
///
/// Last-device-wins: any active touch flips to [`InputMethod::Touch`];
/// any gamepad button press flips to [`InputMethod::Gamepad`]; any
/// keyboard press flips back to [`InputMethod::Keyboard`]. Idle
/// frames leave the resource unchanged so the HUD glyphs don't
/// flicker when the player isn't pressing anything.
///
/// Inputs are taken as `Option<Res<…>>` because the headless / RL
/// builds use `MinimalPlugins`, which doesn't insert Bevy's input
/// resources. With `Option<Res<…>>` the system runs harmlessly there
/// (no input → no method change), and the same affordance pipeline
/// continues to power headless trace assertions about player verbs.
pub fn detect_active_input_method(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    touches: Option<Res<Touches>>,
    mut active: ResMut<ActiveInputMethod>,
) {
    // Touch wins when any finger is down — phones don't pretend to
    // be a keyboard, and a stray keyboard event from an attached
    // bluetooth keyboard shouldn't flip away from touch glyphs while
    // the player has their thumb on the screen.
    if let Some(touches) = touches.as_deref() {
        if touches.iter().next().is_some() {
            let next = InputMethod::Touch;
            if active.0 != next {
                active.0 = next;
            }
            return;
        }
    }

    // TODO(devices): gamepad detection. Bevy 0.18's gamepad API
    // changed shape (Gamepad component on connected entities + per-
    // pad `ButtonInput<GamepadButton>` views). Once verified against
    // the active Bevy version, query for any pad's
    // `digital.get_just_pressed()` and flip to
    // `InputMethod::Gamepad(kind)` here. Until then the keyboard
    // glyphs stay on screen even when the player switches to a pad —
    // the worst-case visual confusion is one wrong glyph row, not
    // missing prompts.

    // Keyboard: any KeyCode just-pressed flips to keyboard.
    if let Some(keys) = keys.as_deref() {
        if keys.get_just_pressed().next().is_some() {
            let next = InputMethod::Keyboard;
            if active.0 != next {
                active.0 = next;
            }
        }
    }
}

/// Pure adapter: name the glyph that represents `action` on the
/// currently-active device.
///
/// - **Keyboard:** keyed off the active `KeyboardPreset`'s `ActionKeys`
///   so a player on the WASD preset sees "J" under Attack while a
///   player on Arrows+ZXC sees "X" — no parallel binding table.
/// - **Gamepad:** hardcoded per [`GamepadKind`]. PlayStation glyphs
///   use the shape-name fallback ("◯", "✕") today; a future polish
///   pass could swap in icon assets via `VariantLabel::icon`.
/// - **Touch:** empty string — the on-screen button itself is the
///   glyph, no subtitle needed.
pub fn glyph_for(
    action: Platformer2dInputActionMonolith,
    preset: &KeyboardPreset,
    bindings: &ambition_input::ActionBindings,
    method: InputMethod,
) -> Cow<'static, str> {
    match method {
        InputMethod::Keyboard => keyboard_glyph(action, preset, bindings),
        InputMethod::Gamepad(kind) => gamepad_glyph(action, kind, bindings),
        InputMethod::Touch => Cow::Borrowed(""),
    }
}

/// The first physical control of a kind this seat has bound to `action`.
///
/// ⚠ **an action bound to nothing yields nothing**, and the caller renders an
/// empty glyph. That is the honest answer and it is what `Special` and
/// `StrongAttack` used to say through a hand-written `""` arm: the difference
/// is that this one stops being empty the moment somebody binds them, with no
/// table to remember to edit.
fn bound_control(
    bindings: &ambition_input::ActionBindings,
    action: Platformer2dInputActionMonolith,
    want_key: bool,
) -> Option<&ambition_input::PhysicalControl> {
    bindings.controls(&action).iter().find(|control| {
        matches!(
            (control, want_key),
            (ambition_input::PhysicalControl::Key(_), true)
                | (ambition_input::PhysicalControl::Button(_), false)
        )
    })
}

/// Keyboard glyph for an action.
///
/// Movement returns the preset's SUMMARY label ("Arrows" / "WASD"), which no
/// single binding can produce — it names four keys at once. Every other verb
/// comes from the seat's live binding.
fn keyboard_glyph(
    action: Platformer2dInputActionMonolith,
    preset: &KeyboardPreset,
    bindings: &ambition_input::ActionBindings,
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
        Platformer2dInputActionMonolith::Pogo if bound_control(bindings, action, true).is_none() => {
            Cow::Borrowed("D+X")
        }
        _ => match bound_control(bindings, action, true) {
            Some(control) => Cow::Owned(control.label()),
            None => Cow::Borrowed(""),
        },
    }
}

/// Gamepad glyph for an action under the given pad classification.
///
/// ⛔ **this used to be a static table that answered TWO questions**, and its
/// own comment named the hazard: *"today this is a static table that matches
/// the bindings authored in `KeyboardPreset::input_map`; if those bindings
/// change the table here needs to follow."* It never followed automatically,
/// and the failure mode is a prompt telling a player to press a button that
/// does nothing — indistinguishable, from the player's side, from a broken
/// binding.
///
/// The two questions are separated now, which is why the table could go:
///
/// * **WHICH control** — read from `ActionBindings`, projected from the very
///   `InputMap` the router reads. Nothing to keep in step.
/// * **HOW TO DRAW IT** — that is what `GamepadKind` is for, and it is a real
///   presentation concern: the same `GamepadButton::South` is "A" on an Xbox
///   pad, "Cross" on a DualShock and "B" on a Switch pad, because Nintendo
///   mirrors the positions.
fn gamepad_glyph(
    action: Platformer2dInputActionMonolith,
    kind: GamepadKind,
    bindings: &ambition_input::ActionBindings,
) -> Cow<'static, str> {
    // Sticks are dual-axis inputs, so `iter_buttonlike` correctly does not list
    // them and no binding projection can name one.
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
    if matches!(action, Platformer2dInputActionMonolith::DashAnalog | Platformer2dInputActionMonolith::AimStick) {
        return Cow::Borrowed("R-Stick");
    }
    match bound_control(bindings, action, false) {
        Some(ambition_input::PhysicalControl::Button(button)) => {
            Cow::Borrowed(vendor_label(*button, kind))
        }
        // Bound to no gamepad control. Empty — and it stops being empty the day
        // somebody binds it, which is exactly what `Special` is waiting for.
        _ => Cow::Borrowed(""),
    }
}

/// How this pad's vendor draws a button. Presentation only — WHICH button is
/// pressed is the binding's answer, not this function's.
fn vendor_label(button: GamepadButton, kind: GamepadKind) -> &'static str {
    match button {
        GamepadButton::South => match kind {
            GamepadKind::PlayStation => "Cross",
            // Switch mirrors the A/B positions: the button in the SOUTH
            // position is physically labelled "B".
            GamepadKind::Switch => "B",
            _ => "A",
        },
        GamepadButton::East => match kind {
            GamepadKind::PlayStation => "Circle",
            GamepadKind::Switch => "A",
            _ => "B",
        },
        GamepadButton::West => match kind {
            GamepadKind::PlayStation => "Square",
            GamepadKind::Switch => "Y",
            _ => "X",
        },
        GamepadButton::North => match kind {
            GamepadKind::PlayStation => "Triangle",
            GamepadKind::Switch => "X",
            _ => "Y",
        },
        GamepadButton::LeftTrigger => match kind {
            GamepadKind::PlayStation => "L1",
            _ => "LB",
        },
        GamepadButton::RightTrigger => match kind {
            GamepadKind::PlayStation => "R1",
            _ => "RB",
        },
        GamepadButton::LeftTrigger2 => match kind {
            GamepadKind::PlayStation => "L2",
            _ => "LT",
        },
        GamepadButton::RightTrigger2 => match kind {
            GamepadKind::PlayStation => "R2",
            _ => "RT",
        },
        GamepadButton::Select => match kind {
            GamepadKind::PlayStation => "Share",
            GamepadKind::Switch => "-",
            _ => "Back",
        },
        GamepadButton::Start => match kind {
            GamepadKind::PlayStation => "Options",
            GamepadKind::Switch => "+",
            _ => "Start",
        },
        GamepadButton::LeftThumb => "L3",
        GamepadButton::RightThumb => "R3",
        GamepadButton::DPadUp => "D-Up",
        GamepadButton::DPadDown => "D-Down",
        GamepadButton::DPadLeft => "D-Left",
        GamepadButton::DPadRight => "D-Right",
        // A Bevy upgrade adding a variant must print something odd, never panic
        // a HUD.
        _ => "Button",
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ambition_input::{ActionBindings, KeyboardPreset};

    /// The seat's bindings, projected from the preset's own `InputMap` — the
    /// same map the router reads. Building them here rather than hand-writing
    /// expectations is the point of the change these tests now cover.
    fn bindings(preset: &KeyboardPreset) -> ActionBindings {
        ActionBindings::from_map(&preset.input_map())
    }

    #[test]
    fn keyboard_glyph_follows_active_preset() {
        let arrows_zxc = KeyboardPreset::arrows_zxc();
        // Arrows+ZXC: Jump = Z, Attack = X, Dash = C.
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Jump, &arrows_zxc, &bindings(&arrows_zxc), InputMethod::Keyboard),
            "Z"
        );
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Attack, &arrows_zxc, &bindings(&arrows_zxc), InputMethod::Keyboard),
            "X"
        );
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Dash, &arrows_zxc, &bindings(&arrows_zxc), InputMethod::Keyboard),
            "C"
        );

        let wasd = KeyboardPreset::wasd_jkl();
        // WASD: Jump = Space, Attack = J, Dash = K.
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Jump, &wasd, &bindings(&wasd), InputMethod::Keyboard),
            "Space"
        );
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Attack, &wasd, &bindings(&wasd), InputMethod::Keyboard),
            "J"
        );
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Dash, &wasd, &bindings(&wasd), InputMethod::Keyboard),
            "K"
        );
    }

    #[test]
    fn gamepad_glyph_switches_face_buttons_by_kind() {
        let preset = KeyboardPreset::arrows_zxc(); // keyboard preset unused for gamepad path
        assert_eq!(
            glyph_for(
                Platformer2dInputActionMonolith::Jump,
                &preset,
                &bindings(&preset),
                InputMethod::Gamepad(GamepadKind::XboxLike)
            ),
            "A"
        );
        assert_eq!(
            glyph_for(
                Platformer2dInputActionMonolith::Jump,
                &preset,
                &bindings(&preset),
                InputMethod::Gamepad(GamepadKind::PlayStation)
            ),
            "Cross"
        );
        assert_eq!(
            glyph_for(
                Platformer2dInputActionMonolith::Attack,
                &preset,
                &bindings(&preset),
                InputMethod::Gamepad(GamepadKind::XboxLike)
            ),
            "X"
        );
        assert_eq!(
            glyph_for(
                Platformer2dInputActionMonolith::Attack,
                &preset,
                &bindings(&preset),
                InputMethod::Gamepad(GamepadKind::PlayStation)
            ),
            "Square"
        );
    }

    #[test]
    fn touch_glyph_is_empty() {
        let preset = KeyboardPreset::arrows_zxc();
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Jump, &preset, &bindings(&preset), InputMethod::Touch),
            ""
        );
        assert_eq!(
            glyph_for(Platformer2dInputActionMonolith::Attack, &preset, &bindings(&preset), InputMethod::Touch),
            ""
        );
    }
}
