//! Switch-arming gate for encounters. `EncounterSwitchIndex` is rebuilt each
//! frame from `SwitchFeature + SwitchOn` components and answers
//! `encounter_armed(id)` (semantics: off/red switch arms, green/on disables,
//! unlinked = always armed, any one off switch arms a multi-switch fight).
//! `SwitchActivationQueue` is the per-frame FIFO of activations the encounter
//! tick drains to apply resets.

use bevy::prelude::{Query, ResMut, Resource};

use super::SwitchActivation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterSwitchLink {
    pub switch_id: String,
    pub target_encounter: String,
    pub on: bool,
}

/// Cached ECS switch state used by the encounter state machine.
///
/// Rebuilt from `SwitchFeature + SwitchOn` components each frame.
#[derive(Resource, Default, Clone, Debug)]
pub struct EncounterSwitchIndex {
    pub links: Vec<EncounterSwitchLink>,
}

impl EncounterSwitchIndex {
    /// Whether `encounter_id` is armed. Off/red switches arm their target;
    /// no linked switch means the encounter is always armed.
    pub fn encounter_armed(&self, encounter_id: &str) -> bool {
        let mut found = false;
        for link in &self.links {
            if link.target_encounter != encounter_id {
                continue;
            }
            found = true;
            if !link.on {
                return true;
            }
        }
        !found
    }

    /// First switch id linked to an encounter, used by the auto-green clear
    /// path. Multi-switch encounters can replace this with a richer policy.
    pub fn switch_id_for_encounter(&self, encounter_id: &str) -> Option<String> {
        self.links
            .iter()
            .find(|link| link.target_encounter == encounter_id)
            .map(|link| link.switch_id.clone())
    }
}

pub fn rebuild_encounter_switch_index(
    mut index: ResMut<EncounterSwitchIndex>,
    switches: Query<(
        &crate::features::FeatureId,
        &crate::features::SwitchFeature,
        &crate::features::SwitchOn,
    )>,
) {
    index.links.clear();
    for (feature_id, switch, switch_on) in &switches {
        let activation = &switch.activation;
        let switch_id = if activation.id.is_empty() {
            feature_id.as_str().to_string()
        } else {
            activation.id.clone()
        };
        index.links.push(EncounterSwitchLink {
            switch_id,
            target_encounter: activation.target_encounter.clone(),
            on: switch_on.0,
        });
    }
}

/// FIFO queue of switch activations produced by the feature systems each frame.
/// The encounter system drains it and applies the matching reset.
#[derive(Resource, Default)]
pub struct SwitchActivationQueue(pub Vec<SwitchActivation>);

#[cfg(test)]
mod switch_index_tests {
    //! Encounter arming from switch state. The authored semantics are
    //! "red (off) = armed, green (on) = disabled", an unlinked encounter
    //! is always armed, and any single off switch arms a multi-switch
    //! encounter. This is the gate the encounter state machine reads.
    use super::*;

    fn link(switch: &str, target: &str, on: bool) -> EncounterSwitchLink {
        EncounterSwitchLink {
            switch_id: switch.into(),
            target_encounter: target.into(),
            on,
        }
    }
    fn index(links: Vec<EncounterSwitchLink>) -> EncounterSwitchIndex {
        EncounterSwitchIndex { links }
    }

    #[test]
    fn unlinked_encounter_is_always_armed() {
        assert!(
            EncounterSwitchIndex::default().encounter_armed("anything"),
            "no linked switch -> always armed"
        );
    }

    #[test]
    fn off_switch_arms_on_switch_disarms() {
        assert!(index(vec![link("s", "enc", false)]).encounter_armed("enc"));
        assert!(!index(vec![link("s", "enc", true)]).encounter_armed("enc"));
    }

    #[test]
    fn any_off_switch_arms_a_multi_switch_encounter() {
        assert!(
            index(vec![link("a", "enc", true), link("b", "enc", false)]).encounter_armed("enc"),
            "one red switch is enough to arm"
        );
        assert!(
            !index(vec![link("a", "enc", true), link("b", "enc", true)]).encounter_armed("enc"),
            "all green -> disabled"
        );
    }

    #[test]
    fn links_for_other_encounters_are_ignored() {
        // An ON switch targeting a different encounter leaves "enc" unlinked -> armed.
        assert!(index(vec![link("s", "other", true)]).encounter_armed("enc"));
    }

    #[test]
    fn switch_id_for_encounter_finds_the_first_match() {
        let idx = index(vec![link("a", "enc", true), link("b", "enc", false)]);
        assert_eq!(idx.switch_id_for_encounter("enc").as_deref(), Some("a"));
        assert_eq!(idx.switch_id_for_encounter("missing"), None);
    }
}

use ambition_engine_core as ae;
use bevy::prelude::{Component, Message};

/// A live Switch interactable (parsed once at LDtk spawn; see
/// `spawn_room_feature_entity`). Moved here from the combat components at E2
/// — the payload is encounter vocabulary.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SwitchFeature {
    pub activation: SwitchActivation,
}

impl SwitchFeature {
    pub fn new(activation: SwitchActivation) -> Self {
        Self { activation }
    }
}

/// Live switch state used by rendering and encounter reset logic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchOn(pub bool);

/// A Switch interactable was activated. Carries the parsed
/// [`SwitchActivation`] directly — the `switch:<id>:<action>:<target>` wire
/// string lives only at the engine `InteractionKind::Custom` boundary.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SwitchActivated {
    pub activation: SwitchActivation,
    pub pos: ae::Vec2,
}
