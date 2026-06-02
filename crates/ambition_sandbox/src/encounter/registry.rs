use std::collections::BTreeMap;

use bevy::prelude::*;

use super::{EncounterPhase, EncounterState};

/// Multi-encounter registry. Keyed by encounter id (matching the
/// `EncounterTrigger.id` field in LDtk). Replaces the older
/// single-encounter `Res<EncounterState>` so the sandbox can carry
/// more than one encounter at once.
#[derive(Resource, Default)]
pub struct EncounterRegistry {
    pub encounters: BTreeMap<String, EncounterState>,
    /// Tracks whether the current LDtk file has been scanned for
    /// encounter triggers yet. Reset by hot reload so an edited LDtk
    /// re-populates the specs.
    pub specs_loaded: bool,
}

impl EncounterRegistry {
    pub fn get(&self, id: &str) -> Option<&EncounterState> {
        self.encounters.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut EncounterState> {
        self.encounters.get_mut(id)
    }

    pub fn ensure(&mut self, id: &str) -> &mut EncounterState {
        self.encounters.entry(id.to_string()).or_default()
    }

    /// True if any encounter is currently locking exits.
    pub fn any_lock_active(&self) -> bool {
        self.encounters.values().any(|e| e.lock_active)
    }

    /// Camera zoom multiplier sourced from the active encounter (if
    /// any). 1.0 if no encounter is in flight. The camera starts
    /// zooming during `Starting` so the ramp finishes before wave 1
    /// spawns.
    pub fn active_camera_zoom(&self) -> f32 {
        for state in self.encounters.values() {
            if matches!(
                state.phase,
                EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
            ) {
                if let Some(spec) = &state.spec {
                    if spec.camera_zoom > 1.0 {
                        return spec.camera_zoom;
                    }
                }
            }
        }
        1.0
    }
}

/// One activation request from a switch interaction.
///
/// Built once when an LDtk `Switch` entity is converted into an
/// `crate::interaction::Interactable` payload and spawned through
/// `crate::features::SwitchFeature`. The encounter pipeline, switch
/// activation queue, and switch index all consume the typed fields
/// directly — only the engine-side `InteractionKind::Custom(String)`
/// boundary still carries the wire format.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SwitchActivation {
    pub id: String,
    pub action: String,
    pub target_encounter: String,
}

impl SwitchActivation {
    /// Parse the `Custom("switch:<id>:<action>:<target>")` payload
    /// produced by `entity_to_runtime` for `Switch` LDtk entities.
    ///
    /// Called exactly once per switch — at LDtk-to-ECS spawn — so
    /// downstream systems can read the typed fields without re-parsing
    /// the wire format every frame.
    pub fn parse_custom(payload: &str) -> Option<Self> {
        let mut parts = payload.split(':');
        if parts.next()? != "switch" {
            return None;
        }
        let id = parts.next()?.to_string();
        let action = parts.next()?.to_string();
        let target_encounter = parts.next().unwrap_or("").to_string();
        Some(Self {
            id,
            action,
            target_encounter,
        })
    }

    /// Inverse of [`Self::parse_custom`]. Used by the LDtk converter to
    /// keep the engine-boundary string format in sync with the typed
    /// fields and by tests that round-trip through the payload form.
    pub fn to_custom_payload(&self) -> String {
        format!(
            "switch:{}:{}:{}",
            self.id, self.action, self.target_encounter
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_activation_round_trips_through_the_custom_payload() {
        let s = SwitchActivation {
            id: "gate_a".into(),
            action: "open".into(),
            target_encounter: "goblin_encounter".into(),
        };
        assert_eq!(
            SwitchActivation::parse_custom(&s.to_custom_payload()),
            Some(s)
        );
    }

    #[test]
    fn parse_custom_allows_an_empty_target() {
        let parsed = SwitchActivation::parse_custom("switch:gate_a:open").unwrap();
        assert_eq!(parsed.id, "gate_a");
        assert_eq!(parsed.action, "open");
        assert_eq!(parsed.target_encounter, "");
    }

    #[test]
    fn parse_custom_rejects_non_switch_and_truncated_payloads() {
        assert_eq!(SwitchActivation::parse_custom("door:gate_a:open:x"), None);
        assert_eq!(SwitchActivation::parse_custom("switch:gate_a"), None);
        assert_eq!(SwitchActivation::parse_custom(""), None);
    }
}
