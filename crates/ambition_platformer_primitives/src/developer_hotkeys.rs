//! Canonical keyboard deck for developer-only host actions.
//!
//! This is the one source file to edit while debugging priorities change.
//! Physical keys are translated once into semantic [`DeveloperAction`] messages;
//! simulation, presentation, shell, and tooling systems consume those actions
//! without independently claiming function keys.

use std::collections::HashSet;

use bevy::prelude::*;

/// Semantic developer actions. Keep physical keyboard policy out of consumers.
#[derive(Clone, Copy, Debug, Eq, Hash, Message, PartialEq)]
pub enum DeveloperAction {
    ToggleDebugOverlay,
    ToggleSlowMotion,
    ToggleInspector,
    ToggleWorldInspector,
    ToggleOverviewCamera,
    ToggleFpsOverlay,
    TogglePortalGun,
    DumpGameplayTrace,
    DumpPortalViewCones,
    RequestRollbackProof,
    QuitToHome,
    ApplyLdtkReload,
    ToggleLdtkAutoApply,
}

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
        Self {
            key,
            shift: false,
            control: false,
            alt: false,
        }
    }

    pub const fn shift(key: KeyCode) -> Self {
        Self {
            key,
            shift: true,
            control: false,
            alt: false,
        }
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

/// Runtime copy of the canonical deck. Tests and future developer settings can
/// replace this resource before [`DeveloperHotkeyPlugin`] is installed.
#[derive(Resource, Clone, Debug)]
pub struct DeveloperHotkeyBindings(pub Vec<DeveloperHotkeyBinding>);

impl Default for DeveloperHotkeyBindings {
    fn default() -> Self {
        use DeveloperAction as A;
        use DeveloperKeyChord as C;

        Self(vec![
            DeveloperHotkeyBinding {
                action: A::ToggleDebugOverlay,
                chord: C::key(KeyCode::F1),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleSlowMotion,
                chord: C::key(KeyCode::F2),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleInspector,
                chord: C::key(KeyCode::F3),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleWorldInspector,
                chord: C::key(KeyCode::F4),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleOverviewCamera,
                chord: C::key(KeyCode::F5),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleFpsOverlay,
                chord: C::key(KeyCode::F6),
            },
            DeveloperHotkeyBinding {
                action: A::TogglePortalGun,
                chord: C::key(KeyCode::F7),
            },
            DeveloperHotkeyBinding {
                action: A::DumpGameplayTrace,
                chord: C::key(KeyCode::F8),
            },
            DeveloperHotkeyBinding {
                action: A::DumpPortalViewCones,
                chord: C::shift(KeyCode::F8),
            },
            DeveloperHotkeyBinding {
                action: A::RequestRollbackProof,
                chord: C::key(KeyCode::F9),
            },
            DeveloperHotkeyBinding {
                action: A::QuitToHome,
                chord: C::key(KeyCode::F10),
            },
            DeveloperHotkeyBinding {
                action: A::ApplyLdtkReload,
                chord: C::key(KeyCode::F11),
            },
            DeveloperHotkeyBinding {
                action: A::ToggleLdtkAutoApply,
                chord: C::key(KeyCode::F12),
            },
        ])
    }
}

impl DeveloperHotkeyBindings {
    pub fn chord_for(&self, action: DeveloperAction) -> Option<DeveloperKeyChord> {
        self.0
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.chord)
    }

    pub fn label_for(&self, action: DeveloperAction) -> Option<String> {
        self.chord_for(action).map(DeveloperKeyChord::label)
    }

    fn assert_valid(&self) {
        let mut actions = HashSet::new();
        let mut chords = HashSet::new();
        for binding in &self.0 {
            assert!(
                actions.insert(binding.action),
                "developer action {:?} has more than one physical binding",
                binding.action
            );
            assert!(
                chords.insert(binding.chord),
                "developer chord {} is assigned more than once",
                binding.chord.label()
            );
        }
    }
}

/// Installs the single physical-key reader. Consumers add no keyboard systems;
/// they listen for [`DeveloperAction`] messages instead.
pub struct DeveloperHotkeyPlugin;

impl Plugin for DeveloperHotkeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeveloperHotkeyBindings>();
        app.world()
            .resource::<DeveloperHotkeyBindings>()
            .assert_valid();
        app.add_message::<DeveloperAction>()
            .add_systems(PreUpdate, emit_developer_actions);
    }
}

fn emit_developer_actions(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Res<DeveloperHotkeyBindings>,
    mut actions: MessageWriter<DeveloperAction>,
) {
    let Some(keys) = keys else {
        return;
    };
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
    fn default_developer_actions_and_chords_are_unique() {
        DeveloperHotkeyBindings::default().assert_valid();
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
    fn current_developer_deck_keeps_f6_and_f7_for_debug_features() {
        let bindings = DeveloperHotkeyBindings::default();
        assert_eq!(
            bindings.chord_for(DeveloperAction::ToggleFpsOverlay),
            Some(DeveloperKeyChord::key(KeyCode::F6))
        );
        assert_eq!(
            bindings.chord_for(DeveloperAction::TogglePortalGun),
            Some(DeveloperKeyChord::key(KeyCode::F7))
        );
    }
}
