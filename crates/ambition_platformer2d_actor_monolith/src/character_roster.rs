//! Ambition catalog fixture used only by this crate's conformance tests.
//!
//! Runtime character authority is the App-local `CharacterCatalog` resource.
//! This module deliberately contains no install seam, cache, or process-global
//! lookup API; it simply parses the checked-in Ambition data for tests that pin
//! catalog/sprite integration without introducing a dependency on the content
//! crate.

use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};

pub(crate) fn catalog() -> CharacterCatalog {
    CharacterCatalog::from_data(parse_catalog(include_str!(
        "../../../game/ambition_content/assets/data/character_catalog.ron"
    )))
}

#[cfg(test)]
mod tests;
