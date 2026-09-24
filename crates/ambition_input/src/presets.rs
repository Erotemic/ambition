//! Default binding presets: the selectable keyboard layouts (`PresetId` /
//! `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad
//! bindings that seed leafwing's input map for `Platformer2dInputActionMonolith`.

use super::*;

/// Minimum stick-axis magnitude before a `GamepadControlDirection` binding
/// reads as pressed. This suppresses spring-return overshoot: a stick released
/// from a deep down push bounces briefly positive on Y. Without the threshold,
/// leafwing fires `MoveUp` on that frame and a double-tap-down MorphBall exits
/// at once. 0.5 is well past typical overshoot (about 0.1) and still triggers
/// at half deflection.
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
    /// Walk: hold to cap movement into the walk band.
    ///
    /// `ShiftRight` in every preset, unlike `grab` and `taunt`. Walk is a
    /// modifier the hand rests on, so it goes under the little finger. `ShiftLeft`
    /// is already `modifier` in three of the four presets.
    pub walk: KeyCode,
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
                attack: KeyCode::KeyX,
                burst: KeyCode::KeyC,
                secondary: KeyCode::KeyA,
                special: KeyCode::KeyG,
                shield: KeyCode::KeyE,
                grab: KeyCode::KeyS,
                taunt: KeyCode::KeyT,
                interact: KeyCode::KeyF,
                walk: KeyCode::ShiftRight,
                // The modifier slot gives Mary-O both run and fire: `modifier_held` is her
                // run, `modifier_pressed` is her spark (see `mary_o::movement`). The
                // `attack` binding on the same key is a separate action that is inert for
                // her, because `resolve_control_slots` calls `clear_attack` on a body with
                // no melee verb. For Sanic, X resolves to his `spin_dash` technique and his
                // modifier is not read.
                //
                // The gamepad-Special note below refuses a double binding for a different
                // reason. Only an actor with both a melee verb and a modifier technique
                // would fire two actions here; none exists.
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
                burst: KeyCode::KeyK,
                secondary: KeyCode::KeyL,
                special: KeyCode::KeyG,
                shield: KeyCode::KeyI,
                grab: KeyCode::KeyO,
                taunt: KeyCode::KeyN,
                interact: KeyCode::KeyE,
                walk: KeyCode::ShiftRight,
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
                burst: KeyCode::KeyW,
                attack: KeyCode::KeyE,
                secondary: KeyCode::KeyR,
                special: KeyCode::KeyH,
                shield: KeyCode::KeyT,
                grab: KeyCode::KeyY,
                taunt: KeyCode::KeyU,
                interact: KeyCode::KeyF,
                walk: KeyCode::ShiftRight,
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
                burst: KeyCode::KeyI,
                attack: KeyCode::KeyP,
                secondary: KeyCode::KeyO,
                special: KeyCode::KeyH,
                shield: KeyCode::KeyJ,
                grab: KeyCode::KeyY,
                taunt: KeyCode::KeyN,
                interact: KeyCode::KeyE,
                walk: KeyCode::ShiftRight,
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

    /// Build a fresh leafwing `InputMap` for this preset.
    ///
    /// Preset cycling swaps this component on the player entity. The preset is
    /// data, so a later TOML/RON keybinding config can deserialize into the same
    /// shape. Gated behind `input` because the return type is leafwing's.
    #[cfg(feature = "input")]
    pub fn input_map(&self) -> InputMap<Platformer2dInputActionMonolith> {
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
    ) -> InputMap<Platformer2dInputActionMonolith> {
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
    fn insert_keyboard_bindings(&self, map: &mut InputMap<Platformer2dInputActionMonolith>) {
        let keyboard_move = match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => VirtualDPad::arrow_keys(),
            PresetId::WasdJkl | PresetId::WasdUipo => VirtualDPad::wasd(),
        };
        map.insert_dual_axis(Platformer2dInputActionMonolith::Move, keyboard_move);
        map.insert(
            Platformer2dInputActionMonolith::MoveLeft,
            self.movement.left,
        );
        map.insert(
            Platformer2dInputActionMonolith::MoveRight,
            self.movement.right,
        );
        map.insert(Platformer2dInputActionMonolith::MoveUp, self.movement.up);
        map.insert(
            Platformer2dInputActionMonolith::MoveDown,
            self.movement.down,
        );
        map.insert(Platformer2dInputActionMonolith::Jump, self.actions.jump);
        map.insert(Platformer2dInputActionMonolith::Attack, self.actions.attack);
        map.insert(Platformer2dInputActionMonolith::Burst, self.actions.burst);
        map.insert(
            Platformer2dInputActionMonolith::Reset,
            self.actions.select_reset,
        );
        map.insert(Platformer2dInputActionMonolith::Reset, KeyCode::Delete);
        map.insert(Platformer2dInputActionMonolith::Start, self.actions.pause);

        map.insert(
            Platformer2dInputActionMonolith::Blink,
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
            Platformer2dInputActionMonolith::Special,
            self.actions.special,
        );
        map.insert(Platformer2dInputActionMonolith::Shield, self.actions.shield);
        map.insert(Platformer2dInputActionMonolith::Grab, self.actions.grab);
        map.insert(Platformer2dInputActionMonolith::Taunt, self.actions.taunt);
        map.insert(
            Platformer2dInputActionMonolith::Interact,
            self.actions.interact,
        );
        map.insert(Platformer2dInputActionMonolith::Walk, self.actions.walk);
        map.insert(
            Platformer2dInputActionMonolith::Modifier,
            self.actions.modifier,
        );
        map.insert(
            Platformer2dInputActionMonolith::Utility,
            self.actions.utility,
        );
        map.insert(Platformer2dInputActionMonolith::Map, self.actions.map);
        map.insert(
            Platformer2dInputActionMonolith::Inventory,
            self.actions.inventory,
        );
        map.insert(
            Platformer2dInputActionMonolith::Projectile,
            self.actions.projectile,
        );
        map.insert(
            Platformer2dInputActionMonolith::TrailToggle,
            self.actions.trail_toggle,
        );
        insert_optional(
            map,
            Platformer2dInputActionMonolith::Pogo,
            self.actions.dedicated_pogo,
        );

        // Menu navigation. D-pad and arrow keys hit the same MenuNavigate* actions;
        // the analog stick gives MenuStick for repeat handling; Enter, Space and
        // South map to MenuSelect.
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateUp,
            KeyCode::ArrowUp,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateUp,
            KeyCode::KeyW,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateDown,
            KeyCode::ArrowDown,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateDown,
            KeyCode::KeyS,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateLeft,
            KeyCode::ArrowLeft,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateLeft,
            KeyCode::KeyA,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateRight,
            KeyCode::ArrowRight,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuNavigateRight,
            KeyCode::KeyD,
        );

        map.insert(Platformer2dInputActionMonolith::MenuSelect, KeyCode::Enter);
        map.insert(
            Platformer2dInputActionMonolith::MenuSelect,
            KeyCode::NumpadEnter,
        );
        map.insert(Platformer2dInputActionMonolith::MenuSelect, KeyCode::Space);
        // Also accept the configured Jump and Interact keys as confirm, so dialogue
        // and cutscene habits still work. Enter is the canonical menu confirm.
        map.insert(
            Platformer2dInputActionMonolith::MenuSelect,
            self.actions.jump,
        );
        map.insert(
            Platformer2dInputActionMonolith::MenuSelect,
            self.actions.interact,
        );

        map.insert(Platformer2dInputActionMonolith::MenuBack, KeyCode::Escape);
        map.insert(
            Platformer2dInputActionMonolith::MenuBack,
            KeyCode::Backspace,
        );

        // Paged-menu page turn: `MoveLeft`/`MoveRight` already own A/D, so
        // paging uses Q/E.
        map.insert(Platformer2dInputActionMonolith::MenuPageLeft, KeyCode::KeyQ);
        map.insert(
            Platformer2dInputActionMonolith::MenuPageRight,
            KeyCode::KeyE,
        );
    }
}

