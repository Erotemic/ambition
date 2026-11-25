//! Default binding presets: the selectable keyboard layouts (`PresetId` /
//! `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad
//! bindings that seed leafwing's input map for `InputAction`.

use super::*;

/// Minimum stick-axis magnitude before a `GamepadControlDirection` binding
/// reads as pressed. This suppresses spring-return overshoot: a stick released
/// from a deep down push bounces briefly positive on Y. Without the threshold,
/// leafwing fires `MoveUp` on that frame and a double-tap-down MorphBall exits
/// at once. 0.5 is well past typical overshoot (about 0.1) and still triggers
/// at half deflection.
#[cfg(feature = "input")]
const STICK_DIRECTION_THRESHOLD: f32 = 0.5;

#[cfg(feature = "input")]
use super::InputAction as Act;

/// The standard pad buttons. A game layout changes gameplay bindings.
/// Session and UI bindings stay the same.
#[cfg(feature = "input")]
pub const STANDARD_PAD: &[(InputAction, GamepadButton)] = &[
    (Act::Jump, GamepadButton::South),
    (Act::Blink, GamepadButton::East),
    (Act::Attack, GamepadButton::West),
    (Act::Projectile, GamepadButton::North),
    (Act::Utility, GamepadButton::LeftTrigger),
    (Act::Shield, GamepadButton::LeftTrigger2),
    (Act::Interact, GamepadButton::RightTrigger),
    (Act::Burst, GamepadButton::RightTrigger2),
    (Act::Map, GamepadButton::LeftThumb),
    (Act::Inventory, GamepadButton::RightThumb),
    (Act::Reset, GamepadButton::Select),
    (Act::Start, GamepadButton::Start),
];

/// The menu's buttons, on every pad and under every gameplay layout: A
/// confirms, B backs out, the bumpers turn pages, the D-pad moves. A keyboard
/// preset binds each to the key standing in for that button
/// ([`KeyboardPreset::key_for_button`]): Jump's key confirms, Blink's backs
/// out. Changing a menu control is one row here.
#[cfg(feature = "input")]
pub const MENU_BUTTONS: &[(InputAction, GamepadButton)] = &[
    (Act::MenuSelect, GamepadButton::South),
    (Act::MenuBack, GamepadButton::East),
    (Act::MenuPageLeft, GamepadButton::LeftTrigger),
    (Act::MenuPageRight, GamepadButton::RightTrigger),
    (Act::MenuNavigateUp, GamepadButton::DPadUp),
    (Act::MenuNavigateDown, GamepadButton::DPadDown),
    (Act::MenuNavigateLeft, GamepadButton::DPadLeft),
    (Act::MenuNavigateRight, GamepadButton::DPadRight),
];

/// The keyboard's own menu keys, beside each preset's stand-ins. Enter is
/// also Start: gameplay pauses; an interface confirms. Escape also binds
/// Start: it pauses in a session and goes Back in an open pause menu.
#[cfg(feature = "input")]
pub const KEYBOARD_MENU_KEYS: &[(InputAction, KeyCode)] = &[
    (Act::MenuSelect, KeyCode::Enter),
    (Act::MenuSelect, KeyCode::NumpadEnter),
    (Act::MenuSelect, KeyCode::Space),
    (Act::MenuBack, KeyCode::Escape),
    (Act::MenuBack, KeyCode::Backspace),
    (Act::MenuBack, ANDROID_BACK),
    (Act::MenuPageLeft, KeyCode::KeyQ),
    (Act::MenuPageRight, KeyCode::KeyE),
    (Act::MenuPageLeft, KeyCode::PageUp),
    (Act::MenuPageRight, KeyCode::PageDown),
    (Act::MenuNavigateUp, KeyCode::ArrowUp),
    (Act::MenuNavigateDown, KeyCode::ArrowDown),
    (Act::MenuNavigateLeft, KeyCode::ArrowLeft),
    (Act::MenuNavigateRight, KeyCode::ArrowRight),
];

