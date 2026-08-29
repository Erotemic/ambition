//! Sprite-sheet specs for every character target plus per-spec
//! geometry helpers (`sprite_render_size`, `feet_anchor_for`,
//! `build_character_sprite`).
//!
//! Generator output (`tools/ambition_sprite2d_renderer`) writes a
//! `*_spritesheet.ron` next to each PNG. The RON manifest is the canonical
//! source for generator-known geometry (frame sizes, rows, anchors); this file
//! adds gameplay tuning the generator cannot infer.

#![allow(unused_imports)]
use std::collections::HashMap;
use std::sync::OnceLock;

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::anim::CharacterAnim;
use super::CharacterSpriteAsset;
pub use crate::{trimmed_render, FrameTrim};
use crate::{AtlasPage, NormPoint, SheetRecord};

/// One animation row's runtime metadata. The pixel geometry (rects, pages,
/// trim) lives in the underlying [`SheetRecord`] and is read through the shared
/// [`ambition_sprite_sheet`] frame algebra; this is just the per-row timing the
/// animator advances on.
#[derive(Clone, Copy, Debug)]
pub struct RowInfo {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for one of the generated sheets.
///
/// Rows are sparse and ordered exactly as the generator emits them, so a
/// sandbag can list only idle/hit/death while the player can still list
/// the full movement/combat set.
///
/// The dynamic fields (`label_width`, `frame_width`, `frame_height`,
/// `rows`, `feet_anchor_y`) come from the RON manifest at first access;
/// the tuning fields (`collision_scale`, `frame_sample_inset`,
/// `y_offset`) live in this file because they're gameplay decisions
/// about how a sprite is *used*, not facts about how it was drawn.
#[derive(Clone, Debug)]
pub struct CharacterSheetSpec {
    pub label_width: u32,
    /// Pixel offset from the top of the sheet PNG before the first row. Defaults to 0 for sheets
    /// whose row 0 starts at the top of the image.
    pub y_offset: u32,
    /// Per-frame width in source-image pixels. The generator crops each
    /// sheet to the union of opaque-pixel bboxes across every frame,
    /// so this is *not* always 128 — pirate is 103, shark is 162.
    /// Authoritative value lives in the paired `*_spritesheet.ron`.
    pub frame_width: u32,
    pub frame_height: u32,
    /// Page image filenames (just the file name, resolved against the page-0
    /// image's directory at load time). `[record.image]` for a single-page
    /// sheet; one entry per page for a split sheet. Indexed by frame page.
    pub page_images: Vec<String>,
    /// Which `record.rows` index each [`CharacterAnim`] this sheet maps
    /// resolves to. Rows the enum doesn't name (animations authored ahead of
    /// the gameplay logic that will drive them) stay in `record` and still
    /// occupy atlas cells — they're just not selectable through this enum yet.
    anim_rows: Vec<(CharacterAnim, usize)>,
    /// The published sheet record: the single source of per-frame rects, page
    /// assignment, and trim. Every atlas / flat-index / trim query delegates to
    /// its [`ambition_sprite_sheet`] frame algebra, so the character path shares
    /// one implementation with the boss, prop, and projectile readers.
    record: SheetRecord,
    /// Vestigial for any sheet that publishes a body (181 of 183 baked),
    /// and read only by the fallback in [`sprite_render_size_scaled`].
    ///
    /// It multiplied the collision box's max dimension to get the rendered
    /// sprite's height, with the width taken from the padded frame's aspect — so
    /// it was a per-sheet correction for how much empty space the generator's
    /// crop happened to leave, and the 180 baked sheets spanned 10.9x in how
    /// big a character was drawn against its own box. The quad now comes from the
    /// sheet's own body rectangle, which makes that ratio 1.0 by construction.
    pub collision_scale: f32,
    /// Sprite anchor y (normalized; negative shifts the sprite up so feet
    /// land near the collision-box bottom). Authoritative value lives in
    /// the RON's `body_metrics.feet_anchor_norm.y`.
    pub feet_anchor_y: f32,
    /// Sprite anchor x (normalized; the body's own centre as a fraction of the
    /// frame, measured from the frame's centre). Authoritative value lives in
    /// the RON's `body_metrics.feet_anchor_norm.x`.
    ///
    /// ⛔⛔ THIS WAS A HARD-CODED `0.0` AT THE ANCHOR, and the sheets have carried
    /// the right number all along. `0.0` means "centre the art on the FRAME",
    /// and a frame is not a character: the art sits wherever the packer's crop
    /// left it, so a body drawn `0.0` is drawn off its own collision box by
    /// exactly how far off-centre it was packed. Invisible for the population
    /// that happens to sit near the middle (the other polygons are within 4%)
    /// and unmissable for the ones that do not — `projectile_polygon` is 17%
    /// of a 377px frame and `officer` is 25% of a 326px one, which is a box
    /// standing beside its own fighter.
    pub feet_anchor_x: f32,
    /// Pixel inset on every URect to prevent bilinear filtering from
    /// pulling neighboring frame pixels at the seam.
    pub frame_sample_inset: u32,
}

/// The gameplay-tuning fields that don't appear in the RON manifest.
/// One `SheetTuning` per sprite id is the smallest hand-typed delta
/// between the RON and a runnable `CharacterSheetSpec`.
pub struct SheetTuning {
    collision_scale: f32,
    feet_anchor_y_override: Option<f32>,
    frame_sample_inset: u32,
}

impl Default for SheetTuning {
    fn default() -> Self {
        DEFAULT_TUNING
    }
}

impl SheetTuning {
    /// Build runtime tuning from a catalog row's authored sprite-tuning fields.
    pub fn from_parts(
        collision_scale: f32,
        frame_sample_inset: u32,
        feet_anchor_y: Option<f32>,
    ) -> Self {
        Self {
            collision_scale,
            feet_anchor_y_override: feet_anchor_y,
            frame_sample_inset,
        }
    }
}

impl CharacterSheetSpec {
    /// Which `record.rows` index draws this semantic pose, if any.
    ///
    /// The mapping is built once at spec load (`anim_rows`); this is the read of
    /// it, so a consumer outside this module does not need the field public.
    /// The sheet KEY this spec was loaded from — how the record is asked for
    /// again, and the id a consumer outside the engine (the moveset inspector's
    /// atlas table) joins its own copy of the sheet on.
    pub fn sheet_key(&self) -> &str {
        &self.record.key
    }

