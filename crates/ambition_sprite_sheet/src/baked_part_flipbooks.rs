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
//! Publication prepares and retains every full-resolution and quality table.
//! A realization reads the admitted table without file IO or decoding. Native
//! file preparation uses the development cache (`published_cache`).

use crate::character::rigged::{RiggedSpriteAsset, RiggedSpriteError};

/// One embedded table, as `build.rs` could read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakedPartFlipbook {
    /// Read at build time and embedded as bincode.
    Encoded(std::sync::Arc<[u8]>),
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
            let published = ambition_sprite_part_format::read_ron(&text).ok()?;
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
pub(crate) const PUBLISHED_SCHEMA_SOURCE: &str = ambition_sprite_part_format::SCHEMA_SOURCE;



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
/// them: each decoded table and the landmark tables.
pub struct PartTables {
    rows: Vec<PartFlipbookRow>,
    aliases: std::collections::HashMap<String, String>,
    /// Each decoded table, with the modification time its
    /// file had before it was read.
    parsed: std::collections::HashMap<String, (RiggedSpriteAsset, Option<std::time::SystemTime>)>,
    landmarks: std::sync::OnceLock<Landmarks>,
    decoded_landmarks: std::sync::OnceLock<Vec<std::sync::OnceLock<std::sync::Arc<ambition_characters::actor::landmarks::BodyLandmarkTable>>>>,
}

impl PartTables {
    /// An empty preparation result for these sources.
    pub(crate) fn new(rows: Vec<PartFlipbookRow>) -> Self {
        let aliases = crate::library_aliases(rows.iter().map(|(key, ..)| *key));
        Self { rows, aliases, parsed: Default::default(), landmarks: Default::default(), decoded_landmarks: Default::default() }
    }

    /// Every flipbook, as `(key, table, file)` sorted by key.
    pub fn rows(&self) -> &[PartFlipbookRow] {
        &self.rows
    }

    pub(crate) fn canonical_key<'a>(&'a self, key: &'a str) -> &'a str {
        self.aliases.get(key).map(String::as_str).unwrap_or(key)
    }

    /// Decode and retain every table before publication. Reuse a table only
    /// when its source and file stamp are unchanged. Project landmarks from
    /// the full-resolution sources. Refuse the generation if any table fails.
    pub(crate) fn prepared(rows: Vec<PartFlipbookRow>, previous: &Self) -> Result<Self, Vec<(String, String)>> {
        let mut tables = Self::new(rows);
        let old = &previous.parsed;
        let mut parsed = std::collections::HashMap::new();
        let mut failed = Vec::new();
        let mut all_kept = true;
        for (key, table, file) in &tables.rows {
            let seen = modified(table);
            let same_source = previous.rows.binary_search_by_key(key, |row| row.0).ok()
                .is_some_and(|index| previous.rows[index].1 == *table);
            let stamp_available = !matches!(table, BakedPartFlipbook::File(_)) || seen.is_some();
            match old.get(*key) {
                Some((asset, was)) if stamp_available && *was == seen && same_source => {
                    parsed.insert((*key).to_owned(), (asset.clone(), seen));
                }
                _ => {
                    if crate::quality_variant_marker(key).is_none() { all_kept = false; }
                    match table.decode() {
                        Ok(asset) => {
                            parsed.insert((*key).to_owned(), (asset, seen));
                        }
                        Err(error) => {
                            let source = if crate::quality_variant_marker(key).is_some() {
                                file.split_once("/assets/").map_or(*file, |(_, under)| under)
                            } else {
                                file.rsplit('/').next().unwrap_or(file)
                            };
                            failed.push((source.to_owned(), error.to_string()));
                        }
                    }
                }
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
        tables.parsed = parsed;
        Ok(tables)
    }

    /// The table admitted with this immutable generation.
    pub(crate) fn decoded(&self, target: &str) -> Option<RiggedSpriteAsset> {
        self.parsed.get(self.canonical_key(target)).map(|(asset, _)| asset.clone())
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
    let key = crate::baked_sheet_rons::current(content).part_tables().canonical_key(key);
    let tables = part_flipbooks(&content);
    tables.binary_search_by(|(name, _, _)| (*name).cmp(key)).ok().map(|index| tables[index].1.clone())
}

/// The RON text a table was built from, read from its file on the BUILDING
/// host: for tests that publish a variant of a shipped table. `None` when the
/// key is unpublished or the file is gone.
#[doc(hidden)]
pub fn published_ron_on_build_host(content: &crate::SpriteContent, key: &str) -> Option<String> {
    let key = crate::baked_sheet_rons::current(content).part_tables().canonical_key(key);
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
