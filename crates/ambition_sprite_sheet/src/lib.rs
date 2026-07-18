//! Runtime sprite-sheet metadata registry.
//!
//! Procedural generators emit `*_spritesheet.ron` manifests alongside the YAML
//! audit sidecars. Runtime code reads the baked RON table through [`SheetRegistry`]
//! so sprite dimensions, row layout, and body metrics stay aligned with generated
//! sheets.
//!
//! Authoring tools may keep using YAML for inspection; runtime consumers should use
//! the RON data embedded by `build.rs` through
//! the host crate's baked sheet table. Re-running sprite generation and
//! then building is enough to refresh the baked table for desktop, Android, wasm,
//! and other targets.

// SheetRecord / SheetRow / BodyMetrics / FrameRect / PixelRect /
// PixelPoint / NormPoint carry the full generator-emitted schema.
// Several fields are diagnostic or reserved for future consumers
// (atlas viewer, per-frame anchor probes) — silence the unused-field
// warnings at the module level so the schema stays whole.
#![allow(
    dead_code,
    reason = "deserialize surface that mirrors the on-disk RON schema; not every field is queried at runtime yet"
)]

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;
use tracing::{info, warn};

mod frames;
pub use frames::{trimmed_render, AtlasPage, FrameTrim};

pub mod baked_portrait_rons;
pub mod baked_sheet_rons;
pub mod boss;
pub mod character;
pub mod game_assets;
pub mod sprite_packs;

pub mod pack;
pub use pack::{PackCatalogError, PackFrame, PackTarget, ResolvedFrame, SpritePackCatalog};

pub mod portrait;
pub use portrait::{
    baked_portrait_registry, parse_portrait_manifest, PortraitClipRecord, PortraitFrameRect,
    PortraitSheetManifest, PortraitSheetRegistry, PortraitSheetRegistryPlugin,
};

/// One sprite-sheet's metadata as serialized by the generator. Field
/// names mirror the RON shape exactly; reorder cautiously.
///
/// The RON file shape is always `[SheetRecord, SheetRecord, …]` — a
/// list, even for single-target sheets. Most lists have length 1, but
/// shared PNGs (e.g. `creator_lab_props_spritesheet.png` packs 8 props
/// into one image) carry one record per sub-target, each with a
/// distinct `y_offset`. The list shape is uniform so loaders and the
/// generator emitters don't branch.
#[derive(Debug, Clone, Deserialize)]
pub struct SheetRecord {
    /// Unique target id (matches the generator's `TARGET_NAME`, the
    /// YAML's `target` field, and the PNG filename root). Use this as
    /// the key when looking up a sheet.
    pub target: String,
    /// PNG filename, relative to the sprites asset dir. May be shared
    /// across multiple records when several targets pack onto the same
    /// sheet image (in which case `y_offset` selects each target's row
    /// band). For multi-page sheets this is page 0 (same as `images[0]`).
    pub image: String,
    /// Page image filenames for sheets split across multiple PNGs. A sheet
    /// with one animation per row can grow taller than the GPU texture limit
    /// (16384px); the generator then splits the animation rows across several
    /// page images so each PNG stays within the limit. Each [`SheetRow::page`]
    /// indexes into this list, and that row's `rects` are in that page image's
    /// own coordinate space (each page starts at y=0). Empty (the common case)
    /// ⇒ the whole sheet is the single `image` and every row is page 0.
    #[serde(default)]
    pub images: Vec<String>,
    pub label_width: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    /// Pixel offset from the top of the shared sheet PNG before this
    /// target's first row. `0` for sheets whose row 0 starts at the
    /// top of the image (the common case). Lab-prop entries on the
    /// shared `creator_lab_props_spritesheet.png` set this to
    /// `prop_index * frame_height` so each prop addresses its own row
    /// band of the packed image.
    #[serde(default)]
    pub y_offset: u32,
    /// Derived geometry the generator computed from the rendered art:
    /// alpha-bbox of the body, foot pixel, and the normalized foot
    /// anchor (`feet_anchor_norm.y` is what
    /// `CharacterSheetSpec::feet_anchor_y` mirrors today).
    #[serde(default)]
    pub body_metrics: Option<BodyMetrics>,
    /// Per-target gameplay tuning authored alongside the sheet. When absent,
    /// callers use their Rust fallback tuning.
    #[serde(default)]
    pub tuning: Option<SheetTuningSpec>,
    pub rows: Vec<SheetRow>,
}

