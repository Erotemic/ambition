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

use std::collections::{BTreeSet, HashMap};

use bevy::prelude::*;
use serde::Deserialize;
use tracing::{info, warn};

pub mod binding;
pub use binding::{AnimRow, AnimRowRef, BoundAnimRow};

mod frames;
pub use frames::{trimmed_render, AtlasPage, FrameTrim};

pub mod baked_portrait_rons;
pub mod baked_sheet_rons;
pub mod boss;
pub mod character;
pub mod fx;
pub mod game_assets;
pub mod sprite_packs;

pub mod pack;
pub use pack::{PackCatalogError, PackFrame, PackTarget, ResolvedFrame, SpritePackCatalog};

pub mod portrait;
mod snapshot_impls;
pub use portrait::{
    available_portrait_targets, baked_portrait_registry, parse_portrait_manifest,
    PortraitClipRecord, PortraitFrameRect, PortraitSheetManifest, PortraitSheetRegistry,
    PortraitSheetRegistryPlugin,
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
    /// **Which way this sheet's ART WAS DRAWN** — `true` when the generator
    /// rendered the neutral pose facing **left** (−x), the opposite of the
    /// renderer's standing assumption that art faces +x (right).
    ///
    /// It is a fact about the ARTWORK, not about the character, which is why it
    /// lives on the sheet: redraw the same character facing the other way and
    /// only this flips. The renderer's mirror decision is therefore *"does the
    /// requested facing differ from the facing this art was drawn in"* —
    /// `flip_x = (facing < 0) XOR authored_faces_left` — rather than
    /// `facing < 0`.
    ///
    /// ⭐ **`false` is the whole population minus a handful**, so the default
    /// keeps every sheet that never mentions the field byte-identical. The
    /// generator only emits it when it is `true`, which today means an
    /// SVG-rigged sheet whose rig declares `features.facing: "west"` (the
    /// Patent Clerk, whose `Side Left` paperdoll view is the drawn source).
    ///
    /// ⚠ this does NOT touch the body's `facing` value, its hitboxes, or its
    /// authored `launch_dir` — those were always right. Only the drawing was
    /// mirrored the wrong way.
    #[serde(default)]
    pub authored_faces_left: bool,
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

    /// The pages this sheet's frames actually draw from.
    ///
    /// [`Self::page_count`] is a COUNT — the highest page index plus one —
    /// which is the right answer for a dedicated sheet, whose pages are its
    /// own and contiguous. It is the wrong answer for a target inside a
    /// SHARED pack: there the target's frames occupy a sparse subset of a
    /// pack-wide page list, and treating `0..page_count` as the load set
    /// pulls in every intervening page of the whole pack. One prop whose
    /// frames land on pages 4 and 53 was loading all 54 ultrapack pages
    /// (~221 megapixels, ~880 MB) at boot.
    ///
    /// Per-frame `rect.page` is authoritative wherever it exists (the packer
    /// places frames freely); `row.page` is the per-row fallback for the
    /// unpacked multi-page layout, so it only counts for rows with no rects.
    /// Returns a sorted, deduplicated set — never empty, so a sheet with no
    /// rows still loads page 0.
    pub fn used_pages(&self) -> BTreeSet<u32> {
        let mut pages = BTreeSet::new();
        for row in &self.rows {
            if row.rects.is_empty() {
                pages.insert(row.page);
            } else {
                pages.extend(row.rects.iter().map(|rect| rect.page));
            }
        }
        if pages.is_empty() {
            pages.insert(0);
        }
        pages
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
    /// **`body_pixel_bbox` is this character's GAMEPLAY BODY, not the extent of
    /// its art.**
    ///
    /// The two rectangles were sharing one field. By default the generator
    /// measures the idle frame's alpha bbox — hat, antenna, outstretched arms
    /// and all — and a target that cares authors a tighter box instead
    /// (`body_metrics_fn`, or a fractional `body_inset` of the measured one).
    /// Nothing distinguished them, so a consumer asking "how big is this
    /// character's body" could be handed a drawing, and the only way to scale a
    /// character correctly was a hand-tuned [`SheetTuning::collision_scale`]
    /// that nothing checked.
    ///
    /// `false` (the default, and the absent case) means MEASURED. Treat a
    /// measured box as evidence about the art and not as a collision box.
    #[serde(default)]
    pub authored_body: bool,
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
    /// Convex polygon for THIS frame, same space and precedence as the coarse
    /// [`AnimationBox::poly`]: when present it is the shape, and `parts`/`bbox`
    /// are the fallback for consumers that cannot express one.
    #[serde(default)]
    pub poly: Vec<(f32, f32)>,
}

impl AnimationBoxFrame {
    pub fn is_populated(&self) -> bool {
        !self.parts.is_empty() || self.bbox.is_some() || !self.poly.is_empty()
    }
}

impl BodyMetrics {
    /// **The pose's body rectangle, in sheet-frame pixels** — the sprite
    /// author's answer to "where is this character, in the frame, right now".
    ///
    /// A body whose silhouette changes shape between poses (a snake that
    /// withdraws into a cardboard box, a boss that unfolds its arms) has no
    /// single honest collision rectangle: the idle-frame bbox is wrong for
    /// every pose that is not idle. When the sheet publishes per-animation
    /// hurtboxes this returns the one for `anim`; otherwise it falls back to
    /// the static [`Self::body_pixel_bbox`], which is the whole answer for the
    /// (common) characters whose silhouette barely moves.
    ///
    /// The `animations` map is keyed by the GENERATOR's row/gameplay key, so
    /// the match runs through [`CharacterAnim::from_name`] — the same alias
    /// table the sheet spec uses to bind rows. That keeps ONE naming authority:
    /// a generator that renames `boxed_idle` cannot silently desync the
    /// hurtbox from the row it belongs to without also losing the row.
    ///
    /// AMBITION_REVIEW(determinism): several row names can alias to one
    /// `CharacterAnim` (`rest` / `front_idle` / `side_idle` all mean `Idle`), so
    /// a sheet carrying two of them offers two candidate rectangles. This is
    /// SIM state — it becomes a collision box — so the winner is the
    /// lexicographically first key rather than whichever the `HashMap` happens
    /// to yield first.
    pub fn pose_body_bbox(&self, anim: character::CharacterAnim) -> Option<PixelRect> {
        self.animations
            .iter()
            .filter(|(name, _)| character::CharacterAnim::from_name(name) == Some(anim))
            .filter_map(|(name, metrics)| Some((name, metrics.hurtbox.as_ref()?.bbox?)))
            .min_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, bbox)| bbox)
            .or(self.body_pixel_bbox)
            .filter(|bbox| bbox.w > 0 && bbox.h > 0)
    }

    /// **How big this character IS in its own frame, for one pose** — the
    /// extent, in frame pixels, of everything the sheet calls its body.
    ///
    /// A disjoint-piece character (a boss with a head, a torso and two hands)
    /// publishes `body_pixel_parts` and its extent is their union; everything
    /// else is [`Self::pose_body_bbox`].
    ///
    /// ⭐ **the single reader of this fact.** It is asked by the collision
    /// derivation, by the sprite-quad sizing, and by the sheet-authored actor
    /// route, and those three used to ask three subtly different questions —
    /// so a body could be DRAWN from one rectangle and COLLIDED from another.
    /// Whatever the answer is, it is now the same answer everywhere.
    pub fn body_pixel_extent(&self, anim: character::CharacterAnim) -> Option<(f32, f32)> {
        if !self.body_pixel_parts.is_empty() {
            let mut min = (f32::MAX, f32::MAX);
            let mut max = (f32::MIN, f32::MIN);
            for part in &self.body_pixel_parts {
                min = (min.0.min(part.x as f32), min.1.min(part.y as f32));
                max = (
                    max.0.max((part.x + part.w) as f32),
                    max.1.max((part.y + part.h) as f32),
                );
            }
            let extent = (max.0 - min.0, max.1 - min.1);
            return (extent.0 > 0.0 && extent.1 > 0.0).then_some(extent);
        }
        self.pose_body_bbox(anim)
            .map(|bbox| (bbox.w as f32, bbox.h as f32))
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PixelRect {
    /// Centre of the rectangle in the same pixel space, as floats — the
    /// quantity a consumer needs to place the rectangle against a frame.
    pub fn center(self) -> (f32, f32) {
        (
            self.x as f32 + self.w as f32 * 0.5,
            self.y as f32 + self.h as f32 * 0.5,
        )
    }
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
    /// Convex polygon for THIS part, in the same frame-pixel space as the rect,
    /// which stays as its bounds and its fallback.
    ///
    /// Multi-part and SHAPED are different axes and a silhouette wants both. A
    /// single hull cannot describe disjoint pieces — one hull over a head, a
    /// torso and an outstretched arm fills every gap between them — and a rect
    /// per piece cannot describe a piece that is not a rectangle. A hooded head
    /// and a flaring cloak are neither.
    #[serde(default)]
    pub poly: Vec<(f32, f32)>,
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
    /// **Targets a later record took from an earlier one with a DIFFERENT frame
    /// grid** — recorded rather than warned about here. See
    /// [`Self::shadowed_targets`].
    shadowed: Vec<ShadowedTarget>,
}

/// One target claimed twice with different frame geometry: the winner crops the
/// loser's image with the wrong grid, IF anything resolves art by that target.
///
/// ⛔ **whether it does is not this crate's to know** (ledger D36). A rig name
/// like `toon` is shared by 17 characters legitimately and nothing looks it up;
/// a character id like `pirate_heavy_broadside_bess` is looked up, and a stale
/// manifest winning that key cost a day and a bisect through the asset tree. The
/// collision is visible only HERE, where both records pass through; which keys
/// are resolvable is visible only to a caller that owns a catalog. So this type
/// carries the fact across that gap instead of guessing at it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowedTarget {
    pub target: String,
    pub loser_image: String,
    pub loser_frame: (u32, u32),
    pub winner_image: String,
    pub winner_frame: (u32, u32),
}

