//! Intro submodule sanity tests.
//!
//! These don't cover the Bevy plugin systems (those need a full App
//! fixture); they verify the data + dispatch contracts that keep the
//! intro dialogue/cutscenes wired into the sandbox dialog runtime.

use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::dialog::intro_dialogue_ids;
use ambition_cutscene::CutsceneLibrary;
use ambition_dialog::DialogState;
use ambition_dialog::DialogueContext;

#[test]
fn every_intro_dialogue_id_is_registered_with_validator() {
    // Each intro dialogue id must be in `known_dialogue_ids` so the LDtk content validator
    // accepts `NpcSpawn.dialogue_id` references.
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

#[test]
fn known_dialogue_ids_contains_every_intro_id() {
    let catalog = crate::character_catalog::load_catalog();
    let known = crate::dialogue::known_dialogue_ids(&catalog);
    for id in intro_dialogue_ids() {
        assert!(
            known.iter().any(|known_id| known_id == id),
            "known_dialogue_ids() missing intro id '{id}'"
        );
    }
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
            crate::character_catalog::CHARACTER_CATALOG_RON,
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

#[test]
fn install_intro_cutscenes_registers_every_bound_script() {
    let mut lib = CutsceneLibrary::default();
    install_intro_cutscenes(&mut lib);
    for (_room, cutscene_id) in intro_room_cutscene_bindings() {
        assert!(
            lib.get(cutscene_id).is_some(),
            "cutscene '{cutscene_id}' bound to a room but not registered in the library"
        );
    }
}
