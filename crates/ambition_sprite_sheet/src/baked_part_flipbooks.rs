//! Compile-time table of every published part-flipbook draw table, sorted by
//! key: `<target>` for the full-resolution table, `<target>.<tier>` for a
//! quality tier's. `build.rs` fills it from the `assets/sprites*` trees: each
//! `<target>_parts.ron` is parsed at BUILD time and embedded as bincode of the
//! same schema (`character::rigged::published`), so the game decodes a table
//! instead of parsing its text. The table is empty on a checkout that never
//! published a flipbook.

use crate::character::rigged::{RiggedSpriteAsset, RiggedSpriteError};

/// One embedded table, as `build.rs` could read it.
#[derive(Debug, Clone, Copy)]
pub enum BakedPartFlipbook {
    /// Read at build time and embedded as bincode.
    Encoded(&'static [u8]),
    /// `build.rs` could not read it: the text is embedded instead, so the game
    /// refuses it with the same error it would have given the RON.
    Unread(&'static str),
}

impl BakedPartFlipbook {
    /// The flipbook, checked.
    pub fn decode(self) -> Result<RiggedSpriteAsset, RiggedSpriteError> {
        match self {
            Self::Encoded(bytes) => RiggedSpriteAsset::from_published_bytes(bytes),
            Self::Unread(text) => RiggedSpriteAsset::from_published_ron(text),
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/baked_part_flipbooks.rs"));

/// The published part flipbook under `key` (`<target>` or `<target>.<tier>`).
pub fn baked_part_flipbook(key: &str) -> Option<BakedPartFlipbook> {
    BAKED_PART_FLIPBOOKS
        .binary_search_by(|(name, _, _)| (*name).cmp(key))
        .ok()
        .map(|index| BAKED_PART_FLIPBOOKS[index].1)
}

/// The RON text a table was built from, read from its file on the BUILDING
/// host: for tests that publish a variant of a shipped table. `None` when the
/// key is unpublished or the file is gone.
#[doc(hidden)]
pub fn published_ron_on_build_host(key: &str) -> Option<String> {
    let index = BAKED_PART_FLIPBOOKS.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok()?;
    std::fs::read_to_string(BAKED_PART_FLIPBOOKS[index].2).ok()
}

/// Every target that publishes a full-resolution part flipbook.
pub fn baked_part_flipbook_targets() -> impl Iterator<Item = &'static str> {
    BAKED_PART_FLIPBOOKS
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| !name.contains('.'))
}
