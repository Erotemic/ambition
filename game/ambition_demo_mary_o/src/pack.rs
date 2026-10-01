//! The Mary-O demo's content pack.
//!
//! The demo states its cast (`assets/data/character_catalog.ron`) and its
//! fighters' moves (`assets/data/movesets/`) as data, and registers them
//! through [`PACK`].

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares, with the path `pack.ron`
/// spells.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets",
    sources: [
        "data/character_catalog.ron",
        "data/movesets/mary_o.ron",
        "audio/music_registry.ron",
        "audio/sfx_registry.ron",
    ],
};
