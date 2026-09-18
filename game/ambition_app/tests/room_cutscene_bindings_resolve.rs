//! Every `(room_id, cutscene_id)` binding names a RUNTIME room id that exists.
//!
//! ⛔⛤ **THE DEFECT THIS GUARD EXISTS FOR.** `default_room_cutscene_bindings`
//! bound `"central_hub_main"` — an LDtk LEVEL identifier — to `test_intro`.
//! `auto_trigger_room_cutscenes` compares against the RUNTIME room id
//! (`room_set.active_spec().id`), and `central_hub_main`/`central_hub_basement`
//! both merge into the runtime room `central_hub_complex` at LDtk conversion
//! (see `game/ambition_app/src/app/resources.rs`'s own panic message for the
//! same trap, and `resvg`-free: this needs no window and no render feature).
//! The row could never match, so the hub's boot cutscene never played, and
//! nothing before this guard could tell a dead binding from a room the player
//! had simply not reached yet. Found by inspection 2026-09-18, not by a test.
//!
//! ⚠ This only checks that the ROOM half of each pair resolves. It does not
//! replay the cutscene, and it does not check the cutscene-id half (a
//! `CutsceneLibrary` miss on trigger is a different, louder failure — a
//! `RoomCutsceneBindings` consumer that cannot find the script panics rather
//! than dropping it silently).

use std::collections::BTreeSet;

/// The runtime room ids the shipped sandbox world actually resolves to,
/// built the same way production does: `WorldManifest` -> `LdtkProject` ->
/// `RoomSet`. No app boot, no window — this is data, not presentation.
fn runtime_room_ids() -> BTreeSet<String> {
    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(
        &world_manifest,
    )
    .expect("the shipped sandbox LDtk world must load for this check to mean anything");
    let room_set = project
        .to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary())
        .expect("the shipped sandbox LDtk world must convert to a RoomSet");
    room_set.rooms.into_iter().map(|room| room.id).collect()
}

/// Non-vacuity control: an id nobody authored must NOT resolve, so a bug that
/// made this check accept anything would be caught here first.
#[test]
fn a_room_id_nobody_authored_does_not_resolve() {
    let rooms = runtime_room_ids();
    assert!(!rooms.is_empty(), "no runtime room resolved at all");
    assert!(
        !rooms.contains("a_room_nobody_ever_authored"),
        "the room population accepts a name nobody authored, so every \
         resolution assertion in this file is green by construction"
    );
    // And the trap this file exists for: the LDtk LEVEL id is not a room id.
    assert!(
        !rooms.contains("central_hub_main"),
        "`central_hub_main` is an LDtk level identifier, not a runtime room id \
         -- if this now resolves, the level/room merge changed and the bindings \
         below may need re-checking for the opposite reason"
    );
}

#[test]
fn every_default_room_cutscene_binding_names_a_room_that_exists() {
    let rooms = runtime_room_ids();
    let bindings = ambition_content::dialogue::cutscene_defaults::default_room_cutscene_bindings();
    let dead: Vec<&str> = bindings
        .bindings
        .iter()
        .map(|(room, _cutscene)| room.as_str())
        .filter(|room| !rooms.contains(*room))
        .collect();
    assert!(
        dead.is_empty(),
        "these `default_room_cutscene_bindings` rows name a room id absent from \
         the runtime `RoomSet`, so their cutscene can never trigger: {dead:?}. \
         Likely cause: an LDtk LEVEL id used where a runtime room id belongs \
         (`central_hub_main` -> `central_hub_complex` is the known trap)."
    );
}

#[test]
fn every_intro_room_cutscene_binding_names_a_room_that_exists() {
    let rooms = runtime_room_ids();
    let dead: Vec<&str> = ambition_content::intro::cutscene::intro_room_cutscene_bindings()
        .iter()
        .map(|(room, _cutscene)| *room)
        .filter(|room| !rooms.contains(*room))
        .collect();
    assert!(
        dead.is_empty(),
        "these `INTRO_ROOM_CUTSCENE_BINDINGS` rows name a room id absent from \
         the runtime `RoomSet`, so their cutscene can never trigger: {dead:?}."
    );
}
