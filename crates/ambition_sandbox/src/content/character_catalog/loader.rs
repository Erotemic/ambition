//! Synchronous bootstrap loader for the character catalog.
//!
//! Mirrors the [`crate::content::data::SandboxDataSpec::load_embedded`]
//! pattern — parse the RON file at compile-time via `include_str!`
//! so the catalog is available without an async asset round-trip.
//! Phase-1 simplification; Phase-N can layer
//! `bevy_common_assets::ron::RonAssetPlugin` on top once a real
//! asset-driven hot-reload story lands.
//!
//! The embedded path is relative to the sandbox crate root and lives
//! under `assets/data/` so it ships with the sandbox.

use std::sync::LazyLock;

use super::entry::CharacterCatalogData;

/// Path constant for tooling that loads the RON file off disk
/// (codegen scripts, hall generator). Relative to the sandbox crate's
/// asset root.
#[allow(
    dead_code,
    reason = "Public path constant for codegen + hall-generator tooling that loads the catalog off disk; the runtime uses include_str! via load_embedded."
)]
pub const CHARACTER_CATALOG_ASSET: &str = "data/character_catalog.ron";

/// Parse the embedded `character_catalog.ron`. Panics on parse error
/// because the file is shipped with the crate — a parse failure is a
/// build-time issue, not a runtime one.
pub fn load_embedded() -> CharacterCatalogData {
    ron::from_str(include_str!("../../../assets/data/character_catalog.ron"))
        .expect("embedded assets/data/character_catalog.ron should parse")
}

/// One-time parse cache so non-Bevy call sites (the LDtk parser,
/// tests, headless tooling) can query the catalog without
/// re-parsing. The Bevy `CharacterCatalog` resource always takes
/// precedence when one is available, but the parser runs inside
/// `convert_npc_spawn` which has no `Res<>` access.
pub static EMBEDDED_CATALOG: LazyLock<CharacterCatalogData> = LazyLock::new(load_embedded);

/// Look up the display name for a character id. Returns `None` if
/// the id is not in the catalog; callers fall back to using the id
/// itself as a label.
pub fn display_name_for_character_id(character_id: &str) -> Option<&'static str> {
    EMBEDDED_CATALOG
        .characters
        .get(character_id)
        .map(|entry| entry.display_name.as_str())
}
