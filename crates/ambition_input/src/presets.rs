//! Device key labels and preset data types. Defaults belong to the host.

use bevy::prelude::KeyCode;

const ANDROID_BACK: KeyCode = KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Android(4));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetId {
    ArrowsZxc,
    WasdJkl,
    ArrowsQwer,
    WasdUipo,
}

impl PresetId {
    pub const ALL: [Self; 4] = [Self::ArrowsZxc, Self::WasdJkl, Self::ArrowsQwer, Self::WasdUipo];
    pub fn by_index(index: usize) -> Self { Self::ALL.get(index).copied().unwrap_or(Self::ArrowsZxc) }
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

#[cfg(all(test, feature = "input"))]
impl KeyboardPreset {
    pub fn presets() -> [Self; 4] { crate::fixture_opinions::presets() }
    pub fn by_index(index: usize) -> Self { Self::of(PresetId::by_index(index)) }
    pub fn of(id: PresetId) -> Self { crate::fixture_opinions::keyboard_preset(id) }
    pub fn arrows_zxc() -> Self { Self::of(PresetId::ArrowsZxc) }
    pub fn wasd_jkl() -> Self { Self::of(PresetId::WasdJkl) }
    pub fn arrows_qwer() -> Self { Self::of(PresetId::ArrowsQwer) }
    pub fn wasd_uipo() -> Self { Self::of(PresetId::WasdUipo) }
    pub fn input_map(&self) -> leafwing_input_manager::prelude::InputMap<crate::InputAction> { self.map_for(crate::BindingSources::Unified) }
    pub fn map_for(&self, sources: crate::BindingSources) -> leafwing_input_manager::prelude::InputMap<crate::InputAction> {
        crate::fixture_opinions::defaults().recipe(self.id).with_sources(sources).build()
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

#[cfg(all(test, feature = "input"))]
mod tests {
    use super::{KeyboardPreset, PresetId, KeyCode, key_name};
    use crate::InputAction;
    use bevy::prelude::GamepadButton;

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

        let default_pad = KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly);
        assert!(
            ActionBindings::from_map(&default_pad)
                .controls(&InputAction::Special)
                .is_empty(),
            "the DEFAULT pad is fully assigned, so it declines to double-bind \
             Special onto a button that already means something"
        );

        let smash_pad = crate::BindingRecipe::gamepad_only().with_layout(crate::fixture_opinions::fixture_layout()).build();
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
