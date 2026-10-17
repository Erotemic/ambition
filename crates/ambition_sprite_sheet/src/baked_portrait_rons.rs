//! The table of `(asset_relative_manifest_path, ron_text)` pairs for every
//! independently published `*_portraits.ron` under `assets/sprites/`.
//!
//! Embedded by `build.rs` where there is no source tree (`static_content`,
//! wasm, Android); read from the tree in a development build.

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_portrait_rons.rs"));

/// Every published portrait manifest, as `(asset path, ron_text)` sorted by
/// path.
/// A part of the current sheet generation
/// ([`crate::baked_sheet_rons::current`]).
pub fn portrait_rons() -> &'static [(&'static str, &'static str)] {
    crate::baked_sheet_rons::current().portrait_rons()
}
