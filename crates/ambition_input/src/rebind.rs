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
/// `pressed` is this frame's physical controls in reader order; the first
/// bindable one wins. A frame with only unbindable presses captures nothing
/// and the row stays armed, because the game cannot store that press.
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
/// The keyboard is read raw, not through the seat's `InputMap`. The map only
/// reports keys it already binds, so reading through it would limit rebinding
/// to keys that are already bound.
///
/// Order is the enum order, not hardware order. `ButtonInput` iterates a hash
/// set, so it cannot say which key was first. Sorting makes a two-key frame
/// give the same result on every run. A capture screen should still ask for
/// one key at a time.
///
/// Pads are included because a couch seat rebinds buttons, not keys.
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
    // Sort so a multi-press frame is deterministic, not hash-ordered.
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
        // `Other` exists so the projection is total. It cannot be rebuilt
        // from its debug string, so a stored `Other` would load as nothing.
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
        // The game binds Escape to both Start and MenuBack on purpose, so a
        // capture must report collisions, not refuse them.
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
        // `ButtonInput` iterates a hash set; the result must not depend on it.
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
        // The keyboard is read raw, so a key no preset binds can be captured.
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
