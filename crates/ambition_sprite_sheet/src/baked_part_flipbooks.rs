//! Compile-time table of every published part-flipbook draw table,
//! `(key, ron_text)` sorted by key: `<target>` for the full-resolution table,
//! `<target>.<tier>` for a quality tier's. `build.rs` fills it with
//! `include_str!` from the full-resolution `assets/sprites` tree, like the body
//! rigs. The table is empty on a checkout that never published a flipbook.

include!(concat!(env!("OUT_DIR"), "/baked_part_flipbooks.rs"));

/// The published part flipbook under `key` (`<target>` or
/// `<target>.<tier>`), as RON text.
pub fn baked_part_flipbook(key: &str) -> Option<&'static str> {
    BAKED_PART_FLIPBOOKS
        .binary_search_by(|(name, _)| (*name).cmp(key))
        .ok()
        .map(|index| BAKED_PART_FLIPBOOKS[index].1)
}

/// Every target that publishes a full-resolution part flipbook.
pub fn baked_part_flipbook_targets() -> impl Iterator<Item = &'static str> {
    BAKED_PART_FLIPBOOKS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !name.contains('.'))
}
