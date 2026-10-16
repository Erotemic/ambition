//! The table of `(key, ron_text)` pairs for every published
//! `*_spritesheet.ron`, sorted by key ([`crate::sprite_tree::sheet_manifests`]:
//! `assets/sprites/`, its quality tiers, and one level of subdirectories).
//!
//! Embedded by `build.rs` where there is no source tree (`static_content`,
//! wasm, Android), so those builds do not read the tree at run time; read from
//! the tree in a development build.
//!
//! The table, everything parsed from it (the record index, the file-root
//! registry, the mechanics digest) and the body rigs published beside the
//! sheets ([`crate::baked_body_rigs`]) are one [`SheetGeneration`]. A
//! development build can publish a new one ([`publish_sheet_generation`]);
//! each reader then reads the new table and the records parsed from it
//! together, so no reader can see a record of one generation beside the text
//! of another.

use std::sync::{OnceLock, RwLock};

#[cfg(embedded_sprites)]
include!(concat!(env!("OUT_DIR"), "/baked_sheet_rons.rs"));

/// The sheet table and the records parsed from it.
pub struct SheetGeneration {
    /// 0 for the table the process read first; each publication adds 1.
    id: u64,
    rons: Vec<(&'static str, &'static str)>,
    /// `(target, ron_text)` sorted by target.
    body_rigs: Vec<(&'static str, &'static str)>,
    part_tables: crate::baked_part_flipbooks::PartTables,
    /// Keyed by file root ([`crate::index_baked_table`]); its map is the
    /// record index.
    registry: crate::SheetRegistry,
    digest: OnceLock<String>,
}

impl SheetGeneration {
    /// The first table is not checked: a stale or broken publish is named
    /// where a reader parses it, as before generations existed.
    fn new(id: u64, texts: SheetTexts) -> Self {
        let registry = crate::SheetRegistry::from_baked_table(&texts.sheets);
        let part_tables = crate::baked_part_flipbooks::PartTables::new(texts.part_flipbooks);
        Self { id, rons: texts.sheets, body_rigs: texts.body_rigs, part_tables, registry, digest: OnceLock::new() }
    }

    fn of(id: u64, prepared: PreparedSheetTable) -> Self {
        Self {
            id,
            rons: prepared.texts.sheets,
            body_rigs: prepared.texts.body_rigs,
            part_tables: prepared.part_tables,
            registry: prepared.registry,
            digest: OnceLock::new(),
        }
    }

    /// Which generation this is: 0 for the table the process read first.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Every sheet manifest, as `(key, ron_text)` sorted by key.
    pub fn rons(&self) -> &[(&'static str, &'static str)] {
        &self.rons
    }

    /// Every body rig, as `(target, ron_text)` sorted by target.
    pub fn body_rigs(&self) -> &[(&'static str, &'static str)] {
        &self.body_rigs
    }

    /// The part flipbooks and what is derived from them.
    pub fn part_tables(&self) -> &crate::baked_part_flipbooks::PartTables {
        &self.part_tables
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

fn first_table() -> SheetTexts {
    #[cfg(embedded_sprites)]
    {
        SheetTexts {
            sheets: BAKED_SHEET_RONS.to_vec(),
            body_rigs: crate::baked_body_rigs::BAKED_BODY_RIGS.to_vec(),
            part_flipbooks: crate::baked_part_flipbooks::first_rows(),
        }
    }
    #[cfg(not(embedded_sprites))]
    {
        read_sheet_table_under(crate::published_tree::owner())
    }
}

/// The texts of one sheet generation.
#[derive(Clone, Debug, Default)]
pub struct SheetTexts {
    /// Every sheet manifest, as `(key, ron_text)` sorted by key.
    pub sheets: Vec<(&'static str, &'static str)>,
    /// Every body rig, as `(target, ron_text)` sorted by target.
    pub body_rigs: Vec<(&'static str, &'static str)>,
    /// Every part flipbook, sorted by key. Named, not read: a flipbook is
    /// read where it is decoded.
    pub part_flipbooks: Vec<crate::baked_part_flipbooks::PartFlipbookRow>,
}

impl SheetTexts {
    /// The texts of the current generation, to edit.
    pub fn current() -> Self {
        let generation = current();
        Self {
            sheets: generation.rons().to_vec(),
            body_rigs: generation.body_rigs().to_vec(),
            part_flipbooks: generation.part_tables().rows().to_vec(),
        }
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
    texts: SheetTexts,
    registry: crate::SheetRegistry,
    part_tables: crate::baked_part_flipbooks::PartTables,
}

/// Parse and check `texts`. A table with a sheet that does not parse, a body
/// rig that does not parse or prepare, or a full-resolution part flipbook
/// that does not decode, is refused, with each such file named: a reader of a
/// rig or a flipbook panics on one it cannot read. Each flipbook is decoded
/// and the landmarks are projected here, off the frame; a flipbook whose file
/// did not change is taken from the current generation.
pub fn prepare_sheet_table(texts: SheetTexts) -> Result<PreparedSheetTable, SheetTableRefused> {
    let mut index = crate::index_baked_table(&texts.sheets);
    let mut failed = std::mem::take(&mut index.failed);
    for (target, text) in &texts.body_rigs {
        let checked = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
            .and_then(|rig| rig.prepare().map(drop).map_err(|error| format!("{error:?}")));
        if let Err(error) = checked {
            failed.push((format!("{target}_body_rig.ron"), error));
        }
    }
    if !failed.is_empty() {
        return Err(SheetTableRefused(failed));
    }
    let part_tables = crate::baked_part_flipbooks::PartTables::prepared(texts.part_flipbooks.clone(), current().part_tables())
        .map_err(SheetTableRefused)?;
    Ok(PreparedSheetTable { texts, registry: crate::SheetRegistry::from_index(index), part_tables })
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
    // The old generation is kept, but not its decoded flipbooks: a reader
    // that still holds it decodes again, and each publication would else keep
    // every decoded table of the generations before it.
    let old = live.unwrap_or_else(|| FIRST.get_or_init(|| SheetGeneration::new(0, first_table())));
    old.part_tables().forget_decoded();
    *live = Some(generation);
    generation.id
}

/// Prepare `rons` and publish it ([`prepare_sheet_table`],
/// [`publish_prepared_sheet_table`]). A refused table leaves the readers on
/// the generation they had.
pub fn publish_sheet_generation(texts: SheetTexts) -> Result<u64, SheetTableRefused> {
    prepare_sheet_table(texts).map(publish_prepared_sheet_table)
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

/// Each body rig file of the tree under `owner`, keyed by its target, sorted
/// by target.
pub fn body_rig_files_under(owner: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    crate::sprite_tree::companions(owner, BODY_RIG_SUFFIX, false)
}

/// The suffix of a body rig file after its target.
pub const BODY_RIG_SUFFIX: &str = "_body_rig.ron";

/// The texts of the tree under `owner`, read now. Empty in a build that
/// embeds its sprites.
pub fn read_sheet_table_under(owner: &std::path::Path) -> SheetTexts {
    #[cfg(embedded_sprites)]
    {
        let _ = owner;
        SheetTexts::default()
    }
    #[cfg(not(embedded_sprites))]
    {
        SheetTexts {
            sheets: crate::published_tree::texts(sheet_files_under(owner)),
            body_rigs: crate::published_tree::texts(body_rig_files_under(owner)),
            part_flipbooks: crate::baked_part_flipbooks::rows_under(owner),
        }
    }
}
