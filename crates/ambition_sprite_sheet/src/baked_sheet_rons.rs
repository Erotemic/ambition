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
    /// `(asset path, ron_text)` sorted by path.
    portrait_rons: Vec<(&'static str, &'static str)>,
    portraits: OnceLock<crate::portrait::PortraitSheetRegistry>,
    /// `(tier, json_text)` sorted by tier.
    pack_catalog_texts: Vec<(&'static str, &'static str)>,
    pack_catalogs: OnceLock<std::collections::HashMap<&'static str, crate::SpritePackCatalog>>,
    fx_sheets: OnceLock<Vec<crate::fx::FxSheet>>,
    authored_effects: OnceLock<std::collections::BTreeMap<&'static str, crate::fx::AuthoredEffect>>,
    /// Keyed by file root ([`crate::index_baked_table`]); its map is the
    /// record index.
    registry: crate::SheetRegistry,
    /// The records of each sheet file, by file root: the next preparation
    /// takes a file whose text did not change from here.
    parsed_files: std::collections::HashMap<String, Vec<crate::SheetRecord>>,
    digest: OnceLock<String>,
    /// Whether each body rig of this generation was parsed and prepared. The
    /// first table is not checked, so a later preparation checks again a rig
    /// it holds unchanged.
    rigs_checked: bool,
}

impl SheetGeneration {
    /// The first table is not checked: a stale or broken publish is named
    /// where a reader parses it, as before generations existed.
    fn new(id: u64, texts: SheetTexts) -> Self {
        let (parsed_files, failed, _) = crate::parse_sheet_files(&texts.sheets, None);
        let registry = crate::SheetRegistry::from_index(crate::index_parsed(&texts.sheets, &parsed_files, failed));
        let part_tables = crate::baked_part_flipbooks::PartTables::new(texts.part_flipbooks);
        Self {
            id,
            rons: texts.sheets,
            body_rigs: texts.body_rigs,
            part_tables,
            portrait_rons: texts.portraits,
            portraits: OnceLock::new(),
            pack_catalog_texts: texts.pack_catalogs,
            pack_catalogs: OnceLock::new(),
            fx_sheets: OnceLock::new(),
            authored_effects: OnceLock::new(),
            registry,
            parsed_files,
            digest: OnceLock::new(),
            rigs_checked: false,
        }
    }

