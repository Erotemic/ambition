//! The table of `(key, ron_text)` pairs for every published
//! `*_spritesheet.ron`, sorted by key ([`crate::sprite_tree::sheet_manifests`]:
//! `assets/sprites/`, its quality tiers, and one level of subdirectories).
//!
//! Embedded by `build.rs` where there is no source tree (`static_content`,
//! wasm, Android), so those builds do not read the tree at run time; read from
//! the tree in a development build.
//!
//! The table and everything parsed from it (the record index, the file-root
//! registry, the mechanics digest) are one [`SheetGeneration`]. A development
//! build can publish a new one ([`publish_sheet_generation`]); each reader
//! then reads the new table and the records parsed from it together, so no
//! reader can see a record of one generation beside the text of another.

use std::sync::{OnceLock, RwLock};

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_sheet_rons.rs"));

/// The sheet table and the records parsed from it.
pub struct SheetGeneration {
    /// 0 for the table the process read first; each publication adds 1.
    id: u64,
    rons: Vec<(&'static str, &'static str)>,
    /// Keyed by file root ([`crate::index_baked_table`]); its map is the
    /// record index.
    registry: crate::SheetRegistry,
    digest: OnceLock<String>,
}

impl SheetGeneration {
    fn new(id: u64, rons: Vec<(&'static str, &'static str)>) -> Self {
        let registry = crate::SheetRegistry::from_baked_table(&rons);
        Self { id, rons, registry, digest: OnceLock::new() }
    }

    fn of(id: u64, prepared: PreparedSheetTable) -> Self {
        Self { id, rons: prepared.rons, registry: prepared.registry, digest: OnceLock::new() }
    }

    /// Which generation this is: 0 for the table the process read first.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Every sheet manifest, as `(key, ron_text)` sorted by key.
    pub fn rons(&self) -> &[(&'static str, &'static str)] {
        &self.rons
    }

    /// The records, keyed by file root.
    pub fn registry(&self) -> &crate::SheetRegistry {
        &self.registry
    }

    /// The digest of the mechanical projection of every record, in key
    /// order. Computed on the first ask.
    pub fn mechanics_digest(&self) -> &str {
        self.digest.get_or_init(|| crate::character::sheets::mechanics_digest_of(&self.registry.sheets))
    }
}

static FIRST: OnceLock<SheetGeneration> = OnceLock::new();
static LIVE: RwLock<Option<&'static SheetGeneration>> = RwLock::new(None);

/// The sheet generation every reader reads now.
pub fn current() -> &'static SheetGeneration {
    if let Some(live) = *LIVE.read().unwrap_or_else(std::sync::PoisonError::into_inner) {
        return live;
    }
    FIRST.get_or_init(|| SheetGeneration::new(0, first_table()))
}

/// Every published sheet manifest, as `(key, ron_text)` sorted by key.
pub fn sheet_rons() -> &'static [(&'static str, &'static str)] {
    current().rons()
}

fn first_table() -> Vec<(&'static str, &'static str)> {
    #[cfg(embedded_sprites)]
    {
        BAKED_SHEET_RONS.to_vec()
    }
    #[cfg(not(embedded_sprites))]
    {
        read_sheet_table_under(crate::published_tree::owner())
    }
}

/// Why a sheet table was not published: each file that does not parse, with
/// its error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetTableRefused(pub Vec<(String, String)>);

impl std::fmt::Display for SheetTableRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} sheet file(s) do not parse:", self.0.len())?;
        for (file, error) in &self.0 {
            write!(f, "\n  {file}: {error}")?;
        }
        Ok(())
    }
}

/// A sheet table parsed and checked, ready to publish. Parsing the whole
/// table costs about a second in a debug build, so a caller can prepare it
/// off the frame and publish it later.
pub struct PreparedSheetTable {
    rons: Vec<(&'static str, &'static str)>,
    registry: crate::SheetRegistry,
}

/// Parse and check `rons`. A table with a file that does not parse is
/// refused, with each such file named.
pub fn prepare_sheet_table(rons: Vec<(&'static str, &'static str)>) -> Result<PreparedSheetTable, SheetTableRefused> {
    let mut index = crate::index_baked_table(&rons);
    if !index.failed.is_empty() {
        return Err(SheetTableRefused(std::mem::take(&mut index.failed)));
    }
    Ok(PreparedSheetTable { rons, registry: crate::SheetRegistry::from_index(index) })
}

/// Make `prepared` the sheet table every reader reads, with the records
/// parsed from it, and return the new generation's id.
///
/// The records feed simulation too (body sizes, hurtboxes, attack volumes,
/// a boss's size), so the caller publishes only where no session is live.
/// Each generation is kept for the life of the process: a reader can hold a
/// `&'static` record of an older one.
pub fn publish_prepared_sheet_table(prepared: PreparedSheetTable) -> u64 {
    let mut live = LIVE.write().unwrap_or_else(std::sync::PoisonError::into_inner);
    let previous = live.map(SheetGeneration::id).unwrap_or_else(|| current_first_id());
    let generation: &'static SheetGeneration = Box::leak(Box::new(SheetGeneration::of(previous + 1, prepared)));
    *live = Some(generation);
    generation.id
}

/// Prepare `rons` and publish it ([`prepare_sheet_table`],
/// [`publish_prepared_sheet_table`]). A refused table leaves the readers on
/// the generation they had.
pub fn publish_sheet_generation(rons: Vec<(&'static str, &'static str)>) -> Result<u64, SheetTableRefused> {
    prepare_sheet_table(rons).map(publish_prepared_sheet_table)
}

fn current_first_id() -> u64 {
    FIRST.get_or_init(|| SheetGeneration::new(0, first_table())).id
}

/// The folder whose `assets/` holds the published sprite tree, in a build
/// that reads it from disk; `None` in a build that embeds its sprites, whose
/// table cannot change.
pub fn published_tree_owner() -> Option<&'static std::path::Path> {
    #[cfg(embedded_sprites)]
    {
        None
    }
    #[cfg(not(embedded_sprites))]
    {
        Some(crate::published_tree::owner())
    }
}

/// Each sheet manifest file of the tree under `owner`, keyed as the table
/// keys it, sorted by key.
pub fn sheet_files_under(owner: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    crate::sprite_tree::sheet_manifests(owner)
}

/// The sheet table of the tree under `owner`, read now. Empty in a build that
/// embeds its sprites.
pub fn read_sheet_table_under(owner: &std::path::Path) -> Vec<(&'static str, &'static str)> {
    #[cfg(embedded_sprites)]
    {
        let _ = owner;
        Vec::new()
    }
    #[cfg(not(embedded_sprites))]
    {
        crate::published_tree::texts(sheet_files_under(owner))
    }
}
