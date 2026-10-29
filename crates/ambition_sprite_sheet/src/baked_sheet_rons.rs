//! Immutable sprite metadata selected by one App.

use std::sync::{Arc, OnceLock};



/// The sheet table and the records parsed from it.
pub struct SheetGeneration {
    /// 0 for the table the process read first; each publication adds 1.
    id: u64,
    rons: Vec<(&'static str, Arc<str>)>,
    /// `(target, ron_text)` sorted by target.
    body_rigs: Vec<(&'static str, Arc<str>)>,
    part_tables: crate::baked_part_flipbooks::PartTables,
    /// `(asset path, ron_text)` sorted by path.
    portrait_rons: Vec<(&'static str, Arc<str>)>,
    portraits: OnceLock<crate::portrait::PortraitSheetRegistry>,
    /// `(tier, json_text)` sorted by tier.
    pack_catalog_texts: Vec<(&'static str, Arc<str>)>,
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
    image_paths: std::collections::BTreeMap<String, String>,
    room_themes: crate::game_assets::RoomThemes,
    warming: std::sync::Once,
    effect_ids: OnceLock<std::collections::HashMap<ambition_vfx::FxId, &'static str>>,
}

impl SheetGeneration {
    /// The first table is not checked: a stale or broken publish is named
    /// where a reader parses it, as before generations existed.
    fn new(id: u64, texts: SheetTexts) -> Self {
        let (parsed_files, failed, _) = crate::parse_sheet_files(&text_pairs(&texts.sheets), None);
        let registry = crate::SheetRegistry::from_index(crate::index_parsed(&text_pairs(&texts.sheets), &parsed_files, failed));
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
            image_paths: texts.image_paths,
            room_themes: texts.room_themes,
            warming: std::sync::Once::new(),
            effect_ids: OnceLock::new(),
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
            image_paths: prepared.texts.image_paths,
            room_themes: prepared.texts.room_themes,
            warming: std::sync::Once::new(),
            effect_ids: OnceLock::new(),
        }
    }

    pub(crate) fn warm_parts(&self, work: impl FnOnce()) { self.warming.call_once(work); }

    pub fn effect_by_id(&self, id: ambition_vfx::FxId) -> Option<&crate::fx::AuthoredEffect> {
        let key = self.effect_ids.get_or_init(|| self.authored_effects().keys()
            .map(|key| (ambition_vfx::FxId::new(key), *key)).collect()).get(&id)?;
        self.authored_effects().get(key)
    }

    pub fn room_themes(&self) -> &crate::game_assets::RoomThemes { &self.room_themes }

    pub fn image_path(&self, logical: &str) -> Option<&str> {
        self.image_paths.get(logical).map(String::as_str)
    }

    /// Which generation this is: 0 for the table the process read first.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Every sheet manifest, as `(key, ron_text)` sorted by key.
    pub fn rons(&self) -> &[(&'static str, Arc<str>)] {
        &self.rons
    }

    /// Every body rig, as `(target, ron_text)` sorted by target.
    pub fn body_rigs(&self) -> &[(&'static str, Arc<str>)] {
        &self.body_rigs
    }

    /// Every portrait manifest, as `(asset path, ron_text)` sorted by path.
    pub fn portrait_rons(&self) -> &[(&'static str, Arc<str>)] {
        &self.portrait_rons
    }

    /// The portrait manifests, parsed on the first ask.
    pub fn portraits(&self) -> &crate::portrait::PortraitSheetRegistry {
        self.portraits.get_or_init(|| crate::portrait::PortraitSheetRegistry::from_baked_table(&self.portrait_rons))
    }

    /// Each sprite pack tier's catalog, as `(tier, json_text)` sorted by tier.
    pub fn pack_catalog_texts(&self) -> &[(&'static str, Arc<str>)] {
        &self.pack_catalog_texts
    }

    /// The sprite pack catalogs, keyed by tier, parsed on the first ask. A
    /// tier that does not parse or validate is disabled.
    pub fn pack_catalogs(&self) -> &std::collections::HashMap<&'static str, crate::SpritePackCatalog> {
        self.pack_catalogs.get_or_init(|| crate::sprite_packs::catalogs_of(&self.pack_catalog_texts))
    }

    /// The effect sheets of this generation's records, built on the first ask.
    pub fn fx_sheets(&self) -> &[crate::fx::FxSheet] {
        self.fx_sheets.get_or_init(|| crate::fx::fx_sheets_of(&self.registry.sheets))
    }

    /// The authored effects of this generation's records, built on the first
    /// ask.
    pub fn authored_effects(&self) -> & std::collections::BTreeMap<&'static str, crate::fx::AuthoredEffect> {
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
        self.digest.get_or_init(|| mechanics_of(&self.registry, &self.body_rigs, &self.part_tables, self.portraits()))
    }
}

/// The immutable sprite content selected by one App.
#[derive(bevy::prelude::Resource, Clone)]
pub struct SpriteContent {
    generation: Arc<SheetGeneration>,
}

impl PartialEq for SpriteContent {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.generation, &other.generation) }
}
impl Eq for SpriteContent {}

impl std::fmt::Debug for SpriteContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SpriteContent").field("generation", &self.generation.id).finish()
    }
}

impl Default for SpriteContent {
    fn default() -> Self {
        static EMPTY: OnceLock<Arc<SheetGeneration>> = OnceLock::new();
        Self { generation: EMPTY.get_or_init(|| Arc::new(SheetGeneration::new(0, SheetTexts::default()))).clone() }
    }
}

impl SpriteContent {
    pub fn from_texts(texts: SheetTexts) -> Result<Self, SheetTableRefused> {
        let mut content = Self::default();
        let prepared = prepare_sheet_table(&content, texts)?;
        publish_prepared_sheet_table(&mut content, prepared);
        Ok(content)
    }
}

pub fn current(content: &SpriteContent) -> &SheetGeneration { &content.generation }

pub fn sheet_rons(content: &SpriteContent) -> &[(&'static str, Arc<str>)] {
    current(content).rons()
}

/// The texts of one sheet generation.
#[derive(Clone, Debug, Default)]
pub struct SheetTexts {
    /// Every sheet manifest, as `(key, ron_text)` sorted by key.
    pub sheets: Vec<(&'static str, Arc<str>)>,
    /// Every body rig, as `(target, ron_text)` sorted by target.
    pub body_rigs: Vec<(&'static str, Arc<str>)>,
    /// Every part flipbook, sorted by key. Named, not read: a flipbook is
    /// read where it is decoded.
    pub part_flipbooks: Vec<crate::baked_part_flipbooks::PartFlipbookRow>,
    /// Every portrait manifest, as `(asset path, ron_text)` sorted by path.
    pub portraits: Vec<(&'static str, Arc<str>)>,
    /// Each sprite pack tier's catalog, as `(tier, json_text)` sorted by tier.
    pub pack_catalogs: Vec<(&'static str, Arc<str>)>,
    pub image_paths: std::collections::BTreeMap<String, String>,
    pub room_themes: crate::game_assets::RoomThemes,
}

impl SheetTexts {
    /// The texts of the current generation, to edit.
    pub fn current(content: &SpriteContent) -> Self {
        let generation = current(content);
        Self {
            sheets: generation.rons().to_vec(),
            body_rigs: generation.body_rigs().to_vec(),
            part_flipbooks: generation.part_tables().rows().to_vec(),
            portraits: generation.portrait_rons().to_vec(),
            pack_catalogs: generation.pack_catalog_texts().to_vec(),
            image_paths: generation.image_paths.clone(),
            room_themes: generation.room_themes.clone(),
        }
    }
}

/// Why a sheet table was not published: each file that does not parse, and
/// each key that two files give, with its error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetTableRefused(pub Vec<(String, String)>);

impl std::fmt::Display for SheetTableRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} sheet file(s) are refused:", self.0.len())?;
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
    pub fn registry(&self) -> &crate::SheetRegistry { &self.registry }

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
        mechanics_of(&self.registry, &self.texts.body_rigs, &self.part_tables, &self.portraits) != live.mechanics_digest()
    }
}

/// Parse and check `texts`. A table with a sheet that does not parse, a body
/// rig that does not parse or prepare, or a full-resolution part flipbook
/// that does not decode, is refused, with each such file named: a reader of a
/// rig or a flipbook panics on one it cannot read. Each flipbook is decoded
/// and the landmarks are projected here, off the frame; a flipbook whose file
/// did not change is taken from the current generation.
pub fn prepare_sheet_table(content: &SpriteContent, mut texts: SheetTexts) -> Result<PreparedSheetTable, SheetTableRefused> {
    texts.sheets.sort_by_key(|row| row.0);
    texts.body_rigs.sort_by_key(|row| row.0);
    texts.part_flipbooks.sort_by_key(|row| row.0);
    texts.portraits.sort_by_key(|row| row.0);
    texts.pack_catalogs.sort_by_key(|row| row.0);
    let mut duplicates = Vec::new();
    for (kind, keys) in [
        ("sheet", texts.sheets.iter().map(|row| row.0).collect::<Vec<_>>()),
        ("rig", texts.body_rigs.iter().map(|row| row.0).collect()),
        ("part", texts.part_flipbooks.iter().map(|row| row.0).collect()),
        ("portrait", texts.portraits.iter().map(|row| row.0).collect()),
        ("pack", texts.pack_catalogs.iter().map(|row| row.0).collect()),
    ] {
        duplicates.extend(keys.windows(2).filter(|pair| pair[0] == pair[1])
            .map(|pair| (pair[0].to_owned(), format!("Duplicate {kind} identity: {}", pair[0]))));
    }
    if !duplicates.is_empty() { return Err(SheetTableRefused(duplicates)); }
    let previous = current(content);
    let (parsed_files, mut failed, parsed_now) = crate::parse_sheet_files(&text_pairs(&texts.sheets), Some((previous.rons(), &previous.parsed_files)));
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
    let part_tables = crate::baked_part_flipbooks::PartTables::prepared(texts.part_flipbooks.clone(), current(content).part_tables())
        .map_err(SheetTableRefused)?;
    let index = crate::index_parsed(&text_pairs(&texts.sheets), &parsed_files, Vec::new());
    if !index.failed.is_empty() { return Err(SheetTableRefused(index.failed)); }
    let registry = crate::SheetRegistry::from_index(index);
    Ok(PreparedSheetTable { texts, registry, parsed_files, parsed_now, part_tables, portraits, pack_catalogs })
}

/// Whether a preparation must parse and prepare the rig `text` of `target`.
/// Not when the current generation holds the same text AND checked it: the
/// first generation was not checked, so the same text there is no proof
/// (found in review, 2026-10-17).
fn rig_needs_check<T: AsRef<str>>(current: &[(&str, T)], current_checked: bool, target: &str, text: &str) -> bool {
    let unchanged = current.binary_search_by(|(name, _)| (*name).cmp(target)).is_ok_and(|at| current[at].1.as_ref() == text);
    !(unchanged && current_checked)
}

/// Make `prepared` the sheet table every reader reads, with the records
/// parsed from it, and return the new generation's id.
///
/// The records feed simulation too (body sizes, hurtboxes, attack volumes,
/// a boss's size), so the caller publishes only where no session is live.
/// A session retains its accepted records through a `SpriteContent` clone.
pub fn publish_prepared_sheet_table(content: &mut SpriteContent, prepared: PreparedSheetTable) -> u64 {
    let generation = Arc::new(SheetGeneration::of(content.generation.id + 1, prepared));
    let id = generation.id;
    content.generation = generation;
    id
}

pub fn publish_sheet_generation(content: &mut SpriteContent, texts: SheetTexts) -> Result<u64, SheetTableRefused> {
    let prepared = prepare_sheet_table(content, texts)?;
    Ok(publish_prepared_sheet_table(content, prepared))
}

#[cfg(test)]
mod tests {
    use super::rig_needs_check;

    #[test]
    fn a_retired_generation_lives_until_its_last_consumer_releases_it() {
        let mut content = super::SpriteContent::from_texts(super::SheetTexts::default()).unwrap();
        let retired = std::sync::Arc::downgrade(&content.generation);
        let session = content.clone();
        super::publish_sheet_generation(&mut content, super::SheetTexts::default()).unwrap();
        assert!(retired.upgrade().is_some());
        drop(session);
        assert!(retired.upgrade().is_none());
    }

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

fn text_pairs<'a>(texts: &'a [(&'static str, Arc<str>)]) -> Vec<(&'static str, &'a str)> {
    texts.iter().map(|(key, text)| (*key, text.as_ref())).collect()
}

fn mechanics_of(
    registry: &crate::SheetRegistry,
    rigs: &[(&'static str, Arc<str>)],
    parts: &crate::baked_part_flipbooks::PartTables,
    portraits: &crate::portrait::PortraitSheetRegistry,
) -> String {
    let mut hash = blake3::Hasher::new();
    let mut piece = |label: &str, bytes: &[u8]| {
        hash.update(&(label.len() as u64).to_le_bytes());
        hash.update(label.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    };
    piece("sheets", crate::character::sheets::mechanics_digest_of(&registry.sheets).as_bytes());
    for (key, text) in rigs.iter().filter(|(key, _)| crate::quality_variant_marker(key).is_none()) {
        let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
            .expect("An admitted generation has valid rigs");
        piece("rig-key", key.as_bytes());
        piece("rig", &bincode::serialize(&rig).expect("Rig schema serialization"));
    }
    piece("landmarks", parts.landmarks().1.as_bytes());
    for target in portraits.available_targets() { piece("portrait", target.as_bytes()); }
    hash.finalize().to_hex().to_string()
}
