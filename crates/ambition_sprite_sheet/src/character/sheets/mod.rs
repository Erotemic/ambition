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
use crate::{AtlasPage, NormPoint, SheetRecord};
// Re-exported from the foundational crate so `super::sheets::{trimmed_render,
// FrameTrim}` paths (the animator, renderer) keep resolving after the trim
// algebra moved into `ambition_sprite_sheet`.
pub use crate::{trimmed_render, FrameTrim};

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
    /// Pixel offset from the top of the sheet PNG before the first row.
    /// Used to share one PNG across multiple sprite specs that each take
    /// a different row block — e.g. the lab-props sheet has 8 props
    /// stacked vertically, each addressed by its own static with
    /// `y_offset = N * frame_height`. Defaults to 0 for sheets whose
    /// row 0 starts at the top of the image.
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
    /// Multiplier applied to the entity's collision-box max dimension to
    /// derive the rendered sprite's height. Width is derived from the
    /// cropped frame's aspect ratio so the character isn't squashed.
    pub collision_scale: f32,
    /// Sprite anchor y (normalized; negative shifts the sprite up so feet
    /// land near the collision-box bottom). Authoritative value lives in
    /// the RON's `body_metrics.feet_anchor_norm.y`.
    pub feet_anchor_y: f32,
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
    /// Lift this spec's resolution-independent gameplay tuning back out as a
    /// [`SheetTuning`] — collision scale, frame-sample inset, and the resolved
    /// feet anchor (pinned as an override so a record with no `body_metrics`,
    /// e.g. a pack-synthesized record, renders with the SAME anchor).
    /// This is how the pack path inherits a base spec's tuning verbatim.
    pub fn tuning(&self) -> SheetTuning {
        SheetTuning::new(self.collision_scale, self.frame_sample_inset)
            .with_feet_anchor_y(self.feet_anchor_y)
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

/// Process-wide index of every baked `SheetRecord`. Single-record files key
/// by filename root to avoid archetype-target collisions; multi-record packed
/// PNGs key each record by its own target.
///
/// §5 classification (restructuring-blueprint): **immutable asset cache** —
/// derived once from the compile-time `BAKED_SHEET_RONS` table, pure and
/// override-free. Correctly a process-global `OnceLock`; not a content
/// registry, so it has no `install_*` seam.
fn record_index() -> &'static HashMap<String, SheetRecord> {
    static INDEX: OnceLock<HashMap<String, SheetRecord>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: HashMap<String, SheetRecord> = HashMap::new();
        for (filename_root, text) in crate::baked_sheet_rons::BAKED_SHEET_RONS {
            let Ok(records) = ron::from_str::<Vec<SheetRecord>>(text) else {
                // Skip malformed RON quietly — the
                // `every_spritesheet_ron_parses_into_sheet_record` test
                // catches these in CI. Avoid panicking at runtime so a
                // hand-edited file under a subdir doesn't kill startup.
                continue;
            };
            if records.len() == 1 {
                let mut record = records.into_iter().next().unwrap();
                record.target = (*filename_root).to_owned();
                index.insert((*filename_root).to_owned(), record);
            } else {
                let scale_suffix = filename_root.rsplit_once('.').and_then(|(_, suffix)| {
                    matches!(suffix, "0_5x" | "0_25x" | "potato").then_some(suffix)
                });
                for mut record in records {
                    if let Some(scale_suffix) = scale_suffix {
                        record.target = format!("{}.{}", record.target, scale_suffix);
                    }
                    index.insert(record.target.clone(), record);
                }
            }
        }
        index
    })
}

/// Look up the baked [`SheetRecord`] for a manifest target key — the same
/// key [`try_load_spec_for_target`] resolves a spec from, so a caller that
/// has a catalog `manifest_target()` can read the record's generator-emitted
/// `body_metrics` / frame dims without going through the Bevy
/// [`SheetRegistry`] resource (works headless / pre-asset-load).
pub fn record_for_target(target: &str) -> Option<&'static SheetRecord> {
    record_index().get(target)
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
            target: "ambition::character_sprites",
            "sheet manifest for target '{target}' has no Idle row; skipping (placeholder rectangle)",
        );
        None
    }
}

