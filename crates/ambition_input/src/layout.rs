//! **A GAME's gamepad layout — what each physical button MEANS in this mode.**
//!
//! The seam the whole input stack is built to have:
//!
//! ```text
//! physical device -> game/application binding profile -> semantic participant actions -> game rules
//! ```
//!
//! [`crate::BindingRecipe`] already owned the first and last of those. This
//! module is the middle one, and it exists because Jon's smash layout
//! (*"a=normal, x=special, b=jump, y=grab, left trigger is shield"*) is a
//! PROFILE CHOICE, not a new universal default: **A=Jump stays right for
//! Ambition.** He said so in the same breath — *"B=jump is the way I like my
//! smash controller, it's probably non standard. Will need to have control
//! profiles eventually."* Editing the shared preset would have made one game's
//! taste every game's default, which is the one regression this module is
//! shaped to prevent.
//!
//! ## Why not a `Vec<BindingOverride>`
//!
//! [`crate::BindingOverride`] is a SETTINGS type: it is what a USER remap
//! persists, keyed by the action's `Debug` spelling so a file outlives the
//! build that wrote it. Reusing it verbatim for a mode's shipped layout would
//! conflate *"the player rebound this"* with *"this mode ships this"* — and
//! those two have different PRECEDENCE, which is the whole reason to keep them
//! apart. A layout is also a different SHAPE: an override can only move an
//! action ONTO a control, and can never say *"under this profile that action
//! has no pad button at all"*, which a permutation of a fully-assigned pad
//! necessarily has to say.
//!
//! ## Keyed by BUTTON, so nothing can double-bind
//!
//! A layout is a table of PHYSICAL BUTTONS, each naming at most one gameplay
//! action. Applying it clears every gameplay action off the buttons the layout
//! claims, then installs what the layout declared. Two consequences fall out by
//! construction rather than by a hand-kept list:
//!
//! * **no button fires two actions.** The base pad is FULLY assigned — every
//!   face, shoulder, trigger and stick button already means something — so a
//!   permutation that only ADDED bindings would leave B meaning Jump AND Blink.
//!   That is exactly the hazard `presets.rs` refused to accept when it left
//!   gamepad-Special unbound rather than double-binding a button.
//! * **an action the layout displaces and does not re-home ends up with no pad
//!   button**, silently and correctly. In a fighting game Blink (teleport),
//!   Utility (fly toggle) and Projectile have no business on a face button; the
//!   layout does not have to enumerate their removal, it just does not name
//!   them.
//!
//! ⚠ **MENU actions are exempt from the clear.** `MenuSelect` shares South with
//! Jump, `MenuBack` shares East with Blink, and the page turns share the
//! bumpers — deliberately, since a paged menu only reads them while it is open.
//! A layout re-homes GAMEPLAY; confirm and cancel stay where every other screen
//! in the game put them.

use bevy::prelude::{GamepadButton, Query, Res, Resource};
use leafwing_input_manager::prelude::InputMap;

use crate::bindings::{BindingRecipe, PhysicalControl};
use crate::Platformer2dInputActionMonolith;

/// One physical button, and the gameplay action a layout puts on it.
///
/// `action: None` is a DECLARED BLANK, not an omission: the layout claims the
/// button (so whatever the base preset had there is cleared) and deliberately
/// leaves it dead. That is how Y works today — Jon's layout says *"y=grab"* and
/// **we do not have grab**, so binding it to a placeholder would be inventing a
/// verb, and leaving Projectile on it would leave a stray Ambition fireball on
/// the button a fighting-game player reaches for to grab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadSlot {
    pub button: GamepadButton,
    /// What this layout puts on the button — or `None` for a button it CLAIMS
    /// and leaves empty.
    ///
    /// ⚠ the `None` is load-bearing, not a placeholder: [`BindingLayout::apply`]
    /// clears every claimed button's gameplay bindings first and only then binds
    /// the ones with an action, so a blank slot is how a layout TAKES A BUTTON
    /// AWAY from the base preset without giving it a new verb. The Smash layout
    /// used it to keep Projectile off North while North was reserved for a grab
    /// that did not exist yet; that reservation was redeemed 2026-08-18 and no
    /// layout blanks a button today. The capability stays because the next one
    /// that needs it is one line, and the alternative — binding something the
    /// layout does not want — is the failure this expresses.
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
/// ⛔ **it is NOT [`crate::settings::ControllerProfileId`]** — that one is
/// HARDWARE CALIBRATION (deadzones, trigger thresholds per pad brand) and is a
/// different axis entirely. A Steam Controller playing smash needs both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BindingLayout {
    /// The base preset's own pad, untouched. Ambition's layout: A=Jump.
    #[default]
    Standard,
    /// Jon's smash layout, for the fighting-game mode.
    Smash,
}