    pub fn row_for_anim(&self, anim: CharacterAnim) -> Option<usize> {
        self.anim_rows
            .iter()
            .find(|(candidate, _)| *candidate == anim)
            .map(|(_, index)| *index)
    }

    /// Lift this spec's resolution-independent gameplay tuning back out as a
    /// [`SheetTuning`] — collision scale, frame-sample inset, and the resolved
    /// feet anchor (pinned as an override so a record with no `body_metrics`,
    /// e.g. a pack-synthesized record, renders with the SAME anchor).
    /// This is how the pack path inherits a base spec's tuning verbatim.
    pub fn tuning(&self) -> SheetTuning {
        SheetTuning::new(self.collision_scale, self.frame_sample_inset)
            .with_feet_anchor_y(self.feet_anchor_y)
    }

    /// Which way this sheet's art was DRAWN — see
    /// [`SheetRecord::authored_faces_left`]. Read straight off the published
    /// record rather than copied into a field, so the spec cannot disagree with
    /// the manifest it was built from.
    ///
    /// The renderer XORs this into the gravity-aware facing flip, exactly as
    /// the boss path has done with [`crate::boss::BossSheetSpec::flip_x`] since
    /// the mockingbird: the mirror asks *"does the requested facing differ from
    /// the drawn facing"*, not *"is facing negative"*.
    pub fn authored_faces_left(&self) -> bool {
        self.record.authored_faces_left
    }
}

impl SheetTuning {
    pub const fn new(collision_scale: f32, frame_sample_inset: u32) -> Self {
        Self {
            collision_scale,
            feet_anchor_y_override: None,
            frame_sample_inset,
        }
    }

    pub const fn with_feet_anchor_y(mut self, feet_anchor_y: f32) -> Self {
        self.feet_anchor_y_override = Some(feet_anchor_y);
        self
    }
}

/// The baked index below is an immutable asset CACHE, and its doc comment says so — "not a
/// content registry, so it has no `install_*` seam". Its character resolved no spec and drew
/// the placeholder rectangle, whatever art it shipped .
///
/// So the content registry is a separate, ordinary resource. A provider fills it
/// from its own RON at plugin-build time, exactly as it registers a character
/// catalog fragment, and the engine's baked sheets stay a cache that nothing has
/// to mutate. Being a RESOURCE rather than a second global is the point: two
/// Apps in one process (which is every test run in this repo) do not share it.
///
/// Consumer records take precedence over baked ones with the same target, and a
/// collision between two AUTHORED records for one target is REFUSED — see
/// [`AuthoredSheets::insert_ron`] for why refusing beats last-writer-wins.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct AuthoredSheets {
    by_target: std::collections::BTreeMap<String, AuthoredRecord>,
}