impl std::fmt::Display for ShadowedTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "target `{}` claimed twice with DIFFERENT frame geometry — {}x{} \
             (image `{}`) is replaced by {}x{} (image `{}`). One of these \
             manifests is stale; the survivor crops with the wrong grid.",
            self.target,
            self.loser_frame.0,
            self.loser_frame.1,
            self.loser_image,
            self.winner_frame.0,
            self.winner_frame.1,
            self.winner_image,
        )
    }
}

impl SheetRegistry {
    pub fn get(&self, target: &str) -> Option<&SheetRecord> {
        self.sheets.get(target)
    }

    /// **Every target a later manifest took from an earlier one with a different
    /// grid**, in insertion order.
    ///
    /// ⭐ **the caller decides which of these MATTER**, because only it knows
    /// which targets something resolves art by — see [`ShadowedTarget`]. A
    /// consumer that reports all of them reproduces the ~30-per-boot Android
    /// noise this replaced; one that reports none re-opens the day-long bisect.
    pub fn shadowed_targets(&self) -> &[ShadowedTarget] {
        &self.shadowed
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
                        // ⛔ **LAST-WINS IS SILENT, AND IT COST A DAY.** A file
                        // dated May — `pirate_heavy_spritesheet.ron`, declaring
                        // 172x138 frames — still claimed the target
                        // `pirate_heavy_broadside_bess`, sorted BEFORE her real
                        // manifest's 319x250, and won the key. She then loaded the
                        // right image and cropped it with rectangles laid out for
                        // a sheet that no longer existed: "not cropping out the
                        // right part of the sprite sheet", in Jon's words, and a
                        // bisect through the asset tree to find it.
                        //
                        // ⚠ **a blanket collision warn would be noise**, which is
                        // why this sat: sheets legitimately share a target
                        // (`toon` x17, `robot` x18, `goblin` x9 — see the
                        // file-root index below). What is NEVER legitimate is two
                        // records claiming one target with DIFFERENT frame
                        // geometry, because that is the case where the winner
                        // crops the loser's image with the wrong grid.
                        //
                        // ⛔⛔ **THIS USED TO `warn!` HERE AND FIRED ~30 TIMES PER
                        // ANDROID BOOT** (ledger D36, Jon's device log). The
                        // condition cannot tell its harmful case from its harmless
                        // one: 17 characters share the rig target `toon` and
                        // legitimately have 17 different frame sizes, and nothing
                        // resolves art by `toon` at all. Measured: 146 targets are
                        // claimed by more than one record, and ZERO images are
                        // described by two different grids — so the sharers differ
                        // in IMAGE, every one of them.
                        //
                        // ⇒ recorded, not reported. The collision is visible only
                        // here; whether the loser is RESOLVABLE is visible only to
                        // a caller with a catalog, and this crate's whole claim is
                        // to be a content-free sprite-sheet vocabulary.
                        if let Some(prior) = registry.sheets.get(&record.target) {
                            if prior.frame_width != record.frame_width
                                || prior.frame_height != record.frame_height
                            {
                                registry.shadowed.push(ShadowedTarget {
                                    target: record.target.clone(),
                                    loser_image: prior.image.clone(),
                                    loser_frame: (prior.frame_width, prior.frame_height),
                                    winner_image: record.image.clone(),
                                    winner_frame: (record.frame_width, record.frame_height),
                                });
                            }
                        }
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
        // ⭐ **ONE line, not one per collision** (ledger D36). The per-record
        // `warn!` this replaced fired ~30 times on every Android boot, and the
        // count was a function of what LOADED rather than of what was wrong —
        // N records sharing a target produce N−1 warnings, so the shared rig
        // targets alone (`toon` x17, `robot` x18, `goblin` x9) can reach 45.
        //
        // ⛔ **NOT silenced, and the row says why not**: the case underneath is
        // real — a May-dated manifest won `pirate_heavy_broadside_bess`, so she
        // loaded the right image and cropped it with a dead grid, found by
        // bisecting the asset tree. What is wrong with the old warning is that it
        // cannot tell that from 17 characters legitimately sharing a rig, and the
        // fact it needs — *is this target one a reader resolves art by* — belongs
        // to a caller with a catalog, not to this crate.
        //
        // ⇒ so: one summary here, the detail on [`Self::shadowed_targets`], and
        // the catalog-aware filter is a separate slice that consumes it.
        //
        // ✔✔ **THAT SLICE EXISTS NOW, AND THIS LINE IS NO LONGER THE ALARM.**
        // `report_shadowed_character_sheets` (in `ambition_app`) reads
        // `shadowed_targets()` against the character catalog and warns only when
        // a shadowed target is a character id — the case that cost a day. It is
        // registered on `Startup`, and it is silent, because **no character id
        // currently collides** (D162: 166 targets, 4 geometry collisions, every
        // one a shared rig).
        //
        // ⛔⛔ **so this summary was shouting on EVERY boot about a condition a
        // better-informed reader had already cleared** — *"39 target(s) claimed
        // twice"* on a line that then explains it is probably fine. A warning
        // that ends by talking you out of itself is training people to skim the
        // channel, which is what the D36 note above was trying to prevent.
        //
        // ⚠ **DEBUG, not deleted, and not narrowed.** Every fact stays in the
        // message; only the volume changes — the level split this repo's own
        // doorway-diagnostic lesson landed on. A composition with no catalog
        // (a headless tool, a demo mounting no cast) still gets the whole list
        // one log level away, which is exactly the *"say nothing rather than
        // report every rig target as suspicious"* policy the consumer states.
        if !registry.shadowed.is_empty() {
            debug!(
                "SheetRegistry: {} target(s) claimed twice with different frame \
                 geometry — a shared RIG target (`toon`, `robot`, `goblin`) is \
                 legitimate and harmless, and a CHARACTER id here is the harmful \
                 case, which `report_shadowed_character_sheets` warns about \
                 against the catalog. First: {}. Full list: \
                 `SheetRegistry::shadowed_targets()`.",
                registry.shadowed.len(),
                registry.shadowed[0],
            );
        }
        registry
    }

