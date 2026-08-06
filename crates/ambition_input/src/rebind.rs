//! **Turning "the player pressed a thing" into a persisted binding override.**
//!
//! The last clause of `character-actions.md` P1, and the half the override model
//! shipped without: `ControlSettings::binding_overrides` has been persisted and
//! honoured since 2026-08-06, and NOTHING in the game could set one. The feature
//! was reachable only by hand-editing a settings file.
//!
//! This module is the CAPTURE — pure, so the rule lives in one testable place
//! and a menu screen is presentation over it rather than the owner of the
//! policy. What a rebind row looks like, which actions it lists, and how a
//! player arms it belong to the menu module (D3 sequenced that after PA3's
//! convergence, so the rows are built ON it).

use crate::bindings::{ActionBindings, PhysicalControl};
use crate::settings::{BindingOverride, OverrideControl};
use crate::Platformer2dInputActionMonolith;

/// The control a capture accepted, if the press was bindable.
///
/// ⚠ **`PhysicalControl::Other` is refused**, and that is the whole reason this
/// is not just "take the first press". That arm exists so the PROJECTION can be
/// total — it carries the debug form of an input the classifier could not name,
/// rather than dropping it and telling a player an action has no control. A
/// binding is the other direction: an override must be constructible, and
/// storing a control nothing can rebuild is a settings file that loads into
/// silence.
pub fn bindable(control: &PhysicalControl) -> Option<OverrideControl> {
    match control {
        PhysicalControl::Key(key) => Some(OverrideControl::Key(*key)),
        PhysicalControl::Button(button) => Some(OverrideControl::Button(*button)),
        PhysicalControl::Other(_) => None,
    }
}

/// The override a capture frame produces, or `None` if nothing bindable was
/// pressed.
///
/// `pressed` is this frame's physical controls in the order the reader saw
/// them; the first bindable one wins. A frame with only unbindable presses
/// captures nothing and leaves the row armed, which is the honest behaviour —
/// the player pressed something the game cannot store, so it must not pretend
/// it stored it.
pub fn capture(
    action: &Platformer2dInputActionMonolith,
    pressed: impl IntoIterator<Item = PhysicalControl>,
) -> Option<BindingOverride> {
    let control = pressed.into_iter().find_map(|control| bindable(&control))?;
    Some(BindingOverride {
        action: crate::bindings::action_name(action),
        control,
    })
}

/// Every OTHER action this control is already bound to, in the seat's live map.
///
/// ⚠ **a duplicate binding is not refused here, it is REPORTED.** leafwing
/// allows one control to drive several actions and the game uses that already —
/// Escape is both `Start` and `MenuBack`, deliberately. So a capture that
/// refused every collision would forbid a shape the game itself ships. What a
/// rebind screen owes the player is knowing: "this is also Attack" is
/// information they can act on, and silently stealing a control from another
/// action is the thing that reads as a bug.
///
/// Returns the other actions' stable names, in the projection's canonical order,
/// so two runs and two machines list them the same way.
pub fn also_bound_to(
    bindings: &ActionBindings,
    action: &Platformer2dInputActionMonolith,
    control: OverrideControl,
) -> Vec<String> {
    let wanted = match control {
        OverrideControl::Key(key) => PhysicalControl::Key(key),
        OverrideControl::Button(button) => PhysicalControl::Button(button),
    };
    let self_name = crate::bindings::action_name(action);
    bindings
        .all()
        .filter(|(name, _)| *name != self_name)
        .filter(|(_, controls)| controls.contains(&wanted))
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::KeyboardPreset;
    use bevy::prelude::KeyCode;

    #[test]
    fn the_first_bindable_press_becomes_the_override() {
        let captured = capture(
            &Platformer2dInputActionMonolith::Jump,
            [PhysicalControl::Key(KeyCode::KeyJ)],
        )
        .expect("a key is bindable");
        assert_eq!(captured.action, "Jump");
        assert_eq!(captured.control, OverrideControl::Key(KeyCode::KeyJ));
    }

    #[test]
    fn an_unnameable_press_captures_nothing_rather_than_something_wrong() {
        // ⛔ `Other` exists so the PROJECTION can be total. Storing one would be
        // a settings file that loads into silence, because nothing can rebuild
        // the control from its debug string.
        assert!(capture(
            &Platformer2dInputActionMonolith::Jump,
            [PhysicalControl::Other("Chord(A, B)".into())]
        )
        .is_none());
    }

    #[test]
    fn an_unbindable_press_does_not_hide_a_bindable_one_behind_it() {
        let captured = capture(
            &Platformer2dInputActionMonolith::Attack,
            [
                PhysicalControl::Other("Chord(A, B)".into()),
                PhysicalControl::Key(KeyCode::KeyK),
            ],
        )
        .expect("the key behind the unnameable press is still bindable");
        assert_eq!(captured.control, OverrideControl::Key(KeyCode::KeyK));
    }

    #[test]
    fn a_collision_is_reported_rather_than_refused() {
        // ⚠ the game itself ships one: Escape drives Start AND MenuBack, on
        // purpose. A capture that refused collisions would forbid that shape.
        let map = KeyboardPreset::arrows_zxc().input_map();
        let bindings = ActionBindings::from_map(&map);
        let start = bindings
            .controls(&Platformer2dInputActionMonolith::Start)
            .iter()
            .find_map(bindable)
            .expect("Start is bound to something bindable");

        let others = also_bound_to(&bindings, &Platformer2dInputActionMonolith::Start, start);
        assert!(
            others.iter().any(|name| name == "MenuBack"),
            "the screen can tell the player this control is also MenuBack, got {others:?}"
        );
        assert!(
            !others.iter().any(|name| name == "Start"),
            "and it does not report the action against itself"
        );
    }

    #[test]
    fn an_unused_control_collides_with_nothing() {
        let map = KeyboardPreset::arrows_zxc().input_map();
        let bindings = ActionBindings::from_map(&map);
        assert!(also_bound_to(
            &bindings,
            &Platformer2dInputActionMonolith::Jump,
            OverrideControl::Key(KeyCode::F13),
        )
        .is_empty());
    }
}