/// Load the **scaled-variant** spec for a manifest target, when its variant
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
/// The pack catalog's per-target [`SheetRecord`] view drops straight onto the
/// same frame algebra every other reader uses — freely-packed rows whose
/// per-frame rects carry their own page + trim offset — so a packed target
/// needs no parallel render path. Returns the spec **and the tier actually
/// used** (the requested tier falls back to `full` when absent) so the
/// caller's page image paths address the catalog the rects came from.
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
    let record = catalog.to_sheet_record(target)?;
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
    // The runtime atlas indexer (`flat_index`) falls back to
    // `Idle` for any animation that doesn't have its own row.
    // Without at least an Idle row, the actor renderer panics
    // on the very first frame. Better to skip these manifests
    // here — caller falls back to the colored-rectangle visual.
    // The renderer-side fix is to ensure every published sheet
    // exposes an `idle` row; until then we drop them safely.
    if spec.maps(CharacterAnim::Idle) {
        Some(spec)
    } else {
        tracing::warn!(
            target: "ambition::character_sprites",
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
        feet_anchor_y,
        frame_sample_inset,
    }
}

/// Player-specific compact robot sheet. Rendered from
/// `tools/ambition_sprite2d_renderer/configs/player_robot.yaml`
/// (`archetype: player_compact`). Shares the same row order as
/// `ROBOT_SHEET` so animation indexing is identical — only the
/// per-frame geometry + anchor differ to match the shrunk
/// silhouette.

/// Absurd General — military-faction NPC sheet. Generated by
/// `tools/ambition_sprite2d_renderer` (archetype: `absurd_general`).
///
/// The generator emits 6 row bands (idle, walk, talk, interact,
/// celebrate, hit) on a 1108×720 sheet with a 4px border between
/// frame cells (frame content 120×116, row pitch 120, column pitch
/// 124). We only declare the `Idle` row here for the stationary
/// faction-leader use case; future work that gives the General
/// animations (talk during dialog, celebrate on encounter clear)
/// will extend `CharacterAnim` and append rows in PNG order so the
/// atlas y-stride stays aligned with the generator output.

// Only prop/story statics consumed by content code remain below.

// ─────────────────────────────────────────────────────────────────
// Toon-target NPC sheets — share the generator's 4-px inter-frame
// padding (col_pitch = content_w + 4, row_pitch = content_h + 4) and
// `feet_anchor_norm.y ≈ -0.47` from `body_metrics`. We declare only
// `Idle` here; rows added later (Walk/Talk) need to land at PNG row
// indices 1, 2, … in order, since `build_atlas` walks rows
// sequentially. `collision_scale ≈ 1 / (body_h / row_pitch)` keeps
// the silhouette scaled to the LDtk collision box.
// ─────────────────────────────────────────────────────────────────

/// Burning Flying Shark — wide 192×128 frames, 4 rows
/// (idle / fly / chomp / dive). Mapped through CharacterAnim as:
/// Idle row → Idle, fly row → Walk (the enemy picker uses Walk when
/// vel.x is non-zero, which is the right choice for an always-moving
/// flyer), chomp row → Slash (attack picker), dive row → Dash. There
/// is no hit / death row in this generated sheet; the resolver falls
/// back to Idle for those animations.
///
/// collision_scale chosen so the visible shark body (160×66 px of
/// the 192×128 frame, per `burning_flying_shark_actor.ron`'s
/// `body_pixel_bbox`) fits the AABB tightly. With cs=0.8 and an
/// authored AABB of (126, 52):
///   render = 126*0.8 = 100.8 tall, 151.2 wide
///   visible body inside render = 151.2 * (160/192) = 126.0 wide,
///   100.8 * (66/128) = 52.0 tall → matches AABB exactly.
///
/// Pre-fix this was 1.4, which gave a 151×101 render with the
/// visible body at ~126×52 inside a (72, 56) AABB — the shark was
/// drawn ~75% wider than its hitbox so player hits at the visible
/// silhouette missed.

/// Puppy slug — a small ground-walker (Crawlid analogue). Generator
/// emits 128×95 frames with rows `idle / walk / wall_walk /
/// ceiling_walk / hurt / death`. Only `idle / walk / hurt / death`
/// are picked up by `CharacterAnim::from_name`; the two surface-
/// variant rows are kept in the sheet for a future wall-wrapping
/// brain.

