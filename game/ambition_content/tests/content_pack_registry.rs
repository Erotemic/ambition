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

/// Adding a source to `pack.ron` without registering its schema in BOTH
/// compositions fails here.
///
/// This is the whole point. Migrating a content family is four edits — a
/// handler, a registration, a `pack.ron` line, and the runtime read — and the
/// registration half is the one with two sites and no compiler error when you
/// miss one. Miss the engine side and the game panics at startup on its own
/// content; miss the CLI side and `ambition_content` reports a pack the game
/// runs fine as unknown-schema.
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
         `ambition_platformer2d::content::engine_schemas()`."
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
         `ambition_content_cli::default_registry()` and link the owning capability crate."
    );
}

/// The two compositions install the same schemas, not merely enough of them.
///
/// Coverage of the shipped pack is the load-bearing half above; this is the tighter statement,
/// and it is true today because every schema Ambition authors is owned by an ENGINE capability.
/// A silent divergence is what must not happen.
#[test]
fn the_tools_composition_and_the_games_composition_are_the_same_set() {
    let game: BTreeSet<String> = ambition_content::pack::pack_schemas()
        .schemas()
        .map(|registration| registration.id.to_string())
        .collect();
    let cli: BTreeSet<String> = ambition_content_cli::default_registry()
        .schemas()
        .map(|registration| registration.id.to_string())
        .collect();

    assert_eq!(
        game, cli,
        "the game and the CLI compose different schema sets, so one of them \
         judges Ambition's content by rules the other does not apply"
    );
}