    fn of(id: u64, prepared: PreparedSheetTable) -> Self {
        Self {
            id,
            rons: prepared.texts.sheets,
            body_rigs: prepared.texts.body_rigs,
            part_tables: prepared.part_tables,
            portrait_rons: prepared.texts.portraits,
            portraits: OnceLock::from(prepared.portraits),
            pack_catalog_texts: prepared.texts.pack_catalogs,
            pack_catalogs: OnceLock::from(prepared.pack_catalogs),
            fx_sheets: OnceLock::new(),
            authored_effects: OnceLock::new(),
            registry: prepared.registry,
            parsed_files: prepared.parsed_files,
            digest: OnceLock::new(),
            rigs_checked: true,
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

    /// Every portrait manifest, as `(asset path, ron_text)` sorted by path.
    pub fn portrait_rons(&self) -> &[(&'static str, &'static str)] {
        &self.portrait_rons
    }

    /// The portrait manifests, parsed on the first ask.
    pub fn portraits(&self) -> &crate::portrait::PortraitSheetRegistry {
        self.portraits.get_or_init(|| crate::portrait::PortraitSheetRegistry::from_baked_table(&self.portrait_rons))
    }

    /// Each sprite pack tier's catalog, as `(tier, json_text)` sorted by tier.
    pub fn pack_catalog_texts(&self) -> &[(&'static str, &'static str)] {
        &self.pack_catalog_texts
    }

    /// The sprite pack catalogs, keyed by tier, parsed on the first ask. A
    /// tier that does not parse or validate is disabled.
    pub fn pack_catalogs(&self) -> &std::collections::HashMap<&'static str, crate::SpritePackCatalog> {
        self.pack_catalogs.get_or_init(|| crate::sprite_packs::catalogs_of(&self.pack_catalog_texts))
    }

    /// The effect sheets of this generation's records, built on the first ask.
    pub fn fx_sheets(&'static self) -> &'static [crate::fx::FxSheet] {
        self.fx_sheets.get_or_init(|| crate::fx::fx_sheets_of(&self.registry.sheets))
    }

    /// The authored effects of this generation's records, built on the first
    /// ask.
    pub fn authored_effects(&'static self) -> &'static std::collections::BTreeMap<&'static str, crate::fx::AuthoredEffect> {
        self.authored_effects.get_or_init(|| crate::fx::authored_effects_of(self.fx_sheets(), &self.registry.sheets))
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
            part_flipbooks: crate::baked_part_flipbooks::BAKED_PART_FLIPBOOKS.to_vec(),
            portraits: crate::baked_portrait_rons::BAKED_PORTRAIT_RONS.to_vec(),
            pack_catalogs: crate::sprite_packs::baked::BAKED_PACK_CATALOGS.to_vec(),
        }
    }
    #[cfg(not(embedded_sprites))]
    {
        read_texts_under(crate::published_tree::owner(), None)
    }
}

/// The folder of the sprite pack catalogs of the tree under `owner`, one
/// folder for each tier.
pub fn pack_catalog_dir_under(owner: &std::path::Path) -> std::path::PathBuf {
    owner.join("assets/sprite_packs")
}

/// Each sprite pack catalog of the tree under `owner`, keyed by its tier.
pub fn pack_catalog_files_under(owner: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    crate::sprite_tree::pack_catalogs(owner)
}

/// Each portrait manifest of the tree under `owner`, keyed by its path under
/// `assets/`.
pub fn portrait_files_under(owner: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    crate::sprite_tree::portrait_manifests(owner)
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
    /// Every portrait manifest, as `(asset path, ron_text)` sorted by path.
    pub portraits: Vec<(&'static str, &'static str)>,
    /// Each sprite pack tier's catalog, as `(tier, json_text)` sorted by tier.
    pub pack_catalogs: Vec<(&'static str, &'static str)>,
}

impl SheetTexts {
    /// The texts of the current generation, to edit.
    pub fn current() -> Self {
        let generation = current();
        Self {
            sheets: generation.rons().to_vec(),
            body_rigs: generation.body_rigs().to_vec(),
            part_flipbooks: generation.part_tables().rows().to_vec(),
            portraits: generation.portrait_rons().to_vec(),
            pack_catalogs: generation.pack_catalog_texts().to_vec(),
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
    parsed_files: std::collections::HashMap<String, Vec<crate::SheetRecord>>,
    parsed_now: usize,
    part_tables: crate::baked_part_flipbooks::PartTables,
    portraits: crate::portrait::PortraitSheetRegistry,
    pack_catalogs: std::collections::HashMap<&'static str, crate::SpritePackCatalog>,
}

impl PreparedSheetTable {
    /// The sheet files this preparation parsed: each file whose text the
    /// current generation does not hold for its root.
    pub fn parsed_now(&self) -> usize {
        self.parsed_now
    }

    /// Whether this table changes a fact that simulation or a cast reads
    /// from `live`: the mechanical projection of a record (body boxes,
    /// hurtboxes, attack volumes, frame timing, tuning; see
    /// [`crate::sheet_mechanics`]), the landmarks, a body rig, or a portrait
    /// manifest (a cast is prepared against the portrait names). A table that
    /// changes none of them changes only where frames are drawn from (pages,
    /// rects, pixels) and the pack catalogs.
    pub fn changes_mechanics_of(&self, live: &SheetGeneration) -> bool {
        self.texts.body_rigs != live.body_rigs
            || self.texts.portraits != live.portrait_rons
            || self.part_tables.landmarks().1 != live.part_tables().landmarks().1
            || crate::character::sheets::mechanics_digest_of(&self.registry.sheets) != live.mechanics_digest()
    }
}

/// Parse and check `texts`. A table with a sheet that does not parse, a body
/// rig that does not parse or prepare, or a full-resolution part flipbook
/// that does not decode, is refused, with each such file named: a reader of a
/// rig or a flipbook panics on one it cannot read. Each flipbook is decoded
/// and the landmarks are projected here, off the frame; a flipbook whose file
/// did not change is taken from the current generation.
pub fn prepare_sheet_table(texts: SheetTexts) -> Result<PreparedSheetTable, SheetTableRefused> {
    let previous = current();
    let (parsed_files, mut failed, parsed_now) = crate::parse_sheet_files(&texts.sheets, Some((previous.rons(), &previous.parsed_files)));
    for (target, text) in &texts.body_rigs {
        if !rig_needs_check(previous.body_rigs(), previous.rigs_checked, target, text) {
            continue;
        }
        let checked = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
            .and_then(|rig| rig.prepare().map(drop).map_err(|error| format!("{error:?}")));
        if let Err(error) = checked {
            failed.push((format!("{target}_body_rig.ron"), error));
        }
    }
    let portraits = crate::portrait::PortraitSheetRegistry::checked(&texts.portraits).map_err(|mut refused| failed.append(&mut refused));
    let pack_catalogs = crate::sprite_packs::checked_catalogs(&texts.pack_catalogs).map_err(|mut refused| failed.append(&mut refused));
    let (Ok(portraits), Ok(pack_catalogs), true) = (portraits, pack_catalogs, failed.is_empty()) else {
        return Err(SheetTableRefused(failed));
    };
    let part_tables = crate::baked_part_flipbooks::PartTables::prepared(texts.part_flipbooks.clone(), current().part_tables())
        .map_err(SheetTableRefused)?;
    let registry = crate::SheetRegistry::from_index(crate::index_parsed(&texts.sheets, &parsed_files, Vec::new()));
    Ok(PreparedSheetTable { texts, registry, parsed_files, parsed_now, part_tables, portraits, pack_catalogs })
}

/// Whether a preparation must parse and prepare the rig `text` of `target`.
/// Not when the current generation holds the same text AND checked it: the
/// first generation was not checked, so the same text there is no proof
/// (found in review, 2026-10-17).
fn rig_needs_check(current: &[(&str, &str)], current_checked: bool, target: &str, text: &str) -> bool {
    let unchanged = current.binary_search_by(|(name, _)| (*name).cmp(target)).is_ok_and(|at| current[at].1 == text);
    !(unchanged && current_checked)
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

/// The folders the sprite tree under `owner` is in: `assets/sprites` and
/// each quality tier's.
pub fn sprite_dirs_under(owner: &std::path::Path) -> Vec<std::path::PathBuf> {
    crate::sprite_tree::sprite_dirs(owner)
}

/// Each body rig file of the tree under `owner`, keyed by its target, sorted
/// by target.
pub fn body_rig_files_under(owner: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    crate::sprite_tree::companions(owner, BODY_RIG_SUFFIX, false)
}

/// The suffix of a body rig file after its target.
pub const BODY_RIG_SUFFIX: &str = "_body_rig.ron";

/// The texts of the tree under `owner`, read now. Empty in a build that
/// embeds its sprites. A text the current generation holds unchanged is that
/// generation's: each text is kept for the life of the process, so a read
/// keeps only what changed.
pub fn read_sheet_table_under(owner: &std::path::Path) -> SheetTexts {
    read_texts_under(owner, Some(current()))
}

/// The texts of the tree under `owner`, taking each unchanged text from
/// `previous`. The first generation has none to take from.
#[cfg_attr(embedded_sprites, allow(unused_variables))]
fn read_texts_under(owner: &std::path::Path, previous: Option<&SheetGeneration>) -> SheetTexts {
    #[cfg(embedded_sprites)]
    {
        SheetTexts::default()
    }
    #[cfg(not(embedded_sprites))]
    {
        use crate::published_tree::texts_reusing;
        let was = |of: fn(&SheetGeneration) -> &[(&'static str, &'static str)]| previous.map(of).unwrap_or_default();
        SheetTexts {
            sheets: texts_reusing(sheet_files_under(owner), was(SheetGeneration::rons)),
            body_rigs: texts_reusing(body_rig_files_under(owner), was(SheetGeneration::body_rigs)),
            part_flipbooks: crate::baked_part_flipbooks::rows_under(owner),
            portraits: texts_reusing(portrait_files_under(owner), was(SheetGeneration::portrait_rons)),
            pack_catalogs: texts_reusing(pack_catalog_files_under(owner), was(SheetGeneration::pack_catalog_texts)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rig_needs_check;

    /// A rig is taken without a check only from a generation that checked it:
    /// an unchanged rig of the unchecked first table is checked again. The
    /// control: an unchanged rig of a checked generation is not.
    #[test]
    fn an_unchanged_rig_is_taken_only_from_a_checked_generation() {
        let current = [("alice", "rig a"), ("bob", "rig b")];
        assert!(!rig_needs_check(&current, true, "bob", "rig b"), "control: an unchanged rig of a checked generation");
        assert!(rig_needs_check(&current, false, "bob", "rig b"), "an unchanged rig of the unchecked first table was not checked");
        assert!(rig_needs_check(&current, true, "bob", "rig b2"), "a changed rig");
        assert!(rig_needs_check(&current, true, "carol", "rig c"), "a new rig");
    }
}
