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
/// `action: None` is a DECLARED BLANK, not an omission: the layout claims the button (so whatever
/// the base preset had there is cleared) and deliberately leaves it dead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadSlot {
    pub button: GamepadButton,
    /// What this layout puts on the button — or `None` for a button it CLAIMS
    /// and leaves empty.
    ///
    /// the `None` is load-bearing, not a placeholder: [`BindingLayout::apply`] clears every claimed
    /// button's gameplay bindings first and only then binds the ones with an action, so a blank
    /// slot is how a layout TAKES A BUTTON AWAY from the base preset without giving it a new verb.
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
/// A fact about the GAME, not about the player and not about the pad hardware.
///  it is NOT [`crate::settings::ControllerProfileId`] — that one is
/// HARDWARE CALIBRATION (deadzones, trigger thresholds per pad brand) and is a
/// different axis entirely. A Steam Controller playing smash needs both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BindingLayout {
    /// The base preset's own pad, untouched. Ambition's layout: A=Jump.
    #[default]
    Standard,
    Smash,
}

/// | DPadUp / DPadDown | Taunt | Move | Movement is stick-only in a fighter, which is what the genre does and what frees a button for a taunt. |
///
/// Everything the table does not name is the base preset's: RightTrigger still
/// shields and interacts, RightTrigger2 still fires the burst, the sticks,
/// Select/Start and the thumb clicks are untouched. *"The rest of the bindings
/// are normal I think."*
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
    //  the D-pad taunts, because a fighting game moves on the STICK. That
    // is the genre's own layout; the base preset's `DPad → Move` is what a
    // platform fighter gives up to get a taunt button at all.
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
    /// The buttons this layout CLAIMS, and what it puts on each. `Standard`
    /// claims nothing — it is the base preset speaking for itself, so there is
    /// no second table to keep in step with `insert_gamepad_bindings`.
    pub fn pad_slots(self) -> &'static [PadSlot] {
        match self {
            Self::Standard => &[],
            Self::Smash => SMASH_PAD,
        }
    }

    /// Layer this layout onto a built map.
    ///
    /// Clear-then-install, in two passes rather than one: a single pass would
    /// let the clear for a later slot remove what an earlier slot just
    /// installed, since one action may legitimately appear on two buttons
    /// (Shield does).
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

/// Take `button` away from every GAMEPLAY action that binds it.
///
///  not `clear_action`, which would drop the action's KEYBOARD half too —
/// a layout re-arranges a pad and must not silently unbind somebody's keys.
/// The removal is by BUTTON, through the same `PhysicalControl` projection a
/// prompt reads, so "what this layout displaced" is by construction what the
/// screen was showing.
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

/// What a game asks its pad to mean. Present means a mode has declared a
/// layout; absent means [`BindingLayout::Standard`] — the base preset speaking
/// for itself, which is Ambition's answer.
///
/// A resource rather than a per-seat setting because it is a fact about the
/// GAME, and every seat in a match plays the same game. That is also why
/// [`apply_active_binding_layout_to_recipes`] reaches EVERY participant and not
/// just the primary, unlike the keyboard-preset sync beside it: a preset is one
/// person's taste, a layout is the mode's.
///
///  DECLARE, don't edit. The alternative — teaching `insert_gamepad_bindings` about smash
/// — would make one game's taste every game's default, and the rule is the opposite: *"B=jump
/// is the way I like my smash controller, it's probably non standard."* A=Jump stays right for
/// Ambition, and it stays right by the smash layout never touching it.
///
/// Same shape as `DeclaredCombatRules`, for the same reason: the declaration
/// carries its OWNER so an experience leaving can give back its own without
/// deleting another provider's. Two games in one binary is the normal case here.
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
/// It writes the RECIPE, not the map — so a layout change goes through exactly
/// the machinery a preset change and a remap already go through
/// (`rebuild_maps_from_recipes` rebuilds, `publish_seat_bindings` re-projects
/// the glyphs, the touch overlay's `Changed<InputMap>` hook re-binds), and
/// there is no second path that could disagree with the first.
///
///  must run AFTER the settings→recipe sync and BEFORE the rebuild. The
/// settings sync rewrites the primary's whole recipe from the persisted preset;
/// it carries the current layout forward for exactly this reason, and the
/// ordering is the belt to that suspenders.
///
///  and the absent case is LIVE, not a no-op. Removing the declaration on
/// the way out of a mode is how the pad goes back to normal — a system that
/// only acted when a declaration existed would leave B jumping in Ambition
/// forever after one smash match.
pub fn apply_active_binding_layout_to_recipes(
    declared: Option<Res<DeclaredBindingLayout>>,
    mut recipes: Query<&mut BindingRecipe>,
) {
    let wanted = declared.map(|d| d.layout).unwrap_or_default();
    for mut recipe in &mut recipes {
        // Write only on a real change: `BindingRecipe` is `Changed`-watched by
        // the rebuild, and touching it every frame would reset every seat's
        // `ActionState` every frame.
        if recipe.layout != wanted {
            recipe.layout = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::KeyboardPreset;

    /// What a button DRIVES, so a test asks about the pad rather than about a
    /// table's insertion order.
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

    ///  the permutation, stated as "one button, one verb".
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

    /// The displaced actions, named — because "it is legitimate for the profile
    /// to leave it unbound" is a DECISION and has to be visible, not a silent
    /// drop nobody noticed.
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

    ///  THE RULING: this is a profile, not a new default.
    ///
    /// Applying the smash layout to one map must not move Ambition's pad. If
    /// this ever goes red, somebody edited the shared preset instead of adding
    /// a layout, and every other game silently inherited one game's taste.
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

    /// `Standard` is the base speaking for itself — no second table to drift.
    #[test]
    fn the_standard_layout_is_the_identity() {
        let mut map = KeyboardPreset::arrows_zxc().input_map();
        let untouched = map.clone();
        BindingLayout::Standard.apply(&mut map);
        assert_eq!(map, untouched);
    }
}
