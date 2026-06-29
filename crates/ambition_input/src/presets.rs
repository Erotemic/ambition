//! Default binding presets: the selectable keyboard layouts (`PresetId` /
//! `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad map
//! (`GAMEPAD_MAP`) that seed leafwing's input map for `SandboxAction`.

use super::*;

/// Minimum magnitude on a stick axis before a
/// `GamepadControlDirection` binding registers as "pressed." Suppresses
/// spring-return overshoot — releasing the left stick from a deep
/// downward push bounces briefly positive on the Y axis; without this
/// threshold leafwing fires a `MoveUp` press the same frame and any
/// downstream double-tap-down → MorphBall flow exits the moment it
/// entered. 0.5 is comfortably past the typical overshoot (~0.1) while
/// still triggering on a deliberate stick push at half-deflection.
#[cfg(feature = "input")]
const STICK_DIRECTION_THRESHOLD: f32 = 0.5;

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
    pub secondary: KeyCode,
    pub quick_action: KeyCode,
    pub interact: KeyCode,
    pub modifier: KeyCode,
    pub utility: KeyCode,
    pub map: KeyCode,
    pub inventory: KeyCode,
    pub projectile: KeyCode,
    pub trail_toggle: KeyCode,
    pub pause: KeyCode,
    pub select_reset: KeyCode,
    /// Optional dedicated pogo key. When `None`, pogo falls back to
    /// the down+attack combo and `action_label` shows "Pogo Down+Attack".
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

    /// Resolve the preset at `index` (the value stored in
    /// `settings.controls.keyboard_preset_index`). Out-of-range
    /// indices fall back to the first preset (`arrows_zxc`) so a stale
    /// or corrupt setting can never panic a HUD/glyph system.
    pub fn by_index(index: usize) -> Self {
        let presets = Self::presets();
        presets.get(index).copied().unwrap_or(presets[0])
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
                secondary: KeyCode::KeyA,
                quick_action: KeyCode::KeyE,
                interact: KeyCode::KeyF,
                modifier: KeyCode::KeyS,
                utility: KeyCode::KeyD,
                map: KeyCode::Tab,
                inventory: KeyCode::KeyI,
                projectile: KeyCode::KeyV,
                trail_toggle: KeyCode::KeyB,
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
                secondary: KeyCode::KeyL,
                quick_action: KeyCode::KeyI,
                interact: KeyCode::KeyE,
                modifier: KeyCode::ShiftLeft,
                utility: KeyCode::KeyU,
                map: KeyCode::Tab,
                inventory: KeyCode::KeyV,
                projectile: KeyCode::KeyH,
                trail_toggle: KeyCode::KeyB,
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
                secondary: KeyCode::KeyR,
                quick_action: KeyCode::KeyT,
                interact: KeyCode::KeyF,
                modifier: KeyCode::ShiftLeft,
                utility: KeyCode::KeyG,
                map: KeyCode::Tab,
                inventory: KeyCode::KeyI,
                projectile: KeyCode::KeyV,
                trail_toggle: KeyCode::KeyB,
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
                secondary: KeyCode::KeyO,
                quick_action: KeyCode::KeyJ,
                interact: KeyCode::KeyE,
                modifier: KeyCode::ShiftLeft,
                utility: KeyCode::KeyK,
                map: KeyCode::Tab,
                inventory: KeyCode::KeyV,
                projectile: KeyCode::KeyL,
                trail_toggle: KeyCode::KeyB,
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
    /// into the same shape instead of rewriting gameplay systems. Gated
    /// behind `input` because the return type is leafwing-owned.
    #[cfg(feature = "input")]
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
            // Gamepad bindings for the discrete `MoveX` actions. Without
            // these, `actions.just_pressed(&SandboxAction::MoveDown)`
            // never fires on a controller — the double-tap-down gesture
            // that enters MorphBall was keyboard-only as a result. Both
            // the DPad and a stick-direction cross past the deadzone
            // generate the same press edge, so DPad → MorphBall feels
            // the same as Down-Arrow → MorphBall.
            //
            // `STICK_DIRECTION_THRESHOLD` keeps spring-return overshoot
            // from registering as a press in the *opposite* direction.
            // After pushing the left stick down and releasing, real
            // hardware briefly snaps positive on the Y axis; without a
            // threshold leafwing's `LEFT_UP` direction (which defaults
            // to `threshold = 0.0`) fires a `MoveUp` press edge, and
            // that edge exits MorphBall the same frame the player
            // entered it.
            .with(SandboxAction::MoveLeft, GamepadButton::DPadLeft)
            .with(
                SandboxAction::MoveLeft,
                GamepadControlDirection::LEFT_LEFT.threshold(STICK_DIRECTION_THRESHOLD),
            )
            .with(SandboxAction::MoveRight, GamepadButton::DPadRight)
            .with(
                SandboxAction::MoveRight,
                GamepadControlDirection::LEFT_RIGHT.threshold(STICK_DIRECTION_THRESHOLD),
            )
            .with(SandboxAction::MoveUp, GamepadButton::DPadUp)
            .with(
                SandboxAction::MoveUp,
                GamepadControlDirection::LEFT_UP.threshold(STICK_DIRECTION_THRESHOLD),
            )
            .with(SandboxAction::MoveDown, GamepadButton::DPadDown)
            .with(
                SandboxAction::MoveDown,
                GamepadControlDirection::LEFT_DOWN.threshold(STICK_DIRECTION_THRESHOLD),
            )
            .with(SandboxAction::Jump, self.actions.jump)
            .with(SandboxAction::Jump, GamepadButton::South)
            .with(SandboxAction::Attack, self.actions.attack)
            .with(SandboxAction::Attack, GamepadButton::West)
            .with(SandboxAction::Dash, self.actions.dash)
            .with(SandboxAction::Dash, GamepadButton::RightTrigger2)
            .with(SandboxAction::Reset, self.actions.select_reset)
            .with(SandboxAction::Reset, KeyCode::Delete)
            .with(SandboxAction::Reset, GamepadButton::Select)
            .with(SandboxAction::Start, self.actions.pause)
            .with(SandboxAction::Start, GamepadButton::Start);

        map.insert(SandboxAction::Blink, self.actions.secondary);
        map.insert(SandboxAction::QuickAction, self.actions.quick_action);
        map.insert(SandboxAction::Interact, self.actions.interact);
        map.insert(SandboxAction::Modifier, self.actions.modifier);
        map.insert(SandboxAction::Utility, self.actions.utility);
        map.insert(SandboxAction::Map, self.actions.map);
        map.insert(SandboxAction::Inventory, self.actions.inventory);
        map.insert(SandboxAction::Projectile, self.actions.projectile);
        map.insert(SandboxAction::TrailToggle, self.actions.trail_toggle);
        insert_optional(&mut map, SandboxAction::Pogo, self.actions.dedicated_pogo);

        // Gamepad bindings. Every action has a button so both input modes
        // are fully playable.
        //   South        Jump
        //   East         Blink, MenuBack
        //   West         Attack
        //   North        Projectile (fireball)
        //   LeftTrigger  Utility (fly toggle)
        //   LeftTrigger2 Modifier
        //   RightTrigger QuickAction, Interact
        //   RightTrigger2 Dash
        //   LeftThumb    Map (click left stick)
        //   RightThumb   Inventory (click right stick)
        //   Select       Reset
        //   Start        Start (pause)
        //   DPad / sticks  Move + MenuNavigate, MenuStick, AimStick
        map.insert(SandboxAction::Blink, GamepadButton::East);
        map.insert(SandboxAction::QuickAction, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Interact, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Modifier, GamepadButton::LeftTrigger2);
        map.insert(SandboxAction::Utility, GamepadButton::LeftTrigger);
        map.insert(SandboxAction::Map, GamepadButton::LeftThumb);
        map.insert(SandboxAction::Inventory, GamepadButton::RightThumb);
        map.insert(SandboxAction::Projectile, GamepadButton::North);

        // Menu navigation seam. Cardinal/D-pad/arrow keys all hit the
        // same MenuNavigate* actions; analog stick provides MenuStick
        // for repeat handling, and Enter/Space/South map to MenuSelect.
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::ArrowUp);
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::KeyW);
        map.insert(SandboxAction::MenuNavigateUp, GamepadButton::DPadUp);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::ArrowDown);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::KeyS);
        map.insert(SandboxAction::MenuNavigateDown, GamepadButton::DPadDown);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::ArrowLeft);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::KeyA);
        map.insert(SandboxAction::MenuNavigateLeft, GamepadButton::DPadLeft);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::ArrowRight);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::KeyD);
        map.insert(SandboxAction::MenuNavigateRight, GamepadButton::DPadRight);

        map.insert(SandboxAction::MenuSelect, KeyCode::Enter);
        map.insert(SandboxAction::MenuSelect, KeyCode::NumpadEnter);
        map.insert(SandboxAction::MenuSelect, KeyCode::Space);
        map.insert(SandboxAction::MenuSelect, GamepadButton::South);
        // Also accept the player's configured Jump key as confirm so
        // existing muscle memory still works, but Enter is the
        // canonical menu confirmation.
        map.insert(SandboxAction::MenuSelect, self.actions.jump);

        map.insert(SandboxAction::MenuBack, KeyCode::Escape);
        map.insert(SandboxAction::MenuBack, KeyCode::Backspace);
        map.insert(SandboxAction::MenuBack, GamepadButton::East);

        // Paged-menu page turn: the L/R shoulder bumpers (L1/LB, R1/RB) turn the
        // page directly in the 3D inventory cube, plus the Q/E keyboard equivalents.
        // The bumpers double as gameplay Utility/QuickAction, but menu page actions
        // are only read while a paged menu is open, so the physical button is shared
        // safely. `MoveLeft`/`MoveRight` already own A/D, so paging uses Q/E.
        map.insert(SandboxAction::MenuPageLeft, GamepadButton::LeftTrigger);
        map.insert(SandboxAction::MenuPageLeft, KeyCode::KeyQ);
        map.insert(SandboxAction::MenuPageRight, GamepadButton::RightTrigger);
        map.insert(SandboxAction::MenuPageRight, KeyCode::KeyE);

        map.insert_dual_axis(SandboxAction::MenuStick, GamepadStick::LEFT);
        map.insert_dual_axis(SandboxAction::AimStick, GamepadStick::RIGHT);
        // RIGHT_Z is the analog right-trigger axis on most pads.
        // Reading it as an axis lets us apply hysteresis ourselves
        // instead of relying on the binary just_pressed edge.
        map.insert_axis(SandboxAction::DashAnalog, GamepadControlAxis::RIGHT_Z);
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
        for (label, key) in [
            ("Blink", self.actions.secondary),
            ("Quick", self.actions.quick_action),
            ("Interact", self.actions.interact),
            ("Modifier", self.actions.modifier),
            ("Fly", self.actions.utility),
            ("Fireball", self.actions.projectile),
            ("Trail", self.actions.trail_toggle),
            ("Map", self.actions.map),
            ("Inventory", self.actions.inventory),
            ("Select", self.actions.select_reset),
        ] {
            parts.push(format!("{} {}", label, key_name(key)));
        }
        parts.join("  |  ")
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

#[cfg(feature = "input")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_index_resolves_each_preset_and_clamps_out_of_range() {
        // Each in-range index returns the matching preset (order must
        // stay aligned with `settings.controls.keyboard_preset_index`).
        assert_eq!(KeyboardPreset::by_index(0).id, PresetId::ArrowsZxc);
        assert_eq!(KeyboardPreset::by_index(1).id, PresetId::WasdJkl);
        assert_eq!(KeyboardPreset::by_index(2).id, PresetId::ArrowsQwer);
        assert_eq!(KeyboardPreset::by_index(3).id, PresetId::WasdUipo);
        // A stale / corrupt index falls back to the first preset
        // rather than panicking a HUD glyph system.
        assert_eq!(KeyboardPreset::by_index(4).id, PresetId::ArrowsZxc);
        assert_eq!(KeyboardPreset::by_index(usize::MAX).id, PresetId::ArrowsZxc);
    }
}