/// One authored record plus the provenance a collision report needs: which file
/// declared it, and what that file said. The declaration text is what makes
/// "the same provider registered twice" distinguishable from "two providers
/// disagree" without a `PartialEq` bound across the whole `SheetRecord` graph.
#[derive(Clone, Debug)]
struct AuthoredRecord {
    record: SheetRecord,
    origin: String,
    declaration: std::sync::Arc<str>,
}

impl AuthoredSheets {
    /// Parse one sheet RON and atomically index every record it declares.
    ///
    /// Single-record files use `file_root` as their target. A target may be
    /// claimed once; an identical re-registration from the same declaration is a
    /// no-op, while a conflicting claim is an error. Validation completes before
    /// any record is inserted.
    pub fn insert_ron(&mut self, file_root: &str, ron: &str) -> Result<usize, String> {
        let records: Vec<SheetRecord> = ron::from_str(ron)
            .map_err(|error| format!("authored sheet '{file_root}' is malformed RON: {error}"))?;
        if records.is_empty() {
            return Err(format!("authored sheet '{file_root}' declares no records"));
        }
        let declaration: std::sync::Arc<str> = std::sync::Arc::from(ron);
        // ⛔ THE SAME KEYING RULE THE BAKED INDEX USES, and it assigns
        // `SheetRecord::key` rather than overwriting the record's authored
        // `target`: a single-record file is named by its ROOT (one product, one
        // page), a packed one by each member's own name.
        let single = records.len() == 1;
        let mut records: Vec<SheetRecord> = records;
        for record in records.iter_mut() {
            record.key = if single {
                file_root.to_owned()
            } else {
                record.target.clone()
            };
        }

        let mut fresh = Vec::with_capacity(records.len());
        for record in records {
            match self.by_target.get(&record.key) {
                Some(held) if held.origin == file_root && *held.declaration == *declaration => {
                    // Same file, same bytes: idempotent, not a decision.
                }
                Some(held) => {
                    return Err(format!(
                        "authored sheet key '{}' is claimed twice: '{}' declared it \
                         and '{file_root}' redeclares it differently. Two authored \
                         sheets for one key resolve by plugin-build order, so this \
                         is refused rather than silently picked — rename one sheet or \
                         register only one of the two.",
                        record.key, held.origin,
                    ));
                }
                None => fresh.push(record),
            }
        }

        let indexed = fresh.len();
        for record in fresh {
            self.by_target.insert(
                record.key.clone(),
                AuthoredRecord {
                    record,
                    origin: file_root.to_owned(),
                    declaration: std::sync::Arc::clone(&declaration),
                },
            );
        }
        Ok(indexed)
    }

    pub fn get(&self, target: &str) -> Option<&SheetRecord> {
        self.by_target.get(target).map(|held| &held.record)
    }

    pub fn is_empty(&self) -> bool {
        self.by_target.is_empty()
    }

    pub fn targets(&self) -> impl Iterator<Item = &str> {
        self.by_target.keys().map(String::as_str)
    }
}

/// A spec for `target` from the AUTHORED registry first, the baked cache second.
///
/// The order is the whole point: a provider that authored a sheet gets its own,
/// and everything else resolves exactly as it always did — engine characters
/// take the identical path they took before this existed, which is what makes
/// this safe to put in front of every lookup.
pub fn try_load_spec_for_target_authored(
    authored: &AuthoredSheets,
    target: &str,
    tuning: &SheetTuning,
) -> Option<CharacterSheetSpec> {
    if let Some(record) = authored.get(target) {
        let spec = spec_from_record(record, tuning);
        if spec.maps(CharacterAnim::Idle) {
            return Some(spec);
        }
        tracing::warn!(
            target: "ambition_platformer2d::character_sprites",
            "authored sheet '{target}' has no Idle row; falling back to the baked \
             index (placeholder rectangle if there is none)",
        );
    }
    try_load_spec_for_target(target, tuning)
}

