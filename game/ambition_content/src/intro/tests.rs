//! Intro submodule sanity tests.
//!
//! These don't cover the Bevy plugin systems (those need a full App
//! fixture); they verify the data + dispatch contracts that keep the
//! intro dialogue/cutscenes wired into the sandbox dialog runtime.

use super::cutscene::install_intro_cutscenes;
use super::dialog::intro_dialogue_ids;
use ambition_cutscene::CutsceneLibrary;
use ambition_dialog::DialogState;
use ambition_dialog::DialogueContext;

#[test]
fn every_intro_dialogue_id_is_registered_with_validator() {
    // Each intro dialogue id must be in `known_dialogue_ids` so the LDtk content validator
    // accepts `NpcSpawn.dialogue_id` references.
    //
    // ⭐ THIS ABSORBED `known_dialogue_ids_contains_every_intro_id`, which
    // asserted the SAME property over the SAME corpus in the same direction —
    // one built a `HashSet` and the other scanned linearly, and neither could
    // fail without the other failing too. Two tests for one fact is the same
    // defect as two authorities for one fact: they cannot disagree usefully,
    // and a reader has to check both to learn they say one thing.
    // ⚠ The name kept is the one that says WHY the property matters; the
    // deleted name said only what the code did.
    let catalog = crate::character_catalog::load_catalog();
    let known: std::collections::HashSet<String> = crate::dialogue::known_dialogue_ids(&catalog)
        .into_iter()
        .collect();
    for id in intro_dialogue_ids() {
        assert!(
            known.contains(*id),
            "intro dialogue id '{id}' is missing from the validator's known list"
        );
    }
}

#[test]
fn dialog_start_sets_dialogue_id_for_intro_and_sandbox() {
    // Sample two intro ids and one sandbox id to make sure the
    // unified registry routes both families through the same
    // dialogue_id surface.
    let mut state = DialogState::default();
    state.start("creator_intro", "Creator", DialogueContext::scripted());
    assert_eq!(state.dialogue_id(), "creator_intro");
    state.start("oiler_intro", "Oiler", DialogueContext::scripted());
    assert_eq!(state.dialogue_id(), "oiler_intro");
    state.start("hub_guide", "Kernel Guide", DialogueContext::scripted());
    assert_eq!(state.dialogue_id(), "hub_guide");
}

/// ⛔⛔ EVERY INTRO `NpcSpawn` MUST NAME A CHARACTER THE CATALOG KNOWS.
///
/// This replaces a test that checked the deleted preload table's display names
/// were unique — a property of a table that published under names the world
/// never used. What actually has to hold now is this: the intro's NPCs are drawn
/// because the room's cast demand raises their sheets, and that demand is keyed
/// on the `character_id` each `NpcSpawn` authors. An id with no catalog row
/// resolves to nothing, the actor draws the placeholder, and — with the preload
/// gone — there is no second road quietly covering for it.
///
/// The preload used to mask exactly this: it decoded eleven sheets at boot under
/// display names, so a mis-keyed placement still found art by accident.
#[test]
fn every_intro_npc_spawn_names_a_character_the_catalog_knows() {
    let Some(world) = crate::worlds::intro_ldtk_text() else {
        // `static_map` off: the world is not embedded, so there is nothing to
        // read. Skipping is correct — the feature-on build is where this holds.
        return;
    };
    let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(
            &crate::character_catalog::character_catalog_ron(),
        ),
    );
    let doc: serde_json::Value =
        serde_json::from_str(world).expect("intro.ldtk is a JSON document");
    let mut checked = 0usize;
    for level in doc["levels"].as_array().into_iter().flatten() {
        let level_id = level["identifier"].as_str().unwrap_or("<unnamed>");
        for layer in level["layerInstances"].as_array().into_iter().flatten() {
            for entity in layer["entityInstances"].as_array().into_iter().flatten() {
                if entity["__identifier"].as_str() != Some("NpcSpawn") {
                    continue;
                }
                let character_id = entity["fieldInstances"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|field| field["__identifier"].as_str() == Some("character_id"))
                    .and_then(|field| field["__value"].as_str())
                    .unwrap_or_default();
                assert!(
                    !character_id.is_empty(),
                    "an `NpcSpawn` in `{level_id}` authors no `character_id`; the cast \
                     demand has nothing to raise and it will draw the placeholder"
                );
                assert!(
                    catalog.get(character_id).is_some(),
                    "`NpcSpawn` in `{level_id}` names `{character_id}`, which has no \
                     character-catalog row — it will draw the placeholder, and nothing \
                     preloads intro art any more"
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 8,
        "premise: the intro authors NPC spawns to check (found {checked}); if this \
         dropped, the test is passing because it looked at nothing"
    );
}

/// Each intro room says the cutscene it starts in `intro.ldtk` (its
/// `entry_cutscene` level field), and the intro library has each script.
/// The pairs were a Rust table until 2026-10-02.
#[test]
fn each_intro_room_says_its_cutscene_in_data_and_the_library_has_it() {
    use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};
    let manifest = crate::worlds::world_manifest();
    let rooms = LdtkProject::load_default_for_dev(&manifest)
        .expect("the shipped worlds load")
        .to_room_set(&manifest, &LdtkVocabulary::engine())
        .expect("the shipped worlds compose")
        .rooms;
    let mut lib = CutsceneLibrary::default();
    install_intro_cutscenes(&mut lib);
    for (room, cutscene) in [
        ("intro_wake_room", "intro_wake"),
        ("intro_raid_corridor", "intro_raid"),
        ("drain_alley", "drain_market_arrival"),
    ] {
        let spec = rooms
            .iter()
            .find(|spec| spec.id == room)
            .unwrap_or_else(|| panic!("`{room}` is a shipped intro room"));
        assert_eq!(
            spec.metadata.entry_cutscene.as_deref(),
            Some(cutscene),
            "`{room}` does not say it starts `{cutscene}`"
        );
        assert!(lib.get(cutscene).is_some(), "`{cutscene}` is not in the intro library");
    }
}

#[test]
fn the_intro_rows_are_in_their_registries_before_the_first_tick() {
    // The dialogue plugin is added AFTER the intro plugin and adds rows to the
    // same registries. A plugin that replaced a registry would lose the other's
    // rows, and an install in a system would be missing until the first tick.
    // No `App::finish`: the sim harness drives `App::update` and never runs it.
    use crate::banter::CombatBanterRegistry;
    use ambition_platformer2d::world::rooms::GatePortalRegistry;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins((
        super::IntroPlugin,
        crate::dialogue::AmbitionDialogueContentPlugin,
    ));

    let world = app.world();
    let library = world.resource::<CutsceneLibrary>();
    for cutscene in ["intro_wake", "intro_raid", "drain_market_arrival"] {
        assert!(
            library.get(cutscene).is_some(),
            "the intro cutscene `{cutscene}` is not in the library"
        );
    }
    let banter = world.resource::<CombatBanterRegistry>();
    assert!(
        banter.pick_hit_bark("Lab Raider", 0).is_some(),
        "the intro raiders have no barks"
    );
    // The dialogue plugin's own rows stay beside the intro's.
    assert!(
        banter.pick_hit_bark("Iron Mary", 0).is_some(),
        "the pirate barks were replaced"
    );
    assert!(
        world
            .resource::<GatePortalRegistry>()
            .is_portal(super::plugin::INTRO_PORTAL_ZONE_ID),
        "the intro portal is not registered"
    );
}