/// Pirate heavy bruiser — three named variants (Broadside Bess,
/// Iron Mary, Salt Annet) sharing the same row layout (idle / walk
/// / slash / taunt / hurt / death) but with palette + proportion
/// differences that auto-crop into slightly different frame sizes
/// per variant. Each variant therefore declares its own spec.

/// Standard pirate sheets. They share the same high-level row layout, but
/// each generated RON owns its own body metrics / feet anchor. Keep raider
/// separate from the admiral so a dismounted shark raider lands with her
/// feet on the floor instead of inheriting the admiral's anchor.

/// Architect — hub research / ADR-explainer NPC.

/// Kernel Guide — onboarding NPC at the hub spawn area.

/// Vault Keeper — persistence / save-seed NPC in the basement.

/// Interdimensional gate ring — the standing stone arch that frames
/// a portal. Two authored rows in
/// `interdimensional_gate_ring_spritesheet.yaml`:
/// - Row 0 = `idle` (8 frames × 140ms) — the always-on slow rotation
/// - Row 1 = `spin` (12 frames × 85ms) — the faster boot-spin
///
/// We borrow `CharacterAnim::Walk` as the semantic slot for the
/// `spin` row (same pattern as [`GATE_PORTAL_SHEET`]). The
/// [`crate::rooms::sync_portal_ring_rotation_system`] requests
/// `Walk` while `GatePortalPhase::Opening` is active and falls back to
/// `Idle` otherwise.

/// Interdimensional gate portal — the shimmering surface inside the
/// ring. Three rows authored in the source PNG
/// (`interdimensional_gate_portal_spritesheet.yaml`): `opening`
/// (8 frames × 80ms = 640ms one-shot), `stable` (8 × 110ms looping),
/// `closing` (8 × 80ms one-shot). The portal's [`crate::rooms::GatePortalPhase`]
/// state machine drives which row to play; this spec borrows
/// existing `CharacterAnim` variants as semantic slots
/// (Idle=opening so the default boot is visible, Walk=stable for
/// the steady "ready" loop, Run=closing for the shutdown
/// one-shot). The runtime's
/// [`crate::rooms::sync_portal_sprite_animation`] system calls
/// `CharacterAnimator::request(...)` with the right variant on
/// phase change.

// ───────────────────────────────────────────────────────────────────
// Lab props — shared `creator_lab_props_spritesheet.png`.
//
// One 128×128 frame per prop; 4 frames per row (subtle idle anim).
// 8 props stacked vertically; each spec below picks its row by
// setting `y_offset = row_index * 128`. The label column on the
// left is 160 px wide. See `assets/sprites/creator_lab_props_spritesheet.yaml`
// for the canonical frame coordinates this matches.
//
// All 8 are pre-registered so authors can drop any of them into a
// scene without touching this file. Place via `NpcSpawn` with
// `name: "Genesis Vat"` (etc.) + `prompt: ""` so the prop renders
// but never opens a dialogue panel.
// ───────────────────────────────────────────────────────────────────

// Lab-prop sheets share one PNG (`creator_lab_props_spritesheet.png`):
// 8 props stacked vertically with row pitch = frame_height. Each prop
// is its own `SheetRecord` in `creator_lab_props_spritesheet.ron`
// (target ids: `genesis_vat`, `specimen_jar`, …) with `y_offset` set
// to `prop_index * frame_height`. From the runtime's perspective they
// are 8 ordinary specs that happen to dereference the same PNG.

/// Diagnostic Cart — the rail / gurney the player wakes on. Rendered
/// by the dedicated `intro_cart` tack-on target. 3 rows ship on disk
/// (idle / roll / jolt); only Idle wires here today. Frame size is
/// 192×128 (wider than tall — the cart is a prop, not a humanoid).
/// The cart authors as an NpcSpawn with `name: "Diagnostic Cart"` so
/// it picks up its sprite from `INTRO_NPC_SPRITE_REGISTRY` — same
/// path the other intro characters use. A dedicated `Prop` entity
/// type lands in a follow-up; for the v1 slice the NpcSpawn slot is
/// the lightest way to get a visible cart without engine churn.