impl SheetRecord {
    /// Number of distinct page images this sheet addresses. `1` for the
    /// common single-PNG case (`images` empty) and for any sheet whose rows
    /// all reference page 0. A freely-packed sheet may carry per-frame pages
    /// beyond the `images` list length, so this takes the max of both.
    pub fn page_count(&self) -> u32 {
        let by_frames = self
            .rows
            .iter()
            .map(|row| {
                let rect_max = row.rects.iter().map(|r| r.page).max().unwrap_or(0);
                row.page.max(rect_max)
            })
            .max()
            .map(|p| p + 1)
            .unwrap_or(1);
        (self.images.len() as u32).max(by_frames)
    }

    /// Filename of the PNG holding `page`. Falls back to the single `image`
    /// when `images` is empty or the index is out of range, so single-page
    /// callers can ignore paging entirely.
    pub fn page_image(&self, page: u32) -> &str {
        self.images
            .get(page as usize)
            .map(String::as_str)
            .unwrap_or(self.image.as_str())
    }
}

/// Per-target gameplay-tuning fields embedded in the spritesheet manifest.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SheetTuningSpec {
    /// Multiplier on the actor's collision AABB when computing the
    /// rendered sprite size. `1.0` = sprite exactly fills the AABB;
    /// `2.1` (the robot's tuning) = sprite is much larger than the
    /// hitbox. Authored per-character to compensate for the fraction
    /// of each frame the actual character art occupies after
    /// auto-crop.
    pub collision_scale: f32,
    /// Inset (pixels) applied to each frame rect when sampling the
    /// atlas. `1` (the common case) trims one pixel from every edge
    /// to avoid bilinear bleed from neighboring frames. `0` for
    /// pixel-perfect sheets that don't need the inset.
    pub frame_sample_inset: u32,
}

/// Body / hurtbox metadata emitted alongside the sprite sheet.
///
/// `body_pixel_bbox` is the single overall bbox (alpha-bbox of the
/// idle/rest frame) — the common case for single-piece characters
/// (player, goblins, small bosses).
///
/// `body_pixel_parts` is the multi-rect representation for
/// **disjointed-piece characters** — giant bosses with head + body
/// + arms + legs that the gameplay code wants to address
/// individually. Each part carries a `name` so consumers can target
/// "head" vs "left_hand" by string. Defaults to empty.
///
/// `animations` carries **per-animation** hurtbox + hitbox data
/// keyed by animation name (e.g. `"floor_slam"`, `"side_sweep"`).
/// Each entry overrides the static body bbox for that animation
/// so a boss whose arms extend out only during attack frames gets
/// the right hurtbox during those frames, and so attack
/// hitboxes are positioned where the sprite author intended.
///
/// Consumer rule (hurtbox): when the current animation has a
/// `AnimationMetrics::hurtbox`, use it. Else when
/// `body_pixel_parts` is non-empty, prefer it. Else fall back to a
/// single-element list built from `body_pixel_bbox`. See
/// the host crate's boss attack-geometry derivation (`world_space_body_aabbs_from_metrics`)
/// for the canonical derivation.
#[derive(Debug, Clone, Deserialize)]
pub struct BodyMetrics {
    #[serde(default)]
    pub body_pixel_bbox: Option<PixelRect>,
    /// Multi-rect hurtbox metadata. Each entry is a named pixel
    /// rectangle in sprite-frame space. Empty = use `body_pixel_bbox`
    /// as the single body.
    #[serde(default)]
    pub body_pixel_parts: Vec<NamedPixelRect>,
    /// Per-animation hurtbox + hitbox overrides. Keyed by the same
    /// animation name the spritesheet rows use (`"rest"`,
    /// `"floor_slam"`, `"side_sweep"`, …). The renderer emits one
    /// entry per animation in the sheet; consumers look up by the
    /// boss's currently-playing animation name.
    #[serde(default)]
    pub animations: std::collections::HashMap<String, AnimationMetrics>,
    #[serde(default)]
    pub feet_pixel: Option<PixelPoint>,
    #[serde(default)]
    pub feet_anchor_norm: Option<NormPoint>,
}

