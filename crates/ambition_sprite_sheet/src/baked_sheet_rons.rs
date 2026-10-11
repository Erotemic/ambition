//! The table of `(key, ron_text)` pairs for every published
//! `*_spritesheet.ron`, sorted by key ([`crate::sprite_tree::sheet_manifests`]:
//! `assets/sprites/`, its quality tiers, and one level of subdirectories).
//!
//! Embedded by `build.rs` where there is no source tree (`static_content`,
//! wasm, Android), so those builds do not read the tree at run time; read from
//! the tree in a development build.

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_sheet_rons.rs"));

/// Every published sheet manifest, as `(key, ron_text)` sorted by key.
pub fn sheet_rons() -> &'static [(&'static str, &'static str)] {
    #[cfg(embedded_sprites)]
    {
        BAKED_SHEET_RONS
    }
    #[cfg(not(embedded_sprites))]
    {
        use crate::published_tree;
        static READ: std::sync::OnceLock<Vec<(&'static str, &'static str)>> = std::sync::OnceLock::new();
        READ.get_or_init(|| published_tree::texts(crate::sprite_tree::sheet_manifests(published_tree::owner())))
    }
}
