//! Are this provider's move tables content: the same tables, read from files
//! instead of compiled in?
//!
//! Three claims that fail for different reasons. One: each file says exactly
//! what its Rust table said (a migration that changed a fighter would hide a
//! balance edit in a plumbing change). Two: the host takes its table from the
//! file, which a parity test cannot show, because it also passes when the host
//! reads the compiled copy. Three: every character a file names is one this
//! game builds.
//!
//! The Rust tables are the oracle, not a fallback. No `authored/*.rs` calls
//! `with_moveset`; `crate::<x>_moveset::<x>_moveset()` is reached only from
//! this file and from `moveset_source_export`. Fast-iteration I2 step 5
//! allows this: *"a test-only old table may be a temporary parity oracle, not
//! a runtime fallback"*. A fallback would give a fighter two tables and hide
//! the failure the second test below checks.

#![cfg(test)]

use ambition_characters::moveset_content_schema::lowered_movesets;

/// The whole contract, compared structurally, for every migrated character:
/// every verb and every move.
///
/// The floor is first. A pack without the sources, or an empty table, would
/// make the equality below trivially true.
#[test]
fn every_content_move_table_is_the_table_it_used_to_compile_with() {
    let table =
        lowered_movesets(crate::pack::prepared()).expect("the shipped pack carries a move section");
    let mapped: usize = crate::authored_movesets::TABLE_CHARACTERS
        .iter()
        .map(|(_, ids)| ids.len())
        .sum();
    assert!(
        mapped >= 15 && table.len() == mapped,
        "the pack carries {} character table(s) against {mapped} mapped — the \
         comparison below would certify whatever subset happened to load",
        table.len()
    );

    for (name, contract) in crate::authored_movesets::tables() {
        let Some(ids) = crate::authored_movesets::characters_for(name) else {
            continue;
        };
        assert!(
            contract.moves.len() >= 20 && contract.verbs.len() >= 20,
            "`{name}`'s ORACLE is {} move(s) / {} verb(s), which is not a \
             fighter's table",
            contract.moves.len(),
            contract.verbs.len()
        );
        for id in ids {
            let from_content = table
                .get(*id)
                .unwrap_or_else(|| panic!("`pack.ron` declares no table for `{id}`"));
            assert_eq!(
                from_content.verbs, contract.verbs,
                "`{id}`'s content file binds different presses than `{name}`'s Rust table"
            );
            assert_eq!(
                from_content.moves.len(),
                contract.moves.len(),
                "`{id}`'s content file carries a different number of moves"
            );
            // Per move, so a failure names which one instead of printing 100 KB.
            for (a, b) in from_content.moves.iter().zip(contract.moves.iter()) {
                assert_eq!(a, b, "`{id}`/`{}` did not survive the export intact", b.id);
            }
        }
    }
}

/// Every character a move file names is one this game builds.
/// `authored_movesets::tables()` keys its entries by the file's name, and many
/// differ from the character id (`alice`/`npc_alice`,
/// `patent_clerk`/`special_patent_clerk`, …; one table serves two ids). A file
/// under the wrong key fails silently: `authored_intrinsics` gets `None` from
/// `table.get(id)` and leaves the fighter as its module built it.
///
/// The Officer alone cannot catch this: his table name and character id are
/// the same string.
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

/// The host takes its table from the file. The tests above compare values and
/// would pass if `authored_intrinsics` still read the compiled tables.
///
/// So this asks the character definition the game builds.
/// `authored_intrinsics` is the one seam every buildable character passes
/// through (`register_declared_cast`'s loop calls it), so what it returns is
/// what the cast is registered with.
///
/// It runs over every migrated character, not a sample: the Officer's table
/// name matches his id, so checking only him would miss wrong keys.
#[test]
fn every_migrated_fighter_the_game_builds_swings_its_file_s_numbers() {
    let table =
        lowered_movesets(crate::pack::prepared()).expect("the shipped pack carries a move section");
    let mut checked = 0usize;
    for (_, ids) in crate::authored_movesets::TABLE_CHARACTERS {
        for id in *ids {
            let definition = crate::character_catalog::authored_intrinsics(
                id,
                ambition_platformer2d::character::CharacterDefinition::new(
                    *id,
                    *id,
                    crate::AMBITION_CONTENT_PROVIDER,
                ),
                crate::pack::prepared(),
            );
            let moveset = definition.moveset.as_ref().unwrap_or_else(|| {
                panic!("`{id}` is built with no moveset, so the pack did not reach it")
            });
            let from_content = table
                .get(*id)
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