/// Jon's smash pad, verbatim from the ruling, plus what each displaced action
/// does about it.
///
/// | button | smash | was | the displaced action |
/// |---|---|---|---|
/// | South (A) | Attack | Jump | Jump moves to East |
/// | East (B)  | Jump   | Blink | **Blink loses its pad button.** A teleport is not a fighting-game verb; its keyboard binding is untouched. |
/// | West (X)  | Special | Attack | Attack moves to South |
/// | North (Y) | **Grab** | Projectile | **The reservation is redeemed (2026-08-18).** Projectile lost its pad button rather than sitting on the grab button, and the button sat blank until the grab it was held for existed. |
/// | LeftTrigger (LB) | Shield | Utility | **Utility (fly toggle) loses its pad button.** |
/// | LeftTrigger2 (LT) | Shield | Modifier | **Modifier loses its pad button.** A fighting game reads walk off the analog stick. |
///
/// ⚠ **both left shoulder buttons shield, on purpose.** Jon said *"left trigger
/// is shield"* about an Xbox pad, where "left trigger" is the ANALOG trigger —
/// which Bevy spells `LeftTrigger2`, because it spells the BUMPER `LeftTrigger`.
/// Rather than guess which of the two he meant, the layout gives Shield both:
/// that is what a fighting game does anyway (L and R both shield), it satisfies
/// his sentence under either reading, and one action on two buttons is not the
/// hazard — two ACTIONS on one button is.
///
/// | DPadUp / DPadDown | **Taunt** | Move | **Movement is stick-only in a fighter**, which is what the genre does and what frees a button for a taunt. |
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
    // ⭐ **the reservation, redeemed.** This was `blank(North)` with a comment
    // saying it was held for a grab that did not exist. Leaving a pad button
    // empty for months is a real cost, and it was the right call: the
    // alternative was Projectile sitting here and then being taken away from
    // players' fingers the day capture landed.
    slot(GamepadButton::North, Platformer2dInputActionMonolith::Grab),
    // ⭐ **the D-pad taunts, because a fighting game moves on the STICK.** That
    // is the genre's own layout; the base preset's `DPad → Move` is what a
    // platform fighter gives up to get a taunt button at all.
    slot(GamepadButton::DPadUp, Platformer2dInputActionMonolith::Taunt),
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
/// ⛔ **not `clear_action`**, which would drop the action's KEYBOARD half too —
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

/// **What a game asks its pad to mean.** Present means a mode has declared a
/// layout; absent means [`BindingLayout::Standard`] — the base preset speaking
/// for itself, which is Ambition's answer.
///
/// A resource rather than a per-seat setting because it is a fact about the
/// GAME, and every seat in a match plays the same game. That is also why
/// [`apply_active_binding_layout_to_recipes`] reaches EVERY participant and not
/// just the primary, unlike the keyboard-preset sync beside it: a preset is one
/// person's taste, a layout is the mode's.
///
/// ⭐ **DECLARE, don't edit.** The alternative — teaching
/// `insert_gamepad_bindings` about smash — would make one game's taste every
/// game's default, and Jon's ruling is the opposite: *"B=jump is the way I like
/// my smash controller, it's probably non standard."* A=Jump stays right for
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
/// ⚠ **must run AFTER the settings→recipe sync and BEFORE the rebuild.** The
/// settings sync rewrites the primary's whole recipe from the persisted preset;
/// it carries the current layout forward for exactly this reason, and the
/// ordering is the belt to that suspenders.
///
/// ⚠ **and the absent case is LIVE, not a no-op.** Removing the declaration on
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
        let mut map = KeyboardPreset::gamepad_only_map();
        BindingLayout::Smash.apply(&mut map);
        map
    }

    /// ⭐ **the permutation, stated as "one button, one verb".**
    ///
    /// The base pad is fully assigned, so the failure this pins is not "the
    /// binding is missing" — it is B firing Jump AND Blink together, which is
    /// what a layout expressed as four additions would have produced.
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

    /// ⭐ **THE RULING: this is a profile, not a new default.**
    ///
    /// Applying the smash layout to one map must not move Ambition's pad. If
    /// this ever goes red, somebody edited the shared preset instead of adding
    /// a layout, and every other game silently inherited one game's taste.
    #[test]
    fn installing_the_smash_layout_does_not_move_the_generic_preset() {
        let before = KeyboardPreset::gamepad_only_map();
        let mut smash = before.clone();
        BindingLayout::Smash.apply(&mut smash);
        let after = KeyboardPreset::gamepad_only_map();

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
