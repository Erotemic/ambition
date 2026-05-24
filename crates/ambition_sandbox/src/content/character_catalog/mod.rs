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

#[allow(unused_imports, reason = "public surface; downstream phases consume these as they land")]
pub use entry::{
    ActionSetPreset, BrainPreset, CharacterBodyKind, CharacterCatalogData, CharacterCatalogEntry,
    CharacterTier, CompositionLayer, MeleePreset, MoveStylePreset, RangedPreset, SpecialPreset,
};
#[allow(unused_imports, reason = "CHARACTER_CATALOG_ASSET used by tooling that loads off disk")]
pub use loader::{
    display_name_for_character_id, load_embedded, CHARACTER_CATALOG_ASSET, EMBEDDED_CATALOG,
};
pub use resolver::{action_set_from_preset, brain_from_preset};

/// Bevy resource holding the parsed catalog. Inserted at Startup by
/// [`CharacterCatalogPlugin`].
#[derive(Resource, Clone, Debug)]
pub struct CharacterCatalog(pub CharacterCatalogData);

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
#[derive(Default)]
pub struct CharacterCatalogPlugin;

impl Plugin for CharacterCatalogPlugin {
    fn build(&self, app: &mut App) {
        let catalog = CharacterCatalog(load_embedded());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::state_machine::StateMachineCfg;
    use crate::brain::Brain;

    #[test]
    fn catalog_loads_without_panic() {
        // The embedded RON should parse and produce a non-empty
        // catalog. Anything else is a build-time error — pin the
        // baseline.
        let data = load_embedded();
        assert!(
            !data.characters.is_empty(),
            "embedded character_catalog.ron should have characters"
        );
        assert!(
            !data.brain_presets.is_empty(),
            "embedded character_catalog.ron should declare brain presets"
        );
        assert!(
            !data.action_set_presets.is_empty(),
            "embedded character_catalog.ron should declare action-set presets"
        );
    }

    #[test]
    fn embedded_catalog_passes_validator() {
        // Every reference in the embedded RON must resolve. Pins
        // the catalog as internally consistent so that the Startup
        // panic never fires under normal builds.
        let data = load_embedded();
        let errors = validator::validate(&data);
        assert!(
            errors.is_empty(),
            "embedded catalog has reference errors: {errors:?}"
        );
    }

    #[test]
    fn every_npc_sprite_registry_entry_has_catalog_entry() {
        // Transitional coverage gate: during Phase 1 the catalog
        // ships alongside `NPC_SPRITE_REGISTRY`. The catalog must
        // cover every label the registry currently registers — this
        // is the bridge that lets Phase 2 swap LDtk NpcSpawn.name
        // for character_id and have it resolve.
        use crate::presentation::character_sprites::all_character_sprite_filenames;
        let data = load_embedded();
        let mut missing: Vec<String> = Vec::new();
        for (label, _filename) in all_character_sprite_filenames() {
            if data.characters.get(label).is_none() {
                missing.push(label.to_string());
            }
        }
        // `BASE_CHARACTER_FILENAMES` contributes labels like "player"
        // / "robot" / "goblin" / "sandbag" that the catalog should
        // also cover. Phase 3 closes any remaining gaps; Phase 1
        // pins coverage for the current registry roster.
        assert!(
            missing.is_empty(),
            "character_catalog.ron is missing entries for sprite labels: {missing:?}"
        );
    }

    #[test]
    fn brain_preset_resolves_to_valid_variant_for_each_entry() {
        // Pin that every character entry's default_brain produces a
        // runtime `Brain` value. Catches preset enum typos at test
        // time rather than first-spawn time.
        let data = load_embedded();
        for (id, entry) in &data.characters {
            let preset = data
                .brain_presets
                .get(&entry.default_brain)
                .unwrap_or_else(|| panic!("character '{id}' missing brain preset"));
            let brain = brain_from_preset(preset, 0.0);
            // Discriminant sanity — every preset variant must round-trip
            // through the resolver to a StateMachine brain. Bosses that
            // ship a BossPattern preset still produce a Brain::StateMachine
            // value (BossPattern is one of its variants).
            assert!(
                matches!(brain, Brain::StateMachine(_)),
                "preset {} resolved to non-StateMachine brain",
                entry.default_brain,
            );
        }
    }

    #[test]
    fn action_set_preset_resolves_for_each_entry() {
        // Pair test for action_set: every entry's default_action_set
        // must produce a runtime ActionSet without panicking.
        let data = load_embedded();
        for (id, entry) in &data.characters {
            let preset = data
                .action_set_presets
                .get(&entry.default_action_set)
                .unwrap_or_else(|| panic!("character '{id}' missing action_set preset"));
            let _ = action_set_from_preset(preset);
        }
    }

    #[test]
    fn validator_reports_missing_brain_preset() {
        // Sanity: validator should detect a default_brain that
        // doesn't exist. Pre-poison the data by mutating a copy.
        let mut data = load_embedded();
        // Pick the first character and break its default_brain.
        let first_id = data.characters.keys().next().cloned().unwrap();
        data.characters
            .get_mut(&first_id)
            .unwrap()
            .default_brain = "DOES_NOT_EXIST".to_string();
        let errors = validator::validate(&data);
        assert!(
            errors.iter().any(|e| e.contains("DOES_NOT_EXIST")),
            "validator should flag missing brain preset; got: {errors:?}"
        );
    }

    #[test]
    fn brain_preset_patrol_offsets_spawn_world_x() {
        // The patrol cfg's `spawn_x` is `spawn_world_x + spawn_local_x`.
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
                assert_eq!(cfg.spawn_x, 105.0);
                assert_eq!(cfg.radius, 64.0);
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    #[test]
    fn display_name_resolves_for_every_catalog_entry() {
        // Phase 2 pin: the LDtk parser reads `character_id` from
        // NpcSpawn instances and looks up the display name via
        // `display_name_for_character_id`. Every catalog entry must
        // therefore round-trip — otherwise the Authored.name field
        // ends up populated with the id (e.g. "npc_alice") instead
        // of the human label ("Alice").
        for (id, entry) in &EMBEDDED_CATALOG.characters {
            let label = display_name_for_character_id(id);
            assert_eq!(
                label,
                Some(entry.display_name.as_str()),
                "display_name_for_character_id('{id}') should return '{}'",
                entry.display_name,
            );
        }
    }

    #[test]
    fn display_name_returns_none_for_unknown_id() {
        // Negative: callers fall back to the id itself when a lookup
        // misses. Pins the contract so a future panic-on-miss change
        // doesn't sneak through.
        assert!(display_name_for_character_id("npc_definitely_not_in_catalog").is_none());
    }

    #[test]
    fn plugin_inserts_resource_and_validates() {
        // Phase-1 contract: adding CharacterCatalogPlugin makes the
        // resource available and the Startup validator runs without
        // panicking against the shipped catalog.
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CharacterCatalogPlugin);
        app.update(); // runs Startup
        let catalog = app
            .world()
            .get_resource::<CharacterCatalog>()
            .expect("CharacterCatalog resource should be inserted");
        assert!(!catalog.is_empty());
    }
}
