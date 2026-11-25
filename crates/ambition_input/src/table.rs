//! Compose the declared policy into an effective contextual binding table.

use bevy::prelude::{Component, GamepadButton, KeyCode, Resource};
use leafwing_input_manager::prelude::{Actionlike, Buttonlike, GamepadControlDirection, InputMap};
use leafwing_input_manager::InputControlKind;
use std::sync::Arc;

use crate::{AxisSource, BindingOverride, BindingSources, InputAction as Action, InputProfile,
    OverrideControl, OverrideDeviceClass, PhysicalControl, PresetId};
use crate::bindings::{action_named, physical_control_of};
use crate::touch_device::TouchVirtualButton;
use crate::touch_state::TouchStickDirection;

/// None removes a control in this profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub profile: InputProfile,
    pub control: OverrideControl,
    pub action: Option<Action>,
}

pub const fn bind(profile: InputProfile, control: OverrideControl, action: Action) -> Binding {
    Binding { profile, control, action: Some(action) }
}

pub const fn unbind(profile: InputProfile, control: OverrideControl) -> Binding {
    Binding { profile, control, action: None }
}

/// Analog assignment. A paired source is active only while its declared
/// button is still assigned to the specified action. Independent axis remaps
/// do not inherit the pair condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisBinding {
    pub source: AxisSource,
    pub action: Action,
    pub requires_button: Option<(OverrideControl, Action)>,
}

