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
    let mut checked = 0usize;
    for (id, entry) in &data.data().characters {
        if entry.default_brain.is_empty() {
            continue;
        }
        checked += 1;
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
    //  and the skip cannot hollow the test out: if every row stopped naming a
    // preset this loop would pass over an empty set and report success.
    assert!(
        checked > 0,
        "no character in the catalog names a brain preset, so this guard checked \
         nothing — either the vocabulary is retired (delete this test) or the \
         field stopped being read"
    );
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
    // Every catalog entry must resolve id -> display_name, since that lookup
    // is how a spawned NPC gets its label (see
    // `spawn_actors::npc_display_label`, pinned by
    // `authored_npc_takes_its_label_from_the_catalog_display_name`).
    let cat = catalog();
    for (id, entry) in &cat.data().characters {
        let label = cat.display_name(id);
        assert_eq!(
            label,
            Some(entry.display_name.as_str()),
            "display_name('{id}') should return '{}'",
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

/// The whole point of shipping suggested lines with the art: a character whose sprite target
/// authored `dialogue_hints` and NO bark pools still speaks on its Hall pedestal, in its own voice.
#[test]
fn a_character_with_only_suggested_lines_still_speaks_in_the_hall() {
    let cat = catalog();
    assert!(
        cat.get("npc_marie_curry")
            .expect("Marie Curry catalog row")
            .barks
            .pool(BarkSituation::Hall)
            .is_empty(),
        "fixture assumption: she authored no Hall pool, only suggested lines",
    );
    assert_eq!(
        cat.bark_line("npc_marie_curry", BarkSituation::Hall, 0),
        Some("Careful, it is still reactive."),
    );
    // ...and the same voice answers every other occasion until a pool exists.
    for situation in [
        BarkSituation::OnHit,
        BarkSituation::Provoked,
        BarkSituation::Idle,
    ] {
        assert!(
            cat.bark_line("npc_marie_curry", situation, 0).is_some(),
            "{situation:?} should reach the fallback pool",
        );
    }
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
        catalog().momentum_params("player_robot_v3").is_none(),
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
