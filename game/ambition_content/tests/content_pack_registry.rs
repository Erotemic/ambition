//! Guard: the tool and the game validate against the SAME schemas.
//!
//! `ambition_content_pack::compile` is deliberately the only validator, so that a pack cannot
//! pass one gate and fail another.

use std::collections::BTreeSet;

use ambition_content_pack::{ContentPackManifest, SchemaId};

/// The shipped manifest, read off disk rather than through the crate's
/// `include_str!`. The file IS the authority a human edits when adding content;
/// reading it here means this guard sees the same bytes they do.
fn shipped_manifest() -> ContentPackManifest {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/pack.ron");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    ron::from_str(&text)
        .unwrap_or_else(|e| panic!("{} parses as a pack manifest: {e}", path.display()))
}

fn declared_schemas() -> BTreeSet<SchemaId> {
    shipped_manifest()
        .sources
        .iter()
        .map(|source| source.schema.clone())
        .collect()
}

/// Adding a source to `pack.ron` without registering its schema fails here.
///
/// This is the load-bearing half and it survives the collapse: it compares the
/// shipped MANIFEST — the file a human edits — against what the compositions
/// install, which no amount of list-sharing can make true by construction.
///
/// ⚠ THE TWO ARMS BELOW NOW ASK ONE QUESTION. `pack_schemas()` and
/// `default_registry()` both forward to `ambition_engine_schemas`, so the
/// registration half is no longer "two sites with no compiler error when you
/// miss one" — it is one site. Both arms are kept because their MESSAGES name
/// different consequences a reader still needs (a startup panic versus a tool
/// refusing a pack the game runs), not because they can disagree.
#[test]
fn every_schema_the_shipped_pack_declares_is_installed_in_both_compositions() {
    let declared = declared_schemas();
    assert!(
        !declared.is_empty(),
        "pack.ron declares no sources at all — this guard would pass vacuously, \
         which is the one way it must never pass"
    );

    let game = ambition_content::pack::pack_schemas();
    let cli = ambition_content_cli::default_registry();

    let missing_from_game: Vec<_> = declared
        .iter()
        .filter(|schema| game.get(schema).is_none())
        .map(|schema| schema.to_string())
        .collect();
    assert!(
        missing_from_game.is_empty(),
        "pack.ron declares {missing_from_game:?}, which the GAME's registry does not install.\n\
         The game panics at startup on its own content. Register the schema in \
         `ambition_engine_schemas::engine_schemas()` — the ONE list both \
         compositions forward to."
    );

    let missing_from_cli: Vec<_> = declared
        .iter()
        .filter(|schema| cli.get(schema).is_none())
        .map(|schema| schema.to_string())
        .collect();
    assert!(
        missing_from_cli.is_empty(),
        "pack.ron declares {missing_from_cli:?}, which the CLI's registry does not install.\n\
         `ambition_content` would refuse a pack the game runs fine. Register the schema in \
         `ambition_engine_schemas::engine_schemas()` and link the owning capability \
         crate there."
    );
}

// ⛔⛤ `the_tools_composition_and_the_games_composition_are_the_same_set` WAS HERE
// AND IS DELETED, because the structure it guarded can no longer express the
// defect. It compared `pack::pack_schemas()` against
// `ambition_content_cli::default_registry()`; both now forward to
// `ambition_engine_schemas::engine_schemas()`, so it asserted a function equal
// to itself. ⇒ The edit that would once have caught a real drift — registering a
// schema in one list and not the other — is UNSPELLABLE now: there is one list.
// A test that cannot fail is not coverage, and keeping it would say the set is
// watched when nothing is watching anything.
