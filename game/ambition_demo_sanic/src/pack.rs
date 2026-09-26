//! The Sanic demo's content pack.
//!
//! The demo states its cast (`assets/data/character_catalog.ron`) and its
//! fighters' moves (`assets/data/movesets/`) as data, and registers them
//! through [`PACK`].

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares, with the path `pack.ron`
/// spells.
pub static PACK: EmbeddedPack = EmbeddedPack::new(
    include_str!("../assets/pack.ron"),
    &[
        (
            "data/character_catalog.ron",
            include_str!("../assets/data/character_catalog.ron"),
        ),
        (
            "data/movesets/sanic.ron",
            include_str!("../assets/data/movesets/sanic.ron"),
        ),
    ],
);
