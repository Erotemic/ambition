//! The GAME's character roster: the embedded `character_catalog.ron`
//! data plus its lookup helpers.
//!
//! `ambition_actor::character_catalog` owns the catalog SCHEMA +
//! parser + preset resolver (machinery, content-free); this module
//! owns Ambition's actual roster DATA — the same machinery/data split
//! as `enemy_archetypes.ron` and the sprite-sheet tuning. The RON
//! lives under this crate's `assets/data/` so it ships with the game
//! and stays readable by the Python tools
//! (`ambition_ldtk_tools.codegen_character_catalog`, hall generator).

use std::sync::LazyLock;

use crate::actor::character_catalog::{
    parse_catalog, CharacterCatalogData, CharacterCatalogPlugin,
};

/// The embedded roster RON (compile-time include; single source of
/// truth shared with the runtime asset root + off-disk tooling).
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");

/// Path constant for tooling that loads the RON file off disk
/// (codegen scripts, hall generator). Relative to the asset root.
pub const CHARACTER_CATALOG_ASSET: &str = "data/character_catalog.ron";

/// Parse the embedded roster. Panics on parse error — a build-time
/// data bug, not a runtime condition.
pub fn load_embedded() -> CharacterCatalogData {
    parse_catalog(CHARACTER_CATALOG_RON)
}

/// One-time parse cache so non-Bevy call sites (the LDtk parser,
/// tests, headless tooling) can query the roster without re-parsing.
/// The Bevy `CharacterCatalog` resource always takes precedence when
/// one is available, but the LDtk parser runs without `Res<>` access.
pub static EMBEDDED_CATALOG: LazyLock<CharacterCatalogData> = LazyLock::new(load_embedded);

/// Look up the display name for a character id. Returns `None` if
/// the id is not in the roster; callers fall back to the id itself.
pub fn display_name_for_character_id(character_id: &str) -> Option<&'static str> {
    EMBEDDED_CATALOG
        .characters
        .get(character_id)
        .map(|entry| entry.display_name.as_str())
}

/// The catalog plugin pre-loaded with this game's roster.
pub fn character_roster_plugin() -> CharacterCatalogPlugin {
    CharacterCatalogPlugin {
        catalog_ron: CHARACTER_CATALOG_RON,
    }
}

#[cfg(test)]
mod tests {
    // The catalog<->sheet integration tests (boss subdir manifests, Idle-row
    // policy, loader coverage) live in `presentation::character_sprites::tests`
    // — they pin SHEET resolution, which is presentation's contract.
    use super::*;
    use crate::actor::character_catalog::*;
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
        data.characters.get_mut(&first_id).unwrap().default_brain = "DOES_NOT_EXIST".to_string();
        let errors = validator::validate(&data);
        assert!(
            errors.iter().any(|e| e.contains("DOES_NOT_EXIST")),
            "validator should flag missing brain preset; got: {errors:?}"
        );
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
        "npc_raid_enforcer",
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
        // pirate_heavy is a multi-variant rig — its real catalog
        // entries are npc_pirate_heavy_broadside_bess /
        // _iron_mary / _salt_annet. The bare name has no catalog
        // entry intentionally.
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
        // robot_heavy is a multi-variant rig like pirate_heavy —
        // the bare name has no catalog entry by design (variants
        // would be the real characters once a publisher exists).
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
        app.add_plugins(character_roster_plugin());
        app.update(); // runs Startup
        let catalog = app
            .world()
            .get_resource::<CharacterCatalog>()
            .expect("CharacterCatalog resource should be inserted");
        assert!(!catalog.is_empty());
    }
}
