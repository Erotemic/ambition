//! Context-specific button bindings. Compile the declaration once per recipe change.

use bevy::prelude::{Component, GamepadButton, KeyCode, Resource};
use leafwing_input_manager::prelude::{Buttonlike, GamepadControlAxis, GamepadControlDirection, GamepadStick, InputMap, VirtualDPad};
use std::sync::Arc;

use crate::{InputAction as Action, InputProfile, OverrideControl, PhysicalControl, PresetId, BindingSources, BindingOverride};
use crate::bindings::{action_named, physical_control_of};

/// A row of default or game-owned policy. `None` removes this control in this profile.
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

/// The existing analog and digital-direction adapters. Their assignment is data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisSource {
    ArrowKeys,
    WasdKeys,
    Dpad,
    LeftStick,
    RightStick,
    RightTrigger,
    LeftStickLeft,
    LeftStickRight,
    LeftStickUp,
    LeftStickDown,
}

impl AxisSource {
    fn admitted(self, sources: BindingSources) -> bool {
        match self {
            Self::ArrowKeys | Self::WasdKeys => sources.admits_keyboard(),
            _ => sources.admits_gamepad(),
        }
    }

    fn install(self, action: Action, map: &mut InputMap<Action>) {
        match self {
            Self::ArrowKeys => { map.insert_dual_axis(action, VirtualDPad::arrow_keys()); }
            Self::WasdKeys => { map.insert_dual_axis(action, VirtualDPad::wasd()); }
            Self::Dpad => { map.insert_dual_axis(action, VirtualDPad::dpad()); }
            Self::LeftStick => { map.insert_dual_axis(action, GamepadStick::LEFT); }
            Self::RightStick => { map.insert_dual_axis(action, GamepadStick::RIGHT); }
            Self::RightTrigger => { map.insert_axis(action, GamepadControlAxis::RIGHT_Z); }
            Self::LeftStickLeft => { map.insert(action, GamepadControlDirection::LEFT_LEFT.threshold(0.5)); }
            Self::LeftStickRight => { map.insert(action, GamepadControlDirection::LEFT_RIGHT.threshold(0.5)); }
            Self::LeftStickUp => { map.insert(action, GamepadControlDirection::LEFT_UP.threshold(0.5)); }
            Self::LeftStickDown => { map.insert(action, GamepadControlDirection::LEFT_DOWN.threshold(0.5)); }
        }
    }
}

/// An analog or stick-direction binding. The destination determines the control kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisBinding {
    pub source: AxisSource,
    pub action: Action,
}

impl AxisBinding {
    pub const fn new(source: AxisSource, action: Action) -> Self { Self { source, action } }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingScheme {
    pub buttons: Vec<Binding>,
    pub axes: Vec<AxisBinding>,
}

/// The host supplies defaults. No game name or default layout is needed by the engine.
/// Each participant retains a cheap clone of the selected catalog.
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

/// Effective button row, including a concrete virtual device button if present.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedButton {
    pub profile: InputProfile,
    pub control: Box<dyn Buttonlike>,
    pub action: Action,
}

/// The effective contextual button table. Leafwing's acquisition map and the
/// menu sampler both read these rows. No action combinations encode a context.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BindingTable {
    rows: Vec<ResolvedButton>,
}

impl BindingTable {
    pub fn rows(&self) -> &[ResolvedButton] { &self.rows }

    pub fn set(&mut self, profile: InputProfile, control: Box<dyn Buttonlike>, action: Option<Action>) {
        if profile == InputProfile::Blocked { return; }
        // Character actions only have a gameplay consumer. Menu actions have a UI consumer.
        if action.is_some_and(|action| !profile.accepts(action)) { return; }
        let index = self.rows.iter().position(|row| row.profile == profile && &row.control == &control);
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
        if !sources.admits(binding.control.device_class()) { return; }
        self.set(binding.profile, boxed_control(binding.control), binding.action);
    }

