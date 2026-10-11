//! The table of `(asset_relative_manifest_path, ron_text)` pairs for every
//! independently published `*_portraits.ron` under `assets/sprites/`.
//!
//! Embedded by `build.rs` where there is no source tree (`static_content`,
//! wasm, Android); read from the tree in a development build.

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_portrait_rons.rs"));

/// Every published portrait manifest, as `(asset path, ron_text)` sorted by
/// path.
pub fn portrait_rons() -> &'static [(&'static str, &'static str)] {
    #[cfg(embedded_sprites)]
    {
        BAKED_PORTRAIT_RONS
    }
    #[cfg(not(embedded_sprites))]
    {
        use crate::published_tree;
        static READ: std::sync::OnceLock<Vec<(&'static str, &'static str)>> = std::sync::OnceLock::new();
        READ.get_or_init(|| published_tree::texts(crate::sprite_tree::portrait_manifests(published_tree::owner())))
    }
}
