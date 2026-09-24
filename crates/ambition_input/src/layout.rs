//! Game/mode-specific gamepad binding profiles.
//!
//! A layout remaps gameplay buttons without changing the shared default or the
//! user's persisted binding overrides. Layout rows are keyed by physical button,
//! so each claimed button maps to at most one gameplay action; displaced actions
//! may intentionally become unbound. Menu bindings are preserved because they
//! are interpreted only while menu contexts are active.

use bevy::prelude::{GamepadButton, Query, Res, Resource};
use leafwing_input_manager::prelude::InputMap;

use crate::Platformer2dInputActionMonolith;
use crate::bindings::{BindingRecipe, PhysicalControl};

/// One physical button, and the gameplay action a layout puts on it.
///
/// `action: None` is a declared blank, not an omission: the layout claims the
/// button (clearing what the base preset had there) and leaves it dead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadSlot {
    pub button: GamepadButton,
    /// What this layout puts on the button, or `None` to claim it and leave it
    /// empty. [`BindingLayout::apply`] clears every claimed button first and
    /// then binds the slots that have an action, so a blank slot removes a
    /// button from the base preset.
    pub action: Option<Platformer2dInputActionMonolith>,
}

const fn slot(button: GamepadButton, action: Platformer2dInputActionMonolith) -> PadSlot {
    PadSlot {
        button,
        action: Some(action),
    }
}

/// Which game/mode layout a seat's pad is arranged for.
///
/// A fact about the game, not the player or the pad hardware. It is not
/// [`crate::settings::ControllerProfileId`], which is hardware calibration
/// (deadzones, trigger thresholds per pad brand). A seat can need both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BindingLayout {
    /// The base preset's own pad, untouched. Ambition's layout: A=Jump.
    #[default]
    Standard,
    Smash,
}

/// The smash pad layout. DPadUp and DPadDown taunt, because a fighter moves on
/// the stick only.
///
/// Buttons not listed keep the base preset: RightTrigger still shields and
/// interacts, RightTrigger2 still fires the burst, and the sticks,
/// Select/Start, and thumb clicks do not change.
const SMASH_PAD: &[PadSlot] = &[
    slot(
        GamepadButton::South,
        Platformer2dInputActionMonolith::Attack,
    ),
    slot(GamepadButton::East, Platformer2dInputActionMonolith::Jump),
    slot(
        GamepadButton::West,
        Platformer2dInputActionMonolith::Special,
    ),
    slot(GamepadButton::North, Platformer2dInputActionMonolith::Grab),
    // The d-pad taunts; a fighter moves on the stick.
    slot(
        GamepadButton::DPadUp,
        Platformer2dInputActionMonolith::Taunt,
    ),
    slot(
        GamepadButton::DPadDown,
        Platformer2dInputActionMonolith::Taunt,
    ),
    slot(
        GamepadButton::LeftTrigger,
        Platformer2dInputActionMonolith::Shield,
    ),
    slot(
        GamepadButton::LeftTrigger2,
        Platformer2dInputActionMonolith::Shield,
    ),
];

impl BindingLayout {
    /// The buttons this layout claims, and what it puts on each. `Standard`
    /// claims nothing, so there is no second table to keep in step with
    /// `insert_gamepad_bindings`.
    pub fn pad_slots(self) -> &'static [PadSlot] {
        match self {
            Self::Standard => &[],
            Self::Smash => SMASH_PAD,
        }
    }

    /// Layer this layout onto a built map.
    ///
    /// Clear all claimed buttons first, then install, in two passes. One pass
    /// would let a later slot's clear remove an earlier slot's install, because
    /// one action can be on two buttons (Shield is).
    pub fn apply(self, map: &mut InputMap<Platformer2dInputActionMonolith>) {
        let slots = self.pad_slots();
        if slots.is_empty() {
            return;
        }
        for slot in slots {
            clear_gameplay_bindings_of(map, slot.button);
        }
        for slot in slots {
            if let Some(action) = slot.action {
                map.insert(action, slot.button);
            }
        }
    }
}

