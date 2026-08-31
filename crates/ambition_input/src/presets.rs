//! Default binding presets: the selectable keyboard layouts (`PresetId` /
//! `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad
//! bindings that seed leafwing's input map for `Platformer2dInputActionMonolith`.

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
    /// The shared dodge/dash BURST press.
    pub burst: KeyCode,
    pub secondary: KeyCode,
    /// Dedicated signature-special key (distinct from `secondary`/Blink).
    pub special: KeyCode,
    pub shield: KeyCode,
    /// Capture attempt. Chosen per preset from that preset's UNUSED letters and
    /// placed next to its existing action cluster, so no preset gained a second
    /// meaning on a key it already spends.
    pub grab: KeyCode,
    /// Taunt. Chosen per preset from that preset's UNUSED letters, for the same
    /// reason and by the same rule as `grab`.
    pub taunt: KeyCode,
    pub interact: KeyCode,
    /// WALK — hold to cap movement into the walk band.
    ///
    /// ⛔ `ShiftRight`, and the SAME key in every preset, unlike `grab`/`taunt`
    /// which are chosen per preset from that preset's unused letters. Walk is a
    /// modifier a hand rests on, not a verb it reaches for, so it belongs under
    /// the little finger wherever the movement keys are — and `ShiftLeft` is
    /// already `modifier` in three of the four.
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

    /// Resolve the preset at `index` (the value stored in
    /// `settings.controls.keyboard_preset_index`). Out-of-range
    /// indices fall back to the first preset (`arrows_zxc`) so a stale
    /// or corrupt setting can never panic a HUD/glyph system.
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

    /// Build a fresh Leafwing `InputMap` for this preset.
    ///
    /// Preset cycling swaps this component on the player entity. Keeping the
    /// preset as data means later TOML/RON keybinding config can deserialize
    /// into the same shape instead of rewriting gameplay systems. Gated
    /// behind `input` because the return type is leafwing-owned.
    #[cfg(feature = "input")]
    pub fn input_map(&self) -> InputMap<Platformer2dInputActionMonolith> {
        self.map_for(crate::BindingSources::Unified)
    }

    /// A map holding only the halves a seat is ELIGIBLE for.
    ///
    /// ⭐⭐ ONE BUILDER FOR EVERY SEAT SHAPE. It replaced `input_map()` +
    /// `gamepad_only_map()`, a pair that encoded "primary = keyboard and pad,
    /// everyone else = pad" in its very shape — which is the assumption that
    /// gave a keyboard player in seat 2 no controls at all while the keyboard
    /// silently drove seat 1 as well.
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
        // Special is a FIRST-CLASS slot with its OWN dedicated key on every
        // preset — no longer aliasing Blink. Dynamic-slot policy for the gamepad:
        // every face/shoulder/trigger/stick button is already assigned (see
        // `insert_gamepad_bindings`), so rather than double-bind a button (which
        // would fire TWO actions at once), gamepad-Special is intentionally left
        // to the remap UX (P5) and to a game's `BindingLayout`. Keyboard (this
        // key) and the touch overlay's dedicated Special button cover it in
        // Ambition. that is a claim about THIS DEFAULT, not about pads — a
        // layout permutes an already-full pad and so can free a button for
        // Special, which is what the smash profile does (X).
        // `special_is_a_dedicated_slot_...` and
        // `the_default_pad_leaves_special_to_a_profile_and_a_profile_can_take_it`
        // pin the two halves.
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

        // Menu navigation seam. Cardinal/D-pad/arrow keys all hit the
        // same MenuNavigate* actions; the analog stick provides MenuStick
        // for repeat handling, and Enter/Space/South map to MenuSelect.
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
        // Also accept the player's configured Jump and Interact keys as
        // confirm so existing dialogue/cutscene muscle memory survives the
        // participant migration. Enter remains the canonical menu confirmation.
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

    // Gamepad bindings for the discrete `MoveX` actions. Without
    // these, `actions.just_pressed(&Platformer2dInputActionMonolith::MoveDown)`
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
    // ⚠ NO GAMEPAD `Walk` BINDING, deliberately. A pad's left stick is ANALOG
    // and has always been able to walk — the defect this action exists for is
    // digital-only. Every gamepad button is already spent, and stealing one to
    // duplicate something the stick does would be a worse trade.
    //
    // ⛔ A D-PAD-ONLY PAD PLAYER STILL CANNOT WALK, and that is a real gap
    // rather than a decision: `Move` binds both the D-pad and the left stick, so
    // the D-pad half has the same 1.0-or-nothing problem the keyboard had. It
    // needs a free button or a chord, and there is no free button.
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

    // The bumpers double as gameplay Utility/Shield, but menu page actions
    // are only read while a paged menu is open, so the physical button is shared
    // safely.
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
    // RIGHT_Z is the analog right-trigger axis on most pads.
    // Reading it as an axis lets us apply hysteresis ourselves
    // instead of relying on the binary just_pressed edge.
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

/// The label a HUD should print for a key.
///
/// public so a GAME's on-screen legend can read the same table the BINDINGS do. Sanic's
/// speedway printed a hardcoded `"START Z: JUMP DOWN+X: REV RELEASE DOWN: DASH D: SUPER"` — and
/// the preset binds no `D` at all.
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

    /// Gate 5 — the dynamic-slot policy for Special, pinned.
    ///
    /// the gamepad half of this policy is a claim about the DEFAULT preset, and made that
    /// distinction load-bearing. The reason there is no gamepad Special here has never been
    /// "Special does not deserve a button" — it is that THIS pad is fully assigned, so adding
    /// one would double-bind a button and fire two actions at once.
    ///
    /// So the policy is now stated with its scope attached, and the test asserts
    /// BOTH halves — the default still declines the button, and a mode layout
    /// may claim one. Neither half is weakened by the other's existence.
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

    /// The gamepad half of the dedicated-slot policy, and its SCOPE.
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

    /// EVERY SLOT A BODY CAN CARRY IS ON THE KEYBOARD, ON EVERY PRESET.
    ///
    /// The existing tests pin the first two ONE CASE AT A TIME, which is a list that grows only
    /// after each new verb has already shipped broken.
    ///
    /// the keyboard is the right subject and the pad is not: a pad may
    /// legitimately decline a slot (the default pad is fully assigned and leaves
    /// `Special` to a game's layout — the test above pins exactly that), while
    /// the keyboard is the device every composition always has. A slot with no
    /// key anywhere is a verb no keyboard player can reach.
    ///
    /// this guards the BINDING, which is one link. The two below it are
    /// guarded by the compiler: `ControlFrame`'s literal in `control.rs` has no
    /// rest pattern, and `brain/player.rs` destructures exhaustively.
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

    /// A second local seat's map must touch NO key.
    ///
    /// Partitioning the controllers between two seats accomplishes nothing if
    /// the keyboard still drives both of them: player one types on the same
    /// keyboard player two would be bound to, so every WASD press would move
    /// both fighters.
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
        // The dual-axis half is where this would slip in unnoticed: a
        // `VirtualDPad` of arrow keys and a `VirtualDPad` of D-pad buttons are
        // the same type, so the count is the only thing that distinguishes
        // them. The full preset binds Move three ways (keys, D-pad, stick).
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
