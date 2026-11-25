//! Shared input opinions. These are ordinary data declarations; games can replace them.

use ambition_input::{bind, ActionKeys, AxisBinding, AxisSource, Binding, BindingDefaults, BindingScheme, InputAction as Action, InputProfile as Context, KeyboardPreset, MovementKeys, OverrideControl, PresetId};
use bevy::prelude::{GamepadButton as Pad, KeyCode as Key};
use bevy::prelude::KeyCode;

const fn key(context: Context, control: Key, action: Action) -> Binding {
    bind(context, OverrideControl::Key(control), action)
}
const fn pad(context: Context, control: Pad, action: Action) -> Binding {
    bind(context, OverrideControl::Button(control), action)
}

// Session controls are not inferred from gameplay controls.
const KEYBOARD_SESSION: &[Binding] = &[
    key(Context::Gameplay,  Key::Enter,       Action::Start),
    key(Context::Gameplay,  Key::NumpadEnter, Action::Start),
    key(Context::Gameplay,  Key::Escape,      Action::Start),
    key(Context::Gameplay,  Key::KeyI,        Action::Inventory),
    key(Context::Gameplay,  Key::KeyM,        Action::Map),
    key(Context::Gameplay,  Key::Delete,      Action::Reset),
    key(Context::Gameplay,  Key::Home,        Action::PartyReturn),
    key(Context::SessionUi, Key::Escape,      Action::Start),
    key(Context::SessionUi, Key::Enter,       Action::MenuSelect),
    key(Context::SessionUi, Key::NumpadEnter, Action::MenuSelect),
    key(Context::SessionUi, Key::Backspace,   Action::MenuBack),
    key(Context::SessionUi, Key::KeyI,        Action::Inventory),
    key(Context::SessionUi, Key::KeyM,        Action::Map),
    key(Context::Paused,    Key::Enter,       Action::MenuSelect),
    key(Context::Paused,    Key::NumpadEnter, Action::MenuSelect),
    key(Context::Paused,    Key::Escape,      Action::MenuBack),
    key(Context::Paused,    Key::Backspace,   Action::MenuBack),
    key(Context::Paused,    Key::KeyI,        Action::Inventory),
    key(Context::Paused,    Key::KeyM,        Action::Map),
    key(Context::Frontend,  Key::Enter,       Action::MenuSelect),
    key(Context::Frontend,  Key::NumpadEnter, Action::MenuSelect),
    key(Context::Frontend,  Key::Escape,      Action::MenuBack),
    key(Context::Frontend,  Key::Backspace,   Action::MenuBack),
];

const GAMEPAD_GAMEPLAY: &[Binding] = &[
    pad(Context::Gameplay, Pad::South,         Action::Jump),
    pad(Context::Gameplay, Pad::East,          Action::Blink),
    pad(Context::Gameplay, Pad::West,          Action::Attack),
    pad(Context::Gameplay, Pad::North,         Action::Projectile),
    pad(Context::Gameplay, Pad::LeftTrigger,   Action::Utility),
    pad(Context::Gameplay, Pad::LeftTrigger2,  Action::Shield),
    pad(Context::Gameplay, Pad::RightTrigger,  Action::Interact),
    pad(Context::Gameplay, Pad::RightTrigger2, Action::Burst),
    pad(Context::Gameplay, Pad::LeftThumb,     Action::Map),
    pad(Context::Gameplay, Pad::RightThumb,    Action::Inventory),
    pad(Context::Gameplay, Pad::Select,        Action::Reset),
    pad(Context::Gameplay, Pad::Start,         Action::Start),
    pad(Context::Gameplay, Pad::DPadLeft,      Action::MoveLeft),
    pad(Context::Gameplay, Pad::DPadRight,     Action::MoveRight),
    pad(Context::Gameplay, Pad::DPadUp,        Action::MoveUp),
    pad(Context::Gameplay, Pad::DPadDown,      Action::MoveDown),
    pad(Context::SessionUi, Pad::Start,        Action::Start),
    pad(Context::SessionUi, Pad::LeftThumb,    Action::Map),
    pad(Context::SessionUi, Pad::RightThumb,   Action::Inventory),
    pad(Context::Paused, Pad::Start,           Action::Start),
    pad(Context::Paused, Pad::LeftThumb,       Action::Map),
    pad(Context::Paused, Pad::RightThumb,      Action::Inventory),
    pad(Context::Frontend, Pad::Start,         Action::Start),
];

