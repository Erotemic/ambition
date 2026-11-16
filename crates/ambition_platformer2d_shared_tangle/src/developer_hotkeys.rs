//! The developer actions: the vocabulary every consumer listens for.
//!
//! The physical keys are the host's (`ambition_platformer2d_host::developer_keys`):
//! it translates them once into [`DeveloperAction`] messages, and a player's
//! settings rebind them, so a key change is no rebuild of this foundation.
//! Simulation, presentation, shell and tooling systems consume the actions
//! without claiming a key.

use std::collections::BTreeMap;

use bevy::prelude::*;

/// Semantic developer actions. Keep physical keyboard policy out of consumers.
#[derive(Clone, Copy, Debug, Eq, Hash, Message, Ord, PartialEq, PartialOrd)]
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
    /// Show what each connected pad's left stick actually reports, with a
    /// peak-hold. Toggling also RESETS the peaks — see the app-side
    /// `gamepad_probe` overlay for why the peak is the measurement.
    ToggleGamepadProbe,
    /// Show what takes part in a blow: the debug overlay in its combat view,
    /// with every body's hurtboxes, the live hitboxes, each boss's guard, the
    /// shots in flight and where they bounced, and a label of each one's
    /// faction, guard, invulnerability and damage. Pressed again, it hides
    /// the overlay.
    ToggleCombatDebug,
}

impl DeveloperAction {
    /// Every action, in declaration order. A settings file names an action by
    /// its [`Self::name`], looked up here.
    pub const ALL: [Self; 15] = [
        Self::ToggleDebugOverlay,
        Self::ToggleSlowMotion,
        Self::ToggleInspector,
        Self::ToggleWorldInspector,
        Self::ToggleOverviewCamera,
        Self::ToggleFpsOverlay,
        Self::TogglePortalGun,
        Self::DumpGameplayTrace,
        Self::DumpPortalViewCones,
        Self::RequestRollbackProof,
        Self::QuitToHome,
        Self::ApplyLdtkReload,
        Self::ToggleLdtkAutoApply,
        Self::ToggleGamepadProbe,
        Self::ToggleCombatDebug,
    ];

    /// The name a settings file uses (`ToggleDebugOverlay`).
    pub fn name(self) -> String { format!("{self:?}") }

    /// The action a settings file names, if any.
    pub fn named(name: &str) -> Option<Self> { Self::ALL.into_iter().find(|action| action.name() == name) }
}

/// What a player presses for each developer action, as the host bound it
/// ("F3", "Shift+F8"). A prompt reads this; it names no key itself.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct DeveloperActionLabels(pub BTreeMap<DeveloperAction, String>);

impl DeveloperActionLabels {
    pub fn label_for(&self, action: DeveloperAction) -> Option<String> { self.0.get(&action).cloned() }
}

/// Ordering seam for consumers that must react before another PreUpdate phase.
///
/// Developer UI uses this to apply F3/F4 before Egui begins its pass, while
/// every other consumer can continue reading the semantic messages later.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeveloperHotkeyEmitSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_is_listed_once_and_found_by_its_name() {
        let mut seen = std::collections::BTreeSet::new();
        for action in DeveloperAction::ALL {
            // The match is exhaustive, so a new action fails to compile here
            // until it is placed, and then fails below until it is listed.
            let _ = match action {
                DeveloperAction::ToggleDebugOverlay | DeveloperAction::ToggleSlowMotion
                | DeveloperAction::ToggleInspector | DeveloperAction::ToggleWorldInspector
                | DeveloperAction::ToggleOverviewCamera | DeveloperAction::ToggleFpsOverlay
                | DeveloperAction::TogglePortalGun | DeveloperAction::DumpGameplayTrace
                | DeveloperAction::DumpPortalViewCones | DeveloperAction::RequestRollbackProof
                | DeveloperAction::QuitToHome | DeveloperAction::ApplyLdtkReload
                | DeveloperAction::ToggleLdtkAutoApply | DeveloperAction::ToggleGamepadProbe
                | DeveloperAction::ToggleCombatDebug => (),
            };
            assert!(seen.insert(action), "{action:?} is listed twice");
            assert_eq!(DeveloperAction::named(&action.name()), Some(action));
        }
        assert_eq!(seen.len(), 15);
        assert_eq!(DeveloperAction::named("NoSuchAction"), None);
    }
}
