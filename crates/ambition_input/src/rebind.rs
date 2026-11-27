//! Pure capture of physical input into persisted binding overrides. Menu
//! presentation and arming policy live outside this module.

use crate::bindings::{ActionBindings, PhysicalControl};
use crate::settings::{BindingOverride, OverrideControl};
use crate::InputAction;

/// The control a capture accepted. `PhysicalControl::Other` is not persistable
/// and is therefore refused.
pub fn bindable(control: &PhysicalControl) -> Option<OverrideControl> {
    match control {
        PhysicalControl::Key(key) => Some(OverrideControl::Key(*key)),
        PhysicalControl::Button(button) => Some(OverrideControl::Button(*button)),
        PhysicalControl::Touch(button) => Some(OverrideControl::Touch(*button)),
        PhysicalControl::TouchDirection(_) => None,
        PhysicalControl::Axis(source) => Some(OverrideControl::Axis(*source)),
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
    action: &InputAction,
    pressed: impl IntoIterator<Item = PhysicalControl>,
) -> Option<BindingOverride> {
    use leafwing_input_manager::prelude::Actionlike;
    let control = pressed.into_iter().filter_map(|control| bindable(&control)).find(|control| {
        let kind = match control {
            OverrideControl::Axis(source) => source.kind(),
            _ => leafwing_input_manager::InputControlKind::Button,
        };
        kind == action.input_control_kind()
    })?;
    Some(BindingOverride {
        action: crate::bindings::action_name(action),
        control,
    })
}

/// Other actions already bound to this control, in canonical order. Duplicate
/// controls may be reused in different profiles. A same-profile remap
/// displaces the previous action.
pub fn also_bound_to(
    bindings: &ActionBindings,
    action: &InputAction,
    control: OverrideControl,
) -> Vec<String> {
    let wanted = match control {
        OverrideControl::Key(key) => PhysicalControl::Key(key),
        OverrideControl::Button(button) => PhysicalControl::Button(button),
        OverrideControl::Touch(button) => PhysicalControl::Touch(button),
        OverrideControl::Axis(source) => PhysicalControl::Axis(source),
    };
    if let Some(rows) = bindings.contextual_rows() {
        let mut profiles = rows.iter().filter(|(_, a, c)| a == action && c.device_class() == Some(control.device_class()))
            .map(|(profile, _, _)| *profile).collect::<Vec<_>>();
        if profiles.is_empty() {
            profiles.extend(rows.iter().filter(|(_, a, _)| a == action).map(|(p, _, _)| *p));
        }
        if profiles.is_empty() {
            profiles.extend(crate::InputProfile::ACTIVE.into_iter().filter(|p| p.accepts(*action)));
        }
        let mut conflicts = rows.iter().filter(|(p, a, c)| profiles.contains(p) && a != action && c == &wanted)
            .map(|(_, a, _)| crate::bindings::action_name(a)).collect::<Vec<_>>();
        conflicts.sort();
        conflicts.dedup();
        return conflicts;
    }
    let self_name = crate::bindings::action_name(action);
    bindings
        .all()
        .filter(|(name, _)| *name != self_name)
        .filter(|(_, controls)| controls.contains(&wanted))
        .map(|(name, _)| name.to_string())
        .collect()
}

/// A live binding and its conflicts for one semantic action.
pub struct BindingRow {
    pub action: InputAction,
    pub binding: String,
    pub also: Vec<String>,
}

/// Read the current bindings in action order for a menu or help screen.
pub fn rebind_rows(
    bindings: &ActionBindings,
    device: crate::ActiveDevice,
) -> impl Iterator<Item = BindingRow> + '_ {
    bindings.all().filter_map(move |(name, controls)| {
        let action = crate::action_named(name)?;
        let control = bindings.control_for(&action, device).or_else(|| controls.first());
        Some(BindingRow {
            action,
            binding: bindings.label_for(&action, device).unwrap_or_default(),
            also: control.and_then(bindable)
                .map_or_else(Vec::new, |control| also_bound_to(bindings, &action, control)),
        })
    })
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
            &InputAction::Jump,
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
            &InputAction::Jump,
            [PhysicalControl::Other("Chord(A, B)".into())]
        )
        .is_none());
    }

    #[test]
    fn an_unbindable_press_does_not_hide_a_bindable_one_behind_it() {
        let captured = capture(
            &InputAction::Attack,
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
        let mut map = KeyboardPreset::arrows_zxc().input_map();
        map.insert(InputAction::Start, KeyCode::F13);
        map.insert(InputAction::MenuBack, KeyCode::F13);
        let bindings = ActionBindings::from_map(&map);
        let start = OverrideControl::Key(KeyCode::F13);

        let others = also_bound_to(&bindings, &InputAction::Start, start);
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
            &InputAction::Jump,
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
            &InputAction::Jump,
            pressed_controls_this_frame(Some(&keys), []),
        )
        .expect("an unbound key is exactly what a rebind is for");
        assert_eq!(captured.control, OverrideControl::Key(KeyCode::F13));
    }
}