/// Remove `button` from every gameplay action that binds it.
///
/// Do not use `clear_action`: it also drops the action's keyboard bindings.
/// Removal is by button, through the same `PhysicalControl` projection that
/// prompts read, so what the layout displaces is what the screen showed.
fn clear_gameplay_bindings_of(
    map: &mut InputMap<Platformer2dInputActionMonolith>,
    button: GamepadButton,
) {
    let target = PhysicalControl::Button(button);
    let displaced: Vec<(Platformer2dInputActionMonolith, Vec<usize>)> = map
        .iter_buttonlike()
        .filter(|(action, _)| !action.is_menu_only())
        .filter_map(|(action, inputs)| {
            let indices: Vec<usize> = inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| crate::bindings::physical_control_of(input.as_ref()) == target)
                .map(|(index, _)| index)
                .collect();
            (!indices.is_empty()).then(|| (*action, indices))
        })
        .collect();

    for (action, indices) in displaced {
        let Some(bindings) = map.get_buttonlike_mut(&action) else {
            continue;
        };
        // Back to front, so an earlier index is still the element it named.
        for index in indices.iter().rev() {
            bindings.remove(*index);
        }
    }
}

/// The pad layout a game declares. When absent, the layout is
/// [`BindingLayout::Standard`] (the base preset, Ambition's layout).
///
/// This is a resource, not a per-seat setting, because it is a fact about the
/// game and all seats play the same game. For this reason
/// [`apply_active_binding_layout_to_recipes`] writes every participant, not
/// only the primary as the keyboard-preset sync does.
///
/// Games declare a layout; they do not edit `insert_gamepad_bindings`. That
/// keeps one game's preference (smash: B=Jump) out of every game's default
/// (Ambition: A=Jump).
///
/// Like `DeclaredCombatRules`, the declaration records its owner, so an
/// experience that leaves removes only its own declaration. Two games in one
/// binary is normal.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct DeclaredBindingLayout {
    /// Which shell experience declared this layout.
    pub declared_by: String,
    pub layout: BindingLayout,
}

impl DeclaredBindingLayout {
    pub fn new(declared_by: impl Into<String>, layout: BindingLayout) -> Self {
        Self {
            declared_by: declared_by.into(),
            layout,
        }
    }

    pub fn is_declared_by(&self, owner: &str) -> bool {
        self.declared_by == owner
    }
}

