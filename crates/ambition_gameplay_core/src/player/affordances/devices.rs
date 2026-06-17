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
//! - [`glyph_for`] — pure adapter `(SandboxAction, &KeyboardPreset,
//!   InputMethod) -> Cow<'static, str>`. Keyboard glyphs come from
//!   the active [`KeyboardPreset`]; gamepad glyphs are hardcoded per
//!   `GamepadKind`; touch returns an empty string (the on-screen
//!   button IS its own glyph, no subtitle needed).

use std::borrow::Cow;

use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::input::{KeyboardPreset, PresetId, SandboxAction};

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
    action: SandboxAction,
    preset: &KeyboardPreset,
    method: InputMethod,
) -> Cow<'static, str> {
    match method {
        InputMethod::Keyboard => Cow::Borrowed(keyboard_glyph(action, preset)),
        InputMethod::Gamepad(kind) => Cow::Borrowed(gamepad_glyph(action, kind)),
        InputMethod::Touch => Cow::Borrowed(""),
    }
}

/// Keyboard binding glyph for an action under the given preset.
/// Movement axes return the preset's movement-label ("Arrows" /
/// "WASD"); action verbs walk the preset's `ActionKeys` table.
fn keyboard_glyph(action: SandboxAction, preset: &KeyboardPreset) -> &'static str {
    let keys = &preset.actions;
    let movement_label = match preset.id {
        PresetId::ArrowsZxc | PresetId::ArrowsQwer => "Arrows",
        PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
    };
    match action {
        SandboxAction::Move
        | SandboxAction::MoveLeft
        | SandboxAction::MoveRight
        | SandboxAction::MoveUp
        | SandboxAction::MoveDown
        | SandboxAction::MenuStick => movement_label,
        SandboxAction::Jump => key_glyph(keys.jump),
        SandboxAction::Attack => key_glyph(keys.attack),
        SandboxAction::Dash => key_glyph(keys.dash),
        SandboxAction::Blink => key_glyph(keys.secondary),
        SandboxAction::QuickAction => key_glyph(keys.quick_action),
        SandboxAction::Interact => key_glyph(keys.interact),
        SandboxAction::Modifier => key_glyph(keys.modifier),
        SandboxAction::Utility => key_glyph(keys.utility),
        SandboxAction::Map => key_glyph(keys.map),
        SandboxAction::Inventory => key_glyph(keys.inventory),
        SandboxAction::Pogo => keys.dedicated_pogo.map_or("D+X", key_glyph),
        SandboxAction::Reset => key_glyph(keys.select_reset),
        SandboxAction::Start => key_glyph(keys.pause),
        SandboxAction::Projectile => key_glyph(keys.projectile),
        SandboxAction::TrailToggle => key_glyph(keys.trail_toggle),
        SandboxAction::MenuNavigateUp
        | SandboxAction::MenuNavigateDown
        | SandboxAction::MenuNavigateLeft
        | SandboxAction::MenuNavigateRight => movement_label,
        SandboxAction::MenuSelect => "Enter",
        SandboxAction::MenuBack => "Esc",
        SandboxAction::MenuPageLeft => "Q",
        SandboxAction::MenuPageRight => "E",
        SandboxAction::DashAnalog | SandboxAction::AimStick => "",
    }
}