/// The gamepad half, identical for every preset and every seat.
///
/// Free-standing because it does not depend on the preset; the preset chooses
/// keys. A second local seat has no preset and uses exactly this.
///
/// Every action has a button so both input modes are fully playable:
///   South        Jump, MenuSelect
///   East         Blink, MenuBack
///   West         Attack
///   North        Projectile (fireball)
///   LeftTrigger  Utility (fly toggle), MenuPageLeft
///   LeftTrigger2 Modifier
///   RightTrigger Shield, Interact, MenuPageRight
///   RightTrigger2 Burst
///   LeftThumb    Map (click left stick)
///   RightThumb   Inventory (click right stick)
///   Select       Reset
///   Start        Start (pause)
///   DPad / sticks  Move + MenuNavigate, MenuStick, AimStick
#[cfg(feature = "input")]
fn insert_gamepad_bindings(map: &mut InputMap<Platformer2dInputActionMonolith>) {
    map.insert_dual_axis(Platformer2dInputActionMonolith::Move, VirtualDPad::dpad());
    map.insert_dual_axis(Platformer2dInputActionMonolith::Move, GamepadStick::LEFT);

    // Gamepad bindings for the discrete `MoveX` actions. Without them,
    // `just_pressed(&Platformer2dInputActionMonolith::MoveDown)` never fires on a
    // controller, and double-tap-down into MorphBall is keyboard-only. The D-pad
    // and a stick direction past the threshold give the same press edge.
    //
    // `STICK_DIRECTION_THRESHOLD` stops spring-return overshoot from pressing
    // the opposite direction. Leafwing's `LEFT_UP` defaults to threshold 0.0,
    // so a released down push would fire `MoveUp` and exit MorphBall on the
    // frame the player entered it.
    map.insert(
        Platformer2dInputActionMonolith::MoveLeft,
        GamepadButton::DPadLeft,
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveLeft,
        GamepadControlDirection::LEFT_LEFT.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveRight,
        GamepadButton::DPadRight,
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveRight,
        GamepadControlDirection::LEFT_RIGHT.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveUp,
        GamepadButton::DPadUp,
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveUp,
        GamepadControlDirection::LEFT_UP.threshold(STICK_DIRECTION_THRESHOLD),
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveDown,
        GamepadButton::DPadDown,
    );
    map.insert(
        Platformer2dInputActionMonolith::MoveDown,
        GamepadControlDirection::LEFT_DOWN.threshold(STICK_DIRECTION_THRESHOLD),
    );

    map.insert(Platformer2dInputActionMonolith::Jump, GamepadButton::South);
    map.insert(Platformer2dInputActionMonolith::Attack, GamepadButton::West);
    map.insert(
        Platformer2dInputActionMonolith::Burst,
        GamepadButton::RightTrigger2,
    );
    map.insert(
        Platformer2dInputActionMonolith::Reset,
        GamepadButton::Select,
    );
    map.insert(Platformer2dInputActionMonolith::Start, GamepadButton::Start);

    map.insert(Platformer2dInputActionMonolith::Blink, GamepadButton::East);
    map.insert(
        Platformer2dInputActionMonolith::Shield,
        GamepadButton::RightTrigger,
    );
    map.insert(
        Platformer2dInputActionMonolith::Interact,
        GamepadButton::RightTrigger,
    );
    // No gamepad `Walk` binding. The analog left stick can already walk, and
    // every button is in use.
    //
    // Known gap: a D-pad-only player cannot walk. `Move` binds the D-pad too,
    // and the D-pad gives only 0 or 1.0, like the keyboard. A fix needs a free
    // button or a chord.
    map.insert(
        Platformer2dInputActionMonolith::Modifier,
        GamepadButton::LeftTrigger2,
    );
    map.insert(
        Platformer2dInputActionMonolith::Utility,
        GamepadButton::LeftTrigger,
    );
    map.insert(
        Platformer2dInputActionMonolith::Map,
        GamepadButton::LeftThumb,
    );
    map.insert(
        Platformer2dInputActionMonolith::Inventory,
        GamepadButton::RightThumb,
    );
    map.insert(
        Platformer2dInputActionMonolith::Projectile,
        GamepadButton::North,
    );

    map.insert(
        Platformer2dInputActionMonolith::MenuNavigateUp,
        GamepadButton::DPadUp,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuNavigateDown,
        GamepadButton::DPadDown,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuNavigateLeft,
        GamepadButton::DPadLeft,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuNavigateRight,
        GamepadButton::DPadRight,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuSelect,
        GamepadButton::South,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuBack,
        GamepadButton::East,
    );

    // The bumpers are also gameplay Utility and Shield. Menu page actions are
    // read only while a paged menu is open, so sharing the button is safe.
    map.insert(
        Platformer2dInputActionMonolith::MenuPageLeft,
        GamepadButton::LeftTrigger,
    );
    map.insert(
        Platformer2dInputActionMonolith::MenuPageRight,
        GamepadButton::RightTrigger,
    );

    map.insert_dual_axis(
        Platformer2dInputActionMonolith::MenuStick,
        GamepadStick::LEFT,
    );
    map.insert_dual_axis(
        Platformer2dInputActionMonolith::AimStick,
        GamepadStick::RIGHT,
    );
    // RIGHT_Z is the analog right-trigger axis on most pads. Read it as an axis
    // so hysteresis is applied here, not by the binary just_pressed edge.
    map.insert_axis(
        Platformer2dInputActionMonolith::BurstAnalog,
        GamepadControlAxis::RIGHT_Z,
    );
}

#[cfg(feature = "input")]
fn insert_optional(
    map: &mut InputMap<Platformer2dInputActionMonolith>,
    action: Platformer2dInputActionMonolith,
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
                .controls(&Platformer2dInputActionMonolith::Special)
                .is_empty(),
            "the DEFAULT pad is fully assigned, so it declines to double-bind \
             Special onto a button that already means something"
        );

        let mut smash_pad = KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly);
        BindingLayout::Smash.apply(&mut smash_pad);
        assert_eq!(
            ActionBindings::from_map(&smash_pad)
                .controls(&Platformer2dInputActionMonolith::Special),
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
            map.get_dual_axislike(&Platformer2dInputActionMonolith::Move)
                .map(Vec::len),
            Some(2),
            "the second seat's Move should be D-pad + left stick and nothing else"
        );
        assert_eq!(
            KeyboardPreset::by_index(0)
                .input_map()
                .get_dual_axislike(&Platformer2dInputActionMonolith::Move)
                .map(Vec::len),
            Some(3),
            "player one's Move should still be keys + D-pad + left stick; if \
             this drops to 2 the keyboard half stopped being installed"
        );
    }
}
