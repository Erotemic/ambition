//! Catalog parsing for the character catalog.
//!
//! This crate owns the catalog SCHEMA + parser + resolver; the GAME
//! owns the roster data. `ambition_actors::character_roster` embeds
//! `assets/data/character_catalog.ron` (which ships with the sandbox)
//! and exposes the parsed `EMBEDDED_CATALOG` to runtime consumers.

use super::entry::CharacterCatalogData;

/// Parse a catalog RON string. Panics on parse error — callers embed
/// the file at compile time, so a parse failure is a build-time data
/// bug, not a runtime condition.
pub fn parse_catalog(ron_text: &str) -> CharacterCatalogData {
    ron::from_str(ron_text).expect("character catalog RON should parse")
}