// These UI rows apply to each interactive profile, not to gameplay.
const GAMEPAD_UI: &[(Pad, Action)] = &[
    (Pad::South,        Action::MenuSelect),
    (Pad::East,         Action::MenuBack),
    (Pad::LeftTrigger,  Action::MenuPageLeft),
    (Pad::RightTrigger, Action::MenuPageRight),
    (Pad::DPadUp,       Action::MenuNavigateUp),
    (Pad::DPadDown,     Action::MenuNavigateDown),
    (Pad::DPadLeft,     Action::MenuNavigateLeft),
    (Pad::DPadRight,    Action::MenuNavigateRight),
];
const KEYBOARD_UI: &[(Key, Action)] = &[
    (Key::Space,     Action::MenuSelect),
    (Key::PageUp,    Action::MenuPageLeft),
    (Key::PageDown,  Action::MenuPageRight),
    (Key::ArrowUp,   Action::MenuNavigateUp),
    (Key::ArrowDown, Action::MenuNavigateDown),
    (Key::ArrowLeft, Action::MenuNavigateLeft),
    (Key::ArrowRight, Action::MenuNavigateRight),
    (Key::Unidentified(bevy::input::keyboard::NativeKeyCode::Android(4)), Action::MenuBack),
];

// Additional keyboard UI controls are declared per preset. There is no lookup
// through the gameplay or gamepad bindings. Q/E page keys are omitted when
// a preset uses that key for confirm/back (the QWER preset confirms on Q).
const CLASSIC_UI: &[(Key, Action)] = &[
    (Key::KeyZ, Action::MenuSelect), (Key::KeyA, Action::MenuBack),
    (Key::KeyQ, Action::MenuPageLeft), (Key::KeyE, Action::MenuPageRight),
];
const JKL_UI: &[(Key, Action)] = &[
    (Key::KeyL, Action::MenuBack), (Key::KeyQ, Action::MenuPageLeft), (Key::KeyE, Action::MenuPageRight),
    (Key::KeyW, Action::MenuNavigateUp), (Key::KeyS, Action::MenuNavigateDown),
    (Key::KeyA, Action::MenuNavigateLeft), (Key::KeyD, Action::MenuNavigateRight),
];
const QWER_UI: &[(Key, Action)] = &[
    (Key::KeyQ, Action::MenuSelect), (Key::KeyR, Action::MenuBack),
    (Key::KeyE, Action::MenuPageRight),
];
const UIPO_UI: &[(Key, Action)] = &[
    (Key::KeyU, Action::MenuSelect), (Key::KeyO, Action::MenuBack),
    (Key::KeyQ, Action::MenuPageLeft), (Key::KeyE, Action::MenuPageRight),
    (Key::KeyW, Action::MenuNavigateUp), (Key::KeyS, Action::MenuNavigateDown),
    (Key::KeyA, Action::MenuNavigateLeft), (Key::KeyD, Action::MenuNavigateRight),
];

const PAD_AXES: &[AxisBinding] = &[
    AxisBinding::new(AxisSource::Dpad, Action::Move),
    AxisBinding::new(AxisSource::LeftStick, Action::Move),
    AxisBinding::new(AxisSource::LeftStick, Action::MenuStick),
    AxisBinding::new(AxisSource::RightStick, Action::AimStick),
    AxisBinding::new(AxisSource::RightTrigger, Action::BurstAnalog),
    AxisBinding::new(AxisSource::LeftStickLeft, Action::MoveLeft),
    AxisBinding::new(AxisSource::LeftStickRight, Action::MoveRight),
    AxisBinding::new(AxisSource::LeftStickUp, Action::MoveUp),
    AxisBinding::new(AxisSource::LeftStickDown, Action::MoveDown),
];