/// Carry the declared layout into every seat's [`BindingRecipe`].
///
/// This writes the recipe, not the map. A layout change therefore uses the
/// same path as preset changes and remaps (`rebuild_maps_from_recipes`
/// rebuilds, `publish_seat_bindings` updates glyphs, the touch overlay's
/// `Changed<InputMap>` hook re-binds). There is no second path.
///
/// Run after the settings-to-recipe sync and before the rebuild. The settings
/// sync rewrites the primary's recipe from the persisted preset and also
/// carries the current layout forward; the ordering is a second guard.
///
/// The absent case also writes. Removing the declaration when a mode exits is
/// how the pad goes back to `Standard`.
pub fn apply_active_binding_layout_to_recipes(
    declared: Option<Res<DeclaredBindingLayout>>,
    mut recipes: Query<&mut BindingRecipe>,
) {
    let wanted = declared.map(|d| d.layout).unwrap_or_default();
    for mut recipe in &mut recipes {
        // Write only on a real change. The rebuild watches `BindingRecipe`
        // with `Changed`, so a write each frame resets every `ActionState`.
        if recipe.layout != wanted {
            recipe.layout = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::KeyboardPreset;

    /// The gameplay actions a button drives, so tests ask about the pad and
    /// not about table insertion order.
    fn gameplay_actions_on(
        map: &InputMap<Platformer2dInputActionMonolith>,
        button: GamepadButton,
    ) -> Vec<Platformer2dInputActionMonolith> {
        let target = PhysicalControl::Button(button);
        let mut found: Vec<_> = map
            .iter_buttonlike()
            .filter(|(action, _)| !action.is_menu_only())
            .filter(|(_, inputs)| {
                inputs
                    .iter()
                    .any(|input| crate::bindings::physical_control_of(input.as_ref()) == target)
            })
            .map(|(action, _)| *action)
            .collect();
        found.sort_by_key(|action| crate::action_name(action));
        found
    }

    fn smash_pad() -> InputMap<Platformer2dInputActionMonolith> {
        let mut map = KeyboardPreset::of(KeyboardPreset::by_index(0).id)
            .map_for(crate::BindingSources::GamepadOnly);
        BindingLayout::Smash.apply(&mut map);
        map
    }

    /// The permutation, stated as "one button, one verb".
    #[test]
    fn every_button_the_smash_layout_claims_drives_exactly_one_verb() {
        let map = smash_pad();
        for (button, expected) in [
            (
                GamepadButton::South,
                vec![Platformer2dInputActionMonolith::Attack],
            ),
            (
                GamepadButton::East,
                vec![Platformer2dInputActionMonolith::Jump],
            ),
            (
                GamepadButton::West,
                vec![Platformer2dInputActionMonolith::Special],
            ),
            (
                GamepadButton::North,
                vec![Platformer2dInputActionMonolith::Grab],
            ),
            (
                GamepadButton::LeftTrigger,
                vec![Platformer2dInputActionMonolith::Shield],
            ),
            (
                GamepadButton::LeftTrigger2,
                vec![Platformer2dInputActionMonolith::Shield],
            ),
        ] {
            assert_eq!(
                gameplay_actions_on(&map, button),
                expected,
                "{button:?} under the smash layout"
            );
        }
    }

    /// The displaced actions are named, because leaving them unbound on the
    /// pad is a decision.
    #[test]
    fn the_actions_smash_displaces_lose_the_pad_and_keep_the_keyboard() {
        let base = KeyboardPreset::arrows_zxc().input_map();
        let mut map = base.clone();
        BindingLayout::Smash.apply(&mut map);

        for action in [
            Platformer2dInputActionMonolith::Blink,
            Platformer2dInputActionMonolith::Projectile,
            Platformer2dInputActionMonolith::Utility,
            Platformer2dInputActionMonolith::Modifier,
        ] {
            let controls = crate::ActionBindings::from_map(&map);
            let bound = controls.controls(&action);
            assert!(
                !bound
                    .iter()
                    .any(|control| matches!(control, PhysicalControl::Button(_))),
                "{action:?} should have no pad button under the smash layout, got {bound:?}"
            );
            assert!(
                bound
                    .iter()
                    .any(|control| matches!(control, PhysicalControl::Key(_))),
                "{action:?} must keep its keyboard binding — a layout re-arranges a PAD"
            );
        }
    }

    /// The menus survive the permutation. Confirm and cancel do not move
    /// because a game mode rearranged its face buttons.
    #[test]
    fn a_layout_rearranges_gameplay_and_leaves_the_menu_alone() {
        let map = smash_pad();
        let bindings = crate::ActionBindings::from_map(&map);
        for (action, button) in [
            (
                Platformer2dInputActionMonolith::MenuSelect,
                GamepadButton::South,
            ),
            (
                Platformer2dInputActionMonolith::MenuBack,
                GamepadButton::East,
            ),
            (
                Platformer2dInputActionMonolith::MenuPageLeft,
                GamepadButton::LeftTrigger,
            ),
        ] {
            assert!(
                bindings
                    .controls(&action)
                    .contains(&PhysicalControl::Button(button)),
                "{action:?} must stay on {button:?}"
            );
        }
    }

    /// The smash layout is a profile, not a new default.
    ///
    /// Applying it to one map must not change Ambition's pad. A failure means
    /// the shared preset was edited instead of adding a layout.
    #[test]
    fn installing_the_smash_layout_does_not_move_the_generic_preset() {
        let before = KeyboardPreset::of(KeyboardPreset::by_index(0).id)
            .map_for(crate::BindingSources::GamepadOnly);
        let mut smash = before.clone();
        BindingLayout::Smash.apply(&mut smash);
        let after = KeyboardPreset::of(KeyboardPreset::by_index(0).id)
            .map_for(crate::BindingSources::GamepadOnly);

        assert_eq!(
            before, after,
            "the base preset is a pure function; a layout may not mutate it"
        );
        assert_eq!(
            gameplay_actions_on(&after, GamepadButton::South),
            vec![Platformer2dInputActionMonolith::Jump],
            "A=Jump is still Ambition's default"
        );
        assert_eq!(
            gameplay_actions_on(&after, GamepadButton::West),
            vec![Platformer2dInputActionMonolith::Attack],
        );
        assert_ne!(smash, after, "…and the smash map really is different");
    }

    /// `Standard` is the base preset unchanged; there is no second table.
    #[test]
    fn the_standard_layout_is_the_identity() {
        let mut map = KeyboardPreset::arrows_zxc().input_map();
        let untouched = map.clone();
        BindingLayout::Standard.apply(&mut map);
        assert_eq!(map, untouched);
    }
}
