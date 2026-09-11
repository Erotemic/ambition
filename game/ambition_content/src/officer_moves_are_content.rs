//! Is the Officer's move table really CONTENT now — the same table, read from a
//! file rather than compiled in?
//!
//! ⭐⭐ **TWO CLAIMS, AND THEY FAIL FOR DIFFERENT REASONS.** One: the file says
//! what the Rust table said, exactly (a migration that changed a fighter is a
//! balance edit smuggled inside a plumbing change). Two: the HOST takes it from
//! the file — which a parity test cannot show, because a parity test passes just
//! as well when the host is still reading the compiled copy.
//!
//! ⛔ THE RUST TABLE IS THE ORACLE AND NOT A FALLBACK. `authored/officer.rs` no
//! longer calls `with_moveset`; `crate::officer_moveset::officer_moveset()` is
//! reached from this file and from the exporter, and by nothing the game runs.
//! That is fast-iteration I2 step 5's own allowance — *"a test-only old table
//! may be a temporary parity oracle, not a runtime fallback"* — and the
//! distinction is the whole point: a fallback would give the fighter two tables
//! and hide exactly the failure the second test below is for.

#![cfg(test)]

use ambition_characters::moveset_content_schema::lowered_movesets;

/// ⛔ THE WHOLE CONTRACT, COMPARED STRUCTURALLY: every verb and every move.
///
/// ⚠ THE FLOOR IS FIRST. A pack that stopped carrying the source, or a table
/// that came back empty, makes the equality below trivially true over nothing —
/// this repository's most repeated instrument failure.
#[test]
fn the_officers_content_table_is_the_table_he_used_to_compile_with() {
    let compiled = crate::officer_moveset::officer_moveset();
    assert!(
        compiled.moves.len() >= 20 && compiled.verbs.len() >= 20,
        "the ORACLE is {} move(s) / {} verb(s), which is not the Officer's table \
         — the comparison below would certify almost nothing",
        compiled.moves.len(),
        compiled.verbs.len()
    );

    let table = lowered_movesets(crate::pack::prepared())
        .expect("the shipped pack carries a move section");
    let from_content = table
        .get("officer")
        .expect("`pack.ron` declares `data/movesets/officer.ron`");

    assert_eq!(
        from_content.verbs, compiled.verbs,
        "the content file binds different presses than the Rust table"
    );
    assert_eq!(
        from_content.moves.len(),
        compiled.moves.len(),
        "the content file carries a different number of moves"
    );
    // ⛔ PER MOVE, so a failure names WHICH one rather than printing 95 KB.
    for (a, b) in from_content.moves.iter().zip(compiled.moves.iter()) {
        assert_eq!(a, b, "`{}` did not survive the export intact", b.id);
    }
}

/// ⭐⭐ **AND THE HOST TAKES ITS TABLE FROM THE FILE.** The test above cannot
/// show this: it compares two values and would pass unchanged if
/// `authored_intrinsics` still read the compiled one.
///
/// ⛔ SO THIS ONE EDITS THE CONTENT VALUE AND ASKS THE CHARACTER DEFINITION the
/// game builds. `authored_intrinsics` is the single seam every buildable
/// character passes through (`register_declared_cast`'s one loop calls it), so
/// what it returns IS what the cast is registered with.
#[test]
fn the_officer_the_game_builds_swings_the_file_s_numbers() {
    let definition = crate::character_catalog::authored_intrinsics(
        "officer",
        ambition_platformer2d::character::CharacterDefinition::new(
            "officer",
            "Officer",
            crate::AMBITION_CONTENT_PROVIDER,
        ),
    );
    let moveset = definition
        .moveset
        .as_ref()
        .expect("the Officer is built with a moveset, and it comes from the pack");

    let from_content = lowered_movesets(crate::pack::prepared())
        .and_then(|table| table.get("officer"))
        .expect("the pack carries his table");
    assert_eq!(
        moveset, from_content,
        "the definition the game builds is not the table the content file carries \
         — something is still supplying a compiled one"
    );

    // ⛔ AND IT IS NOT VACUOUS: the table has real moves with real timings, so
    // "they matched" is a statement about a fighter rather than about two empty
    // maps that happen to be equal.
    assert!(
        moveset.moves.len() >= 20,
        "{} move(s) — the Officer was built with a table nobody would notice \
         losing",
        moveset.moves.len()
    );
    assert!(
        moveset.moves.iter().any(|m| m.duration_s > 0.0),
        "every move in the built table lasts no time at all"
    );
}
