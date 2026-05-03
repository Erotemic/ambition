//! Keyboard/gamepad semantic input model for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine still consumes a compact `ControlFrame`, which keeps movement
//! physics independent from keyboards, gamepads, UI rebinding, or replay input.

use ambition_engine as ae;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

/// Logical player/sandbox inputs understood by the Bevy adapter layer.
///
/// `Move` is dual-axis so analog sticks and virtual D-pads can feed a single
/// movement vector. The cardinal `Move*` button actions intentionally duplicate
/// the directional bindings so systems can still detect edge-triggered gestures
/// such as double-tap-down fast fall and double-tap-up door activation.
#[derive(Actionlike, Clone, Copy, Debug, Hash, PartialEq, Eq, Reflect)]
pub enum SandboxAction {
    #[actionlike(DualAxis)]
    Move,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Attack,
    Dash,
    Blink,
    QuickAction,
    Interact,
    Modifier,
    Utility,
    Map,
    Inventory,
    Pogo,
    Reset,
    Start,
}

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
    pub interact: Option<KeyCode>,
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
                interact: Some(KeyCode::KeyF),
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
                interact: Some(KeyCode::KeyE),
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
                interact: Some(KeyCode::KeyF),
                modifier: Some(KeyCode::ShiftLeft),
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
                interact: Some(KeyCode::KeyE),
                modifier: Some(KeyCode::ShiftLeft),
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

    /// Build a fresh Leafwing `InputMap` for this preset.
    ///
    /// Preset cycling swaps this component on the player entity. Keeping the
    /// preset as data means later TOML/RON keybinding config can deserialize
    /// into the same shape instead of rewriting gameplay systems.
    pub fn input_map(&self) -> InputMap<SandboxAction> {
        let keyboard_move = match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => VirtualDPad::arrow_keys(),
            PresetId::WasdJkl | PresetId::WasdUipo => VirtualDPad::wasd(),
        };

        let mut map = InputMap::default()
            .with_dual_axis(SandboxAction::Move, keyboard_move)
            .with_dual_axis(SandboxAction::Move, VirtualDPad::dpad())
            .with_dual_axis(SandboxAction::Move, GamepadStick::LEFT)
            .with(SandboxAction::MoveLeft, self.movement.left)
            .with(SandboxAction::MoveRight, self.movement.right)
            .with(SandboxAction::MoveUp, self.movement.up)
            .with(SandboxAction::MoveDown, self.movement.down)
            .with(SandboxAction::Jump, self.actions.jump)
            .with(SandboxAction::Jump, GamepadButton::South)
            .with(SandboxAction::Attack, self.actions.attack)
            .with(SandboxAction::Attack, GamepadButton::West)
            .with(SandboxAction::Dash, self.actions.dash)
            .with(SandboxAction::Dash, GamepadButton::RightTrigger2)
            .with(SandboxAction::Reset, self.actions.select_reset)
            .with(SandboxAction::Reset, KeyCode::Delete)
            .with(SandboxAction::Reset, KeyCode::Backspace)
            .with(SandboxAction::Reset, GamepadButton::Select)
            .with(SandboxAction::Start, self.actions.pause)
            .with(SandboxAction::Start, GamepadButton::Start);

        insert_optional(&mut map, SandboxAction::Blink, self.actions.secondary);
        insert_optional(
            &mut map,
            SandboxAction::QuickAction,
            self.actions.quick_action,
        );
        insert_optional(&mut map, SandboxAction::Interact, self.actions.interact);
        insert_optional(&mut map, SandboxAction::Modifier, self.actions.modifier);
        insert_optional(&mut map, SandboxAction::Utility, self.actions.utility);
        insert_optional(&mut map, SandboxAction::Map, self.actions.map);
        insert_optional(&mut map, SandboxAction::Inventory, self.actions.inventory);
        insert_optional(&mut map, SandboxAction::Pogo, self.actions.dedicated_pogo);

        map.insert(SandboxAction::Blink, GamepadButton::East);
        map.insert(SandboxAction::QuickAction, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Interact, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Modifier, GamepadButton::LeftTrigger2);
        map.insert(SandboxAction::Utility, GamepadButton::North);
        map.insert(SandboxAction::Map, GamepadButton::LeftTrigger);
        map.insert(SandboxAction::Inventory, GamepadButton::Select);
        map
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
            ("Interact", self.actions.interact),
            ("Modifier", self.actions.modifier),
            ("Fly", self.actions.utility),
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
    /// Movement-up input was newly pressed this frame. The sandbox uses this
    /// to require a double-tap-up door activation while flying, so upward
    /// flight does not accidentally enter doors.
    pub up_pressed: bool,
    /// Movement-down input was newly pressed this frame. The sandbox uses this
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
    pub fly_toggle_pressed: bool,
    /// Generic context interaction. This is a dedicated interact action plus
    /// the sandbox double-tap-up gesture, not raw held/up movement.
    pub interact_pressed: bool,
    pub reset_pressed: bool,
    pub start_pressed: bool,
}

