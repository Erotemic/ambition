//! Intro submodule sanity tests.
//!
//! These don't cover the Bevy plugin systems (those need a full App
//! fixture); they verify the data + dispatch contracts that keep the
//! intro dialogue/cutscenes wired into the sandbox dialog runtime.

use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::dialog::intro_dialogue_ids;
use super::sprites::intro_npc_sprite_rows;
use crate::dialog::DialogState;
use crate::presentation::cutscene::CutsceneLibrary;

#[test]
fn every_intro_dialogue_id_is_registered_with_validator() {
    // Each intro dialogue id must be in `known_dialogue_ids` so
    // the LDtk content validator accepts `NpcSpawn.dialogue_id`
    // references. With the Yarn migration the dialogue content
    // lives in `.yarn` files; the runtime body smoke-check moved
    // to the bridge's integration tests.
    let known: std::collections::HashSet<&str> =
        crate::dialog::known_dialogue_ids().into_iter().collect();
    for id in intro_dialogue_ids() {
        assert!(
            known.contains(id),
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
    state.start("creator_intro", "Creator");
    assert_eq!(state.dialogue_id(), "creator_intro");
    state.start("oiler_intro", "Oiler");
    assert_eq!(state.dialogue_id(), "oiler_intro");
    state.start("hub_guide", "Kernel Guide");
    assert_eq!(state.dialogue_id(), "hub_guide");
}

#[test]
fn known_dialogue_ids_contains_every_intro_id() {
    let known = crate::dialog::known_dialogue_ids();
    for id in intro_dialogue_ids() {
        assert!(
            known.contains(id),
            "known_dialogue_ids() missing intro id '{id}'"
        );
    }
}

#[test]
fn intro_npc_sprite_rows_have_unique_names() {
    let mut seen = std::collections::HashSet::new();
    for (name, _, _) in intro_npc_sprite_rows() {
        assert!(
            seen.insert(*name),
            "duplicate intro NPC sprite registry name '{name}'"
        );
    }
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