const ANDROID_BACK: KeyCode = KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Android(4));

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
    /// A jump that is always a short hop (`ShortHop`), beside the jump key.
    pub short_hop: KeyCode,
    pub attack: KeyCode,
    /// The shared dodge/dash burst press.
    pub burst: KeyCode,
    pub secondary: KeyCode,
    /// Dedicated signature-special key (distinct from `secondary`/Blink).
    pub special: KeyCode,
    pub shield: KeyCode,
    /// Capture attempt. Each preset uses one of its unused letters, next to its
    /// action cluster, so no key gets a second meaning.
    pub grab: KeyCode,
    /// Taunt. Chosen by the same rule as `grab`.
    pub taunt: KeyCode,
    pub interact: KeyCode,
    /// The shift layer (`ShiftLayer`): both Shift keys, in every preset, so
    /// either hand holds it. Held, Move is a walk.
    pub shift_layer: [KeyCode; 2],
    /// The modifier slot's button: a slot whose sustain is the point. Each
    /// preset uses one of its unused letters, as for `grab`, so no key gets a
    /// second meaning. A body's run or fire does not live here: a technique
    /// goes on the slot whose position it has (Mary-O's run is on Attack),
    /// so every device reaches it through that slot's binding.
    pub modifier: KeyCode,
    pub utility: KeyCode,
    pub map: KeyCode,
    pub inventory: KeyCode,
    pub projectile: KeyCode,
    pub trail_toggle: KeyCode,
    pub pause: KeyCode,
    pub select_reset: KeyCode,
    /// Optional dedicated pogo key. When `None`, pogo falls back to
    /// the down+attack combo and the glyph path shows "D+X".
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

    /// Resolve the preset at `index` (the value in
    /// `settings.controls.keyboard_preset_index`). An out-of-range index falls
    /// back to the first preset (`arrows_zxc`), so a stale or corrupt setting
    /// cannot panic a HUD or glyph system.
    pub fn by_index(index: usize) -> Self {
        let presets = Self::presets();
        presets.get(index).copied().unwrap_or(presets[0])
    }

    /// The preset carrying this id. Total — every `PresetId` is one of the
    /// four rows `presets()` returns.
    pub fn of(id: PresetId) -> Self {
        Self::presets()
            .into_iter()
            .find(|preset| preset.id == id)
            .expect("every PresetId names a preset in presets()")
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

    pub fn wasd_jkl() -> Self {
        Self {
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

    pub fn wasd_uipo() -> Self {
        Self {
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

    /// Build a fresh leafwing `InputMap` for this preset.
    ///
    /// Preset cycling swaps this component on the player entity. The preset is
    /// data, so a later TOML/RON keybinding config can deserialize into the same
    /// shape. Gated behind `input` because the return type is leafwing's.
    #[cfg(feature = "input")]
    pub fn input_map(&self) -> InputMap<InputAction> {
        self.map_for(crate::BindingSources::Unified)
    }

    /// A map with only the halves a seat is eligible for.
    ///
    /// One builder for every seat shape. A split into "primary gets keyboard and
    /// pad, others get pad" gave a keyboard player in seat 2 no controls, while
    /// the keyboard also drove seat 1.
    #[cfg(feature = "input")]
    pub fn map_for(
        &self,
        sources: crate::BindingSources,
    ) -> InputMap<InputAction> {
        let mut map = InputMap::default();
        if sources.admits_keyboard() {
            self.insert_keyboard_bindings(&mut map);
        }
        if sources.admits_gamepad() {
            insert_gamepad_bindings(&mut map);
        }
        map
    }

    /// The keyboard half of a preset: everything whose binding is a key this
    /// preset chose.
    #[cfg(feature = "input")]
    fn insert_keyboard_bindings(&self, map: &mut InputMap<InputAction>) {
        let keyboard_move = match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => VirtualDPad::arrow_keys(),
            PresetId::WasdJkl | PresetId::WasdUipo => VirtualDPad::wasd(),
        };
        map.insert_dual_axis(InputAction::Move, keyboard_move);
        map.insert(
            InputAction::MoveLeft,
            self.movement.left,
        );
        map.insert(
            InputAction::MoveRight,
            self.movement.right,
        );
        map.insert(InputAction::MoveUp, self.movement.up);
        map.insert(
            InputAction::MoveDown,
            self.movement.down,
        );
        map.insert(InputAction::Jump, self.actions.jump);
        map.insert(InputAction::ShortHop, self.actions.short_hop);
        map.insert(InputAction::Attack, self.actions.attack);
        map.insert(InputAction::Burst, self.actions.burst);
        map.insert(
            InputAction::Reset,
            self.actions.select_reset,
        );
        map.insert(InputAction::PartyReturn, KeyCode::Home);
        map.insert(InputAction::Start, self.actions.pause);
        map.insert(InputAction::Start, KeyCode::NumpadEnter);
        map.insert(InputAction::Start, KeyCode::Escape);

        map.insert(
            InputAction::Blink,
            self.actions.secondary,
        );
        // Special has its own key on every preset; it does not alias Blink. The
        // default pad has no free button (see `insert_gamepad_bindings`), and a
        // double binding fires two actions, so gamepad Special is left to the remap
        // UX and to a game's `BindingLayout`. The keyboard and the touch overlay's
        // Special button cover it in Ambition. This applies to the default only: a
        // layout can free a button for Special, as the smash profile does (X).
        // `special_is_a_dedicated_slot_...` and
        // `the_default_pad_leaves_special_to_a_profile_and_a_profile_can_take_it`
        // guard the two halves.
        map.insert(
            InputAction::Special,
            self.actions.special,
        );
        map.insert(InputAction::Shield, self.actions.shield);
        map.insert(InputAction::Grab, self.actions.grab);
        map.insert(InputAction::Taunt, self.actions.taunt);
        map.insert(
            InputAction::Interact,
            self.actions.interact,
        );
        for key in self.actions.shift_layer {
            map.insert(InputAction::ShiftLayer, key);
        }
        map.insert(
            InputAction::Modifier,
            self.actions.modifier,
        );
        map.insert(
            InputAction::Utility,
            self.actions.utility,
        );
        map.insert(InputAction::Map, self.actions.map);
        map.insert(
            InputAction::Inventory,
            self.actions.inventory,
        );
        map.insert(
            InputAction::Projectile,
            self.actions.projectile,
        );
        map.insert(
            InputAction::TrailToggle,
            self.actions.trail_toggle,
        );
        insert_optional(
            map,
            InputAction::Pogo,
            self.actions.dedicated_pogo,
        );

        self.insert_menu_keys(map);
    }

    /// The key this preset gives the role a standard-pad button plays: Jump's
    /// key stands in for A, Blink's for B, the movement keys for the D-pad.
    /// `None` for a button with no keyboard stand-in (shoulders, sticks).
    #[cfg(feature = "input")]
    pub fn key_for_button(&self, button: GamepadButton) -> Option<KeyCode> {
        use Act as A;
        let action = match button {
            GamepadButton::DPadLeft => return Some(self.movement.left),
            GamepadButton::DPadRight => return Some(self.movement.right),
            GamepadButton::DPadUp => return Some(self.movement.up),
            GamepadButton::DPadDown => return Some(self.movement.down),
            _ => STANDARD_PAD.iter().find(|(_, b)| *b == button)?.0,
        };
        Some(match action {
            A::Jump => self.actions.jump,
            A::Blink => self.actions.secondary,
            A::Attack => self.actions.attack,
            A::Projectile => self.actions.projectile,
            _ => return None,
        })
    }

    /// The keyboard's menu controls: each menu button's stand-in key, and the
    /// keyboard's own menu keys. A stand-in wins a key both claim (the
    /// arrows+QWER preset jumps on Q, so Q confirms and does not turn a page),
    /// and no key means two things inside a menu. The keyboard's own keys bind
    /// first so a menu prompt names Enter and Esc.
    #[cfg(feature = "input")]
    fn insert_menu_keys(&self, map: &mut InputMap<InputAction>) {
        let derived: Vec<_> = MENU_BUTTONS.iter()
            .filter_map(|(role, button)| Some((*role, self.key_for_button(*button)?)))
            .collect();
        let own = KEYBOARD_MENU_KEYS.iter().copied()
            .filter(|(_, key)| derived.iter().all(|(_, taken)| taken != key));
        for (role, key) in own.chain(derived.iter().copied()) {
            map.insert(role, key);
        }
    }
}

/// The gamepad half, identical for every preset and every seat.
///
/// Free-standing because it does not depend on the preset; the preset chooses
/// keys. A second local seat has no preset and uses exactly this.
///
/// The buttons are [`STANDARD_PAD`] and [`MENU_BUTTONS`]; the D-pad and left
/// stick also move, the left stick is the menu stick, and the right stick aims.
#[cfg(feature = "input")]
fn insert_gamepad_bindings(map: &mut InputMap<InputAction>) {
    map.insert_dual_axis(InputAction::Move, VirtualDPad::dpad());
    map.insert_dual_axis(InputAction::Move, GamepadStick::LEFT);

    // Gamepad bindings for the discrete `MoveX` actions. Without them,
    // `just_pressed(&InputAction::MoveDown)` never fires on a
    // controller, and double-tap-down into MorphBall is keyboard-only. The D-pad
    // and a stick direction past the threshold give the same press edge.
    //
    // `STICK_DIRECTION_THRESHOLD` stops spring-return overshoot from pressing
    // the opposite direction. Leafwing's `LEFT_UP` defaults to threshold 0.0,
    // so a released down push would fire `MoveUp` and exit MorphBall on the
    // frame the player entered it.
    for (action, button, stick) in [
        (Act::MoveLeft, GamepadButton::DPadLeft, GamepadControlDirection::LEFT_LEFT),
        (Act::MoveRight, GamepadButton::DPadRight, GamepadControlDirection::LEFT_RIGHT),
        (Act::MoveUp, GamepadButton::DPadUp, GamepadControlDirection::LEFT_UP),
        (Act::MoveDown, GamepadButton::DPadDown, GamepadControlDirection::LEFT_DOWN),
    ] {
        map.insert(action, button);
        map.insert(action, stick.threshold(STICK_DIRECTION_THRESHOLD));
    }

    // No gamepad `Walk` binding. The analog left stick can already walk, and
    // every button is in use.
    //
    // Known gap: a D-pad-only player cannot walk. `Move` binds the D-pad too,
    // and the D-pad gives only 0 or 1.0, like the keyboard. A fix needs a free
    // button or a chord.
    //
    // The menu's buttons share pad buttons with gameplay. Menu actions are read
    // only while a menu or interface is open, so sharing is safe.
    for (action, button) in STANDARD_PAD.iter().chain(MENU_BUTTONS) {
        map.insert(*action, *button);
    }

    map.insert_dual_axis(
        InputAction::MenuStick,
        GamepadStick::LEFT,
    );
    map.insert_dual_axis(
        InputAction::AimStick,
        GamepadStick::RIGHT,
    );
    // RIGHT_Z is the analog right-trigger axis on most pads. Read it as an axis
    // so hysteresis is applied here, not by the binary just_pressed edge.
    map.insert_axis(
        InputAction::BurstAnalog,
        GamepadControlAxis::RIGHT_Z,
    );
}

#[cfg(feature = "input")]
fn insert_optional(
    map: &mut InputMap<InputAction>,
    action: InputAction,
    key: Option<KeyCode>,
) {
    if let Some(key) = key {
        map.insert(action, key);
    }
}

/// The label a HUD prints for a key.
///
/// Public so a game's on-screen legend reads the same table as the bindings.
/// A hardcoded legend can name keys the preset does not bind.
pub fn key_name(key: KeyCode) -> &'static str {
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
        KeyCode::Enter => "Enter",
        KeyCode::NumpadEnter => "Enter",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "Alt",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Delete",
        KeyCode::Home => "Home",
        KeyCode::Backspace => "Backspace",
        ANDROID_BACK => "Back",
        KeyCode::PageUp => "PageUp",
        KeyCode::PageDown => "PageDown",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "input")]
    #[test]
    fn standard_pad_actions_use_separate_controls() {
        use crate::{ActionBindings, ActiveDevice, GamepadStyle, PhysicalControl};
        use InputAction as Action;

        let actions = [
            Action::Shield,
            Action::Interact,
            Action::Burst,
            Action::Map,
            Action::Inventory,
            Action::Reset,
        ];
        for preset in KeyboardPreset::presets() {
            let bindings = ActionBindings::from_map(&preset.input_map());
            for (action, button) in [
                (Action::Shield, GamepadButton::LeftTrigger2),
                (Action::Interact, GamepadButton::RightTrigger),
                (Action::Burst, GamepadButton::RightTrigger2),
                (Action::Map, GamepadButton::LeftThumb),
                (Action::Inventory, GamepadButton::RightThumb),
                (Action::Reset, GamepadButton::Select),
            ] {
                let control = PhysicalControl::Button(button);
                assert_eq!(
                    bindings.control_for(
                        &action,
                        ActiveDevice::Gamepad(GamepadStyle::XboxLike),
                    ),
                    Some(&control),
                );
                for other in actions.into_iter().filter(|other| *other != action) {
                    assert!(!bindings.controls(&other).contains(&control));
                }
            }
        }
    }

    /// Every key a preset binds has a name, and so a published picture: the
    /// glyph file is named by the key's label (`PhysicalControl::glyph_path`).
    #[cfg(feature = "input")]
    #[test]
    fn every_bound_key_has_a_name_and_so_a_picture() {
        use crate::{ActionBindings, PhysicalControl};
        for preset in KeyboardPreset::presets() {
            let bindings = ActionBindings::from_map(&preset.input_map());
            for (action, controls) in bindings.all() {
                for control in controls {
                    if let PhysicalControl::Key(key) = control {
                        assert_ne!(key_name(*key), "?", "{}: {action} binds {key:?}, which has no name", preset.name);
                    }
                }
            }
        }
    }

    #[cfg(feature = "input")]
    #[test]
    fn a_menu_confirms_on_a_and_backs_out_on_b_and_their_keyboard_stand_ins() {
        use crate::{ActionBindings, PhysicalControl};
        use InputAction as A;
        let key = |k| PhysicalControl::Key(k);
        let pad = |b| PhysicalControl::Button(b);
        for preset in KeyboardPreset::presets() {
            let bindings = ActionBindings::from_map(&preset.input_map());
            let select = bindings.controls(&A::MenuSelect);
            let back = bindings.controls(&A::MenuBack);
            for control in [key(preset.actions.jump), key(KeyCode::Enter), pad(GamepadButton::South)] {
                assert!(select.contains(&control), "{}: {control:?} confirms", preset.name);
            }
            for control in [key(preset.actions.secondary), key(KeyCode::Escape), key(KeyCode::Backspace),
                pad(GamepadButton::East)] {
                assert!(back.contains(&control), "{}: {control:?} backs out", preset.name);
            }
            // Inside a menu no control means two things.
            let roles = [A::MenuSelect, A::MenuBack, A::MenuPageLeft, A::MenuPageRight,
                A::MenuNavigateUp, A::MenuNavigateDown, A::MenuNavigateLeft, A::MenuNavigateRight];
            for (i, role) in roles.iter().enumerate() {
                for control in bindings.controls(role) {
                    for other in &roles[i + 1..] {
                        assert!(!bindings.controls(other).contains(control),
                            "{}: {control:?} is both {role:?} and {other:?}", preset.name);
                    }
                }
            }
            // A menu prompt names the keyboard's own keys.
            assert_eq!(bindings.controls(&A::MenuSelect).iter().find(|c| matches!(c, PhysicalControl::Key(_))),
                Some(&key(KeyCode::Enter)));
            assert_eq!(bindings.controls(&A::MenuBack).iter().find(|c| matches!(c, PhysicalControl::Key(_))),
                Some(&key(KeyCode::Escape)));
        }
    }

    #[cfg(feature = "input")]
    #[test]
    fn keyboard_start_and_back_are_contextual_and_confirm_has_a_dedicated_control() {
        use crate::{ActionBindings, PhysicalControl};
        for preset in KeyboardPreset::presets() {
            let bindings = ActionBindings::from_map(&preset.input_map());
            let enter = PhysicalControl::Key(KeyCode::Enter);
            let escape = PhysicalControl::Key(KeyCode::Escape);
            assert!(bindings.controls(&InputAction::Start).contains(&enter));
            assert!(bindings.controls(&InputAction::Start).contains(&escape));
            assert!(bindings.controls(&InputAction::MenuBack).contains(&escape));
            assert!(bindings.controls(&InputAction::MenuSelect).contains(&PhysicalControl::Key(KeyCode::Space)));
        }
    }

    #[test]
    fn by_index_resolves_each_preset_and_clamps_out_of_range() {
        // Each in-range index returns the matching preset. The order must match
        // `settings.controls.keyboard_preset_index`.
        assert_eq!(KeyboardPreset::by_index(0).id, PresetId::ArrowsZxc);
        assert_eq!(KeyboardPreset::by_index(1).id, PresetId::WasdJkl);
        assert_eq!(KeyboardPreset::by_index(2).id, PresetId::ArrowsQwer);
        assert_eq!(KeyboardPreset::by_index(3).id, PresetId::WasdUipo);
        // A stale or corrupt index falls back to the first preset; it does not panic.
        assert_eq!(KeyboardPreset::by_index(4).id, PresetId::ArrowsZxc);
        assert_eq!(KeyboardPreset::by_index(usize::MAX).id, PresetId::ArrowsZxc);
    }

    /// The dynamic-slot policy for Special, keyboard half: Special has its own
    /// key and does not alias Blink.
    ///
    /// The default pad has no gamepad Special because it is fully assigned, and a
    /// new button would fire two actions. The next test checks that half, and
    /// that a layout can still claim a button.
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

    /// The gamepad half of the dedicated-slot policy, and its scope.
    #[cfg(feature = "input")]
    #[test]
    fn the_default_pad_leaves_special_to_a_profile_and_a_profile_can_take_it() {
        use crate::bindings::ActionBindings;
        use crate::layout::BindingLayout;

        let default_pad = KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly);
        assert!(
            ActionBindings::from_map(&default_pad)
                .controls(&InputAction::Special)
                .is_empty(),
            "the DEFAULT pad is fully assigned, so it declines to double-bind \
             Special onto a button that already means something"
        );

        let mut smash_pad = KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly);
        BindingLayout::Smash.apply(&mut smash_pad);
        assert_eq!(
            ActionBindings::from_map(&smash_pad)
                .controls(&InputAction::Special),
            [crate::PhysicalControl::Button(GamepadButton::West)],
            "a GAME's layout permutes the pad, so it CAN free a button for \
             Special — that is the difference between a profile and a default"
        );
    }

    /// Every slot a body can carry has a key on every preset.
    ///
    /// The keyboard is the right subject: a pad can decline a slot (the default
    /// pad leaves `Special` to a layout, as the test above checks), but every
    /// composition has a keyboard. A slot with no key is a verb no keyboard
    /// player can reach.
    ///
    /// This guards the binding only. The compiler guards the next two links:
    /// `ControlFrame`'s literal in `control.rs` has no rest pattern, and
    /// `brain/player.rs` destructures exhaustively.
    #[cfg(feature = "input")]
    #[test]
    fn every_control_slot_reaches_a_key_on_every_preset() {
        use crate::bindings::{action_for_slot, ActionBindings};
        use ambition_entity_catalog::action_scheme::CANONICAL_SLOT_ORDER;

        for preset in KeyboardPreset::presets() {
            let map = preset.input_map();
            let bound = ActionBindings::from_map(&map);
            for slot in CANONICAL_SLOT_ORDER {
                let action = action_for_slot(slot).unwrap_or_else(|| {
                    panic!(
                        "{slot:?} maps to no input action at all, so nothing can \
                         press it on any device"
                    )
                });
                assert!(
                    !bound.controls(&action).is_empty(),
                    "{:?}: {slot:?} ({action:?}) has NO key. A body granted that \
                     slot advertises the verb — the action scheme derives it, the \
                     touch overlay draws a button for it — and a keyboard player \
                     cannot press it.",
                    preset.id
                );
            }
        }
    }

    /// Either Shift key holds the shift layer on every preset, and neither
    /// has a second meaning. Until 2026-09-29 Left Shift was the raw modifier
    /// on three presets and unbound on the fourth, so Shift + an arrow ran.
    #[cfg(feature = "input")]
    #[test]
    fn both_shift_keys_hold_the_shift_layer_and_nothing_else_on_every_preset() {
        for preset in KeyboardPreset::presets() {
            let map = preset.input_map();
            for shift in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
                let actions: Vec<_> = map
                    .buttonlike_bindings()
                    .filter(|(_, binding)| binding.as_reflect().downcast_ref::<KeyCode>() == Some(&shift))
                    .map(|(action, _)| *action)
                    .collect();
                assert_eq!(
                    actions,
                    vec![InputAction::ShiftLayer],
                    "{:?}: {shift:?} is bound to {actions:?}",
                    preset.id
                );
            }
        }
    }

    /// A second local seat's map must bind no key. Otherwise the keyboard drives
    /// both seats, and every WASD press moves both fighters.
    #[cfg(feature = "input")]
    #[test]
    fn a_second_seats_map_binds_no_key() {
        let map = KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly);
        for (action, binding) in map.buttonlike_bindings() {
            let path = binding.as_reflect().reflect_type_path();
            assert!(
                !path.contains("KeyCode"),
                "{action:?} is bound to {path} in the second seat's map — the \
                 keyboard belongs to player one"
            );
        }
        // Check the dual-axis half: a `VirtualDPad` of arrow keys and one of D-pad
        // buttons have the same type, so only the count tells them apart. The full
        // preset binds Move three ways (keys, D-pad, stick).
        assert_eq!(
            map.get_dual_axislike(&InputAction::Move)
                .map(Vec::len),
            Some(2),
            "the second seat's Move should be D-pad + left stick and nothing else"
        );
        assert_eq!(
            KeyboardPreset::by_index(0)
                .input_map()
                .get_dual_axislike(&InputAction::Move)
                .map(Vec::len),
            Some(3),
            "player one's Move should still be keys + D-pad + left stick; if \
             this drops to 2 the keyboard half stopped being installed"
        );
    }
}
