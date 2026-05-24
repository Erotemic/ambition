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
    fn every_catalog_sprite_spec_has_idle_row_if_loaded() {
        // The actor renderer's `flat_index` falls back to `Idle`
        // for any animation that doesn't have its own row. A spec
        // *without* an Idle row crashes on the first frame. This
        // test walks every catalog id, asks the sprite loader for
        // a spec, and verifies the spec either declines to load
        // (None) or includes an Idle row — never an Idle-less spec
        // that the runtime would unwrap into a panic.
        //
        // Caught a real crash 2026-05-24 when the manifest-driven
        // fallback loaded a spec for a character whose generated
        // sheet only had run/walk rows (no idle).
        use crate::presentation::character_sprites::sheet_for_character_id;
        let data = load_embedded();
        for cid in data.characters.keys() {
            let Some(spec) = sheet_for_character_id(cid) else {
                continue;
            };
            let has_idle = spec
                .rows
                .iter()
                .any(|(anim, _)| matches!(
                    anim,
                    crate::presentation::character_sprites::CharacterAnim::Idle,
                ));
            assert!(
                has_idle,
                "catalog id '{cid}' loaded a spec without an Idle row; \
                 sheet_for_character_id must return None or a spec with Idle",
            );
        }
    }

    #[test]
    fn sprite_loader_resolves_a_sheet_for_most_catalog_entries() {
        // Phase 6 + manifest-driven fallback (2026-05-24): every
        // catalog id either resolves to a hardcoded `*_SHEET` const
        // (for the entries that need bespoke tuning) or falls back
        // to the manifest-driven `try_load_spec_for_character_id`
        // path (everything else with a sheet on disk).
        //
        // The Hall of Characters is the visible consumer of this
        // coverage — every pedestal whose `sheet_for_character_id`
        // returns `None` shows a colored-rectangle fallback. Pin
        // a generous lower bound (>=70 of ~99) so the Hall stays
        // mostly populated; the few stragglers (robot_heavy and
        // similar variant-only targets) ship later when their
        // publisher lands.
        use crate::presentation::character_sprites::sheet_for_character_id;
        let data = load_embedded();
        let covered = data
            .characters
            .keys()
            .filter(|cid| sheet_for_character_id(cid).is_some())
            .count();
        assert!(
            covered >= 70,
            "expected >=70 catalog ids to resolve to a sheet spec (hardcoded const \
             or manifest); got {covered}",
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

    /// Snapshot of every renderer-registered character_id at the
    /// time of the Phase 3 codegen (2026-05-24). The catalog must
    /// cover every entry below; the codegen script is the source
    /// of truth for this list. Regenerate with
    ///   `python -m ambition_ldtk_tools.codegen_character_catalog`
    /// when the renderer adds new targets.
    const RENDERER_COVERAGE_TARGETS: &[&str] = &[
        // === [characters] tackon targets ===
        "npc_agent_swarm",
        "npc_ai_slop",
        "npc_bear_mauler",
        "npc_boss",
        "npc_burning_flying_shark",
        "npc_colonial_statesman",
        "npc_creator",
        "npc_dark_lord",
        "npc_fascist_enforcer",
        "npc_flying_spaghetti_monster_boss",
        "npc_galwah",
        "npc_ghoul_skulker",
        "npc_girdle",
        "npc_gnu_ton_boss",
        "goblin",
        "npc_goblin_forest_spear",
        "npc_hand_saint",
        "npc_helpful_liar",
        "npc_mantis_lancer",
        "npc_mockingbird_boss",
        "npc_ninja_heavy",
        "npc_ninja_shadow_duelist",
        "npc_ninja_shadow_oni_leader",
        "npc_pirate_admiral",
        "npc_pirate_cutlass_viper",
        "npc_pirate_heavy",
        "npc_pirate_lookout",
        "npc_pirate_navigator",
        "npc_pirate_quartermaster",
        "npc_pirate_raider",
        "npc_player_extended",
        "player",
        "npc_president_portrait",
        "npc_puppy_slug",
        "npc_puppy_slug_variant2",
        "npc_raptor_stalker",
        "robot",
        "npc_robot_guardian",
        "npc_robot_heavy",
        "npc_robot_runner",
        "sandbag",
        "npc_smart_house",
        "npc_spaghetti_event",
        "npc_synthetic_friend",
        "npc_trex_enemy",
        "npc_viking_heavy_shieldmaiden",
        "npc_viking_heavy_warrior",
        "npc_viking_shieldmaiden",
        "npc_viking_warrior",
        "npc_weird_hermit",
        // === [review_npcs] adapter rigs + tackons ===
        "npc_general", // absurd_general → npc_general
        "npc_alice",
        "npc_architect",
        "npc_bob",
        "npc_craig",
        "npc_erdish",
        "npc_eve",
        "npc_general_hero",
        "npc_goblin_brute_hammer",
        "npc_goblin_cave_dagger",
        "npc_goblin_desert_bow",
        "npc_goblin_frost_sword",
        "npc_goblin_shaman_staff",
        "npc_judy",
        "npc_kernel_guide",
        "npc_mallory",
        "npc_merchant_prototype",
        "npc_oiler",
        "npc_olivia",
        "npc_peggy",
        "npc_player_combat_review",
        "npc_player_social_review",
        "npc_player_traversal_review",
        "npc_robot_archivist",
        "npc_robot_caster",
        "npc_robot_diver",
        "npc_robot_engineer",
        "npc_robot_medic",
        "npc_robot_miner",
        "npc_sandbag_armored_review",
        "npc_sandbag_full_review",
        "npc_sybil",
        "npc_trent",
        "npc_trudy",
        "npc_vault_keeper",
        "npc_victor",
        "npc_walter",
    ];

    #[test]
    fn every_renderer_target_has_catalog_entry_or_explicit_exclusion() {
        // Phase 3 pin: the catalog covers every renderer-registered
        // character. The renderer is the upstream source of truth
        // for what spritesheets exist; the catalog must keep pace.
        // Snapshot above is regenerated by the codegen script;
        // adding a renderer target without a catalog entry trips
        // this test.
        let data = load_embedded();
        let mut missing: Vec<&str> = Vec::new();
        for target in RENDERER_COVERAGE_TARGETS {
            if !data.characters.contains_key(*target) {
                missing.push(target);
            }
        }
        assert!(
            missing.is_empty(),
            "renderer targets missing catalog entries: {missing:?}",
        );
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
