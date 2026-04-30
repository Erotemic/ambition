//! Keyboard/gamepad semantic input model for the sandbox.
//!
//! `KeyboardPreset` is deliberately data-like: each preset maps physical keys to
//! generic Ambition actions. Systems should consume `ControlFrame`, not hard-code
//! key names or layout assumptions.

use ambition_engine as ae;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetId {
    ArrowsZxc,
    WasdJkl,
    ArrowsQwer,
    WasdUipo,
}

#[derive(Clone, Copy, Debug)]
pub struct MovementKeys {
    pub left: KeyCode,
    pub right: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
}

#[derive(Clone, Copy, Debug)]
pub struct ActionKeys {
    pub jump: KeyCode,
    pub attack: KeyCode,
    pub dash: KeyCode,
    pub secondary: Option<KeyCode>,
    pub quick_action: Option<KeyCode>,
    pub modifier: Option<KeyCode>,
    pub utility: Option<KeyCode>,
    pub map: Option<KeyCode>,
    pub inventory: Option<KeyCode>,
    pub pause: KeyCode,
    pub select_reset: KeyCode,
    pub dedicated_pogo: Option<KeyCode>,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyboardPreset {
    pub id: PresetId,
    pub name: &'static str,
    pub movement: MovementKeys,
    pub actions: ActionKeys,
}

impl KeyboardPreset {
    pub fn presets() -> [Self; 4] {
        [
            Self::arrows_zxc(),
            Self::wasd_jkl(),
            Self::arrows_qwer(),
            Self::wasd_uipo(),
        ]
    }

    pub fn arrows_zxc() -> Self {
        Self {
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
                attack: KeyCode::KeyX,
                dash: KeyCode::KeyC,
                secondary: Some(KeyCode::KeyA),
                quick_action: Some(KeyCode::KeyE),
                modifier: Some(KeyCode::KeyS),
                utility: Some(KeyCode::KeyD),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn wasd_jkl() -> Self {
        Self {
            id: PresetId::WasdJkl,
            name: "custom PC: WASD + Space/J/K/L/I/U",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::Space,
                attack: KeyCode::KeyJ,
                dash: KeyCode::KeyK,
                secondary: Some(KeyCode::KeyL),
                quick_action: Some(KeyCode::KeyI),
                modifier: Some(KeyCode::ShiftLeft),
                utility: Some(KeyCode::KeyU),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn arrows_qwer() -> Self {
        Self {
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
                dash: KeyCode::KeyW,
                attack: KeyCode::KeyE,
                secondary: Some(KeyCode::KeyR),
                quick_action: None,
                modifier: None,
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn wasd_uipo() -> Self {
        Self {
            id: PresetId::WasdUipo,
            name: "chirality B: WASD + UIPO",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyU,
                dash: KeyCode::KeyI,
                attack: KeyCode::KeyP,
                secondary: Some(KeyCode::KeyO),
                quick_action: None,
                modifier: None,
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn movement_label(&self) -> &'static str {
        match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => "Arrow keys",
            PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
        }
    }

    pub fn action_label(&self) -> String {
        let mut parts = vec![
            format!("Jump {}", key_name(self.actions.jump)),
            format!("Attack {}", key_name(self.actions.attack)),
            format!("Dash {}", key_name(self.actions.dash)),
        ];
        if let Some(k) = self.actions.dedicated_pogo {
            parts.push(format!("Pogo {}", key_name(k)));
        } else {
            parts.push("Pogo Down+Attack".to_string());
        }
        let optional = [
            ("Blink", self.actions.secondary),
            ("Quick", self.actions.quick_action),
            ("Modifier", self.actions.modifier),
            ("Utility", self.actions.utility),
            ("Map", self.actions.map),
            ("Inventory", self.actions.inventory),
            ("Select", Some(self.actions.select_reset)),
        ];
        for (label, key) in optional {
            if let Some(k) = key {
                parts.push(format!("{} {}", label, key_name(k)));
            }
        }
        parts.join("  |  ")
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControlFrame {
    pub axis_x: f32,
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    /// Movement-down key was newly pressed this frame. The sandbox uses this
    /// to recognize double-tap-down for fast-fall without making down+attack
    /// automatically fast-fall.
    pub down_pressed: bool,
    /// Double-tap-down recognized by the sandbox input gesture detector.
    pub fast_fall_pressed: bool,
    pub blink_pressed: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub attack_pressed: bool,
    pub pogo_pressed: bool,
    pub reset_pressed: bool,
    pub start_pressed: bool,
}

impl ControlFrame {
    pub fn read(keys: &ButtonInput<KeyCode>, preset: KeyboardPreset) -> Self {
        let mut axis_x = 0.0;
        let mut axis_y = 0.0;
        if keys.pressed(preset.movement.left) {
            axis_x -= 1.0;
        }
        if keys.pressed(preset.movement.right) {
            axis_x += 1.0;
        }
        if keys.pressed(preset.movement.up) {
            axis_y -= 1.0;
        }
        if keys.pressed(preset.movement.down) {
            axis_y += 1.0;
        }
        let down_pressed = keys.just_pressed(preset.movement.down);
        let blink_key = preset.actions.secondary;
        let blink_pressed = blink_key.map(|key| keys.just_pressed(key)).unwrap_or(false);
        let blink_held = blink_key.map(|key| keys.pressed(key)).unwrap_or(false);
        let blink_released = blink_key.map(|key| keys.just_released(key)).unwrap_or(false);
        let reset_pressed = keys.just_pressed(preset.actions.select_reset)
            || keys.just_pressed(KeyCode::Delete)
            || keys.just_pressed(KeyCode::Backspace);
        Self {
            axis_x,
            axis_y,
            jump_pressed: keys.just_pressed(preset.actions.jump),
            jump_held: keys.pressed(preset.actions.jump),
            jump_released: keys.just_released(preset.actions.jump),
            dash_pressed: keys.just_pressed(preset.actions.dash),
            down_pressed,
            fast_fall_pressed: false,
            blink_pressed,
            blink_held,
            blink_released,
            attack_pressed: keys.just_pressed(preset.actions.attack),
            pogo_pressed: preset
                .actions
                .dedicated_pogo
                .map(|key| keys.just_pressed(key))
                .unwrap_or(false),
            reset_pressed,
            start_pressed: keys.just_pressed(preset.actions.pause),
        }
    }

    pub fn engine_input(self, control_dt: f32) -> ae::InputState {
        ae::InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            blink_pressed: self.blink_pressed,
            blink_held: self.blink_held,
            blink_released: self.blink_released,
            fast_fall_pressed: self.fast_fall_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            reset_pressed: false,
            control_dt,
        }
    }
}

pub const GAMEPAD_MAP: &[(&str, &str)] = &[
    ("L-stick / D-pad", "movement"),
    ("A / Cross", "jump / confirm"),
    ("X / Square", "primary attack"),
    ("RT / R2", "dash"),
    ("B / Circle", "blink / special"),
    ("RB / R1", "quick action placeholder"),
    ("LT / L2", "modifier placeholder"),
    ("Y / Triangle", "utility action placeholder"),
    ("LB / L1", "map placeholder"),
    ("Back / Touchpad", "inventory or sandbox reset"),
    ("Start / Options", "pause / menu"),
];

fn key_name(key: KeyCode) -> &'static str {
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
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::Space => "Space",
        KeyCode::ShiftLeft => "LShift",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Delete",
        KeyCode::Backspace => "Backspace",
        _ => "?",
    }
}
