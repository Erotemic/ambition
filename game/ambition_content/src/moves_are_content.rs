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
/// silently: `with_pack_moveset` gets `None` from `table.get(id)` and leaves
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
/// `with_pack_moveset` is the one seam every buildable character passes
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
        let definition = crate::character_catalog::with_pack_moveset(
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
        // v2's table is the theorem chain alone, by design (see
        // `the_theorem_chain_is_two_hits_on_one_timeline`).
        assert!(
            moveset.moves.len() >= if id == "player_robot_v2" { 1 } else { 20 },
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

/// The robot lineage takes its tables from the file, on its own road.
///
/// The lineage does not pass `register_declared_cast` (that loop skips it), so
/// the test above does not prove its road. `player_robot_lineage::definition`
/// is what that road builds, and it must apply the same seam.
///
/// The population is `LINEAGE` itself, not a prefix filter over the buildable
/// cast, and v0 is in it with no table: a file entry for it would also have to
/// arrive.
#[test]
fn the_robot_lineage_wears_the_tables_its_file_carries() {
    let table = lowered_movesets(crate::pack::prepared()).expect("a move section");
    let mut checked = 0usize;
    for incarnation in crate::player_robot_lineage::LINEAGE {
        let built = crate::player_robot_lineage::definition(incarnation).moveset;
        assert_eq!(
            built.as_ref(),
            table.get(incarnation.id),
            "`{}` is built with a move table that is not the one the pack \
             carries for it, so something other than the file supplies its moves",
            incarnation.id
        );
        checked += usize::from(built.is_some());
    }
    assert!(
        checked >= 2,
        "only {checked} lineage member(s) have a move table; v2 and v3 both do"
    );
}
