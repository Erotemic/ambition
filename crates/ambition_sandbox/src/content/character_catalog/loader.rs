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

use super::entry::CharacterCatalogData;

/// Path constant for tooling that loads the RON file off disk
/// (codegen scripts, hall generator). Relative to the sandbox crate's
/// asset root.
pub const CHARACTER_CATALOG_ASSET: &str = "data/character_catalog.ron";

/// Parse the embedded `character_catalog.ron`. Panics on parse error
/// because the file is shipped with the crate — a parse failure is a
/// build-time issue, not a runtime one.
pub fn load_embedded() -> CharacterCatalogData {
    ron::from_str(include_str!("../../../assets/data/character_catalog.ron"))
        .expect("embedded assets/data/character_catalog.ron should parse")
}
