use bevy::prelude::Resource;

use super::SwitchActivation;

/// Whether `encounter_id` is currently armed (will fire when the
/// player crosses the trigger). Looks up linked switches in the
/// runtime: a switch with `target_encounter == encounter_id` arms
/// the encounter when its `on` flag is false (red). Multiple linked
/// switches OR together (any one off → armed). No linked switches
/// means the encounter is always armed.
pub fn encounter_armed_by_switch(
    encounter_id: &str,
    switches: &[crate::features::SwitchRuntime],
) -> bool {
    let mut found = false;
    for sw in switches {
        let Some(act) = SwitchActivation::parse_custom(&sw.custom_payload) else {
            continue;
        };
        if act.target_encounter != encounter_id {
            continue;
        }
        found = true;
        if !sw.on {
            // Off (red) = armed.
            return true;
        }
    }
    !found
}

/// Find the switch id (LDtk `id` field, matching the persisted save's
/// `switches` key) that targets `encounter_id`, if any. Returns the
/// first match — multi-switch encounters can extend this later.
pub fn switch_id_for_encounter(
    encounter_id: &str,
    switches: &[crate::features::SwitchRuntime],
) -> Option<String> {
    for sw in switches {
        let Some(act) = SwitchActivation::parse_custom(&sw.custom_payload) else {
            continue;
        };
        if act.target_encounter == encounter_id {
            return Some(act.id);
        }
    }
    None
}

/// FIFO queue of switch activations produced by the feature runtime
/// each frame. The encounter system drains it and applies the
/// matching reset.
#[derive(Resource, Default)]
pub struct SwitchActivationQueue(pub Vec<SwitchActivation>);