    /// Like [`from_baked_table`], but keys each sheet by its **file root**
    /// (the table's first tuple element) instead of `record.target`.
    ///
    /// Several sheets legitimately share one `target` — e.g. `robot` and
    /// `player_robot_v3` are both authored against the `"robot"` adapter, so
    /// `record.target == "robot"` for both and they collide in the
    /// target-keyed registry. File roots are unique (one per
    /// `*_spritesheet.ron`), so this keeps them distinct. Use it when you
    /// need a specific sheet variant (the player's `player_robot_v3`, not the
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
/// This used to live behind `ambition_platformer2d_actor_monolith::character_sprites::registry`, but
/// it is pure sprite-sheet presentation vocabulary: the data source is the
/// baked `*_spritesheet.ron` table owned by this crate, and the installed
/// resource is [`SheetRegistry`]. Keeping the plugin here lets apps/content
/// install sheet metadata without routing through the actor crate.
pub struct SheetRegistryPlugin;

/// Present once [`SheetRegistryPlugin`] has built. See its `is_unique`.
#[derive(Resource)]
struct SheetRegistryInstalled;

impl Plugin for SheetRegistryPlugin {
    /// ⚠ **Idempotent, because more than one plugin legitimately NEEDS this.**
    /// Sprite metadata is not a game's choice — a render system that draws from
    /// a sheet cannot run without it — so any plugin that installs such a system
    /// installs this too, and the composition that already had it must not
    /// panic. `WorldLabelLayoutPlugin` carries the same pair for the same
    /// reason; a marker resource rather than `is_plugin_added` because the
    /// answer has to survive being asked by a plugin group mid-build.
    ///
    /// Until 2026-07-31 the only installer was `game/ambition_app`, which is
    /// why `HostProjectileVisualsPlugin` could not simply be moved out of it:
    /// the systems failed `Res<SheetRegistry>` validation the moment another
    /// composition ran them.
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<SheetRegistryInstalled>() {
            return;
        }
        app.insert_resource(SheetRegistryInstalled);
        app.init_resource::<SheetRegistry>()
            // The provider-authored half (U1). Initialised here so a provider
            // can register its sheets in ANY plugin-build order, and never
            // repopulated from the baked table: authored records are content,
            // not cache, and a Startup system that rebuilt them would erase
            // whatever a consumer declared before the app first ran.
            .init_resource::<crate::character::sheets::AuthoredSheets>()
            .add_systems(Startup, init_sheet_registry);
    }
}

