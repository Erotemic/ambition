//! Are this provider's move tables content?
//!
//! The files under `assets/data/movesets/` are the source of the fighters'
//! moves. Two claims that fail for different reasons. One: the host takes its
//! table from the file, not from compiled code. Two: every character a file
//! names is one this game builds.
//!
#![cfg(test)]

use ambition_characters::moveset_content_schema::lowered_movesets;

/// Every character a move file names is one this game builds.
///
/// A file names its characters by entity id. A table under a wrong id fails
/// silently: `authored_intrinsics` gets `None` from `table.get(id)` and leaves
/// the fighter with no authored moves.
#[test]
fn every_character_the_move_section_names_is_one_this_game_builds() {
    let buildable: std::collections::BTreeSet<&str> =
        crate::character_catalog::buildable_cast().collect();
    assert!(
        buildable.len() >= 20,
        "{} buildable character(s) — the set this compares against is not the \
         cast, so the check below would certify almost anything",
        buildable.len()
    );
    let table =
        lowered_movesets(crate::pack::prepared()).expect("the shipped pack carries a move section");
    assert!(!table.is_empty(), "the move section is empty");
    let strangers: Vec<&str> = table
        .keys()
        .map(String::as_str)
        .filter(|id| !buildable.contains(id))
        .collect();
    assert!(
        strangers.is_empty(),
        "`pack.ron` carries move tables for {strangers:?}, which this game builds \
         no character for. A move file under the wrong key is SILENT."
    );
}

/// The host takes its table from the file.
///
/// This asks the character definition the game builds.
/// `authored_intrinsics` is the one seam every buildable character passes
/// through (`register_declared_cast`'s loop calls it), so what it returns is
/// what the cast is registered with.
///
/// It runs over every character the move files name, not a sample.
#[test]
fn every_migrated_fighter_the_game_builds_swings_its_file_s_numbers() {
    let table =
        lowered_movesets(crate::pack::prepared()).expect("the shipped pack carries a move section");
    let mut checked = 0usize;
    for id in table.keys().map(String::as_str) {
        let definition = crate::character_catalog::authored_intrinsics(
            id,
            ambition_platformer2d::character::CharacterDefinition::new(
                id,
                id,
                crate::AMBITION_CONTENT_PROVIDER,
            ),
            crate::pack::prepared(),
        );
        let moveset = definition.moveset.as_ref().unwrap_or_else(|| {
            panic!("`{id}` is built with no moveset, so the pack did not reach it")
        });
        let from_content = table
            .get(id)
            .unwrap_or_else(|| panic!("the pack carries no table for `{id}`"));
        assert_eq!(
            moveset, from_content,
            "the definition the game builds for `{id}` is not the table the \
             content file carries — something is still supplying a compiled one"
        );
        // Not vacuous: real moves with real timings, not two equal empty maps.
        assert!(
            moveset.moves.len() >= 20,
            "`{id}` was built with {} move(s) — a table nobody would notice \
             losing",
            moveset.moves.len()
        );
        assert!(
            moveset.moves.iter().any(|m| m.duration_s > 0.0),
            "every move `{id}` was built with lasts no time at all"
        );
        checked += 1;
    }
    assert!(
        checked >= 15,
        "only {checked} character(s) were checked, which is not the migrated set"
    );
}

/// The robot lineage is excluded, for a reason a reader can check.
///
/// `player_robot` has three cast ids (`player_robot_v3`,
/// `player_robot_fable`, `player_robot_v2`). But
/// `player_robot_lineage::register` builds its definitions with
/// `definition_from(&catalog, incarnation)` and never calls
/// `authored_intrinsics`, and `register_declared_cast` skips lineage ids.
#[test]
fn the_robot_lineage_does_not_reach_the_seam_the_pack_applies_moves_at() {
    let table = lowered_movesets(crate::pack::prepared()).expect("a move section");
    // The population is `LINEAGE` itself, not a prefix filter over the
    // buildable cast: the lineage registers on its own road, and not all its
    // members are in that cast.
    let lineage: Vec<&str> = crate::player_robot_lineage::LINEAGE
        .iter()
        .map(|incarnation| incarnation.id)
        .collect();
    assert!(
        lineage.len() >= 3,
        "{lineage:?} — the lineage this excludes is not the lineage"
    );
    let claimed: Vec<&str> = lineage
        .iter()
        .copied()
        .filter(|id| table.contains_key(*id))
        .collect();
    assert!(
        claimed.is_empty(),
        "the pack carries move tables for {claimed:?}, which register on the \
         lineage's own road and never pass through `authored_intrinsics` — so \
         those tables would be loaded by nobody"
    );
}