impl AxisBinding {
    pub const fn new(source: AxisSource, action: Action) -> Self {
        Self { source, action, requires_button: None }
    }
    pub const fn paired(source: AxisSource, action: Action, control: OverrideControl, button: Action) -> Self {
        Self { source, action, requires_button: Some((control, button)) }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingScheme {
    pub buttons: Vec<Binding>,
    pub axes: Vec<AxisBinding>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingDefaults(Arc<Vec<(PresetId, BindingScheme)>>);

impl BindingDefaults {
    pub fn new(schemes: Vec<(PresetId, BindingScheme)>) -> Self { Self(Arc::new(schemes)) }
    pub fn scheme(&self, preset: PresetId) -> Option<&BindingScheme> {
        self.0.iter().find(|(id, _)| *id == preset).map(|(_, scheme)| scheme)
    }
    pub fn recipe(&self, preset: PresetId) -> crate::BindingRecipe {
        crate::BindingRecipe::new(preset, self.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedButton {
    pub profile: InputProfile,
    pub control: Box<dyn Buttonlike>,
    pub action: Action,
}

/// Effective policy, after defaults, game changes and user overrides.
/// The acquisition map is a projection, not a second binding authority.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BindingTable {
    rows: Vec<ResolvedButton>,
    axes: Vec<AxisBinding>,
}

impl BindingTable {
    pub fn rows(&self) -> &[ResolvedButton] { &self.rows }
    pub fn axes(&self) -> &[AxisBinding] { &self.axes }

    pub fn set(&mut self, profile: InputProfile, control: Box<dyn Buttonlike>, action: Option<Action>) {
        if profile == InputProfile::Blocked { return; }
        if action.is_some_and(|action| !profile.accepts(action) || action.input_control_kind() != InputControlKind::Button) { return; }
        let index = self.rows.iter().position(|row| row.profile == profile && row.control.as_ref() == control.as_ref());
        if let Some(index) = index {
            if let Some(action) = action { self.rows[index].action = action; }
            else { self.rows.remove(index); }
        } else if let Some(action) = action {
            self.rows.push(ResolvedButton { profile, control, action });
        }
    }

    pub fn bind_button(&mut self, profile: InputProfile, control: impl Buttonlike, action: Action) {
        self.set(profile, Box::new(control), Some(action));
    }

    pub fn apply(&mut self, binding: Binding, sources: BindingSources) {
        self.apply_devices(binding, sources, false);
    }

    fn apply_devices(&mut self, binding: Binding, sources: BindingSources, touch: bool) {
        if !admitted(sources, touch, binding.control.device_class()) { return; }
        if let Some(control) = boxed_control(binding.control) {
            self.set(binding.profile, control, binding.action);
        }
    }

    /// Move the action in its existing profiles on this device. A collision
    /// in one profile has no effect on the same physical control elsewhere.
    pub fn apply_override(&mut self, over: &BindingOverride, sources: BindingSources) {
        self.apply_button_override(over, sources, false);
    }

    fn apply_button_override(&mut self, over: &BindingOverride, sources: BindingSources, touch: bool) {
        let Some(control) = boxed_control(over.control) else { return; };
        if !admitted(sources, touch, over.control.device_class()) { return; }
        let Some(action) = action_named(&over.action) else { return; };
        if action.input_control_kind() != InputControlKind::Button { return; }
        let class = over.control.device_class();
        let mut profiles = Vec::new();
        for row in &self.rows {
            if row.action == action && control_class(row.control.as_ref()) == Some(class) && !profiles.contains(&row.profile) {
                profiles.push(row.profile);
            }
        }
        if profiles.is_empty() {
            for row in &self.rows {
                if row.action == action && !profiles.contains(&row.profile) { profiles.push(row.profile); }
            }
        }
        if profiles.is_empty() {
            profiles.extend(InputProfile::ACTIVE.into_iter().filter(|profile| profile.accepts(action)));
        }
        for profile in profiles {
            self.rows.retain(|row| !(row.profile == profile && row.action == action && control_class(row.control.as_ref()) == Some(class)));
            self.set(profile, control.clone(), Some(action));
        }
    }

    pub fn install_buttons(&self, map: &mut InputMap<Action>) {
        for row in &self.rows { map.insert_boxed(row.action, row.control.clone()); }
    }

    pub fn action_for(&self, profile: InputProfile, control: &PhysicalControl) -> Option<Action> {
        self.rows.iter().find(|row| row.profile == profile && physical_control_of(row.control.as_ref()) == *control).map(|row| row.action)
    }

    /// A projection for the input adapter. Character actions have exactly
    /// one permitted profile; UI and session button intents use direct lookup.
    pub fn acquisition_map(&self) -> InputMap<Action> {
        let mut map = InputMap::default();
        self.install_buttons(&mut map);
        for axis in &self.axes { axis.source.install(axis.action, self, &mut map); }
        map
    }

    /// Project the selected profile for glyphs and inspection.
    pub fn map_for_profile(&self, profile: InputProfile) -> InputMap<Action> {
        let mut map = InputMap::default();
        for row in self.rows.iter().filter(|row| row.profile == profile) {
            map.insert_boxed(row.action, row.control.clone());
        }
        for axis in self.axes.iter().filter(|axis| profile.accepts(axis.action)) {
            axis.source.install(axis.action, self, &mut map);
        }
        map
    }

    /// Import an explicitly supplied Leafwing map. A participant with a
    /// recipe does not use this path. No contextual policy is inferred from
    /// combinations of actions.
    pub fn from_action_map(map: &InputMap<Action>) -> Self {
        let mut table = Self::default();
        for (action, controls) in map.iter_buttonlike() {
            for profile in InputProfile::ACTIVE.into_iter().filter(|profile| profile.accepts(*action)) {
                for control in controls { table.set(profile, control.clone(), Some(*action)); }
            }
        }
        table
    }
}

fn admitted(sources: BindingSources, touch: bool, class: OverrideDeviceClass) -> bool {
    if class == OverrideDeviceClass::Touch { touch } else { sources.admits(class) }
}

fn boxed_control(control: OverrideControl) -> Option<Box<dyn Buttonlike>> {
    Some(match control {
        OverrideControl::Key(key) => Box::new(key),
        OverrideControl::Button(button) => Box::new(button),
        OverrideControl::Touch(button) => Box::new(TouchVirtualButton(button)),
        OverrideControl::Axis(_) => return None,
    })
}

pub(crate) fn control_class(control: &dyn Buttonlike) -> Option<OverrideDeviceClass> {
    let reflected = control.as_reflect();
    if reflected.downcast_ref::<KeyCode>().is_some() { Some(OverrideDeviceClass::Keyboard) }
    else if reflected.downcast_ref::<GamepadButton>().is_some() { Some(OverrideDeviceClass::Gamepad) }
    else if reflected.downcast_ref::<TouchVirtualButton>().is_some() { Some(OverrideDeviceClass::Touch) }
    else { None } // Analog threshold edges are not digital direction buttons.
}

fn physical(control: OverrideControl) -> PhysicalControl {
    match control {
        OverrideControl::Key(key) => PhysicalControl::Key(key),
        OverrideControl::Button(button) => PhysicalControl::Button(button),
        OverrideControl::Touch(button) => PhysicalControl::Touch(button),
        OverrideControl::Axis(source) => PhysicalControl::Axis(source),
    }
}

fn set_axis(axes: &mut Vec<AxisBinding>, row: AxisBinding) {
    if row.source.kind() != row.action.input_control_kind() { return; }
    if matches!(row.source, AxisSource::KeyboardDirections | AxisSource::PadDirections) && row.action != Action::Move { return; }
    // A stick can move in gameplay and navigate a UI, but cannot be two
    // independent gameplay axes in the same profile.
    axes.retain(|old| old.source != row.source || !InputProfile::ACTIVE.into_iter().any(|p| p.accepts(old.action) && p.accepts(row.action)));
    axes.push(row);
}

impl BindingScheme {
    pub fn resolve(&self, sources: BindingSources, changes: &[Binding], overrides: &[BindingOverride]) -> (InputMap<Action>, BindingTable) {
        self.resolve_devices(sources, false, changes, overrides, None)
    }

    pub fn resolve_devices(&self, sources: BindingSources, touch: bool, changes: &[Binding], overrides: &[BindingOverride], game_axes: Option<&[AxisBinding]>) -> (InputMap<Action>, BindingTable) {
        let mut table = BindingTable::default();
        for row in self.buttons.iter().chain(changes) { table.apply_devices(*row, sources, touch); }
        for over in overrides { table.apply_button_override(over, sources, touch); }
        let mut axes = Vec::new();
        for row in game_axes.unwrap_or(&self.axes) {
            if !admitted(sources, touch, row.source.device_class()) { continue; }
            if row.requires_button.is_some_and(|(control, action)| table.action_for(InputProfile::Gameplay, &physical(control)) != Some(action)) { continue; }
            set_axis(&mut axes, *row);
        }
        for over in overrides {
            let OverrideControl::Axis(source) = over.control else { continue; };
            if !admitted(sources, touch, source.device_class()) { continue; }
            let Some(action) = action_named(&over.action) else { continue; };
            if action.input_control_kind() != source.kind() { continue; }
            if matches!(source, AxisSource::KeyboardDirections | AxisSource::PadDirections) && action != Action::Move { continue; }
            axes.retain(|row| !(row.action == action && row.source.device_class() == source.device_class()
                && !matches!(row.source, AxisSource::KeyboardDirections | AxisSource::PadDirections)));
            set_axis(&mut axes, AxisBinding::new(source, action));
        }
        // Threshold edges belong to the actual movement axis, never to a
        // particular gamepad stick independently of its selected action.
        for axis in &axes {
            if axis.action != Action::Move { continue; }
            let directions = match axis.source {
                AxisSource::LeftStick => Some([GamepadControlDirection::LEFT_UP, GamepadControlDirection::LEFT_DOWN,
                    GamepadControlDirection::LEFT_LEFT, GamepadControlDirection::LEFT_RIGHT]),
                AxisSource::RightStick => Some([GamepadControlDirection::RIGHT_UP, GamepadControlDirection::RIGHT_DOWN,
                    GamepadControlDirection::RIGHT_LEFT, GamepadControlDirection::RIGHT_RIGHT]),
                _ => None,
            };
            let actions = [Action::MoveUp, Action::MoveDown, Action::MoveLeft, Action::MoveRight];
            if let Some(directions) = directions {
                for (direction, action) in directions.into_iter().zip(actions) {
                    table.bind_button(InputProfile::Gameplay, direction.threshold(0.5), action);
                }
            }
            if axis.source == AxisSource::TouchStick {
                for (direction, action) in [TouchStickDirection::Up, TouchStickDirection::Down,
                    TouchStickDirection::Left, TouchStickDirection::Right].into_iter().zip(actions) {
                    table.bind_button(InputProfile::Gameplay, direction, action);
                }
            }
        }
        table.axes = axes;
        (table.acquisition_map(), table)
    }
}

#[cfg(test)]
mod tests {
    use super::{bind, BindingScheme};
    use crate::{InputAction as Action, InputProfile as Context, OverrideControl, PhysicalControl,
        BindingSources, BindingOverride};
    use bevy::prelude::KeyCode as Key;

    #[test]
    fn remapping_back_does_not_change_gameplay_pause_or_terminal_pause() {
        let escape = OverrideControl::Key(Key::Escape);
        let scheme = BindingScheme { buttons: vec![
            bind(Context::Gameplay, escape, Action::Start),
            bind(Context::SessionUi, escape, Action::Start),
            bind(Context::Paused, escape, Action::MenuBack),
        ], axes: Vec::new() };
        let (_, table) = scheme.resolve(BindingSources::Unified, &[], &[
            BindingOverride::key("MenuBack", Key::F13),
        ]);
        for context in [Context::Gameplay, Context::SessionUi] {
            assert_eq!(table.action_for(context, &PhysicalControl::Key(Key::Escape)), Some(Action::Start));
        }
        assert_eq!(table.action_for(Context::Paused, &PhysicalControl::Key(Key::Escape)), None);
        assert_eq!(table.action_for(Context::Paused, &PhysicalControl::Key(Key::F13)), Some(Action::MenuBack));
    }

    #[test]
    fn game_and_user_changes_only_displace_rows_in_their_context() {
        let south = OverrideControl::Key(Key::KeyZ);
        let scheme = BindingScheme { buttons: vec![
            bind(Context::Gameplay, south, Action::Jump),
            bind(Context::Paused, south, Action::MenuSelect),
        ], axes: Vec::new() };
        let changes = [bind(Context::Gameplay, south, Action::Attack)];
        let (_, table) = scheme.resolve(BindingSources::Unified, &changes, &[
            BindingOverride::key("Attack", Key::F13),
        ]);
        assert_eq!(table.action_for(Context::Gameplay, &PhysicalControl::Key(Key::KeyZ)), None);
        assert_eq!(table.action_for(Context::Gameplay, &PhysicalControl::Key(Key::F13)), Some(Action::Attack));
        assert_eq!(table.action_for(Context::Paused, &PhysicalControl::Key(Key::KeyZ)), Some(Action::MenuSelect));
    }

    #[test]
    fn touch_override_changes_only_the_touch_gameplay_row() {
        use crate::touch_state::TouchActionButton as Touch;
        let recipe = crate::fixture_catalog::recipe(crate::PresetId::ArrowsZxc).with_touch(true)
            .with_overrides(vec![BindingOverride { action: "Jump".into(), control: OverrideControl::Touch(Touch::Attack) }]);
        let (_, table) = recipe.resolve();
        assert_eq!(table.action_for(Context::Gameplay, &PhysicalControl::Touch(Touch::Jump)), None);
        assert_eq!(table.action_for(Context::Gameplay, &PhysicalControl::Touch(Touch::Attack)), Some(Action::Jump));
        assert_eq!(table.action_for(Context::Paused, &PhysicalControl::Touch(Touch::Jump)), Some(Action::MenuSelect));
        assert_eq!(table.action_for(Context::Gameplay, &PhysicalControl::Key(Key::KeyZ)), Some(Action::Jump));
        let (_, absent) = recipe.with_touch(false).resolve();
        assert!(!absent.rows().iter().any(|row| matches!(crate::physical_control_of(row.control.as_ref()), PhysicalControl::Touch(_))));
    }

    #[test]
    fn changing_burst_button_drops_the_old_paired_trigger_axis() {
        use crate::AxisSource;
        use bevy::prelude::GamepadButton as Pad;
        let recipe = crate::fixture_catalog::recipe(crate::PresetId::ArrowsZxc);
        assert!(recipe.resolve().1.axes().iter().any(|row| row.action == Action::BurstAnalog));
        let recipe = recipe.with_overrides(vec![BindingOverride::button("Burst", Pad::North)]);
        assert!(!recipe.resolve().1.axes().iter().any(|row| row.action == Action::BurstAnalog));
        let recipe = recipe.with_overrides(vec![BindingOverride::button("Burst", Pad::North), BindingOverride::axis("BurstAnalog", AxisSource::LeftTrigger)]);
        assert!(recipe.resolve().1.axes().iter().any(|row| row.action == Action::BurstAnalog && row.source == AxisSource::LeftTrigger));
    }

    #[test]
    fn remapping_the_movement_stick_changes_its_threshold_edges() {
        use crate::AxisSource;
        use leafwing_input_manager::prelude::GamepadControlDirection as Direction;
        let recipe = crate::fixture_catalog::recipe(crate::PresetId::ArrowsZxc)
            .with_overrides(vec![BindingOverride::axis("Move", AxisSource::RightStick)]);
        let (_, table) = recipe.resolve();
        assert!(table.axes().iter().any(|row| row.action == Action::Move && row.source == AxisSource::RightStick));
        assert!(!table.axes().iter().any(|row| row.action == Action::Move && row.source == AxisSource::LeftStick));
        assert!(table.axes().iter().any(|row| row.action == Action::MenuStick && row.source == AxisSource::LeftStick));
        let physical = crate::physical_control_of(&Direction::RIGHT_UP.threshold(0.5));
        assert_eq!(table.action_for(Context::Gameplay, &physical), Some(Action::MoveUp));
        let physical = crate::physical_control_of(&Direction::LEFT_UP.threshold(0.5));
        assert_eq!(table.action_for(Context::Gameplay, &physical), None);
    }

    #[test]
    fn a_direction_remap_changes_the_vector_and_the_button_edge() {
        use bevy::prelude::{App, Vec2};
        use leafwing_input_manager::plugin::InputManagerPlugin;
        use leafwing_input_manager::prelude::{ActionState, Buttonlike};
        let (map, _) = crate::fixture_catalog::recipe(crate::PresetId::ArrowsZxc)
            .with_sources(BindingSources::KeyboardOnly)
            .with_overrides(vec![BindingOverride::key("MoveRight", Key::F13)]).resolve();
        let mut app = App::new();
        app.add_plugins((bevy::input::InputPlugin, bevy::time::TimePlugin, InputManagerPlugin::<Action>::default()));
        let seat = app.world_mut().spawn((map, ActionState::<Action>::default())).id();
        app.update();
        Key::ArrowRight.press(app.world_mut());
        app.update();
        let state = app.world().get::<ActionState<Action>>(seat).unwrap();
        assert_eq!(state.clamped_axis_pair(&Action::Move), Vec2::ZERO);
        assert!(!state.pressed(&Action::MoveRight));
        Key::ArrowRight.release(app.world_mut());
        Key::F13.press(app.world_mut());
        app.update();
        let state = app.world().get::<ActionState<Action>>(seat).unwrap();
        assert_eq!(state.clamped_axis_pair(&Action::Move), Vec2::X);
        assert!(state.just_pressed(&Action::MoveRight));
    }

}
