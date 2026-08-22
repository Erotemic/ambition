//! Real-provider proof for App-local authored catalogs.
//!
//! This deliberately stops short of activating all three games: the remaining
//! runtime lookup migration is a separate acceptance slice. It proves the hard
//! composition property underneath that work — all linked providers can publish
//! their immutable definitions into one App without plugin-order authority or
//! process-global cross-App contamination.

use ambition_platformer2d::boss_encounter::{BossCatalog, BossCatalogRegistry};
use ambition_platformer2d::audio::catalog::AudioCatalogRegistry;
use ambition_platformer2d::characters::actor::character_catalog::{
    CharacterCatalog, CharacterCatalogDefaults, CharacterCatalogOwners,
};
use bevy::prelude::*;

fn register_ambition(app: &mut App) {
    ambition_content::character_catalog::register(app);
    ambition_content::bosses::register(app);
    ambition_content::audio_registries::register(app);
}

fn register_sanic(app: &mut App) {
    ambition_demo_sanic::install_sanic_content(app);
}

fn register_mary_o(app: &mut App) {
    ambition_demo_mary_o::install_mary_o_content(app);
}

fn character_ids(app: &App) -> Vec<String> {
    app.world()
        .resource::<CharacterCatalog>()
        .iter()
        .map(|(id, _)| id.clone())
        .collect()
}

fn audio_providers(app: &App) -> Vec<String> {
    app.world()
        .resource::<AudioCatalogRegistry>()
        .providers()
        .map(str::to_string)
        .collect()
}

fn boss_providers(app: &App) -> Vec<String> {
    app.world()
        .resource::<BossCatalogRegistry>()
        .providers()
        .map(str::to_string)
        .collect()
}

#[test]
fn three_real_providers_compose_independent_of_registration_order() {
    let mut forward = App::new();
    register_ambition(&mut forward);
    register_sanic(&mut forward);
    register_mary_o(&mut forward);

    let mut reverse = App::new();
    register_mary_o(&mut reverse);
    register_sanic(&mut reverse);
    register_ambition(&mut reverse);

    assert_eq!(character_ids(&forward), character_ids(&reverse));
    assert_eq!(
        forward.world().resource::<CharacterCatalog>(),
        reverse.world().resource::<CharacterCatalog>()
    );
    assert_eq!(audio_providers(&forward), audio_providers(&reverse));
    for id in ["sanic", "npc_snakes_on_a_paper_plane", "npc_ai_slop"] {
        assert_eq!(
            forward
                .world()
                .resource::<CharacterCatalogOwners>()
                .provider_for(id),
            reverse
                .world()
                .resource::<CharacterCatalogOwners>()
                .provider_for(id),
            "`{id}`'s owning provider depends on registration ORDER, which is the \
             cross-App contamination this test exists to refuse"
        );
    }
    assert_eq!(boss_providers(&forward), boss_providers(&reverse));

    let catalog = forward.world().resource::<CharacterCatalog>();
    for id in ["player_robot_v3", "sanic", "super_sanic", "mary_o"] {
        assert!(
            catalog.get(id).is_some(),
            "missing real provider character {id}"
        );
    }

    let defaults = forward.world().resource::<CharacterCatalogDefaults>();
    assert_eq!(defaults.for_provider("ambition"), Some("player_robot_v3"));
    assert_eq!(defaults.for_provider("sanic"), Some("sanic"));
    assert_eq!(defaults.for_provider("mary_o"), Some("mary_o"));

    let owners = forward.world().resource::<CharacterCatalogOwners>();
    assert_eq!(owners.provider_for("player_robot_v3"), Some("ambition"));
    assert_eq!(owners.provider_for("sanic"), Some("sanic"));
    assert_eq!(owners.provider_for("mary_o"), Some("mary_o"));

    let audio = forward.world().resource::<AudioCatalogRegistry>();
    assert!(audio.music_for("ambition").is_some());
    assert!(audio.sfx_for("ambition").is_some());
    assert!(audio.music_for("sanic").is_some());
    assert!(audio.sfx_for("sanic").is_some());
    assert!(audio.music_for("mary_o").is_none());
    audio
        .combined_music_registry("ambition")
        .expect("real provider music ids must compose without collision");

    // THERE IS NO HOSTILE-ROSTER LIST TO LEAVE ANY MORE (AC6). Every creature in every provider
    // is a CHARACTER, so the composition property this test exists for is asserted against the
    // authority they actually live in: the owners map.
    assert_eq!(
        owners.provider_for("sanic"),
        Some("sanic"),
        "Sanic's content still composes App-locally — the badnik moved from its \
         roster fragment to a character, not out of the provider"
    );
    assert_eq!(
        owners.provider_for("npc_snakes_on_a_paper_plane"),
        Some("mary_o"),
        "the plane swarms moved provider WITH their bodies (2026-08-13): Mary-O \
         authors their catalog rows and definitions, and the Hall stages them \
         from the merged catalog"
    );
    assert_eq!(boss_providers(&forward), vec!["ambition"]);
    let bosses = forward.world().resource::<BossCatalog>();
    assert!(bosses.behavior("clockwork_warden").is_some());
    assert!(bosses.encounter("clockwork_warden").is_some());
}