fn init_sheet_registry(mut registry: ResMut<SheetRegistry>) {
    *registry = baked_sheet_registry();
}

/// Register a sheet a PROVIDER authored, keyed by the file root a catalog row
/// names (`manifest: "outlander_spritesheet.ron"` is file root `outlander`).
///
/// The character-catalog seam's twin: a provider says who its characters are
/// through `CharacterCatalogFragment`, and says what their sheets look like
/// through this. Before it existed, the second half was only expressible by
/// putting a RON in the ENGINE's asset tree and rebuilding the engine — which a
/// third party cannot do (queue U1).
pub trait AuthoredSheetAppExt {
    /// Panics on malformed RON, deliberately and with the file root in the
    /// message: a provider registering a broken sheet at plugin-build time has
    /// shipped a broken character, and discovering that as a placeholder
    /// rectangle three screens later is how the whole class of art bug hides.
    fn register_character_sheet_ron(&mut self, file_root: &str, ron: &str) -> &mut Self;
}

impl AuthoredSheetAppExt for App {
    fn register_character_sheet_ron(&mut self, file_root: &str, ron: &str) -> &mut Self {
        self.init_resource::<crate::character::sheets::AuthoredSheets>();
        let mut authored = self
            .world_mut()
            .resource_mut::<crate::character::sheets::AuthoredSheets>();
        match authored.insert_ron(file_root, ron) {
            Ok(_) => {}
            Err(message) => panic!("{message}"),
        }
        self
    }
}

#[cfg(test)]
mod tests;
