//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

// The catalog<->sheet integration tests (boss subdir manifests, Idle-row
// policy, loader coverage) live in `presentation::character_sprites::tests`
// — they pin SHEET resolution, which is presentation's contract.
use super::*;
use ambition_characters::actor::character_catalog::*;
use ambition_characters::brain::Brain;

#[test]
fn catalog_loads_without_panic() {
    // The embedded RON should parse and produce a non-empty
    // catalog. Anything else is a build-time error — pin the
    // baseline.
    let data = catalog();
    assert!(
        !data.data().characters.is_empty(),
        "embedded character_catalog.ron should have characters"
    );
    assert!(
        !data.data().brain_presets.is_empty(),
        "embedded character_catalog.ron should declare brain presets"
    );
    assert!(
        !data.data().action_set_presets.is_empty(),
        "embedded character_catalog.ron should declare action-set presets"
    );
}

#[test]
fn embedded_catalog_passes_validator() {
    // Every reference in the embedded RON must resolve. Pins
    // the catalog as internally consistent so that the Startup
    // panic never fires under normal builds.
    let data = catalog();
    let errors = validator::validate(data.data());
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
    let data = catalog();
    for (id, entry) in &data.data().characters {
        let preset = data
            .data()
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
    let data = catalog();
    for (id, entry) in &data.data().characters {
        let preset = data
            .data()
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
    let catalog = catalog();
    let mut data = catalog.data().clone();
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
    let cat = catalog();
    for (id, entry) in &cat.data().characters {
        let label = cat.display_name(id);
        assert_eq!(
            label,
            Some(entry.display_name.as_str()),
            "display_name_for_character_id('{id}') should return '{}'",
            entry.display_name,
        );
    }
}

#[test]
fn character_id_round_trips_through_display_name() {
    // The unified actor sprite identity is resolved from the display name
    // (every actor carries one) back to the catalog id. Catalog validation
    // rejects duplicate display names, so every entry must round-trip
    // id → name → id.
    for (id, entry) in &catalog().data().characters {
        assert_eq!(
            catalog().id_for_display_name(&entry.display_name),
            Some(id.as_str()),
            "'{}' should round-trip back to id '{id}'",
            entry.display_name,
        );
    }
    assert_eq!(
        catalog().id_for_display_name("Definitely Not A Character"),
        None
    );
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
    "npc_mary_marzakhani",
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
    let data = catalog();
    let mut missing: Vec<&str> = Vec::new();
    for target in RENDERER_COVERAGE_TARGETS {
        if !data.data().characters.contains_key(*target) {
            missing.push(target);
        }
    }
    assert!(
        missing.is_empty(),
        "renderer targets missing catalog entries: {missing:?}",
    );
}

#[test]
fn exemplar_barks_resolve_from_catalog() {
    use ambition_characters::actor::character_catalog::BarkSituation;
    // The Pirate Admiral scaffold exemplar carries an on_hit + provoked +
    // hall pool. Catalog-first resolution must return them (the npcs.rs
    // legacy table is now only a fallback for unmigrated rows).
    assert_eq!(
        catalog().bark_line("npc_pirate_admiral", BarkSituation::OnHit, 0),
        Some("Belay that, ye barnacle!"),
    );
    // on_hit rotates with strike count.
    assert_eq!(
        catalog().bark_line("npc_pirate_admiral", BarkSituation::OnHit, 1),
        Some("Mind the epaulettes, scallywag!"),
    );
    assert_eq!(
        catalog().bark_line("npc_pirate_admiral", BarkSituation::Provoked, 0),
        Some("Broadside, ye bilge rat!"),
    );
    assert!(
        catalog()
            .bark_line("npc_pirate_admiral", BarkSituation::Hall, 0)
            .is_some(),
        "admiral should have a Hall bark"
    );
    // A row with no authored pool for a situation returns None so the
    // firing site falls back.
    assert_eq!(
        catalog().bark_line("npc_kernel_guide", BarkSituation::Idle, 0),
        None,
    );
    // Unknown id is always None.
    assert_eq!(
        catalog().bark_line("npc_not_a_character", BarkSituation::OnHit, 0),
        None,
    );
}

#[test]
fn exemplar_hall_dialogue_ids_resolve() {
    // hall_dialogue_id round-trips against the catalog. (The
    // known-dialogue-id fold-in and the hall.yarn node cross-check are
    // CONTENT-conformance tests — they live with the yarn payload in
    // `ambition_content::dialogue::yarn`.)
    assert_eq!(
        catalog().hall_dialogue_id("npc_pirate_admiral"),
        Some("hall_pirate_admiral"),
    );
    assert_eq!(catalog().hall_dialogue_id("npc_not_a_character"), None);
}

#[test]
fn built_in_roster_non_momentum_and_unknown_ids_have_no_momentum_profile() {
    // Momentum identities are App-local catalog data. The Ambition roster does
    // not own Sanic; standalone providers test their own momentum rows locally.
    assert!(
        catalog().momentum_params("player").is_none(),
        "the protagonist authors no surface-momentum profile"
    );
    assert!(catalog().momentum_params("npc_not_a_character").is_none());
}

#[test]
fn display_name_returns_none_for_unknown_id() {
    // Negative: callers fall back to the id itself when a lookup
    // misses. Pins the contract so a future panic-on-miss change
    // doesn't sneak through.
    assert!(catalog()
        .display_name("npc_definitely_not_in_catalog")
        .is_none());
}

#[test]
fn plugin_inserts_resource_and_validates() {
    // Phase-1 contract: adding CharacterCatalogPlugin makes the
    // resource available and the Startup validator runs without
    // panicking against the shipped catalog.
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterCatalogPlugin {
        catalog_ron: include_str!(
            "../../../../game/ambition_content/assets/data/character_catalog.ron"
        ),
    });
    app.update(); // runs Startup
    let catalog = app
        .world()
        .get_resource::<CharacterCatalog>()
        .expect("CharacterCatalog resource should be inserted");
    assert!(!catalog.is_empty());
}
