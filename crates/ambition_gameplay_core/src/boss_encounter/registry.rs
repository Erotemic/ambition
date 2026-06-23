//! `BossEncounterRegistry` — the read-only boss DATA CATALOG.
//!
//! Holds the authored `BossProfile`s (thresholds / music / reward data) keyed by
//! archetype id. The live phase/HP state used to live here too (a string-keyed
//! map the boss entity mirrored), but R3 of the boss entity-local refactor moved
//! that ONTO the entity (`BossStatus.health` + `BossStatus.encounter`) and
//! deleted the map. `update_boss_encounters` now reads this catalog to SEED each
//! boss's entity-local state, and `BossProfile` selection is the only thing the
//! registry does. See `docs/planning/boss-entity-local-refactor.md`.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

use super::BossProfile;

#[derive(Resource, Default)]
pub struct BossEncounterRegistry {
    /// Authored boss data, keyed by archetype id. Read-only at runtime.
    pub profiles: BTreeMap<String, BossProfile>,
    /// True once we've installed the default boss profiles.
    pub specs_loaded: bool,
}

impl BossEncounterRegistry {
    pub fn ensure_profile(&mut self, profile: BossProfile) {
        let id = profile.id.clone();
        self.profiles.entry(id).or_insert(profile);
    }

    pub fn profile(&self, id: &str) -> Option<&BossProfile> {
        self.profiles.get(id)
    }
}
