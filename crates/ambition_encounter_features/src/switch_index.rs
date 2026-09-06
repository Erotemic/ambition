//! The switch INDEX rebuild, which stayed behind when the switch types left.
//!
//! ⭐ THE TYPES MOVED TO `ambition_encounter` — they are built from that crate's
//! `SwitchActivation` and belong beside it. This system did not, because it
//! reads a switch's `FeatureId`, which is `ambition_combat` vocabulary that
//! `ambition_encounter` deliberately does not link. A move that took the system
//! too would have added a crate edge to carry one field read.

use ambition_encounter::switches::{
    EncounterSwitchIndex, EncounterSwitchLink, SwitchFeature, SwitchOn,
};
use bevy::prelude::{Query, ResMut};

pub fn rebuild_encounter_switch_index(
    mut index: ResMut<EncounterSwitchIndex>,
    switches: Query<(
        &ambition_combat::components::FeatureId,
        &SwitchFeature,
        &SwitchOn,
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
