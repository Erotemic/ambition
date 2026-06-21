//! Character catalog — the data-driven registry of every character
//! the sandbox can spawn. The catalog is the single source of truth
//! for `(character_id, sprite path, default brain, default action
//! set)` tuples that NPCs / enemies / bosses pull from at spawn.
//!
//! Architectural posture (to be ADR 0017):
//!
//! > **Rust = behavior, RON = content, LDtk = space.**
//!
//! Brain *variants* (the `MeleeBrute` evaluator) and ActionSet
//! *variants* (the `Swipe` hitbox shape) stay typed in Rust so the
//! compiler enforces exhaustiveness. Brain *configs* (aggro radius,
//! attack range) and ActionSet *specs* (windup timing) move to RON
//! so adding a new character — Skeletal Crow, AI-era Spaghetti
//! Event, etc. — is a RON edit, not a Rust patch.
//!
//! The plugin shape is intentionally minimal: load the RON, hand it
//! to a Bevy `Resource`, and run a Startup validator that panics if
//! a `default_brain` references a preset that doesn't exist. No hot
//! reload yet — the resource is built once at startup. Future work
//! can lift this into a separate `ambition_character_catalog` crate
//! when a second Ambition-powered game needs it.

use bevy::prelude::*;

pub mod entry;
pub mod loader;
pub mod resolver;
pub mod validator;

#[allow(
    unused_imports,
    reason = "public surface; downstream phases consume these as they land"
)]
pub use entry::{
    ActionSetPreset, BrainPreset, CharacterBodyKind, CharacterCatalogData, CharacterCatalogEntry,
    CharacterTier, CompositionLayer, MeleePreset, MoveStylePreset, RangedPreset, SpecialPreset,
    SpriteTuningSpec,
};
#[allow(
    unused_imports,
    reason = "CHARACTER_CATALOG_ASSET used by tooling that loads off disk"
)]
pub use loader::parse_catalog;
pub use resolver::{action_set_from_preset, brain_from_preset};

/// Bevy resource holding the parsed catalog. Inserted at Startup by
/// [`CharacterCatalogPlugin`].
#[derive(Resource, Clone, Debug)]
pub struct CharacterCatalog(pub CharacterCatalogData);

#[allow(
    dead_code,
    reason = "Public catalog API for future spawn-site consumers (EnemySpawn / BossSpawn migrations, custom spawn paths). Tested but not yet wired into a runtime call site."
)]
impl CharacterCatalog {
    /// Look up a character by id. Returns `None` if the id is not
    /// in the catalog — callers fall back to a placeholder spawn.
    pub fn get(&self, id: &str) -> Option<&CharacterCatalogEntry> {
        self.0.characters.get(id)
    }

    /// Iterate every (id, entry) pair. Stable order — `BTreeMap`.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CharacterCatalogEntry)> {
        self.0.characters.iter()
    }

    /// Count of registered characters.
    pub fn len(&self) -> usize {
        self.0.characters.len()
    }

    /// True iff the catalog has no characters. Pretty much never
    /// the case in a real build; included so `clippy::len_without_is_empty`
    /// stays quiet without `allow(missing_is_empty)`.
    pub fn is_empty(&self) -> bool {
        self.0.characters.is_empty()
    }

    /// Resolve a character_id into a runtime [`crate::brain::Brain`]
    /// using its catalog default. `spawn_world_x` becomes the patrol
    /// center for `Patrol`-brain characters.
    pub fn build_default_brain(
        &self,
        character_id: &str,
        spawn_world_x: f32,
    ) -> Option<crate::brain::Brain> {
        let entry = self.get(character_id)?;
        let preset = self.0.brain_presets.get(&entry.default_brain)?;
        Some(brain_from_preset(preset, spawn_world_x))
    }

    /// Resolve a character_id into a runtime [`crate::brain::action_set::ActionSet`]
    /// using its catalog default.
    pub fn build_default_action_set(
        &self,
        character_id: &str,
    ) -> Option<crate::brain::action_set::ActionSet> {
        let entry = self.get(character_id)?;
        let preset = self.0.action_set_presets.get(&entry.default_action_set)?;
        Some(action_set_from_preset(preset))
    }
}

/// Plugin: load the catalog at app build, install it as a resource,
/// and run the validator on Startup. Pre-release stance is fail-loud:
/// the validator panics on internal inconsistency rather than
/// degrading silently.
pub struct CharacterCatalogPlugin {
    /// The catalog RON text (the game embeds its roster and passes it
    /// in — this crate owns schema + parsing, never the data).
    pub catalog_ron: &'static str,
}

impl Plugin for CharacterCatalogPlugin {
    fn build(&self, app: &mut App) {
        let catalog = CharacterCatalog(parse_catalog(self.catalog_ron));
        app.insert_resource(catalog);
        app.add_systems(Startup, validate_catalog_on_startup);
    }
}

/// Startup system: validate the catalog and panic with the joined
/// errors if any. Runs once.
pub fn validate_catalog_on_startup(catalog: Res<CharacterCatalog>) {
    let errors = validator::validate(&catalog.0);
    if !errors.is_empty() {
        panic!(
            "character_catalog.ron has {} reference error(s):\n  - {}",
            errors.len(),
            errors.join("\n  - "),
        );
    }
}

// The roster-data tests (catalog parses, validator passes over the real
// roster, every entry's presets resolve, renderer coverage, display
// names, plugin install) live in `ambition_gameplay_core::character_roster` —
// they pin the GAME's data, which lives there. This crate keeps only
// the synthetic resolver-math tests below.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{Brain, StateMachineCfg};

    #[test]
    fn brain_preset_patrol_offsets_spawn_world_x() {
        // The patrol lane center is `spawn_world_x + spawn_local_x`.
        // Pin the offset arithmetic so a refactor that drops the add
        // breaks here rather than at first-spawn.
        let preset = BrainPreset::Patrol {
            spawn_local_x: 5.0,
            radius: 64.0,
            speed: 32.0,
            aggressiveness: 0.0,
            aggro_radius: 80.0,
            attack_range: 0.0,
        };
        let brain = brain_from_preset(&preset, 100.0);
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.lane.center_x, 105.0);
                assert_eq!(cfg.lane.radius_px, 64.0);
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }
}
