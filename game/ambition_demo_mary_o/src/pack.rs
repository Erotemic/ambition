//! The Mary-O demo's content pack.
//!
//! The demo states its cast (`assets/data/character_catalog.ron`) and its
//! fighters' moves (`assets/data/movesets/`) as data, and registers them
//! through [`PACK`].

use ambition_platformer2d::content::EmbeddedPack;

/// `assets/pack.ron` and every source it declares: the manifest is the one
/// list, so a source line added there is played with no Rust build.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! { root: "assets" };
