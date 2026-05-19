//! Intro submodule sanity tests.
//!
//! These don't cover the Bevy plugin systems (those need a full App
//! fixture); they verify the data + dispatch contracts that keep the
//! intro dialogue/cutscenes wired into the sandbox dialog runtime.

use super::cutscene::{install_intro_cutscenes, intro_room_cutscene_bindings};
use super::dialog::{intro_dialogue_ids, IntroDialog};
use super::sprites::intro_npc_sprite_rows;
use crate::dialog::DialogMode;
use crate::presentation::cutscene::CutsceneLibrary;

#[test]
fn every_intro_dialogue_id_round_trips() {
    for id in intro_dialogue_ids() {
        let intro = IntroDialog::from_dialogue_id(id)
            .unwrap_or_else(|| panic!("intro dialogue id '{id}' has no IntroDialog variant"));
        assert!(
            !intro.nodes().is_empty(),
            "intro dialogue '{id}' resolved but has zero nodes"
        );
    }
}

#[test]
fn dialog_mode_dispatches_intro_ids_to_intro_variant() {
    // Sample two intro ids and one sandbox id to make sure the
    // fallback dispatch (Intro first, sandbox second) doesn't break
    // existing sandbox routing.
    assert!(matches!(
        DialogMode::from_dialogue_id("creator_intro"),
        DialogMode::Intro(IntroDialog::CreatorIntro)
    ));
    assert!(matches!(
        DialogMode::from_dialogue_id("oiler_intro"),
        DialogMode::Intro(IntroDialog::OilerIntro)
    ));
    assert!(matches!(
        DialogMode::from_dialogue_id("hub_guide"),
        DialogMode::HubGuide
    ));
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
