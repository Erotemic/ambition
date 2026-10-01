//! The Pocket demo's content pack.
//!
//! The demo states its cast (`assets/data/character_catalog.ron`) as data and
//! registers it through [`PACK`].

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares, with the path `pack.ron`
/// spells.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets",
    sources: [
        "data/character_catalog.ron",
        "audio/sfx_registry.ron",
    ],
};
