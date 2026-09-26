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
    /// Used only by the fallback in [`sprite_render_size_scaled`], for a sheet
    /// that publishes no body (almost every sheet publishes one).
    ///
    /// It scales the collision box's largest dimension to the rendered sprite
    /// height. The normal path sizes the quad from the sheet's body rectangle
    /// instead, which needs no per-sheet correction.
    pub collision_scale: f32,
    /// Sprite anchor y (normalized; negative shifts the sprite up so feet
    /// land near the collision-box bottom). Authoritative value lives in
    /// the RON's `body_metrics.feet_anchor_norm.y`.
    pub feet_anchor_y: f32,
    /// Sprite anchor x (normalized: the body's centre as a fraction of the
    /// frame, measured from the frame's centre). Authoritative value lives in
    /// the RON's `body_metrics.feet_anchor_norm.x`.
    ///
    /// Do not hard-code `0.0`. That centres the art on the frame, but the
    /// packer's crop can leave the body off-centre (up to 25% of the frame),
    /// and the art then draws beside its own collision box.
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
    /// The sheet key this spec was loaded from. Use it to ask for the record
    /// again. Outside consumers (the moveset inspector's atlas table) join on it.
    pub fn sheet_key(&self) -> &str {
        &self.record.key
    }

    /// Which `record.rows` index draws this pose, if any. The mapping is built
    /// once at spec load (`anim_rows`).
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

    /// Which way this sheet's art was drawn. See
    /// [`SheetRecord::authored_faces_left`]. It is read from the record, not
    /// copied, so the spec cannot disagree with its manifest.
    ///
    /// The renderer XORs this into the facing flip, as the boss path does with
    /// [`crate::boss::BossSheetSpec::flip_x`].
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

/// Sheets that providers author at plugin build.
///
/// The baked index is an immutable cache with no install seam, so this is a
/// separate resource. A provider fills it from its own RON, as it registers a
/// catalog fragment. It is a resource, not a global, so two Apps in one
/// process (every test run) do not share it.
///
/// Authored records take precedence over baked ones with the same target. Two
/// authored records for one target are refused; see
/// [`AuthoredSheets::insert_ron`].
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
    /// Canonical generation material for every authored sheet this App holds.
    ///
    /// Authored sheets are mechanical: a record carries body metrics for the
    /// collision body and attack geometry, so `register_character_sheet_ron`
    /// changes the simulation. This dump feeds `PreparedContentIdentity`, which
    /// the rollback timeline contract compares.
    ///
    /// It uses the declaration text, not the parsed record. That text is what
    /// the provider said, it is already kept for collision reports, and it
    /// needs no separate serialization contract.
    ///
    /// `origin` is not included. Moving a declaration to another file changes
    /// nothing a body simulates, so it must not change mechanical identity.
    ///
    /// Known defect: reformatting a sheet RON changes the identity. This fails
    /// safe (a false difference refuses a snapshot; a false match would restore
    /// into the wrong world). The fix is to hash the parsed mechanical fields;
    /// see `Q122`.
    ///
    /// Ordered by target (the map key), so the dump does not depend on
    /// registration order.
    pub fn deterministic_dump(&self) -> String {
        let mut out = String::new();
        for (target, record) in &self.by_target {
            out.push_str(target);
            out.push('\t');
            // The declaration contains newlines, so length-prefix it. A line
            // in it that looks like a row header then cannot forge a row.
            out.push_str(&record.declaration.len().to_string());
            out.push('\t');
            out.push_str(&record.declaration);
            out.push('\n');
        }
        out
    }

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
        // The same keying rule as the baked index: a single-record file is
        // keyed by its root, a packed file by each member's target. Set
        // `SheetRecord::key`; do not overwrite the authored `target`.
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

/// A spec for `target` from the authored registry first, then the baked cache.
/// Engine characters take the same path as before, so this is safe in front
/// of every lookup.
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

/// Build the baked record index now, so a gameplay frame does not.
///
/// Its first callers run per frame: `sync_sprite_posed_bodies` (sim schedule)
/// and `rendering/actors/animation.rs`. Warming moves the parse cost to
/// `Startup`.
pub fn warm_record_index() {
    let _ = record_index();
}

/// Process-wide index of every baked [`SheetRecord`], keyed by
/// [`crate::index_baked_table`]'s rule (file root; a packed atlas keys each
/// record by its own target). [`crate::SheetRegistry`] calls the same rule, so
/// the two cannot disagree.
///
/// An immutable asset cache derived once from the compile-time
/// `BAKED_SHEET_RONS` table, so a process-global `OnceLock` is correct. It is
/// not a content registry, so it has no `install_*` seam (see
/// `docs/architecture/engine-architecture.md`).
fn record_index() -> &'static HashMap<String, SheetRecord> {
    static INDEX: OnceLock<HashMap<String, SheetRecord>> = OnceLock::new();
    INDEX.get_or_init(|| crate::index_baked_table(crate::baked_sheet_rons::BAKED_SHEET_RONS).sheets)
}

/// Every baked sheet key ([`SheetRecord::key`]), sorted: the names a
/// character's `sheet` reference resolves against. These are keys, never rig
/// targets (many sheets share one rig). The list is baked, so a provider does
/// not need to supply it to catch a typo.
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