pub fn presets() -> [KeyboardPreset; 4] { [arrows_zxc(), wasd_jkl(), arrows_qwer(), wasd_uipo()] }
pub fn keyboard_preset(id: PresetId) -> KeyboardPreset {
    match id {
        PresetId::ArrowsZxc => arrows_zxc(), PresetId::WasdJkl => wasd_jkl(),
        PresetId::ArrowsQwer => arrows_qwer(), PresetId::WasdUipo => wasd_uipo(),
    }
}

pub fn defaults() -> BindingDefaults {
    BindingDefaults::new(presets().into_iter().map(|preset| {
        let mut buttons = KEYBOARD_SESSION.to_vec();
        buttons.extend_from_slice(GAMEPAD_GAMEPLAY);
        let a = preset.actions;
        let m = preset.movement;
        for (control, action) in [
            (m.left, Action::MoveLeft), (m.right, Action::MoveRight),
            (m.up, Action::MoveUp), (m.down, Action::MoveDown),
            (a.jump, Action::Jump), (a.short_hop, Action::ShortHop),
            (a.attack, Action::Attack), (a.burst, Action::Burst),
            (a.secondary, Action::Blink), (a.special, Action::Special),
            (a.shield, Action::Shield), (a.grab, Action::Grab),
            (a.taunt, Action::Taunt), (a.interact, Action::Interact),
            (a.shift_layer[0], Action::ShiftLayer), (a.shift_layer[1], Action::ShiftLayer),
            (a.modifier, Action::Modifier), (a.utility, Action::Utility),
            (a.projectile, Action::Projectile), (a.trail_toggle, Action::TrailToggle),
        ] { buttons.push(key(Context::Gameplay, control, action)); }
        if let Some(control) = a.dedicated_pogo { buttons.push(key(Context::Gameplay, control, Action::Pogo)); }
        let additional = match preset.id {
            PresetId::ArrowsZxc => CLASSIC_UI, PresetId::WasdJkl => JKL_UI,
            PresetId::ArrowsQwer => QWER_UI, PresetId::WasdUipo => UIPO_UI,
        };
        for context in [Context::SessionUi, Context::Paused, Context::Frontend] {
            buttons.extend(KEYBOARD_UI.iter().chain(additional).map(|&(control, action)| key(context, control, action)));
            buttons.extend(GAMEPAD_UI.iter().map(|&(control, action)| pad(context, control, action)));
        }
        let movement = match preset.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => AxisSource::ArrowKeys,
            PresetId::WasdJkl | PresetId::WasdUipo => AxisSource::WasdKeys,
        };
        let mut axes = vec![AxisBinding::new(movement, Action::Move)];
        axes.extend_from_slice(PAD_AXES);
        (preset.id, BindingScheme { buttons, axes })
    }).collect())
}

pub fn arrows_zxc() -> KeyboardPreset {
    KeyboardPreset {
        id: PresetId::ArrowsZxc,
        name: "classic action: arrows + Z/X/C",
        movement: MovementKeys {
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
        },
        actions: ActionKeys {
            jump: KeyCode::KeyZ,
            short_hop: KeyCode::KeyQ,
            attack: KeyCode::KeyX,
            burst: KeyCode::KeyC,
            secondary: KeyCode::KeyA,
            special: KeyCode::KeyG,
            shield: KeyCode::KeyE,
            grab: KeyCode::KeyS,
            taunt: KeyCode::KeyT,
            interact: KeyCode::KeyF,
            shift_layer: [KeyCode::ShiftLeft, KeyCode::ShiftRight],
            modifier: KeyCode::KeyR,
            utility: KeyCode::KeyD,
            map: KeyCode::KeyM,
            inventory: KeyCode::KeyI,
            projectile: KeyCode::KeyV,
            trail_toggle: KeyCode::KeyB,
            pause: KeyCode::Enter,
            select_reset: KeyCode::Delete,
            dedicated_pogo: None,
        },
    }
}

