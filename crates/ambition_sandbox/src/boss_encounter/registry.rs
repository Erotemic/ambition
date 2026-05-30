use std::collections::BTreeMap;

use bevy::prelude::Resource;

use super::BossProfile;

#[derive(Resource, Default)]
pub struct BossEncounterRegistry {
    pub encounters: BTreeMap<String, crate::boss_encounter::BossEncounterState>,
    pub profiles: BTreeMap<String, BossProfile>,
    /// id -> the boss runtime id we wired to. Used to route damage.
    pub runtime_ids: BTreeMap<String, String>,
    /// True once we've registered the default boss profiles/specs.
    pub specs_loaded: bool,
}

impl BossEncounterRegistry {
    pub fn ensure(&mut self, spec: crate::boss_encounter::BossEncounterSpec) {
        let id = spec.id.clone();
        self.encounters
            .entry(id)
            .or_insert_with(|| crate::boss_encounter::BossEncounterState::new(spec));
    }

    pub fn ensure_profile(&mut self, profile: BossProfile) {
        let id = profile.id.clone();
        self.ensure(profile.encounter.clone());
        self.profiles.entry(id).or_insert(profile);
    }

    pub fn get(&self, id: &str) -> Option<&crate::boss_encounter::BossEncounterState> {
        self.encounters.get(id)
    }

    pub fn profile(&self, id: &str) -> Option<&BossProfile> {
        self.profiles.get(id)
    }

    pub fn link_runtime(&mut self, encounter_id: &str, runtime_id: &str) {
        self.runtime_ids
            .insert(encounter_id.to_string(), runtime_id.to_string());
    }

    pub fn active_phase(&self) -> Option<(&str, crate::boss_encounter::BossEncounterPhase)> {
        for (id, state) in &self.encounters {
            if !matches!(
                state.phase,
                crate::boss_encounter::BossEncounterPhase::Dormant
            ) {
                return Some((id.as_str(), state.phase));
            }
        }
        None
    }
}