/// Per-animation authored / derived hit + hurt box data. The
/// renderer fills `hurtbox` from each animation's alpha-bbox by
/// default; adapters declare `hitbox` rectangles explicitly for
/// each attack animation. Either may be `None` (meaning "fall
/// back to the static `body_pixel_bbox`" or "this animation has
/// no attack hitbox").
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationMetrics {
    /// Optional frame duration for sampling `AnimationBox::frames`.
    /// Uses the same units as `SheetRow::duration_secs`. Generators
    /// only need to fill this when they emit per-frame gameplay boxes.
    #[serde(default)]
    pub frame_duration_secs: Option<f32>,
    /// Hurtbox for this animation (where the *player's* attacks
    /// register hits on this actor). Multi-rect if the sprite has
    /// disjoint body parts; single-rect via `bbox` for simple
    /// bodies. `None` = fall back to `BodyMetrics::body_pixel_bbox`.
    #[serde(default)]
    pub hurtbox: Option<AnimationBox>,
    /// Hitbox for this animation (where *this actor's* attack
    /// damages the player). Non-attack animations leave this `None`.
    /// Attack-flavored animations (`floor_slam`, `side_sweep`,
    /// `spike_halo`, etc.) author one or more rects.
    #[serde(default)]
    pub hitbox: Option<AnimationBox>,
}

/// One animation's hit-or-hurt box, expressed as multi-rect parts
/// + an optional fallback single bbox. Mirrors the
/// `body_pixel_parts` / `body_pixel_bbox` split.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationBox {
    /// Multi-rect representation. Use `parts` when the sprite has
    /// disjoint pieces (head + arms + body). Empty = fall back to
    /// `bbox`.
    #[serde(default)]
    pub parts: Vec<NamedPixelRect>,
    /// Single-rect fallback. Most attack hitboxes are one box;
    /// most hurtboxes derived from alpha bounds are one box.
    #[serde(default)]
    pub bbox: Option<PixelRect>,
    /// Optional convex polygon (sprite-frame pixel points, same space as
    /// `bbox`). When non-empty, a consumer that supports shaped volumes (the
    /// player/actor attack hitbox) builds a convex hitbox conforming to the
    /// effect — a blade arc, a cone — instead of the `bbox`. Empty = use `bbox`.
    /// Older manifests without this field deserialize unchanged.
    #[serde(default)]
    pub poly: Vec<(f32, f32)>,
    /// Optional per-frame boxes for animation rows whose gameplay
    /// geometry should move with the drawn pose. When populated,
    /// consumers sample this by the current animation elapsed time
    /// before falling back to the coarse per-animation `parts`/`bbox`.
    #[serde(default)]
    pub frames: Vec<AnimationBoxFrame>,
}

impl AnimationBox {
    /// True iff this box has at least one rectangle (either parts,
    /// bbox, or per-frame data populated). Used by consumers as the
    /// "should I use this or fall back?" gate.
    pub fn is_populated(&self) -> bool {
        !self.parts.is_empty()
            || self.bbox.is_some()
            || !self.poly.is_empty()
            || self.frames.iter().any(AnimationBoxFrame::is_populated)
    }
}

/// One sampled frame of an [`AnimationBox`]. Same rectangle shape as
/// the coarse box, but indexed by animation time. This is intentionally
/// optional so old manifests keep deserializing unchanged.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationBoxFrame {
    #[serde(default)]
    pub parts: Vec<NamedPixelRect>,
    #[serde(default)]
    pub bbox: Option<PixelRect>,
}

