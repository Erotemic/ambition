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