/// News board — wall-mounted bulletin board prop rendered by the
/// `news_board` tack-on target. Single Idle row (6 frames @ 165 ms)
/// for the LED blink + sticky-note flutter. Sheet is 64×96 per
/// frame after the renderer's label column (104 px wide).
///
/// `collision_scale: 1.50` makes the board render visibly bigger
/// than its 32×48 LDtk collision box so the art reads as a
/// proper bulletin board rather than a thumbnail. `feet_anchor_y`
/// pins the board's bottom row against the collision-box bottom
/// (no baked drop shadow — see the project rule on no shadows).

/// Cut-rope arena rope prop. Authored as a narrow LDtk `Prop` above
/// the anvil; the art fills the authored hitbox so the cuttable area
/// matches the visible rope.

/// Cut-rope arena anvil trap. Runtime code moves the authored prop
/// when the rope is cut; this sheet supplies the visible heavy anvil.

/// Cut-rope arena piano trap. Shares the same authored LDtk prop slot as
/// the anvil; the cut-rope arena system swaps the prop sprite based on the
/// replay cycle's selected heavy-object kind.

/// Generic reusable explosion VFX sheet. The rows are mapped onto
/// CharacterAnim slots by `CharacterAnim::from_name`; consumers pick
/// a row through `ExplosionKind` instead of hard-coding atlas indices.

/// Creator — the researcher who wakes the player. Rendered by the
/// dedicated `creator` tack-on target (not the toon-side adapter), so
/// the sheet is wider (160×192) and starts after a 108px label column.
/// 4 animation rows ship on disk (idle/speak/gesture/walk); only Idle
/// is wired here today — when CharacterAnim grows a Talk variant,
/// the speak row at index 1 lands automatically because the renderer
/// looks the row up by enum discriminant.

/// Raid Enforcer — uniformed later-game raid grunt. Toon-side
/// adapter render; the dedicated `raid_enforcer` archetype reads
/// as "officer cap + storm uniform + rifle" and also serves as a
/// temporary generic raid silhouette until more specific art lands.

/// Oiler — street mechanic / Eulerian gate-keeper NPC who finds the
/// player in the drain alley after the intro escape. Toon-side adapter
/// render; matches the Oiler review config (configs/review/oiler.yaml).

/// Erdish — wandering graph-theory eccentric. Bespoke prop-free scholar
/// render; matches the Erdish review config (configs/review/erdish.yaml).

/// Alice — unofficial cartographer. Toon-side adapter render; the
/// `alice_cryptographer` archetype reads as "cautious local with a
/// scarf and a sealed envelope". Matches the Alice review config
/// (configs/review/alice.yaml) and the
/// `alice_spritesheet.yaml`/`.png` pair that ships in
/// `crates/ambition_actors/assets/sprites/`.

/// Bob — field cartographer. Toon-side adapter render; the
/// `bob_engineer` archetype is wider in the shoulders (engineer
/// silhouette) so the frame is correspondingly wider than Alice's.
/// Matches the Bob review config (configs/review/bob.yaml).

/// Merchant Prototype — placeholder shopkeeper NPC.

// ─────────────────────────────────────────────────────────────────
// Robot-target faction-leader sheets. Tightly packed (no inter-frame
// padding), `feet_anchor_norm.y ≈ -0.328`, body fills ~83% of the
// row pitch → `collision_scale ≈ 1.20`.
// ─────────────────────────────────────────────────────────────────

/// Fretjaw — Goblin Cantina chieftain (faction leader of the
/// rowdy training-pit faction). Goblin-target generator output:
/// label_w=120, no inter-frame padding, body fills ~86% of the
/// 128-tall row.

/// Captain Pulse — Pulse Voyagers faction leader.

/// Chadwick Disruptor III — Tech-Bros Basement faction leader.

/// Shadow Oni Leader / Shadow Duelist — both ship the same generator
/// layout (idle / walk / run / jump / fall / slash / hit / death /
/// blink_out / blink_in / dash; 128×128 frames, no inter-frame
/// padding, label_width = 100). Mirrors the PIRATE_SHEET pattern:
/// two filenames, one indexing contract. We load `ninja_shadow_duelist`
/// as the representative — both ninja manifests report
/// `feet_anchor_norm.y = -0.4921875` and identical row layout, so the
/// pair stays interchangeable.
mod atlas;
mod geometry;
pub use atlas::*;
pub use geometry::*;

#[cfg(test)]
mod tests;
