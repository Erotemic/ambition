use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::Resource;

#[derive(Resource, Default)]
pub struct BossEncounterRegistry {
    pub encounters: BTreeMap<String, ae::BossEncounterState>,
    /// id -> the boss runtime id we wired to. Used to route damage.
    pub runtime_ids: BTreeMap<String, String>,
    /// True once we've registered the default boss specs.
    pub specs_loaded: bool,
}

impl BossEncounterRegistry {
    pub fn ensure(&mut self, spec: ae::BossEncounterSpec) {
        let id = spec.id.clone();
        self.encounters
            .entry(id)
            .or_insert_with(|| ae::BossEncounterState::new(spec));
    }

    pub fn get(&self, id: &str) -> Option<&ae::BossEncounterState> {
        self.encounters.get(id)
    }

    pub fn link_runtime(&mut self, encounter_id: &str, runtime_id: &str) {
        self.runtime_ids
            .insert(encounter_id.to_string(), runtime_id.to_string());
    }

    pub fn active_phase(&self) -> Option<(&str, ae::BossEncounterPhase)> {
        for (id, state) in &self.encounters {
            if !matches!(state.phase, ae::BossEncounterPhase::Dormant) {
                return Some((id.as_str(), state.phase));
            }
        }
        None
    }
}