/// Process-wide index of every baked [`SheetRecord`], keyed by
/// [`crate::index_baked_table`]'s one rule — file root, except that a packed
/// atlas keys each record by its own target.
///
/// §5 classification (per the old restructuring blueprint, folded into
/// `docs/planning/engine/architecture.md`): immutable asset cache — derived once
/// from the compile-time `BAKED_SHEET_RONS` table, pure and override-free.
/// Correctly a process-global `OnceLock`; not a content registry, so it has no
/// `install_*` seam.
///
/// ⛔ it holds the SAME keying rule as [`crate::SheetRegistry`] because it calls
/// it. The two used to hand-roll it separately and disagreed: the registry keyed
/// by the record's authored `target` and this one by file root, so one shared engine resource
/// answered "give me sheet `robot`" with `tech_bro_disruptor`'s page while this
/// index answered correctly.
/// Build the baked record index NOW, so a gameplay frame does not.
///
/// ⛔⛔ THE SAME 870-ENTRY TABLE AS `init_sheet_registry`, PARSED AGAIN, AND ITS
/// FIRST CALLER IS A FRAME. Measured on hardware 2026-08-29, the sibling index in
/// `attack_hitbox` cost **189ms** the first time a punch asked for it. This one is
/// reached from `posed_body::{42,68}` — which `sync_sprite_posed_bodies` runs in
/// the SIM schedule every frame — and from `rendering/actors/animation.rs`.
/// Nothing warmed it, so the first frame to pose or draw a character paid the
/// parse.
///
/// ⭐ The `OnceLock` is not the defect. "Lazily" is: it means *on whichever frame
/// first asks*. Warming keeps the cache and moves the cost to `Startup`.
pub fn warm_record_index() {
    let _ = record_index();
}

fn record_index() -> &'static HashMap<String, SheetRecord> {
    static INDEX: OnceLock<HashMap<String, SheetRecord>> = OnceLock::new();
    INDEX.get_or_init(|| crate::index_baked_table(crate::baked_sheet_rons::BAKED_SHEET_RONS).sheets)
}

/// Every baked SHEET KEY, sorted — the vocabulary a character's `sheet`
/// reference resolves against.
///
/// ⛔ these are keys ([`SheetRecord::key`]), never rig targets: a rig target is
/// which adapter DREW a sheet and 48 sheets share five of them. The engine
/// always knows this list — it is baked — so a provider should never have to
/// hand it over just to have its typo caught.
pub fn available_sheet_keys() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = record_index().keys().map(String::as_str).collect();
    out.sort_unstable();
    out
}

/// Look up the baked [`SheetRecord`] under a sheet KEY — the same key
/// [`try_load_spec_for_target`] resolves a spec from, so a caller holding a
/// catalog's sheet reference can read the record's generator-emitted
/// `body_metrics` / frame dims without going through the Bevy
/// [`SheetRegistry`] resource (works headless / pre-asset-load).
pub fn record_for_sheet_key(key: &str) -> Option<&'static SheetRecord> {
    record_index().get(key)
}

/// This body's geometry is authored by its spritesheet, per pose.
///
/// Presence is the opt-in: a body without it keeps whatever collision box its
/// spawn authored, exactly as before. Opting in hands the box to the art, which
/// is only meaningful for a sheet that publishes per-animation body metrics —
/// for one that doesn't, every pose resolves to the same static idle bbox and
/// this degenerates to "size the body to its art", which is still an improvement
/// on a hand-guessed rectangle but is not why the seam exists.
///
///  the DECLARATION lives here and the per-tick derivation does not. This
/// is two facts about a sheet — which [`record_for_sheet_key`] key the boxes come
/// from, and how many world units one of its pixels covers — so it belongs to
/// the crate that owns sheet targets. The pass that resolves the pose and writes
/// the collision box, sprite quad and quad offset (`sync_sprite_posed_bodies`)
/// is an ECS system over components this crate does not own, and stays above.
///
/// That split is what lets the writer sit BESIDE the actor crate rather than
/// under it: the actor crate's character projection declares a posed body by
/// naming this type, the derivation reads it, and neither has to name the other.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SpritePosedBody {
    /// The sheet manifest target the boxes are read from (`"solid_snake"`).
    pub target: String,
    /// World units per sheet pixel. The ONE authored number: it fixes the
    /// body's on-screen scale, and every box follows from the art at that
    /// scale. Uniform by construction, so the art is never distorted.
    pub world_per_pixel: f32,
}