impl ControlFrame {
    pub fn read_gameplay(actions: &ActionState<SandboxAction>) -> Self {
        let mut axis = actions.clamped_axis_pair(&SandboxAction::Move);
        // Walk modifier: Shift on keyboard, LT2 on gamepad. Cardinal /
        // D-pad input arrives at unit magnitude (run); the modifier caps
        // the move vector so digital input becomes walk speed. Analog
        // sticks already pace via magnitude — capping at WALK_FACTOR
        // only kicks in when the stick is pushed past walk speed, so
        // LT2 acts as a "max-speed governor" for stick users while
        // letting them still creep slowly without the modifier.
        if actions.pressed(&SandboxAction::Modifier) {
            const WALK_FACTOR: f32 = 0.45;
            let magnitude = axis.length();
            if magnitude > WALK_FACTOR {
                axis = axis * (WALK_FACTOR / magnitude);
            }
        }
        let up_pressed = actions.just_pressed(&SandboxAction::MoveUp);
        let down_pressed = actions.just_pressed(&SandboxAction::MoveDown);
        Self {
            axis_x: axis.x,
            // Ambition's simulation uses screen-space world coordinates: +Y is
            // downward. Leafwing's virtual D-pads use the usual +Y-up convention.
            axis_y: -axis.y,
            jump_pressed: actions.just_pressed(&SandboxAction::Jump),
            jump_held: actions.pressed(&SandboxAction::Jump),
            jump_released: actions.just_released(&SandboxAction::Jump),
            dash_pressed: actions.just_pressed(&SandboxAction::Dash),
            up_pressed,
            down_pressed,
            fast_fall_pressed: false,
            blink_pressed: actions.just_pressed(&SandboxAction::Blink),
            blink_held: actions.pressed(&SandboxAction::Blink),
            blink_released: actions.just_released(&SandboxAction::Blink),
            attack_pressed: actions.just_pressed(&SandboxAction::Attack),
            pogo_pressed: actions.just_pressed(&SandboxAction::Pogo),
            fly_toggle_pressed: actions.just_pressed(&SandboxAction::Utility),
            interact_pressed: actions.just_pressed(&SandboxAction::Interact),
            reset_pressed: actions.just_pressed(&SandboxAction::Reset),
            start_pressed: actions.just_pressed(&SandboxAction::Start),
        }
    }

    pub fn read_menu(actions: &ActionState<SandboxAction>) -> Self {
        Self {
            start_pressed: actions.just_pressed(&SandboxAction::Start),
            ..default()
        }
    }

    pub fn engine_input(self, control_dt: f32) -> ae::InputState {
        // Down held + jump just-pressed is the explicit "drop through one-way"
        // gesture. Holding down alone no longer drops the player through.
        let drop_through_pressed = self.axis_y > 0.35 && self.jump_pressed;
        ae::InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            fly_toggle_pressed: self.fly_toggle_pressed,
            blink_pressed: self.blink_pressed,
            blink_held: self.blink_held,
            blink_released: self.blink_released,
            fast_fall_pressed: self.fast_fall_pressed,
            drop_through_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            reset_pressed: false,
            control_dt,
        }
    }
}

pub const GAMEPAD_MAP: &[(&str, &str)] = &[
    ("L-stick / D-pad", "movement / aim"),
    ("A / Cross", "jump / confirm"),
    ("X / Square", "primary attack"),
    ("RT / R2", "dash"),
    ("B / Circle", "blink / special"),
    ("RB / R1", "interact / quick action"),
    ("LT / L2", "modifier placeholder"),
    ("Y / Triangle", "fly toggle / utility"),
    ("LB / L1", "map placeholder"),
    ("Back / Touchpad", "inventory or sandbox reset"),
    ("Start / Options", "pause / menu"),
];

fn insert_optional(map: &mut InputMap<SandboxAction>, action: SandboxAction, key: Option<KeyCode>) {
    if let Some(key) = key {
        map.insert(action, key);
    }
}

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
