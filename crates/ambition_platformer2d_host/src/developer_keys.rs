//! The physical keys of the developer actions: the default deck, the player's
//! settings over it, and the one reader that turns a pressed chord into a
//! [`DeveloperAction`].
//!
//! The actions are the foundation's (`developer_hotkeys`); the keys are the
//! host's, so a key change edits this crate or the settings file, never the
//! foundation every domain builds on.

use std::collections::HashSet;

use ambition_platformer2d_shared_tangle::developer_hotkeys::{
    DeveloperAction, DeveloperActionLabels, DeveloperHotkeyEmitSet,
};
use bevy::prelude::*;

/// One exact keyboard chord. Extra modifiers suppress a match so `F8` and
/// `Shift+F8` are distinct rather than both firing from the shifted chord.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeveloperKeyChord {
    pub key: KeyCode,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

impl DeveloperKeyChord {
    pub const fn key(key: KeyCode) -> Self {
        Self { key, shift: false, control: false, alt: false }
    }

    pub const fn shift(key: KeyCode) -> Self {
        Self { key, shift: true, control: false, alt: false }
    }

    pub fn label(self) -> String {
        let mut pieces = Vec::new();
        if self.control {
            pieces.push("Ctrl".to_owned());
        }
        if self.alt {
            pieces.push("Alt".to_owned());
        }
        if self.shift {
            pieces.push("Shift".to_owned());
        }
        pieces.push(format!("{:?}", self.key));
        pieces.join("+")
    }

    fn matches(self, keys: &ButtonInput<KeyCode>) -> bool {
        if !keys.just_pressed(self.key) {
            return false;
        }
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let control = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
        shift == self.shift && control == self.control && alt == self.alt
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeveloperHotkeyBinding {
    pub action: DeveloperAction,
    pub chord: DeveloperKeyChord,
}

/// The deck in force: the defaults with the player's settings over them.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DeveloperHotkeyBindings(pub Vec<DeveloperHotkeyBinding>);

impl Default for DeveloperHotkeyBindings {
    fn default() -> Self {
        use DeveloperAction as A;
        use DeveloperKeyChord as C;
        let bind = |action, chord| DeveloperHotkeyBinding { action, chord };
        Self(vec![
            bind(A::ToggleDebugOverlay, C::key(KeyCode::F1)),
            // Beside the overlay it is a view of: F1 shows the overlay as set,
            // Shift+F1 shows it in its combat view.
            bind(A::ToggleCombatDebug, C::shift(KeyCode::F1)),
            bind(A::ToggleSlowMotion, C::key(KeyCode::F2)),
            bind(A::ToggleInspector, C::key(KeyCode::F3)),
            bind(A::ToggleWorldInspector, C::key(KeyCode::F4)),
            bind(A::ToggleOverviewCamera, C::key(KeyCode::F5)),
            bind(A::ToggleFpsOverlay, C::key(KeyCode::F6)),
            bind(A::TogglePortalGun, C::key(KeyCode::F7)),
            // Beside the FPS overlay on purpose: both answer "what is this
            // machine actually doing", and F1..F12 unshifted are all spoken for.
            bind(A::ToggleGamepadProbe, C::shift(KeyCode::F6)),
            bind(A::DumpGameplayTrace, C::key(KeyCode::F8)),
            bind(A::DumpPortalViewCones, C::shift(KeyCode::F8)),
            bind(A::RequestRollbackProof, C::key(KeyCode::F9)),
            bind(A::QuitToHome, C::key(KeyCode::F10)),
            bind(A::ApplyLdtkReload, C::key(KeyCode::F11)),
            bind(A::ToggleLdtkAutoApply, C::key(KeyCode::F12)),
        ])
    }
}

impl DeveloperHotkeyBindings {
    pub fn chord_for(&self, action: DeveloperAction) -> Option<DeveloperKeyChord> {
        self.0.iter().find(|binding| binding.action == action).map(|binding| binding.chord)
    }

    pub fn label_for(&self, action: DeveloperAction) -> Option<String> {
        self.chord_for(action).map(DeveloperKeyChord::label)
    }

    /// The defaults with `overrides` over them, or the first problem: a chord
    /// bound to two actions. An override naming no action is skipped.
    pub fn with_overrides(overrides: &[ambition_input::DeveloperKeyOverride]) -> Result<Self, String> {
        let mut deck = Self::default();
        for over in overrides {
            let Some(action) = DeveloperAction::named(&over.action) else { continue };
            let chord = DeveloperKeyChord { key: over.key, shift: over.shift, control: over.control, alt: over.alt };
            match deck.0.iter_mut().find(|binding| binding.action == action) {
                Some(binding) => binding.chord = chord,
                None => deck.0.push(DeveloperHotkeyBinding { action, chord }),
            }
        }
        deck.problem().map_or(Ok(deck), Err)
    }

    fn problem(&self) -> Option<String> {
        let mut actions = HashSet::new();
        let mut chords = HashSet::new();
        for binding in &self.0 {
            if !actions.insert(binding.action) {
                return Some(format!("developer action {:?} has more than one physical binding", binding.action));
            }
            if !chords.insert(binding.chord) {
                return Some(format!("developer chord {} is assigned more than once", binding.chord.label()));
            }
        }
        None
    }

    fn labels(&self) -> DeveloperActionLabels {
        DeveloperActionLabels(self.0.iter().map(|binding| (binding.action, binding.chord.label())).collect())
    }
}

/// Installs the single physical-key reader. Consumers add no keyboard systems;
/// they listen for [`DeveloperAction`] messages instead.
pub struct DeveloperHotkeyPlugin;

impl Plugin for DeveloperHotkeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeveloperHotkeyBindings>();
        let deck = app.world().resource::<DeveloperHotkeyBindings>();
        if let Some(problem) = deck.problem() {
            panic!("{problem}");
        }
        let labels = deck.labels();
        app.insert_resource(labels);
        app.add_message::<DeveloperAction>()
            .add_systems(PreUpdate, apply_developer_key_settings.before(DeveloperHotkeyEmitSet))
            .add_systems(
                PreUpdate,
                emit_developer_actions
                    .in_set(DeveloperHotkeyEmitSet)
                    // `ButtonInput<KeyCode>` is populated by Bevy's input pass
                    // in this same schedule. Without the edge, a physical F-key
                    // can become visible only after this reader has sampled the
                    // frame.
                    .after(bevy::input::InputSystems),
            );
    }
}