impl SpritePosedBody {
    pub fn new(target: impl Into<String>, world_per_pixel: f32) -> Self {
        Self {
            target: target.into(),
            world_per_pixel,
        }
    }
}

/// Load a sheet spec for an explicit manifest record key with the given tuning.
/// Returns `None` when the manifest target is absent so catalog-driven sprite
/// loading can fall back to colored rectangles.
pub fn try_load_spec_for_target(target: &str, tuning: &SheetTuning) -> Option<CharacterSheetSpec> {
    let record = record_index().get(target)?;
    let spec = spec_from_record(record, tuning);
    if spec.maps(CharacterAnim::Idle) {
        Some(spec)
    } else {
        tracing::warn!(
            target: "ambition_platformer2d::character_sprites",
            "sheet manifest for target '{target}' has no Idle row; skipping (placeholder rectangle)",
        );
        None
    }
}

/// A spec for a sheet whose rows are addressed by NAME rather than by pose.
///
///  the difference from [`try_load_spec_for_target`] is the `idle` refusal,
/// and it is deliberate on both sides. That one refuses a sheet with no idle
/// row because the character path indexes through [`CharacterAnim`] and would
/// panic asking such a sheet for a pose. A sheet of EFFECTS has no poses at all
/// — eleven of the twelve shipped FX sheets have no row `CharacterAnim` names —
/// and its consumer resolves rows through
/// [`CharacterSheetSpec::clip_slot`], which needs no pose. See
/// [`crate::fx`].
pub fn try_load_row_addressed_spec(
    target: &str,
    tuning: &SheetTuning,
) -> Option<CharacterSheetSpec> {
    record_index()
        .get(target)
        .map(|r| spec_from_record(r, tuning))
}

/// Load the scaled-variant spec for a manifest target, when its variant
/// record was baked (the generator produced `sprites_<suffix>/…` and `build.rs`
/// embedded it). Returns `None` for `Full` or when no variant record exists, so
/// the caller falls back to the base spec — keeping the atlas rects matched to
/// whichever PNG actually loads.
///
/// The variant record carries scaled frame rects / frame size / body metrics;
/// `tuning` (collision_scale, frame-sample inset, feet anchor) is
/// resolution-independent and is reused verbatim. Gameplay collision is
/// unaffected — it reads the base record via
/// `sprite_body_collision_for_character_id`.
pub fn try_load_spec_for_target_scaled(
    target: &str,
    tuning: &SheetTuning,
    scale: super::TextureResolutionScale,
) -> Option<CharacterSheetSpec> {
    let suffix = scale.asset_id_suffix()?;
    let record = record_index().get(&format!("{target}.{suffix}"))?;
    let spec = spec_from_record(record, tuning);
    spec.maps(CharacterAnim::Idle).then_some(spec)
}

/// Load a spec for `target` from the quality-tiered ultrapack catalogs
/// (shared-page packs installed under `assets/sprite_packs/<tier>/`).
///
/// The pack catalog's per-target [`SheetRecord`] view drops straight onto the same frame
/// algebra every other reader uses — freely-packed rows whose per-frame rects carry their own
/// page + trim offset — so a packed target needs no parallel render path.
///
/// The synthesized record has no `body_metrics`: `tuning` must carry the
/// feet anchor / collision scale (lift them from the base per-target spec),
/// and gameplay geometry keeps reading BASE data — packs are visual storage
/// truth only.
pub fn try_load_pack_spec_for_target(
    target: &str,
    tuning: &SheetTuning,
    scale: super::TextureResolutionScale,
) -> Option<(CharacterSheetSpec, &'static str)> {
    let (tier, catalog) = crate::sprite_packs::catalog_for_scale(scale)?;
    let mut record = catalog.to_sheet_record(target)?;
    // A pack is STORAGE for the same drawing, so the drawing's own facing
    // rides along. The synthesized record describes where the pixels sit in
    // the atlas; it cannot know which way the body in them points, and
    // repacking a sheet does not redraw it. Inherited from the base manifest
    // for exactly the reason the caller inherits `tuning` from the base spec.
    //
    //  without this the Patent Clerk faced the right way from his own sheet
    // and backwards again from the ultrapack — and he is packed at all four
    // tiers, so that is the path most devices actually take.
    record.authored_faces_left = record_for_sheet_key(target)
        .map(|base| base.authored_faces_left)
        .unwrap_or(false);
    let spec = spec_from_record(&record, tuning);
    spec.maps(CharacterAnim::Idle).then_some((spec, tier))
}

