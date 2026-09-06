//! Pure capture of physical input into persisted binding overrides. Menu
//! presentation and arming policy live outside this module.

use crate::bindings::{ActionBindings, PhysicalControl};
use crate::settings::{BindingOverride, OverrideControl};
use crate::Platformer2dInputActionMonolith;

/// The control a capture accepted. `PhysicalControl::Other` is not persistable
/// and is therefore refused.
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

/// Other actions already bound to this control, in canonical order. Duplicate
/// bindings are reported rather than rejected because one control may drive
/// multiple actions.
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

/// This frame's physical presses, in a stable order, for a capture.
///
///  the keyboard is read RAW, not through the seat's `InputMap`. A rebind
/// screen has to see a key the map does not bind — that is the entire point of
/// rebinding — and the map only reports actions it already knows. Routing a
/// capture through the map would make the set of rebindable controls exactly the
/// set already bound, so a player could permute their bindings and never reach a
/// key the preset never used.
///
///  order is the enum's, not the hardware's. `ButtonInput` iterates a hash
/// set, so "the first key pressed this frame" is not a fact it can supply; two
/// keys down on one frame would resolve differently run to run, and a rebind
/// that lands on a different key each time is worse than one that refuses. The
/// pressed set is sorted so a two-key frame is at least DECIDED — and a capture
/// screen should tell the player to press one key at a time regardless.
///
/// Pads are included for the same reason the override model has a gamepad half:
/// a couch seat rebinds a button, not a key.
#[cfg(feature = "input")]
pub fn pressed_controls_this_frame(
    keys: Option<&bevy::input::ButtonInput<bevy::prelude::KeyCode>>,
    pads: impl IntoIterator<Item = bevy::prelude::GamepadButton>,
) -> Vec<PhysicalControl> {
    let mut out: Vec<PhysicalControl> = keys
        .into_iter()
        .flat_map(|keys| keys.get_just_pressed())
        .map(|key| PhysicalControl::Key(*key))
        .collect();
    out.extend(pads.into_iter().map(PhysicalControl::Button));
    // `PhysicalControl` derives `Ord`; sorting makes a multi-press frame
    // deterministic instead of hash-ordered.
    out.sort();
    out
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
        //  `Other` exists so the PROJECTION can be total. Storing one would be
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
        //  the game itself ships one: Escape drives Start AND MenuBack, on
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

    #[test]
    fn a_multi_key_frame_resolves_the_same_way_every_run() {
        //  `ButtonInput` iterates a hash set, so "the first key this frame" is
        // not a fact it can supply. Two runs picking different keys is a rebind
        // that lands somewhere new each time, which is worse than refusing.
        let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyZ);
        keys.press(KeyCode::KeyA);
        let first = pressed_controls_this_frame(Some(&keys), []);
        let again = pressed_controls_this_frame(Some(&keys), []);
        assert_eq!(first, again, "the order is decided, not hash-ordered");
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn a_key_no_preset_binds_is_still_capturable() {
        //  the reason the keyboard is read RAW. Routing a capture through the
        // seat's `InputMap` would make the rebindable set exactly the bound set,
        // so a player could permute their bindings and never reach a new key.
        let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::F13);
        let map = KeyboardPreset::arrows_zxc().input_map();
        let bindings = ActionBindings::from_map(&map);
        assert!(
            !bindings
                .all()
                .any(|(_, controls)| controls.contains(&PhysicalControl::Key(KeyCode::F13))),
            "precondition: no preset binds F13"
        );

        let captured = capture(
            &Platformer2dInputActionMonolith::Jump,
            pressed_controls_this_frame(Some(&keys), []),
        )
        .expect("an unbound key is exactly what a rebind is for");
        assert_eq!(captured.control, OverrideControl::Key(KeyCode::F13));
    }
}