    /// Move this action on the selected device in each of its declared profiles.
    /// A same-profile collision is replaced; other profiles are unaffected.
    pub fn apply_override(&mut self, over: &BindingOverride, sources: BindingSources) {
        use leafwing_input_manager::prelude::Actionlike;
        if !sources.admits(over.control.device_class()) { return; }
        let Some(action) = action_named(&over.action) else { return; };
        if action.input_control_kind() != leafwing_input_manager::InputControlKind::Button { return; }
        let class = over.control.device_class();
        let mut profiles = Vec::new();
        for row in &self.rows {
            if row.action == action && control_class(&row.control) == Some(class) && !profiles.contains(&row.profile) {
                profiles.push(row.profile);
            }
        }
        // A previously unbound action can acquire a binding. Use the action's
        // existing profiles on another device, or its consumer's natural profile.
        if profiles.is_empty() {
            for row in &self.rows {
                if row.action == action && !profiles.contains(&row.profile) { profiles.push(row.profile); }
            }
        }
        if profiles.is_empty() {
            profiles.extend(InputProfile::ACTIVE.into_iter().filter(|profile| profile.accepts(action)));
        }
        for profile in profiles {
            self.rows.retain(|row| !(row.profile == profile && row.action == action && control_class(&row.control) == Some(class)));
            self.set(profile, boxed_control(over.control), Some(action));
        }
    }

    /// The raw map is an acquisition projection. UI resolution uses the table,
    /// not this union. Gameplay-only actions exist only in the gameplay rows.
    pub fn install_buttons(&self, map: &mut InputMap<Action>) {
        for row in &self.rows {
            map.insert_boxed(row.action, row.control.clone());
        }
    }

    pub fn action_for(&self, profile: InputProfile, control: &PhysicalControl) -> Option<Action> {
        self.rows.iter().find(|row| row.profile == profile && physical_control_of(row.control.as_ref()) == *control).map(|row| row.action)
    }

    /// Project the selected profile for prompts, remap previews and census.
    pub fn map_for_profile(&self, profile: InputProfile) -> InputMap<Action> {
        let mut map = InputMap::default();
        for row in self.rows.iter().filter(|row| row.profile == profile) {
            map.insert_boxed(row.action, row.control.clone());
        }
        map
    }

    /// A fixture can inject an action map without a recipe. Interpret each
    /// action only in its consumer's profile; do not infer key-specific policy.
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

fn boxed_control(control: OverrideControl) -> Box<dyn Buttonlike> {
    match control {
        OverrideControl::Key(key) => Box::new(key),
        OverrideControl::Button(button) => Box::new(button),
    }
}

fn control_class(control: &Box<dyn Buttonlike>) -> Option<crate::OverrideDeviceClass> {
    let reflected = control.as_ref().as_reflect();
    if reflected.downcast_ref::<KeyCode>().is_some() { Some(crate::OverrideDeviceClass::Keyboard) }
    else if reflected.downcast_ref::<GamepadButton>().is_some() { Some(crate::OverrideDeviceClass::Gamepad) }
    else { None }
}

impl BindingScheme {
    pub fn resolve(&self, sources: BindingSources, changes: &[Binding], overrides: &[BindingOverride]) -> (InputMap<Action>, BindingTable) {
        let mut table = BindingTable::default();
        for row in self.buttons.iter().chain(changes) { table.apply(*row, sources); }
        for over in overrides { table.apply_override(over, sources); }
        let mut map = InputMap::default();
        for axis in &self.axes {
            if !axis.source.admitted(sources) { continue; }
            // Do not leave the old trigger live after a button remap displaces Burst.
            if axis.action == Action::BurstAnalog && axis.source == AxisSource::RightTrigger
                && table.action_for(InputProfile::Gameplay, &PhysicalControl::Button(GamepadButton::RightTrigger2)) != Some(Action::Burst) { continue; }
            let direction = match axis.source {
                AxisSource::LeftStickLeft => Some(GamepadControlDirection::LEFT_LEFT),
                AxisSource::LeftStickRight => Some(GamepadControlDirection::LEFT_RIGHT),
                AxisSource::LeftStickUp => Some(GamepadControlDirection::LEFT_UP),
                AxisSource::LeftStickDown => Some(GamepadControlDirection::LEFT_DOWN),
                _ => None,
            };
            if let Some(direction) = direction {
                table.bind_button(InputProfile::Gameplay, direction.threshold(0.5), axis.action);
            } else {
                axis.source.install(axis.action, &mut map);
            }
        }
        table.install_buttons(&mut map);
        (map, table)
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
}
