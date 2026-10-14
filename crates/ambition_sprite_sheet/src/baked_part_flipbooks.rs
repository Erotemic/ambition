//! The table of every published part-flipbook draw table, sorted by key:
//! `<target>` for the full-resolution table, `<target>.<tier>` for a quality
//! tier's, from the `assets/sprites*` trees. The table is empty on a checkout
//! that never published a flipbook.
//!
//! Where there is no source tree (`static_content`, wasm, Android), `build.rs`
//! parses each `<target>_parts.ron` and embeds it as bincode of the same
//! schema (`character::rigged::published`), so the game decodes a table and
//! does not parse its text.
//!
//! A development build embeds nothing, so that a sprite publish compiles
//! nothing. It names each published file, and reads a table when a sheet first
//! realizes it: as bincode from its cache (`published_cache`) when a run
//! before this one parsed the same file, and from the RON text when not. The
//! parse is about 50 ms a table, against 3.5 ms to decode (measured
//! 2026-10-11), and it is on the main thread; with no cache the hub's
//! neighbour prefetch stalled for 1.1 s on the cast of the hall (2026-10-14).

use crate::character::rigged::{RiggedSpriteAsset, RiggedSpriteError};

/// One embedded table, as `build.rs` could read it.
#[derive(Debug, Clone, Copy)]
pub enum BakedPartFlipbook {
    /// Read at build time and embedded as bincode.
    Encoded(&'static [u8]),
    /// The published RON text: a table `build.rs` could not read (the game
    /// then refuses it with the error the RON gives).
    Ron(&'static str),
    /// The published RON file, by its path: a development build's table. It
    /// is read when it is decoded.
    File(&'static str),
}

impl BakedPartFlipbook {
    /// The flipbook, checked.
    pub fn decode(self) -> Result<RiggedSpriteAsset, RiggedSpriteError> {
        match self {
            Self::Encoded(bytes) => RiggedSpriteAsset::from_published_bytes(bytes),
            Self::Ron(text) => RiggedSpriteAsset::from_published_ron(text),
            Self::File(path) => decode_published_file(path),
        }
    }
}

/// The flipbook of the published file `path`, through the development cache.
#[cfg(any(not(embedded_sprites), test))]
fn decode_published_file(path: &str) -> Result<RiggedSpriteAsset, RiggedSpriteError> {
    let file = std::path::Path::new(path);
    let cached = crate::published_cache::derived(
        "part-table",
        &[PUBLISHED_SCHEMA_SOURCE],
        &[file],
        |bytes| RiggedSpriteAsset::from_published_bytes(bytes).ok(),
        || {
            let text = std::fs::read_to_string(file).ok()?;
            let published = crate::sprite_tree::read_part_flipbook(&text).ok()?;
            bincode::serialize(&published).ok()
        },
    );
    match cached {
        Some(asset) => Ok(asset),
        // The file does not read or the table is refused: the RON road gives
        // the error that says why.
        None => {
            let text = std::fs::read_to_string(file).map_err(|error| RiggedSpriteError::Parse(format!("{path}: {error}")))?;
            RiggedSpriteAsset::from_published_ron(&text)
        }
    }
}

/// An embedded build has no file to read: `build.rs` writes no such row.
#[cfg(not(any(not(embedded_sprites), test)))]
fn decode_published_file(path: &str) -> Result<RiggedSpriteAsset, RiggedSpriteError> {
    Err(RiggedSpriteError::Parse(format!("{path}: an embedded build reads no published file")))
}

/// The source of the structures a table's bincode is written with: a part of
/// the cache key, so that a build with another structure reads no old bytes.
#[cfg(any(not(embedded_sprites), test))]
pub(crate) const PUBLISHED_SCHEMA_SOURCE: &str = include_str!("character/rigged/published.rs");

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_part_flipbooks.rs"));

/// Every published flipbook as `(key, table, file)`, sorted by key. `file` is
/// the published file on the host that built (or runs) the game.
pub fn part_flipbooks() -> &'static [(&'static str, BakedPartFlipbook, &'static str)] {
    #[cfg(embedded_sprites)]
    {
        BAKED_PART_FLIPBOOKS
    }
    #[cfg(not(embedded_sprites))]
    {
        use crate::published_tree;
        type Row = (&'static str, BakedPartFlipbook, &'static str);
        static READ: std::sync::OnceLock<Vec<Row>> = std::sync::OnceLock::new();
        // The files are named and not read: a table is read when a sheet
        // realizes it (`BakedPartFlipbook::File`).
        READ.get_or_init(|| {
            crate::sprite_tree::companions(published_tree::owner(), "_parts.ron", true)
                .into_iter()
                .map(|(key, path)| {
                    let file = published_tree::kept(path.display().to_string());
                    (published_tree::kept(key), BakedPartFlipbook::File(file), file)
                })
                .collect()
        })
    }
}

/// The published part flipbook under `key` (`<target>` or `<target>.<tier>`).
pub fn baked_part_flipbook(key: &str) -> Option<BakedPartFlipbook> {
    let tables = part_flipbooks();
    tables.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok().map(|index| tables[index].1)
}

/// The RON text a table was built from, read from its file on the BUILDING
/// host: for tests that publish a variant of a shipped table. `None` when the
/// key is unpublished or the file is gone.
#[doc(hidden)]
pub fn published_ron_on_build_host(key: &str) -> Option<String> {
    let tables = part_flipbooks();
    let index = tables.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok()?;
    std::fs::read_to_string(tables[index].2).ok()
}

/// Every target that publishes a full-resolution part flipbook.
pub fn baked_part_flipbook_targets() -> impl Iterator<Item = &'static str> {
    part_flipbooks()
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| !name.contains('.'))
}
