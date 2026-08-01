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
    /// Dedicated signature-special key (distinct from `secondary`/Blink).
    pub special: KeyCode,
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
                special: KeyCode::KeyG,
                quick_action: KeyCode::KeyE,
                interact: KeyCode::KeyF,
                // ⚠ **X IS THE CLASSIC B BUTTON: hold to run, press to fire.**
                // Binding `attack` AND `modifier` to the same key is DELIBERATE
                // and is what Jon asked for ("z is jump, and x is hold to run,
                // or if you have the spark blossom it fireballs if you press
                // it"). It reads like a collision and has now been filed as a
                // bug by two separate reviewers. It is not one. Do not "resolve"
                // it by unbinding either action.
                //
                // How it actually works, since the obvious explanation is wrong:
                // the run/fire pair is delivered ENTIRELY by the modifier slot —
                // `modifier_held` is her run, `modifier_pressed` is her spark
                // (see `mary_o::movement`). The `attack` binding on the same key
                // is a SEPARATE semantic action that is simply inert for her.
                // What makes it inert is the ATTACK side: `resolve_control_slots`
                // calls `clear_attack` on a body that declares no melee verb, so
                // `melee_pressed` never survives to a consumer. (A previous
                // version of this comment credited the modifier slot for that
                // gating. It does not gate: its arm has no `None` case and
                // clears nothing.) For Sanic, X resolves to his `spin_dash`
                // technique and his modifier goes unread.
                //
                // The note about gamepad-Special below refuses double-binding
                // for a DIFFERENT reason — an unclaimed slot with no contextual
                // meaning — and does not contradict this. The one composition
                // that would genuinely fire two actions is an actor carrying
                // both a melee verb and a modifier technique; no such actor
                // exists, and that, not the binding, is what would need guarding.
                //
                // It used to be `S`, which is a WASD key on a preset that steers
                // with the arrows: the run button sat in the middle of the
                // keyboard, nowhere near the two buttons the layout is named for.
                modifier: KeyCode::KeyX,
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
                special: KeyCode::KeyG,
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
                special: KeyCode::KeyH,
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
                special: KeyCode::KeyH,
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
        let mut map = InputMap::default();
        self.insert_keyboard_bindings(&mut map);
        insert_gamepad_bindings(&mut map);
        map
    }

    /// A map with the GAMEPAD half only.
    ///
    /// This is what a second local seat gets. Handing player two the full
    /// preset would bind them to the same keyboard player one is using, which
    /// is the couch bug one layer up from the one device assignment fixes:
    /// partitioning the controllers is pointless if WASD still moves both
    /// fighters.
    ///
    /// Not a reduced or "good enough" binding set — it is the same gamepad
    /// bindings player one has, so the two seats are symmetric on the pad and
    /// nobody is playing a worse version of the game because they joined second.
    /// (`Special` has no gamepad button on either seat: every face, shoulder,
    /// trigger and stick button is already assigned, and double-binding one
    /// would fire two actions at once. It is left to the remap UX, as
    /// `special_is_a_dedicated_slot_...` pins.)
    #[cfg(feature = "input")]
    pub fn gamepad_only_map() -> InputMap<SandboxAction> {
        let mut map = InputMap::default();
        insert_gamepad_bindings(&mut map);
        map
    }

    /// The keyboard half of a preset: everything whose binding is a key this
    /// preset chose.
    #[cfg(feature = "input")]
    fn insert_keyboard_bindings(&self, map: &mut InputMap<SandboxAction>) {
        let keyboard_move = match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => VirtualDPad::arrow_keys(),
            PresetId::WasdJkl | PresetId::WasdUipo => VirtualDPad::wasd(),
        };
        map.insert_dual_axis(SandboxAction::Move, keyboard_move);
        map.insert(SandboxAction::MoveLeft, self.movement.left);
        map.insert(SandboxAction::MoveRight, self.movement.right);
        map.insert(SandboxAction::MoveUp, self.movement.up);
        map.insert(SandboxAction::MoveDown, self.movement.down);
        map.insert(SandboxAction::Jump, self.actions.jump);
        map.insert(SandboxAction::Attack, self.actions.attack);
        map.insert(SandboxAction::Dash, self.actions.dash);
        map.insert(SandboxAction::Reset, self.actions.select_reset);
        map.insert(SandboxAction::Reset, KeyCode::Delete);
        map.insert(SandboxAction::Start, self.actions.pause);

        map.insert(SandboxAction::Blink, self.actions.secondary);
        // Special is a FIRST-CLASS slot with its OWN dedicated key on every
        // preset — no longer aliasing Blink. Dynamic-slot policy for the gamepad:
        // every face/shoulder/trigger/stick button is already assigned (see
        // `insert_gamepad_bindings`), so rather than double-bind a button (which
        // would fire TWO actions at once), gamepad-Special is intentionally left
        // to the remap UX (P5). Keyboard (this key) and the touch overlay's
        // dedicated Special button cover it meanwhile.
        // `special_is_a_dedicated_slot_...` pins this policy.
        map.insert(SandboxAction::Special, self.actions.special);
        map.insert(SandboxAction::QuickAction, self.actions.quick_action);
        map.insert(SandboxAction::Interact, self.actions.interact);
        map.insert(SandboxAction::Modifier, self.actions.modifier);
        map.insert(SandboxAction::Utility, self.actions.utility);
        map.insert(SandboxAction::Map, self.actions.map);
        map.insert(SandboxAction::Inventory, self.actions.inventory);
        map.insert(SandboxAction::Projectile, self.actions.projectile);
        map.insert(SandboxAction::TrailToggle, self.actions.trail_toggle);
        insert_optional(map, SandboxAction::Pogo, self.actions.dedicated_pogo);

        // Menu navigation seam. Cardinal/D-pad/arrow keys all hit the
        // same MenuNavigate* actions; the analog stick provides MenuStick
        // for repeat handling, and Enter/Space/South map to MenuSelect.
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::ArrowUp);
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::KeyW);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::ArrowDown);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::KeyS);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::ArrowLeft);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::KeyA);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::ArrowRight);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::KeyD);

        map.insert(SandboxAction::MenuSelect, KeyCode::Enter);
        map.insert(SandboxAction::MenuSelect, KeyCode::NumpadEnter);
        map.insert(SandboxAction::MenuSelect, KeyCode::Space);
        // Also accept the player's configured Jump and Interact keys as
        // confirm so existing dialogue/cutscene muscle memory survives the
        // participant migration. Enter remains the canonical menu confirmation.
        map.insert(SandboxAction::MenuSelect, self.actions.jump);
        map.insert(SandboxAction::MenuSelect, self.actions.interact);

        map.insert(SandboxAction::MenuBack, KeyCode::Escape);
        map.insert(SandboxAction::MenuBack, KeyCode::Backspace);

        // Paged-menu page turn: `MoveLeft`/`MoveRight` already own A/D, so
        // paging uses Q/E.
        map.insert(SandboxAction::MenuPageLeft, KeyCode::KeyQ);
        map.insert(SandboxAction::MenuPageRight, KeyCode::KeyE);
    }
}

