//! `BossEncounterRegistry` — the read-only boss DATA CATALOG.
//!
//! Holds the authored `BossProfile`s (thresholds / music / reward data) keyed by
//! archetype id. Live state is entity-local (HP on the shared `BodyHealth`,
//! phase in `BossEncounter.encounter`), NOT here; `update_boss_encounters` reads this catalog
//! to SEED each boss's entity-local state, and `BossProfile` selection is the
//! only thing the registry does. See `docs/planning/boss-entity-local-refactor.md`.

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
