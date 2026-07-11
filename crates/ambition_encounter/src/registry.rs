//! `EncounterRegistry` resource: the `id -> Entity` INDEX into the live
//! encounter entities (E1 — the live state lives on the entity's
//! [`EncounterState`](crate::EncounterState) component, not here). Keyed by id
//! (matching LDtk `EncounterTrigger.id`) so consumers resolve an id to its
//! entity in one hop. Also `SwitchActivation` — the typed
//! `switch:<id>:<action>:<target>` payload parsed once at LDtk→ECS spawn and
//! consumed by the switch-arming gate (`switches.rs`) and the encounter tick.

use std::collections::BTreeMap;

use bevy::prelude::*;

/// Index from encounter id → the live encounter entity that owns its
/// [`EncounterState`](crate::EncounterState). Reduced from the old
/// state-holding map to a pure index at E1: the entity is the sole live-state
/// authority, so nothing is duplicated here.
#[derive(Resource, Default)]
pub struct EncounterRegistry {
    /// Encounter id → live encounter entity.
    pub ids: BTreeMap<String, Entity>,
    /// Tracks whether the current LDtk file has been scanned for
    /// encounter triggers yet. Reset by hot reload so an edited LDtk
    /// re-populates the specs.
    pub specs_loaded: bool,
}

impl EncounterRegistry {
    /// The live entity for an encounter id, if one is spawned.
    pub fn entity(&self, id: &str) -> Option<Entity> {
        self.ids.get(id).copied()
    }

    /// Record (or replace) the live entity for an encounter id.
    pub fn insert(&mut self, id: impl Into<String>, entity: Entity) {
        self.ids.insert(id.into(), entity);
    }

    /// Forget an encounter id (its entity despawned / room changed).
    pub fn remove(&mut self, id: &str) -> Option<Entity> {
        self.ids.remove(id)
    }
}

/// One activation request from a switch interaction.
///
/// Built once when an LDtk `Switch` entity is converted into an
/// `ambition_interaction::Interactable` payload and spawned through
/// the host crate's switch feature component. The encounter pipeline, switch
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