/// This body's geometry comes from its spritesheet, per pose.
///
/// Presence is the opt-in. A body without it keeps the collision box its
/// spawn authored. The opt-in matters for a sheet with per-animation body
/// metrics; on other sheets every pose resolves to the static idle bbox.
///
/// Only the declaration lives here: which [`record_for_sheet_key`] key the
/// boxes come from, and the world size of one pixel. The per-tick system that
/// writes the collision box, sprite quad, and offset
/// (`sync_sprite_posed_bodies`) works on components this crate does not own,
/// so it lives above. The actor crate declares a posed body by naming this
/// type, and neither crate has to name the other.
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

/// A spec for a sheet whose rows are addressed by name, not by pose.
///
/// Unlike [`try_load_spec_for_target`], this does not refuse a sheet with no
/// idle row. The character path indexes through [`CharacterAnim`] and would
/// panic on such a sheet. An effects sheet has no poses, and its consumer
/// resolves rows through [`CharacterSheetSpec::clip_slot`]. See [`crate::fx`].
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
    // A pack is storage for the same drawing, so inherit the drawing's facing
    // from the base manifest, as the caller inherits `tuning`. The synthesized
    // record cannot know it. Without this, a left-drawn sheet (Patent Clerk)
    // faces backward on the ultrapack path, which most devices use.
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
    // `flat_index` falls back to `Idle` for an animation with no row, and the
    // actor renderer panics without an Idle row. Skip such manifests; the
    // caller falls back to a colored rectangle.
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

/// Fallback tuning for catalog entries with no hardcoded `SheetTuning`.
/// `collision_scale = 1.5` keeps the sprite neither tiny nor oversized, and
/// `frame_sample_inset = 1` matches most tunings.
const DEFAULT_TUNING: SheetTuning = SheetTuning::new(1.5, 1);

fn spec_from_record(record: &SheetRecord, tuning: &SheetTuning) -> CharacterSheetSpec {
    // Manifest-authored tuning overrides Rust fallback tuning.
    let (collision_scale, frame_sample_inset) = match record.tuning {
        Some(t) => (t.collision_scale, t.frame_sample_inset),
        None => (tuning.collision_scale, tuning.frame_sample_inset),
    };
    // Map the rows this enum names to their `record.rows` index. Other rows
    // stay in `record` and still occupy atlas cells, but `CharacterAnim`
    // cannot select them.
    let anim_rows: Vec<(CharacterAnim, usize)> = record
        .rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| CharacterAnim::from_name(&row.animation).map(|anim| (anim, idx)))
        .collect();
    // The same authored point as `feet_anchor_y`, but with no override. A `y`
    // override is a gameplay choice about where feet plant; the horizontal
    // centre is a measurement of the art.
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

/// The three geometry facts one pose resolves to, in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosedBodyGeometry {
    /// Collision + hurt box extents.
    pub collision: Vec2,
    /// Sprite quad extents (the whole sheet frame).
    pub render: Vec2,
    /// Where to draw the quad's centre, relative to the body's centre. It is
    /// non-zero whenever the art is not centred in its frame, which is normal.
    pub sprite_offset: Vec2,
}

/// Resolve one pose's geometry from the baked sheet registry.
///
/// `None` when the target has no record or no usable body metrics. The caller
/// then leaves the body as authored; a fallback to "the whole frame is the
/// body" would inflate every collision box on such a sheet.
pub fn posed_body_geometry(
    target: &str,
    anim: CharacterAnim,
    world_per_pixel: f32,
) -> Option<PosedBodyGeometry> {
    let record = record_for_sheet_key(target)?;
    let metrics = record.body_metrics.as_ref()?;
    let bbox = metrics.pose_body_bbox(anim)?;
    let frame_w = record.frame_width.max(1) as f32;
    let frame_h = record.frame_height.max(1) as f32;
    let (cx, cy) = bbox.center();
    Some(PosedBodyGeometry {
        collision: Vec2::new(bbox.w as f32, bbox.h as f32) * world_per_pixel,
        render: Vec2::new(frame_w, frame_h) * world_per_pixel,
        // Sheet pixel space and world space both run +y downward (see the
        // `coordinate_system` block in each actor sidecar), so this is a plain
        // scale. It puts the art's body rectangle on the collision box.
        sprite_offset: Vec2::new(frame_w * 0.5 - cx, frame_h * 0.5 - cy) * world_per_pixel,
    })
}

/// The sheet's authored gameplay body, in sheet pixels. `None` when the sheet
/// only measured one (`BodyMetrics::authored_body` is false).
///
/// Uses the `Idle` pose, the standing body that `sync_sprite_posed_bodies`
/// restores `base_size` to.
pub fn authored_body_pixel_size(target: &str) -> Option<Vec2> {
    let record = record_for_sheet_key(target)?;
    let metrics = record.body_metrics.as_ref()?;
    if !metrics.authored_body {
        return None;
    }
    // Use the same function as the per-tick sync, at scale 1.0 (pixels), so
    // the two cannot disagree.
    posed_body_geometry(target, CharacterAnim::Idle, 1.0)
        .map(|geometry| geometry.collision)
        .filter(|size| size.x > 0.0 && size.y > 0.0)
}

#[cfg(test)]
mod tests;
