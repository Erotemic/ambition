//! Catalog parsing for the character catalog.
//!
//! This crate owns the catalog schema, parser, and resolver; experience
//! providers own the roster data they register in each Bevy `App`.

use super::entry::CharacterCatalogData;

/// Parse a catalog RON string without panicking.
///
/// Provider registration is a transactional build-time boundary. Malformed
/// authored data must therefore become a structured registration error instead
/// of unwinding before the previous valid App-local assembly can be preserved.
pub fn try_parse_catalog(ron_text: &str) -> Result<CharacterCatalogData, String> {
    ron::from_str(ron_text).map_err(|error| error.to_string())
}

/// Parse a trusted embedded catalog RON string.
///
/// Tests and narrow fixtures that intentionally embed one known catalog may use
/// this convenience wrapper. Provider-facing registration uses
/// [`try_parse_catalog`] so malformed fragments remain ordinary errors.
pub fn parse_catalog(ron_text: &str) -> CharacterCatalogData {
    try_parse_catalog(ron_text).expect("character catalog RON should parse")
}
