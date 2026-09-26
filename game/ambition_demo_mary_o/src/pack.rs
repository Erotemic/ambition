//! The Mary-O demo's content pack.
//!
//! The demo states its fighters' moves as data (`assets/data/movesets/`) and
//! reads them back through [`PACK`] where it registers a character.

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares, with the path `pack.ron`
/// spells.
pub static PACK: EmbeddedPack = EmbeddedPack::new(
    include_str!("../assets/pack.ron"),
    &[(
        "data/movesets/mary_o.ron",
        include_str!("../assets/data/movesets/mary_o.ron"),
    )],
);