#[test]
fn separate_apps_select_independent_provider_sets() {
    let mut sanic = App::new();
    register_sanic(&mut sanic);

    let mut mary_o = App::new();
    register_mary_o(&mut mary_o);

    let sanic_catalog = sanic.world().resource::<CharacterCatalog>();
    assert!(sanic_catalog.get("sanic").is_some());
    assert!(sanic_catalog.get("mary_o").is_none());

    let mary_o_catalog = mary_o.world().resource::<CharacterCatalog>();
    assert!(mary_o_catalog.get("mary_o").is_some());
    assert!(mary_o_catalog.get("sanic").is_none());

    assert!(sanic
        .world()
        .resource::<AudioCatalogRegistry>()
        .music_for("sanic")
        .is_some());
    assert!(mary_o
        .world()
        .get_resource::<AudioCatalogRegistry>()
        .is_none());

    // The App-local property is asserted over the character owners map: Sanic's own provider,
    // and nobody else's.
    let sanic_owners = sanic
        .world()
        .get_resource::<CharacterCatalogOwners>()
        .expect("Sanic content publishes its own catalog");
    assert_eq!(sanic_owners.provider_for("sanic"), Some("sanic"));

    // Mary-O's every enemy is a character too: the plane
    // swarms — her last rows, kept as a standalone-build fallback — are her own
    // registered characters, like the snake and the slop before them.
    let mary_o_owners = mary_o
        .world()
        .get_resource::<CharacterCatalogOwners>()
        .expect("Mary-O content publishes its own catalog");
    assert_eq!(
        mary_o_owners.provider_for("npc_snakes_on_a_paper_plane"),
        Some("mary_o"),
        "the plane swarms' catalog rows travel with their one provider"
    );

    for app in [&sanic, &mary_o] {
        assert!(app.world().get_resource::<BossCatalogRegistry>().is_none());
        assert!(app.world().get_resource::<BossCatalog>().is_none());
    }
}

/// #13 — the FULL Hall validates against the merged Ambition + Sanic + Mary-O
/// catalog. The embedded (Ambition-only) content check tolerates the Hall's
/// cross-provider characters (`sanic`, `mary_o`, …); with every provider loaded,
/// each Hall NPC's `character_id` exists and its `brain_override` resolves inside
/// that character's provider namespace. It also proves every exhibit has a Hall
/// bark and a catalog-backed LDtk dialogue binding that names a real Yarn node.
#[test]
fn the_full_hall_validates_with_all_three_provider_catalogs() {
    use ambition_platformer2d::ldtk_map::{field_string, LdtkProject};

    // The world manifest (which names the Hall's secondary world) must be
    // installed before any world load, exactly as the content plugin does.

    let mut app = App::new();
    register_ambition(&mut app);
    register_sanic(&mut app);
    register_mary_o(&mut app);
    let catalog = app.world().resource::<CharacterCatalog>();

    let yarn_titles: std::collections::BTreeSet<&str> = ambition_content::dialogue::YARN_SOURCES
        .iter()
        .flat_map(|(_, source)| source.lines())
        .filter_map(|line| line.strip_prefix("title:"))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .collect();

    let project = LdtkProject::load_default_for_dev(&ambition_content::worlds::world_manifest())
        .expect("embedded LDtk loads");
    let mut checked = 0;
    for level in &project.levels {
        if level.identifier != "hall_of_characters" {
            continue;
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let character_id = field_string(entity, "character_id")
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty());
            let brain_override = field_string(entity, "brain_override")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let Some(character_id) = character_id else {
                continue;
            };
            catalog
                .validate_brain_override(&character_id, brain_override.as_deref())
                .unwrap_or_else(|error| {
                    panic!(
                        "Hall NpcSpawn '{}' ({character_id}) fails full-host validation: {error}",
                        entity.iid
                    )
                });

            let entry = catalog
                .get(&character_id)
                .unwrap_or_else(|| panic!("validated Hall character {character_id} disappeared"));
            // The exhibit must have SOMETHING to say. Resolved through `bark`, not the raw `hall`
            // pool: a character generated from its sprite target ships suggested lines and no
            // per-situation pools, and it speaks those on its pedestal.
            assert!(
                entry
                    .bark(
                        ambition_platformer2d::characters::actor::character_catalog::BarkSituation::Hall,
                        0
                    )
                    .is_some(),
                "Hall exhibit {character_id} has nothing to say -- no hall bark pool and no \
                 fallback_dialogue. Author `dialogue_hints.suggested_barks` on its sprite target."
            );
            // A bespoke Yarn conversation is OPTIONAL -- most exhibits are just a
            // body with a voice. What is not optional is agreement: if either
            // side names a dialogue scene, both must name the same live one.
            let authored_dialogue_id =
                field_string(entity, "dialogue_id").filter(|id| !id.trim().is_empty());
            match (entry.hall_dialogue_id.as_deref(), &authored_dialogue_id) {
                (Some(expected), Some(authored)) => {
                    assert_eq!(
                        authored.trim(),
                        expected,
                        "Hall exhibit {character_id} dialogue binding drifted from its catalog row"
                    );
                    assert!(
                        yarn_titles.contains(expected),
                        "Hall exhibit {character_id} references missing Yarn node {expected}"
                    );
                }
                (Some(expected), None) => panic!(
                    "Hall exhibit {character_id} declares hall_dialogue_id '{expected}' but its \
                     LDtk spawn authors none"
                ),
                (None, Some(authored)) => panic!(
                    "Hall exhibit {character_id} LDtk spawn authors dialogue_id '{authored}' with \
                     no catalog hall_dialogue_id to match it"
                ),
                (None, None) => {}
            }
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "expected the Hall to place several catalog NPCs (validated {checked})"
    );
}