/// The gamepad half, identical for every preset and every seat.
///
/// Free-standing rather than a method because it depends on nothing about the
/// preset — the preset chooses KEYS. A second local seat has no preset (it does
/// not use the keyboard at all) and needs exactly this.
///
/// Every action has a button so both input modes are fully playable:
///   South        Jump, MenuSelect
///   East         Blink, MenuBack
///   West         Attack
///   North        Projectile (fireball)
///   LeftTrigger  Utility (fly toggle), MenuPageLeft
///   LeftTrigger2 Modifier
///   RightTrigger QuickAction, Interact, MenuPageRight
///   RightTrigger2 Dash
///   LeftThumb    Map (click left stick)
///   RightThumb   Inventory (click right stick)
///   Select       Reset
///   Start        Start (pause)
///   DPad / sticks  Move + MenuNavigate, MenuStick, AimStick
#[cfg(feature = "input")]
fn insert_gamepad_bindings(map: &mut InputMap<SandboxAction>) {
    map.insert_dual_axis(SandboxAction::Move, VirtualDPad::dpad());
    map.insert_dual_axis(SandboxAction::Move, GamepadStick::LEFT);

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
    map.insert(SandboxAction::MoveLeft, GamepadButton::DPadLeft);
    map.insert(
        SandboxAction::MoveLeft,
        GamepadControlDirection::LEFT_LEFT.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(SandboxAction::MoveRight, GamepadButton::DPadRight);
    map.insert(
        SandboxAction::MoveRight,
        GamepadControlDirection::LEFT_RIGHT.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(SandboxAction::MoveUp, GamepadButton::DPadUp);
    map.insert(
        SandboxAction::MoveUp,
        GamepadControlDirection::LEFT_UP.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(SandboxAction::MoveDown, GamepadButton::DPadDown);
    map.insert(
        SandboxAction::MoveDown,
        GamepadControlDirection::LEFT_DOWN.threshold(STICK_DIRECTION_THRESHOLD),
    );

    map.insert(SandboxAction::Jump, GamepadButton::South);
    map.insert(SandboxAction::Attack, GamepadButton::West);
    map.insert(SandboxAction::Dash, GamepadButton::RightTrigger2);
    map.insert(SandboxAction::Reset, GamepadButton::Select);
    map.insert(SandboxAction::Start, GamepadButton::Start);

    map.insert(SandboxAction::Blink, GamepadButton::East);
    map.insert(SandboxAction::QuickAction, GamepadButton::RightTrigger);
    map.insert(SandboxAction::Interact, GamepadButton::RightTrigger);
    map.insert(SandboxAction::Modifier, GamepadButton::LeftTrigger2);
    map.insert(SandboxAction::Utility, GamepadButton::LeftTrigger);
    map.insert(SandboxAction::Map, GamepadButton::LeftThumb);
    map.insert(SandboxAction::Inventory, GamepadButton::RightThumb);
    map.insert(SandboxAction::Projectile, GamepadButton::North);

    map.insert(SandboxAction::MenuNavigateUp, GamepadButton::DPadUp);
    map.insert(SandboxAction::MenuNavigateDown, GamepadButton::DPadDown);
    map.insert(SandboxAction::MenuNavigateLeft, GamepadButton::DPadLeft);
    map.insert(SandboxAction::MenuNavigateRight, GamepadButton::DPadRight);
    map.insert(SandboxAction::MenuSelect, GamepadButton::South);
    map.insert(SandboxAction::MenuBack, GamepadButton::East);

    // The bumpers double as gameplay Utility/QuickAction, but menu page actions
    // are only read while a paged menu is open, so the physical button is shared
    // safely.
    map.insert(SandboxAction::MenuPageLeft, GamepadButton::LeftTrigger);
    map.insert(SandboxAction::MenuPageRight, GamepadButton::RightTrigger);

    map.insert_dual_axis(SandboxAction::MenuStick, GamepadStick::LEFT);
    map.insert_dual_axis(SandboxAction::AimStick, GamepadStick::RIGHT);
    // RIGHT_Z is the analog right-trigger axis on most pads.
    // Reading it as an axis lets us apply hysteresis ourselves
    // instead of relying on the binary just_pressed edge.
    map.insert_axis(SandboxAction::DashAnalog, GamepadControlAxis::RIGHT_Z);
}

impl KeyboardPreset {
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

pub(crate) fn key_name(key: KeyCode) -> &'static str {
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

    /// Gate 5 (GPT-5.6 review) — the dynamic-slot policy for Special, pinned.
    /// Special is a dedicated first-class slot: every keyboard preset binds it to
    /// its OWN key, distinct from Blink (`secondary`) — the old alias is retired
    /// at the binding layer too. The gamepad is fully assigned, so gamepad-Special
    /// is deliberately deferred to remap; keyboard + the touch Special button are
    /// its routes today. (If a future edit adds a gamepad Special binding, update
    /// the policy comment in `input_map`.)
    #[test]
    fn special_is_a_dedicated_slot_distinct_from_blink_on_every_preset() {
        for preset in KeyboardPreset::presets() {
            assert_ne!(
                preset.actions.special, preset.actions.secondary,
                "{:?}: Special must not alias Blink (secondary)",
                preset.id
            );
        }
    }

    /// A second local seat's map must touch NO key.
    ///
    /// Partitioning the controllers between two seats accomplishes nothing if
    /// the keyboard still drives both of them: player one types on the same
    /// keyboard player two would be bound to, so every WASD press would move
    /// both fighters.
    #[cfg(feature = "input")]
    #[test]
    fn a_second_seats_map_binds_no_key() {
        let map = KeyboardPreset::gamepad_only_map();
        for (action, binding) in map.buttonlike_bindings() {
            let path = binding.as_reflect().reflect_type_path();
            assert!(
                !path.contains("KeyCode"),
                "{action:?} is bound to {path} in the second seat's map — the \
                 keyboard belongs to player one"
            );
        }
        // The dual-axis half is where this would slip in unnoticed: a
        // `VirtualDPad` of arrow keys and a `VirtualDPad` of D-pad buttons are
        // the same type, so the count is the only thing that distinguishes
        // them. The full preset binds Move three ways (keys, D-pad, stick).
        assert_eq!(
            map.get_dual_axislike(&SandboxAction::Move).map(Vec::len),
            Some(2),
            "the second seat's Move should be D-pad + left stick and nothing else"
        );
        assert_eq!(
            KeyboardPreset::by_index(0)
                .input_map()
                .get_dual_axislike(&SandboxAction::Move)
                .map(Vec::len),
            Some(3),
            "player one's Move should still be keys + D-pad + left stick; if \
             this drops to 2 the keyboard half stopped being installed"
        );
    }
}