impl AnimationBoxFrame {
    pub fn is_populated(&self) -> bool {
        !self.parts.is_empty() || self.bbox.is_some()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A named pixel rectangle in sprite-frame space, used for
/// multi-part body / hurtbox metadata. The `name` lets gameplay
/// code address parts individually (`head`, `body`, `left_hand`,
/// `right_hand`, …). For single-piece characters, leave
/// `body_pixel_parts` empty and use `body_pixel_bbox` instead.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NamedPixelRect {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl NamedPixelRect {
    pub fn rect(&self) -> PixelRect {
        PixelRect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PixelPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct NormPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SheetRow {
    pub animation: String,
    pub row_index: u32,
    pub frame_count: u32,
    pub duration_ms: u32,
    pub duration_secs: f32,
    /// Which page image (index into [`SheetRecord::images`]) this row's frames
    /// live in. `0` (the default) for single-page sheets and for the first
    /// page of a split sheet. The row's `rects` are in that page's own pixel
    /// space, so two rows on different pages may legitimately share `y` values.
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub rects: Vec<FrameRect>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrameRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Page image (index into [`SheetRecord::images`]) this frame lives in.
    /// `0` by default. The atlas packer places frames freely for best fill, so
    /// frames of one animation may land on different pages — this is the
    /// authoritative per-frame page for packed sheets ([`SheetRow::page`] is
    /// only a per-row default kept for the unpacked multi-page layout).
    #[serde(default)]
    pub page: u32,
    /// Trim offset of this rect within the LOGICAL frame, in logical-frame
    /// pixels `(off_x, off_y)`. The atlas packer trims each frame to its opaque
    /// alpha bounding box for storage; `w`/`h` are then the trimmed size and
    /// `off` is where that trimmed box sat inside the full
    /// `frame_width`×`frame_height` logical frame. `(0, 0)` (the default) means
    /// the frame is untrimmed (`w`/`h` == the logical frame size). The runtime
    /// adds `off` back so trimmed pixels draw exactly where the full frame did.
    #[serde(default)]
    pub off: (i32, i32),
    /// Per-frame named anchors emitted by `frame_meta_fn` (e.g.
    /// `hand_anchor`, `muzzle_anchor`). Generators that don't use
    /// `frame_meta_fn` leave this empty.
    #[serde(default)]
    pub anchors: HashMap<String, NormPoint>,
}

/// Resource looked up by sprite target id. Populated at startup by
/// [`init_sheet_registry`].
#[derive(Resource, Debug, Default)]
pub struct SheetRegistry {
    sheets: HashMap<String, SheetRecord>,
}

impl SheetRegistry {
    pub fn get(&self, target: &str) -> Option<&SheetRecord> {
        self.sheets.get(target)
    }

    pub fn len(&self) -> usize {
        self.sheets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &SheetRecord)> {
        self.sheets.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Look up the body metrics + frame size for a sprite target.
    /// Used by gameplay code (boss combat_size derivation, hurtbox
    /// math) so the sprite RON is the single source of truth for
    /// where the visible body sits inside the frame.
    ///
    /// Returns `(metrics, frame_width, frame_height)` when the
    /// target exists *and* has body_metrics; `None` otherwise.
    pub fn body_metrics(&self, target: &str) -> Option<(&BodyMetrics, u32, u32)> {
        let record = self.sheets.get(target)?;
        let metrics = record.body_metrics.as_ref()?;
        Some((metrics, record.frame_width, record.frame_height))
    }

    /// Build a fully-populated registry from a baked `(filename_root, ron_text)`
    /// table — the `*_spritesheet.ron` manifests the game bakes at build time.
    /// Pure (no Bevy `App` / `Startup` schedule): the host crate owns the baked
    /// table (it knows where its sprite assets live) and passes it in, so this
    /// crate stays a content-free, reusable sprite-sheet vocabulary. Most files
    /// are a length-1 list; shared-PNG sheets (lab props) carry multiple records.
    pub fn from_baked_table(table: &[(&str, &str)]) -> Self {
        let mut registry = Self::default();
        let mut loaded = 0usize;
        let mut failed: Vec<(String, String)> = Vec::new();
        for (filename_root, text) in table {
            // Below-full quality-variant RONs (`sprites_0_5x` / `_0_25x` /
            // `_potato`, baked as `<root>.<marker>` by `build.rs::baked_key_for_path`)
            // are embedded in the SAME table but MUST NOT enter this target-keyed
            // registry: every variant of `robot_slash` carries the identical
            // `target: "robot_slash"`, so the last one inserted (potato, 8px
            // frames) would silently clobber the full-res base. Any consumer that
            // reads `Res<SheetRegistry>` directly against a full-res PNG (the slash
            // VFX, shrine/projectile visuals) would then crop with potato-scale
            // rects. Variant records reach the resolution-pair loader through the
            // separate file-root-keyed indices (`from_baked_table_by_file_root`,
            // `character::sheets::record_index`) that key them distinctly.
            if is_quality_variant_file_root(filename_root) {
                continue;
            }
            match ron::from_str::<Vec<SheetRecord>>(text) {
                Ok(records) => {
                    for record in records {
                        registry.sheets.insert(record.target.clone(), record);
                        loaded += 1;
                    }
                }
                Err(err) => {
                    failed.push(((*filename_root).to_owned(), err.to_string()));
                }
            }
        }

        info!(
            "SheetRegistry: loaded {loaded} sheets from baked table ({} failed)",
            failed.len()
        );
        for (file, err) in failed {
            warn!("SheetRegistry: failed to parse baked {file}: {err}");
        }
        registry
    }

    /// Like [`from_baked_table`], but keys each sheet by its **file root**
    /// (the table's first tuple element) instead of `record.target`.
    ///
    /// Several sheets legitimately share one `target` — e.g. `robot` and
    /// `player_robot` are both authored against the `"robot"` adapter, so
    /// `record.target == "robot"` for both and they collide in the
    /// target-keyed registry. File roots are unique (one per
    /// `*_spritesheet.ron`), so this keeps them distinct. Use it when you
    /// need a specific sheet variant (the player's `player_robot`, not the
    /// enemy `robot`). Multi-record files keep only the first record.
    pub fn from_baked_table_by_file_root(table: &[(&str, &str)]) -> Self {
        let mut registry = Self::default();
        for (file_root, text) in table {
            match ron::from_str::<Vec<SheetRecord>>(text) {
                Ok(records) => {
                    if let Some(record) = records.into_iter().next() {
                        registry.sheets.insert((*file_root).to_owned(), record);
                    }
                }
                Err(err) => {
                    warn!("SheetRegistry: failed to parse baked {file_root}: {err}");
                }
            }
        }
        registry
    }
}

/// True when a baked file root names a below-full quality variant — the
/// `<root>.0_5x` / `.0_25x` / `.potato` keys `build.rs::baked_key_for_path`
/// emits for the `sprites_0_5x` / `sprites_0_25x` / `sprites_potato` folders.
/// Kept in sync with that function; the base (full-res) root carries no marker.
fn is_quality_variant_file_root(root: &str) -> bool {
    root.ends_with(".0_5x") || root.ends_with(".0_25x") || root.ends_with(".potato")
}

/// Build a [`SheetRegistry`] from the build-script baked RON table.
///
/// This is headless-friendly and keeps render/tooling callers from depending on
/// the actor crate just to inspect generated sprite metadata.
pub fn baked_sheet_registry() -> SheetRegistry {
    SheetRegistry::from_baked_table(baked_sheet_rons::BAKED_SHEET_RONS)
}

/// Bevy plugin that installs and populates the baked character sheet registry.
///
/// This used to live behind `ambition_actors::character_sprites::registry`, but
/// it is pure sprite-sheet presentation vocabulary: the data source is the
/// baked `*_spritesheet.ron` table owned by this crate, and the installed
/// resource is [`SheetRegistry`]. Keeping the plugin here lets apps/content
/// install sheet metadata without routing through the actor crate.
pub struct SheetRegistryPlugin;

impl Plugin for SheetRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SheetRegistry>()
            .add_systems(Startup, init_sheet_registry);
    }
}

fn init_sheet_registry(mut registry: ResMut<SheetRegistry>) {
    *registry = baked_sheet_registry();
}

#[cfg(test)]
mod tests;
