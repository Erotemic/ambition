//! Are this provider's move tables really CONTENT now — the same tables, read
//! from files rather than compiled in?
//!
//! ⭐⭐ **THREE CLAIMS, AND THEY FAIL FOR DIFFERENT REASONS.** One: each file
//! says what its Rust table said, exactly (a migration that changed a fighter is
//! a balance edit smuggled inside a plumbing change). Two: the HOST takes its
//! table from the file — which a parity test cannot show, because a parity test
//! passes just as well when the host is still reading the compiled copy. Three:
//! every character a file NAMES is one this game builds, which is the claim that
//! nearly went wrong.
//!
//! ⛔ THE RUST TABLES ARE THE ORACLE AND NOT A FALLBACK. No `authored/*.rs`
//! calls `with_moveset` any more; `crate::<x>_moveset::<x>_moveset()` is reached
//! from this file and from `moveset_source_export`, and by nothing the game
//! runs. That is fast-iteration I2 step 5's own allowance — *"a test-only old
//! table may be a temporary parity oracle, not a runtime fallback"* — and the
//! distinction is the whole point: a fallback would give a fighter two tables
//! and hide exactly the failure the second test below is for.

#![cfg(test)]

use ambition_characters::moveset_content_schema::lowered_movesets;

/// ⛔ THE WHOLE CONTRACT, COMPARED STRUCTURALLY, FOR EVERY MIGRATED CHARACTER:
/// every verb and every move.
///
/// ⚠ THE FLOOR IS FIRST. A pack that stopped carrying the sources, or a table
/// that came back empty, makes the equality below trivially true over nothing —
/// this repository's most repeated instrument failure.
#[test]
fn every_content_move_table_is_the_table_it_used_to_compile_with() {
    let table = lowered_movesets(crate::pack::prepared())
        .expect("the shipped pack carries a move section");
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
            // ⛔ PER MOVE, so a failure names WHICH one rather than printing 100 KB.
            for (a, b) in from_content.moves.iter().zip(contract.moves.iter()) {
                assert_eq!(a, b, "`{id}`/`{}` did not survive the export intact", b.id);
            }
        }
    }
}

/// ⛔⛔ **EVERY CHARACTER A MOVE FILE NAMES IS ONE THIS GAME BUILDS — AND NINE OF
/// NINETEEN NEARLY WERE NOT.** `authored_movesets::tables()` keys its entries by
/// *"the name a failure should print"*, which is the FILE's name: eight of them
/// are a rename of the character id (`alice`/`npc_alice`,
/// `patent_clerk`/`special_patent_clerk`, …) and a ninth is one table for two
/// ids. A file written under the wrong key is SILENT — `authored_intrinsics`
/// asks `table.get(id)`, gets `None`, and leaves the fighter exactly as its own
/// module built it.
///
/// ⚠ **THE OFFICER COULD NEVER HAVE CAUGHT THIS**, which is why the migration
/// did not stop at him: his table name and his character id are the same string,
/// so every test about him passes under both spellings.
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
    let table = lowered_movesets(crate::pack::prepared())
        .expect("the shipped pack carries a move section");
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

/// ⭐⭐ **AND THE HOST TAKES ITS TABLE FROM THE FILE.** The tests above cannot
/// show this: they compare values, and would pass unchanged if
/// `authored_intrinsics` still read the compiled ones.
///
/// ⛔ SO THIS ONE ASKS THE CHARACTER DEFINITION the game actually builds.
/// `authored_intrinsics` is the single seam every buildable character passes
/// through (`register_declared_cast`'s one loop calls it), so what it returns IS
/// what the cast is registered with.
///
/// ⛔⛔ AND IT RUNS OVER EVERY MIGRATED CHARACTER, NOT A SAMPLE. The Officer is
/// the one whose table name and character id happen to match, so a test that
/// checked only him would pass with the other sixteen keyed wrongly.
#[test]
fn every_migrated_fighter_the_game_builds_swings_its_file_s_numbers() {
    let table = lowered_movesets(crate::pack::prepared())
        .expect("the shipped pack carries a move section");
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
            // ⛔ AND IT IS NOT VACUOUS: real moves with real timings, so "they
            // matched" is a statement about a fighter rather than about two
            // empty maps that happen to be equal.
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

/// ⛔ AND THE ROBOT LINEAGE IS EXCLUDED FOR A REASON A READER CAN CHECK.
///
/// ⛔⛤ I FIRST WROTE THAT `player_robot` HAS NO CAST ID. It has THREE
/// (`player_robot_v3`, `player_robot_fable`, `player_robot_v2`) — a peer
/// re-deriving the mapping found it. The TRUE claim is that
/// `player_robot_lineage::register` builds its definitions with
/// `definition_from(&catalog, incarnation)` and never calls
/// `authored_intrinsics`, and `register_declared_cast` skips lineage ids
/// outright. ⇒ A guard written on the false premise would have stayed green for
/// the wrong reason and stopped being re-derived.
#[test]
fn the_robot_lineage_does_not_reach_the_seam_the_pack_applies_moves_at() {
    let table = lowered_movesets(crate::pack::prepared()).expect("a move section");
    // ⛔⛤ THE POPULATION IS `LINEAGE` ITSELF, NOT A PREFIX FILTER OVER THE
    // BUILDABLE CAST. My first version asked `buildable_cast()` for ids starting
    // `player_robot` and floored it at three because the CATALOG has three rows
    // — and got TWO, because the lineage registers on its own road and its
    // members are not all in that cast. A floor guessed from a different
    // population is a floor that fails for a reason unrelated to its subject.
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
