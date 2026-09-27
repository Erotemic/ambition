//! The character facets a content pack authors, folded into a definition.
//!
//! A facet is a file that a capability owns about one character: its move
//! table (`moveset_content_schema`) and its platform-fighter values
//! (`smash_fighter`). Each capability folds its own facet, beside its schema.
//! This is the one list of those folds, so a registration road calls one
//! function and names no capability. A new character facet adds its fold here.

use crate::actor::definition::CharacterDefinition;

/// `definition` with every character facet `pack` authors for its id.
pub fn fold_character_facets(
    pack: &ambition_content_pack::PreparedContentPack,
    definition: CharacterDefinition,
) -> CharacterDefinition {
    let definition = crate::moveset_content_schema::fold_moveset(pack, definition);
    crate::smash_fighter::content_schema::fold_into_definition(pack, definition)
}
