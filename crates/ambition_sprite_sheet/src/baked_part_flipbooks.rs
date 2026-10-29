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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakedPartFlipbook {
    /// Read at build time and embedded as bincode.
    Encoded(&'static [u8]),
    /// The published RON text: a table `build.rs` could not read (the game
    /// then refuses it with the error the RON gives).
    Ron(std::sync::Arc<str>),
    /// The published RON file, by its path: a development build's table. It
    /// is read when it is decoded.
    File(&'static str),
}

impl BakedPartFlipbook {
    /// The flipbook, checked.
    pub fn decode(&self) -> Result<RiggedSpriteAsset, RiggedSpriteError> {
        match self {
            Self::Encoded(bytes) => RiggedSpriteAsset::from_published_bytes(bytes),
            Self::Ron(text) => RiggedSpriteAsset::from_published_ron(text),
            Self::File(path) => decode_published_file(path),
        }
    }
}

/// The flipbook of the published file `path`, through the development cache.
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


/// The source of the structures a table's bincode is written with: a part of
/// the cache key, so that a build with another structure reads no old bytes.
pub(crate) const PUBLISHED_SCHEMA_SOURCE: &str = include_str!("character/rigged/published.rs");



/// One published flipbook: `(key, table, file)`.
pub type PartFlipbookRow = (&'static str, BakedPartFlipbook, &'static str);

/// Every published flipbook as `(key, table, file)`, sorted by key. `file` is
/// the published file on the host that built (or runs) the game. A part of
/// the current sheet generation ([`crate::baked_sheet_rons::current`]).
pub fn part_flipbooks(content: &crate::SpriteContent) -> &[PartFlipbookRow] {
    crate::baked_sheet_rons::current(&content).part_tables().rows()
}

/// The suffix of a part flipbook file after its key.
pub const PART_FLIPBOOK_SUFFIX: &str = "_parts.ron";

/// The modification time of a development build's flipbook file; `None` for
/// an embedded table, which cannot change.
fn modified(table: &BakedPartFlipbook) -> Option<std::time::SystemTime> {
    match table {
        BakedPartFlipbook::File(path) => std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok(),
        BakedPartFlipbook::Encoded(_) | BakedPartFlipbook::Ron(_) => None,
    }
}

/// A landmark projection: `(target, bincode)` sorted by target, and its
/// digest.
pub(crate) type Landmarks = (Vec<(&'static str, Vec<u8>)>, String);

/// The part flipbooks of one sheet generation, and what is derived from
/// them: each decoded full-resolution table and the landmark tables.
pub struct PartTables {
    rows: Vec<PartFlipbookRow>,
    /// Each decoded full-resolution table, with the modification time its
    /// file had before it was read.
    parsed: std::sync::Mutex<std::collections::HashMap<String, (RiggedSpriteAsset, Option<std::time::SystemTime>)>>,
    landmarks: std::sync::OnceLock<Landmarks>,
    /// The modification time of each quality tier's table file when this
    /// generation read its rows. A sheet decodes a tier table where it
    /// realizes at that tier, and panics on one that does not decode, so the
    /// next preparation checks each tier table whose file changed since.
    tiers_seen: std::collections::HashMap<&'static str, Option<std::time::SystemTime>>,
    decoded_landmarks: std::sync::OnceLock<Vec<std::sync::OnceLock<std::sync::Arc<ambition_characters::actor::landmarks::BodyLandmarkTable>>>>,
}

impl PartTables {
    /// Tables decoded when first asked for.
    pub(crate) fn new(rows: Vec<PartFlipbookRow>) -> Self {
        let tiers_seen = rows.iter().filter(|(key, ..)| crate::quality_variant_marker(key).is_some()).map(|(key, table, _)| (*key, modified(table))).collect();
        Self { rows, parsed: Default::default(), landmarks: Default::default(), tiers_seen, decoded_landmarks: Default::default() }
    }

    /// Every flipbook, as `(key, table, file)` sorted by key.
    pub fn rows(&self) -> &[PartFlipbookRow] {
        &self.rows
    }

    /// Decode each full-resolution table of `rows`, and project the
    /// landmarks, so that a publication is checked before it is played and
    /// costs the frame nothing. A table whose file did not change since
    /// `previous` decoded it is taken from `previous`. Each tier table whose
    /// file changed since `previous` read it is decoded to check it, and not
    /// kept: a sheet decodes it where it realizes at that tier. A table that
    /// does not decode is returned with its file named and its error.
    pub(crate) fn prepared(rows: Vec<PartFlipbookRow>, previous: &Self) -> Result<Self, Vec<(String, String)>> {
        let tables = Self::new(rows);
        let old = previous.parsed.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut parsed = std::collections::HashMap::new();
        let mut failed = Vec::new();
        let mut all_kept = true;
        for (key, table, file) in tables.rows.iter().filter(|(key, ..)| crate::quality_variant_marker(key).is_none()) {
            let seen = modified(table);
            match old.get(*key) {
                Some((asset, was)) if seen.is_some() && *was == seen
                    && previous.rows.iter().any(|(old_key, old_table, _)| old_key == key && old_table == table) => {
                    parsed.insert((*key).to_owned(), (asset.clone(), seen));
                }
                _ => {
                    all_kept = false;
                    match table.decode() {
                        Ok(asset) => {
                            parsed.insert((*key).to_owned(), (asset, seen));
                        }
                        Err(error) => failed.push((file.rsplit('/').next().unwrap_or(file).to_owned(), error.to_string())),
                    }
                }
            }
        }
        drop(old);
        for (key, table, file) in tables.rows.iter().filter(|(key, ..)| crate::quality_variant_marker(key).is_some()) {
            let seen = tables.tiers_seen.get(key).copied().flatten();
            if seen.is_some() && previous.tiers_seen.get(key) == Some(&seen)
                && previous.rows.iter().any(|(old_key, old_table, _)| old_key == key && old_table == table) {
                continue;
            }
            if let Err(error) = table.decode() {
                // By its path under `assets/`: a tier table has the file name
                // of its full-resolution table.
                failed.push((file.split_once("/assets/").map_or(*file, |(_, under)| under).to_owned(), error.to_string()));
            }
        }
        if !failed.is_empty() {
            return Err(failed);
        }
        let same_files = all_kept && previous.rows.len() == tables.rows.len();
        let landmarks = match previous.landmarks.get() {
            Some(landmarks) if same_files => landmarks.clone(),
            _ => crate::baked_landmarks::project_rows(&tables.rows),
        };
        let _ = tables.landmarks.set(landmarks);
        *tables.parsed.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = parsed;
        Ok(tables)
    }

    /// The decoded full-resolution table of `target`, decoded now when it is
    /// not yet.
    pub(crate) fn decoded(&self, target: &str) -> Option<Result<RiggedSpriteAsset, RiggedSpriteError>> {
        let index = self.rows.binary_search_by(|(name, _, _)| (*name).cmp(target)).ok()?;
        let table = &self.rows[index].1;
        if let Some((asset, _)) = self.parsed.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(target) {
            return Some(Ok(asset.clone()));
        }
        // Decoded with the lock open: the warm threads and the game do not
        // wait for each other. Two decodes of one table give one table.
        let seen = modified(table);
        let asset = match table.decode() {
            Ok(asset) => asset,
            Err(error) => return Some(Err(error)),
        };
        self.parsed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(target.to_owned(), (asset.clone(), seen));
        Some(Ok(asset))
    }

    /// Drop each decoded table: this generation is no longer the current one.
    pub(crate) fn forget_decoded(&self) {
        self.parsed.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear();
    }

    /// The landmark tables of this generation, projected when first asked
    /// for.
    pub(crate) fn landmarks(&self) -> &Landmarks {
        self.landmarks.get_or_init(|| crate::baked_landmarks::project_rows(&self.rows))
    }

    /// One decode slot for each landmark table of [`Self::landmarks`].
    pub(crate) fn decoded_landmarks(&self) -> &[std::sync::OnceLock<std::sync::Arc<ambition_characters::actor::landmarks::BodyLandmarkTable>>] {
        self.decoded_landmarks.get_or_init(|| self.landmarks().0.iter().map(|_| std::sync::OnceLock::new()).collect())
    }
}

/// The published part flipbook under `key` (`<target>` or `<target>.<tier>`).
pub fn baked_part_flipbook(content: &crate::SpriteContent, key: &str) -> Option<BakedPartFlipbook> {
    let key = crate::shared_baked_sheet_registry(content).canonical_key(key).unwrap_or(key);
    let tables = part_flipbooks(&content);
    tables.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok().map(|index| tables[index].1.clone())
}

/// The RON text a table was built from, read from its file on the BUILDING
/// host: for tests that publish a variant of a shipped table. `None` when the
/// key is unpublished or the file is gone.
#[doc(hidden)]
pub fn published_ron_on_build_host(content: &crate::SpriteContent, key: &str) -> Option<String> {
    let key = crate::shared_baked_sheet_registry(content).canonical_key(key).unwrap_or(key);
    let tables = part_flipbooks(&content);
    let index = tables.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok()?;
    std::fs::read_to_string(tables[index].2).ok()
}

/// Every target that publishes a full-resolution part flipbook.
pub fn baked_part_flipbook_targets(content: &crate::SpriteContent) -> impl Iterator<Item = &'static str> + '_ {
    part_flipbooks(&content)
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| crate::quality_variant_marker(name).is_none())
}