pub fn wasd_jkl() -> KeyboardPreset {
    KeyboardPreset {
        id: PresetId::WasdJkl,
        name: "custom PC: WASD + Space/J/K/L/F/U",
        movement: MovementKeys {
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::KeyW,
            down: KeyCode::KeyS,
        },
        actions: ActionKeys {
            jump: KeyCode::Space,
            short_hop: KeyCode::KeyC,
            attack: KeyCode::KeyJ,
            burst: KeyCode::KeyK,
            secondary: KeyCode::KeyL,
            special: KeyCode::KeyG,
            shield: KeyCode::KeyF,
            grab: KeyCode::KeyO,
            taunt: KeyCode::KeyN,
            interact: KeyCode::KeyE,
            shift_layer: [KeyCode::ShiftLeft, KeyCode::ShiftRight],
            modifier: KeyCode::KeyP,
            utility: KeyCode::KeyU,
            map: KeyCode::KeyM,
            inventory: KeyCode::KeyI,
            projectile: KeyCode::KeyH,
            trail_toggle: KeyCode::KeyB,
            pause: KeyCode::Enter,
            select_reset: KeyCode::Delete,
            dedicated_pogo: None,
        },
    }
}

pub fn arrows_qwer() -> KeyboardPreset {
    KeyboardPreset {
        id: PresetId::ArrowsQwer,
        name: "chirality A: arrows + QWER",
        movement: MovementKeys {
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
        },
        actions: ActionKeys {
            jump: KeyCode::KeyQ,
            short_hop: KeyCode::KeyA,
            burst: KeyCode::KeyW,
            attack: KeyCode::KeyE,
            secondary: KeyCode::KeyR,
            special: KeyCode::KeyH,
            shield: KeyCode::KeyT,
            grab: KeyCode::KeyY,
            taunt: KeyCode::KeyU,
            interact: KeyCode::KeyF,
            shift_layer: [KeyCode::ShiftLeft, KeyCode::ShiftRight],
            modifier: KeyCode::KeyS,
            utility: KeyCode::KeyG,
            map: KeyCode::KeyM,
            inventory: KeyCode::KeyI,
            projectile: KeyCode::KeyV,
            trail_toggle: KeyCode::KeyB,
            pause: KeyCode::Enter,
            select_reset: KeyCode::Delete,
            dedicated_pogo: None,
        },
    }
}

pub fn wasd_uipo() -> KeyboardPreset {
    KeyboardPreset {
        id: PresetId::WasdUipo,
        name: "chirality B: WASD + UFPO",
        movement: MovementKeys {
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::KeyW,
            down: KeyCode::KeyS,
        },
        actions: ActionKeys {
            jump: KeyCode::KeyU,
            short_hop: KeyCode::KeyC,
            burst: KeyCode::KeyF,
            attack: KeyCode::KeyP,
            secondary: KeyCode::KeyO,
            special: KeyCode::KeyH,
            shield: KeyCode::KeyJ,
            grab: KeyCode::KeyY,
            taunt: KeyCode::KeyN,
            interact: KeyCode::KeyE,
            shift_layer: [KeyCode::ShiftLeft, KeyCode::ShiftRight],
            modifier: KeyCode::KeyG,
            utility: KeyCode::KeyK,
            map: KeyCode::KeyM,
            inventory: KeyCode::KeyI,
            projectile: KeyCode::KeyL,
            trail_toggle: KeyCode::KeyB,
            pause: KeyCode::Enter,
            select_reset: KeyCode::Delete,
            dedicated_pogo: None,
        },
    }
}


// Synthetic layout for the engine's unit tests; not a game dependency.
#[cfg(test)]
pub fn fixture_layout() -> ambition_input::BindingLayout {
    ambition_input::BindingLayout::new([
        pad(Context::Gameplay, Pad::South, Action::Attack),
        pad(Context::Gameplay, Pad::West, Action::Special),
    ])
}