/// Gamepad glyph for an action under the given pad classification.
/// Today this is a static table that matches the bindings authored
/// in `KeyboardPreset::input_map`; if those bindings change the
/// table here needs to follow.
fn gamepad_glyph(action: SandboxAction, kind: GamepadKind) -> &'static str {
    // Face button glyphs vary by vendor; everything else is shared.
    let (south, east, west, north) = match kind {
        GamepadKind::XboxLike | GamepadKind::Generic => ("A", "B", "X", "Y"),
        // PlayStation labels rendered as ASCII fallbacks until icon
        // assets land — the shape names are unambiguous.
        GamepadKind::PlayStation => ("Cross", "Circle", "Square", "Triangle"),
        // Switch: the *physical* labels match Xbox letters but the
        // positions of B/A are mirrored — render the physical label
        // matching position (south = "B" on Switch, east = "A").
        GamepadKind::Switch => ("B", "A", "Y", "X"),
    };
    let (lb, rb, lt, rt) = match kind {
        GamepadKind::PlayStation => ("L1", "R1", "L2", "R2"),
        _ => ("LB", "RB", "LT", "RT"),
    };
    let (select, start) = match kind {
        GamepadKind::PlayStation => ("Share", "Options"),
        GamepadKind::Switch => ("-", "+"),
        _ => ("Back", "Start"),
    };
    match action {
        SandboxAction::Move
        | SandboxAction::MoveLeft
        | SandboxAction::MoveRight
        | SandboxAction::MoveUp
        | SandboxAction::MoveDown
        | SandboxAction::MenuStick => "L-Stick",
        SandboxAction::Jump => south,
        SandboxAction::Attack => west,
        SandboxAction::Dash => rt,
        SandboxAction::Blink => east,
        SandboxAction::QuickAction => rb,
        SandboxAction::Interact => rb,
        SandboxAction::Modifier => lt,
        SandboxAction::Utility => lb,
        SandboxAction::Projectile => north,
        SandboxAction::TrailToggle => "",
        SandboxAction::Map => "L3",
        SandboxAction::Inventory => "R3",
        SandboxAction::Pogo => "D+X",
        SandboxAction::Reset => select,
        SandboxAction::Start => start,
        SandboxAction::MenuNavigateUp
        | SandboxAction::MenuNavigateDown
        | SandboxAction::MenuNavigateLeft
        | SandboxAction::MenuNavigateRight => "D-Pad",
        SandboxAction::MenuSelect => south,
        SandboxAction::MenuBack => east,
        SandboxAction::MenuPageLeft => lb,
        SandboxAction::MenuPageRight => rb,
        SandboxAction::DashAnalog | SandboxAction::AimStick => "R-Stick",
    }
}

/// Single-character glyph for the most common keyboard keys used in
/// touch-button subtitles. Duplicates a small subset of the
/// `KeyboardPreset::key_name` table because the preset module keeps
/// it private (#fn key_name); rather than expose that helper across
/// crates we keep our own narrow copy here.
fn key_glyph(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::ArrowLeft => "←",
        KeyCode::ArrowRight => "→",
        KeyCode::ArrowUp => "↑",
        KeyCode::ArrowDown => "↓",
        KeyCode::Space => "Space",
        KeyCode::ShiftLeft => "Shift",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Del",
        KeyCode::Backspace => "Bksp",
        KeyCode::Enter => "Enter",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeyboardPreset;

    #[test]
    fn keyboard_glyph_follows_active_preset() {
        let arrows_zxc = KeyboardPreset::arrows_zxc();
        // Arrows+ZXC: Jump = Z, Attack = X, Dash = C.
        assert_eq!(
            glyph_for(SandboxAction::Jump, &arrows_zxc, InputMethod::Keyboard),
            "Z"
        );
        assert_eq!(
            glyph_for(SandboxAction::Attack, &arrows_zxc, InputMethod::Keyboard),
            "X"
        );
        assert_eq!(
            glyph_for(SandboxAction::Dash, &arrows_zxc, InputMethod::Keyboard),
            "C"
        );

        let wasd = KeyboardPreset::wasd_jkl();
        // WASD: Jump = Space, Attack = J, Dash = K.
        assert_eq!(
            glyph_for(SandboxAction::Jump, &wasd, InputMethod::Keyboard),
            "Space"
        );
        assert_eq!(
            glyph_for(SandboxAction::Attack, &wasd, InputMethod::Keyboard),
            "J"
        );
        assert_eq!(
            glyph_for(SandboxAction::Dash, &wasd, InputMethod::Keyboard),
            "K"
        );
    }

    #[test]
    fn gamepad_glyph_switches_face_buttons_by_kind() {
        let preset = KeyboardPreset::arrows_zxc(); // keyboard preset unused for gamepad path
        assert_eq!(
            glyph_for(
                SandboxAction::Jump,
                &preset,
                InputMethod::Gamepad(GamepadKind::XboxLike)
            ),
            "A"
        );
        assert_eq!(
            glyph_for(
                SandboxAction::Jump,
                &preset,
                InputMethod::Gamepad(GamepadKind::PlayStation)
            ),
            "Cross"
        );
        assert_eq!(
            glyph_for(
                SandboxAction::Attack,
                &preset,
                InputMethod::Gamepad(GamepadKind::XboxLike)
            ),
            "X"
        );
        assert_eq!(
            glyph_for(
                SandboxAction::Attack,
                &preset,
                InputMethod::Gamepad(GamepadKind::PlayStation)
            ),
            "Square"
        );
    }

    #[test]
    fn touch_glyph_is_empty() {
        let preset = KeyboardPreset::arrows_zxc();
        assert_eq!(
            glyph_for(SandboxAction::Jump, &preset, InputMethod::Touch),
            ""
        );
        assert_eq!(
            glyph_for(SandboxAction::Attack, &preset, InputMethod::Touch),
            ""
        );
    }
}