pub fn try_load_spec_for_character_id(character_id: &str) -> Option<CharacterSheetSpec> {
    let index = record_index();
    let record = index.get(character_id).or_else(|| {
        character_id
            .strip_prefix("npc_")
            .and_then(|stripped| index.get(stripped))
    })?;
    let spec = spec_from_record(record, &DEFAULT_TUNING);
    // The runtime atlas indexer (`flat_index`) falls back to `Idle` for any animation that
    // doesn't have its own row. Without at least an Idle row, the actor renderer panics on the
    // very first frame. Better to skip these manifests here — caller falls back to the
    // colored-rectangle visual.
    if spec.maps(CharacterAnim::Idle) {
        Some(spec)
    } else {
        tracing::warn!(
            target: "ambition_platformer2d::character_sprites",
            "character_sprites: skip spec for catalog id '{character_id}' \
             (manifest has no recognized Idle row; rows = {:?})",
            spec.mapped_anims().collect::<Vec<_>>(),
        );
        None
    }
}

/// Fallback tuning for catalog entries that don't have a hardcoded
/// `SheetTuning`. The values are middle-of-the-road — `collision_scale
/// = 1.5` keeps the sprite from being microscopic or overscaled, and
/// `frame_sample_inset = 1` is the same value most existing tunings
/// use. Catalog entries that need different visuals can graduate to
/// a hardcoded const + an explicit `SheetTuning::new(...)` later.
const DEFAULT_TUNING: SheetTuning = SheetTuning::new(1.5, 1);

fn spec_from_record(record: &SheetRecord, tuning: &SheetTuning) -> CharacterSheetSpec {
    // Manifest-authored tuning overrides Rust fallback tuning.
    let (collision_scale, frame_sample_inset) = match record.tuning {
        Some(t) => (t.collision_scale, t.frame_sample_inset),
        None => (tuning.collision_scale, tuning.frame_sample_inset),
    };
    // Map the rows this enum names to their `record.rows` index. Rows the enum
    // doesn't recognize stay in `record` (and still occupy atlas cells via the
    // shared frame algebra) but aren't selectable through `CharacterAnim`. The
    // per-frame rect / trim / page handling all lives in the algebra now, so
    // there is nothing to copy here.
    let anim_rows: Vec<(CharacterAnim, usize)> = record
        .rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| CharacterAnim::from_name(&row.animation).map(|anim| (anim, idx)))
        .collect();
    // ⭐ THE SAME FACT AS `feet_anchor_y`, off the same authored point, and it
    // needs no override: a `y` override exists because a character may want its
    // feet planted somewhere other than where the art puts them, which is a
    // GAMEPLAY choice. There is no equivalent for `x` — the body's horizontal
    // centre is a measurement of the art, not a decision about it.
    let feet_anchor_x = record
        .body_metrics
        .as_ref()
        .and_then(|b| b.feet_anchor_norm)
        .map(|p: NormPoint| p.x)
        .unwrap_or(0.0);
    let feet_anchor_y = tuning.feet_anchor_y_override.unwrap_or_else(|| {
        record
            .body_metrics
            .as_ref()
            .and_then(|b| b.feet_anchor_norm)
            .map(|p: NormPoint| p.y)
            .unwrap_or(-0.5)
    });
    // Page image filenames: the explicit `images` list when the sheet was
    // split, else the single `image` as the sole page-0 entry. Resolved
    // against the page-0 image's directory at load time.
    let page_images = if record.images.is_empty() {
        vec![record.image.clone()]
    } else {
        record.images.clone()
    };
    CharacterSheetSpec {
        label_width: record.label_width,
        y_offset: record.y_offset,
        frame_width: record.frame_width,
        frame_height: record.frame_height,
        page_images,
        anim_rows,
        record: record.clone(),
        collision_scale,
        feet_anchor_x,
        feet_anchor_y,
        frame_sample_inset,
    }
}

mod atlas;
mod geometry;
pub use atlas::*;
pub use geometry::*;

#[cfg(test)]
mod tests;