/// The player's settings over the defaults, whenever the settings change. A
/// set that binds one chord twice keeps the defaults and says why.
fn apply_developer_key_settings(
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut deck: ResMut<DeveloperHotkeyBindings>,
    mut labels: ResMut<DeveloperActionLabels>,
) {
    let Some(settings) = settings.filter(|settings| settings.is_changed()) else { return };
    let next = DeveloperHotkeyBindings::with_overrides(&settings.controls.developer_keys).unwrap_or_else(|problem| {
        warn!(target: "ambition::developer_keys", "the settings' developer keys are not used: {problem}");
        DeveloperHotkeyBindings::default()
    });
    if *deck != next {
        *labels = next.labels();
        *deck = next;
    }
}

fn emit_developer_actions(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Res<DeveloperHotkeyBindings>,
    mut actions: MessageWriter<DeveloperAction>,
) {
    let Some(keys) = keys else { return };
    for binding in &bindings.0 {
        if binding.chord.matches(&keys) {
            actions.write(binding.action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_developer_actions_and_chords_are_unique_and_cover_every_action() {
        let deck = DeveloperHotkeyBindings::default();
        assert_eq!(deck.problem(), None);
        for action in DeveloperAction::ALL {
            assert!(deck.chord_for(action).is_some(), "{action:?} has no default key");
        }
    }

    #[test]
    fn shifted_and_unshifted_f8_are_exact_distinct_chords() {
        let plain = DeveloperKeyChord::key(KeyCode::F8);
        let shifted = DeveloperKeyChord::shift(KeyCode::F8);
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::F8);
        assert!(plain.matches(&keys));
        assert!(!shifted.matches(&keys));
        keys.press(KeyCode::ShiftLeft);
        assert!(!plain.matches(&keys));
        assert!(shifted.matches(&keys));
    }

    #[test]
    fn the_default_deck_keeps_f3_f6_and_f7() {
        let bindings = DeveloperHotkeyBindings::default();
        assert_eq!(bindings.chord_for(DeveloperAction::ToggleInspector), Some(DeveloperKeyChord::key(KeyCode::F3)));
        assert_eq!(bindings.chord_for(DeveloperAction::ToggleFpsOverlay), Some(DeveloperKeyChord::key(KeyCode::F6)));
        assert_eq!(bindings.chord_for(DeveloperAction::TogglePortalGun), Some(DeveloperKeyChord::key(KeyCode::F7)));
    }

    #[test]
    fn a_settings_file_moves_one_key_and_a_clash_keeps_the_defaults() {
        let over = |action: &str, key| ambition_input::DeveloperKeyOverride {
            action: action.to_owned(), key, shift: false, control: true, alt: false,
        };
        let deck = DeveloperHotkeyBindings::with_overrides(&[over("ToggleInspector", KeyCode::KeyI), over("NoSuchAction", KeyCode::KeyJ)])
            .expect("a free chord");
        assert_eq!(deck.label_for(DeveloperAction::ToggleInspector).as_deref(), Some("Ctrl+KeyI"));
        assert_eq!(deck.chord_for(DeveloperAction::ToggleFpsOverlay), Some(DeveloperKeyChord::key(KeyCode::F6)));
        let clash = ambition_input::DeveloperKeyOverride {
            action: "ToggleInspector".to_owned(), key: KeyCode::F6, shift: false, control: false, alt: false,
        };
        assert!(DeveloperHotkeyBindings::with_overrides(&[clash]).is_err());
    }
}
