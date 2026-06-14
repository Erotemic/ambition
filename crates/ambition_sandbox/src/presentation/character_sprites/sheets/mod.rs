//! Sprite-sheet specs for every character target plus per-spec
//! geometry helpers (`sprite_render_size`, `feet_anchor_for`,
//! `build_character_sprite`).
//!
//! Generator output (`tools/ambition_sprite2d_renderer`) writes a
//! `*_spritesheet.ron` next to each PNG. The RON manifest is the canonical
//! source for generator-known geometry (frame sizes, rows, anchors); this file
//! adds gameplay tuning the generator cannot infer.

use std::collections::HashMap;
use std::sync::OnceLock;

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::anim::CharacterAnim;
use super::assets::CharacterSpriteAsset;
use super::registry::{NormPoint, SheetRecord};

#[derive(Clone, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
    /// Row's y-position in the PNG, measured in row units (multiply
    /// by `frame_height` to get pixels). Copied verbatim from the
    /// RON manifest so the atlas builder can address each row by
    /// its authored y even when intermediate rows were filtered
    /// out by `CharacterAnim::from_name`. Kept as a fallback for
    /// when the RON omits `frame_rects` — the primary atlas-build
    /// path uses `frame_rects` directly to honor inter-frame
    /// padding (which uniform grid math misses).
    pub row_index: u32,
    /// Per-frame source rectangles in the PNG, copied verbatim from
    /// the RON `rects` field. Used as the authoritative atlas-cell
    /// coordinates: many generated sheets pad between frames (the
    /// toon target's row pitch is 93 even though frame_height = 89),
    /// so deriving x/y from grid stride alone misaligns every cell
    /// by the padding amount and the GPU samples adjacent-frame
    /// pixels → visible tearing. `None` for legacy sheets that
    /// pre-date the rects field — the builder falls back to grid
    /// math (with `row_index`) for those.
    pub frame_rects: Option<Vec<URect>>,
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
    pub rows: Vec<(CharacterAnim, AnimRow)>,
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
    /// Build runtime tuning from a catalog row's authored
    /// [`SpriteTuningSpec`](crate::actor::character_catalog::SpriteTuningSpec).
    pub fn from_spec(spec: crate::actor::character_catalog::SpriteTuningSpec) -> Self {
        Self {
            collision_scale: spec.collision_scale,
            feet_anchor_y_override: spec.feet_anchor_y,
            frame_sample_inset: spec.frame_sample_inset,
        }
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

    #[allow(dead_code)]
    pub const fn with_feet_anchor_y(mut self, feet_anchor_y: f32) -> Self {
        self.feet_anchor_y_override = Some(feet_anchor_y);
        self
    }
}

/// Process-wide index of every baked `SheetRecord`. Single-record files key
/// by filename root to avoid archetype-target collisions; multi-record packed
/// PNGs key each record by its own target.
fn record_index() -> &'static HashMap<String, SheetRecord> {
    static INDEX: OnceLock<HashMap<String, SheetRecord>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: HashMap<String, SheetRecord> = HashMap::new();
        for (filename_root, text) in super::baked_sheet_rons::BAKED_SHEET_RONS {
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
                for record in records {
                    index.insert(record.target.clone(), record);
                }
            }
        }
        index
    })
}

/// Load a sheet spec for an explicit manifest record key with the given tuning.
/// Returns `None` when the manifest target is absent so catalog-driven sprite
/// loading can fall back to colored rectangles.
pub fn try_load_spec_for_target(target: &str, tuning: &SheetTuning) -> Option<CharacterSheetSpec> {
    let record = record_index().get(target)?;
    let spec = spec_from_record(record, tuning);
    if spec
        .rows
        .iter()
        .any(|(anim, _)| *anim == CharacterAnim::Idle)
    {
        Some(spec)
    } else {
        bevy::log::warn!(
            target: "ambition::character_sprites",
            "sheet manifest for target '{target}' has no Idle row; skipping (placeholder rectangle)",
        );
        None
    }
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
    if spec
        .rows
        .iter()
        .any(|(anim, _)| *anim == CharacterAnim::Idle)
    {
        Some(spec)
    } else {
        bevy::log::warn!(
            target: "ambition::character_sprites",
            "character_sprites: skip spec for catalog id '{character_id}' \
             (manifest has no recognized Idle row; rows = {:?})",
            spec.rows.iter().map(|(a, _)| a).collect::<Vec<_>>(),
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
    let rows: Vec<(CharacterAnim, AnimRow)> = record
        .rows
        .iter()
        .filter_map(|row| {
            let anim = CharacterAnim::from_name(&row.animation)?;
            // Convert RON `FrameRect` (i32 fields, may include
            // negative authoring values for off-canvas placement)
            // into UVec2-backed URects. Drop the whole vector if
            // any rect has negative coords — fall back to grid
            // math in `build_atlas` rather than panicking on the
            // cast.
            let frame_rects = if row.rects.is_empty() {
                None
            } else if row
                .rects
                .iter()
                .any(|r| r.x < 0 || r.y < 0 || r.w <= 0 || r.h <= 0)
            {
                None
            } else {
                Some(
                    row.rects
                        .iter()
                        .map(|r| URect {
                            min: UVec2::new(r.x as u32, r.y as u32),
                            max: UVec2::new((r.x + r.w) as u32, (r.y + r.h) as u32),
                        })
                        .collect(),
                )
            };
            Some((
                anim,
                AnimRow {
                    frame_count: row.frame_count as usize,
                    duration_secs: row.duration_secs,
                    row_index: row.row_index as u32,
                    frame_rects,
                },
            ))
        })
        .collect();
    let feet_anchor_y = tuning.feet_anchor_y_override.unwrap_or_else(|| {
        record
            .body_metrics
            .as_ref()
            .and_then(|b| b.feet_anchor_norm)
            .map(|p: NormPoint| p.y)
            .unwrap_or(-0.5)
    });
    CharacterSheetSpec {
        label_width: record.label_width,
        y_offset: record.y_offset,
        frame_width: record.frame_width,
        frame_height: record.frame_height,
        rows,
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

/// Erdish — wandering graph-theory eccentric. Toon-side adapter render;
/// matches the Erdish review config (configs/review/erdish.yaml).

/// Alice — unofficial cartographer. Toon-side adapter render; the
/// `alice_cryptographer` archetype reads as "cautious local with a
/// scarf and a sealed envelope". Matches the Alice review config
/// (configs/review/alice.yaml) and the
/// `alice_spritesheet.yaml`/`.png` pair that ships in
/// `crates/ambition_sandbox/assets/sprites/`.

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

/// Per-target sprite render size. The generator's character occupies only
/// part of the 128×128 frame, so the rendered quad must be larger than
/// the collision box for the visible body to roughly match the hitbox.
///
/// TODO(gen2d-collision-aware): teach the generator to write
/// `body_pixel_extent` + `feet_y_pixel` into the spritesheet YAML and
/// load them at runtime, replacing these per-spec constants with values
/// derived from each sheet's actual rendered body. The per-spec tuning
/// already isolates the override per target so the migration is local.
pub fn sprite_render_size(spec: &CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, 1.0)
}

/// Render-size helper with an additional presentation-only scale.
///
/// The collision box remains gameplay authority; this scale is only for
/// placeholder sprites while final art is still in flux.
pub fn sprite_render_size_scaled(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    visual_scale: f32,
) -> Vec2 {
    // Height is collision-driven; width preserves the cropped frame's
    // aspect ratio so the character isn't horizontally squashed when the
    // generator crop produces non-square frames (e.g. robot 120×128).
    let height =
        collision.x.max(collision.y).max(8.0) * spec.collision_scale * visual_scale.max(0.05);
    let width = height * (spec.frame_width as f32 / spec.frame_height as f32);
    Vec2::new(width, height)
}

/// Presentation-only scale for the temporary player sprite.
///
/// The robot sheet's `collision_scale` compensates for transparent/cropped
/// frame space; this extra factor gives the placeholder a slightly more
/// heroic read against the tuned 30×48 movement body without changing
/// gameplay collision.
pub const PLAYER_PLACEHOLDER_VISUAL_SCALE: f32 = 1.16;

pub fn player_placeholder_render_size(spec: &CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, PLAYER_PLACEHOLDER_VISUAL_SCALE)
}

/// Sprite anchor that places the rendered character's feet on the bottom
/// of the collision box (rather than at its centre).
pub fn feet_anchor_for(spec: &CharacterSheetSpec, collision: Vec2) -> Anchor {
    feet_anchor_for_render_size(spec, collision, sprite_render_size(spec, collision))
}

/// Sprite anchor for an explicit render size. This keeps the feet planted when
/// presentation-only scaling makes the sprite larger than its collider.
pub fn feet_anchor_for_render_size(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    render_size: Vec2,
) -> Anchor {
    let render_height = render_size.y.max(1.0);
    let half_collision_y = collision.y * 0.5;
    let ay = spec.feet_anchor_y + half_collision_y / render_height;
    Anchor(Vec2::new(0.0, ay))
}

/// Build the textured sprite for a character given its collision-box size.
pub fn build_character_sprite(asset: &CharacterSpriteAsset, collision: Vec2) -> Sprite {
    build_character_sprite_with_render_size(asset, sprite_render_size(&asset.spec, collision))
}

/// Build the textured sprite with an explicit presentation render size.
pub fn build_character_sprite_with_render_size(
    asset: &CharacterSpriteAsset,
    render_size: Vec2,
) -> Sprite {
    let mut sprite = Sprite::from_atlas_image(
        asset.texture.clone(),
        bevy::image::TextureAtlas {
            layout: asset.layout.clone(),
            index: asset.spec.flat_index(CharacterAnim::Idle, 0),
        },
    );
    sprite.custom_size = Some(render_size);
    sprite
}

impl CharacterSheetSpec {
    fn row_index(&self, anim: CharacterAnim) -> Option<usize> {
        self.rows.iter().position(|(row_anim, _)| *row_anim == anim)
    }

    pub fn resolve_anim(&self, anim: CharacterAnim) -> CharacterAnim {
        if self.row_index(anim).is_some() {
            return anim;
        }
        if matches!(anim, CharacterAnim::LedgeClimb)
            && self.row_index(CharacterAnim::LedgeGrab).is_some()
        {
            return CharacterAnim::LedgeGrab;
        }
        CharacterAnim::Idle
    }

    pub(super) fn row(&self, anim: CharacterAnim) -> &AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        &self.rows[idx].1
    }

    /// Build the atlas layout for this sheet. Accounts for `y_offset`
    /// so multiple specs can share one PNG (e.g. lab-props), each
    /// addressing its own row block.
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        // Atlas image size has to cover every cell — derive it from
        // the rects when we have them (so inter-frame padding is
        // included), and fall back to grid math (cells = frame_w ×
        // frame_h, label inset on the left) otherwise.
        let (total_w, total_h) = atlas_extent(self);
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (_, row) in self.rows.iter() {
            // Authoritative path: use the RON's per-frame rects. The
            // generator emits the EXACT pixel coords of every frame
            // (including padding between cells), so any drift caused
            // by inter-frame padding, label-column width changes, or
            // row-stride ≠ frame_height vanishes.
            if let Some(rects) = row.frame_rects.as_ref() {
                for r in rects.iter().take(row.frame_count) {
                    layout.add_texture(inset_rect(*r, inset));
                }
                continue;
            }
            // Legacy path: grid math, using the AUTHORED `row_index`
            // so dropping intermediate rows doesn't shift later rows
            // upward into the wrong band of pixels.
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = self.y_offset + row.row_index * self.frame_height;
                let cell = URect {
                    min: UVec2::new(x, y),
                    max: UVec2::new(x + self.frame_width, y + self.frame_height),
                };
                layout.add_texture(inset_rect(cell, inset));
            }
        }
        layout
    }

    pub fn flat_index(&self, anim: CharacterAnim, frame: usize) -> usize {
        let resolved = self.resolve_anim(anim);
        let row = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        let frames_before: usize = self.rows[..row]
            .iter()
            .map(|(_, row)| row.frame_count)
            .sum();
        let max_frame = self.rows[row].1.frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    /// Pixel extent of the atlas texture addressed by this sheet spec.
    ///
    /// Custom sprite materials use this to convert a flat atlas frame index
    /// into normalized UVs without depending on Bevy's private
    /// `TextureAtlasLayout` internals. The calculation intentionally matches
    /// [`Self::build_atlas`].
    pub fn atlas_texture_size(&self) -> UVec2 {
        let (w, h) = atlas_extent(self);
        UVec2::new(w.max(1), h.max(1))
    }

    /// Return the inset pixel rect for a flat atlas index.
    ///
    /// This mirrors [`Self::build_atlas`]'s rect insertion order: rows are
    /// concatenated in spec order, and each row contributes `frame_count`
    /// rects. It gives custom materials the same frame crop used by the
    /// ordinary Bevy `Sprite` path, including the bilinear-filtering inset.
    pub fn texture_rect_for_flat_index(&self, index: usize) -> Option<URect> {
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        let mut flat = 0usize;
        for (_, row) in self.rows.iter() {
            if let Some(rects) = row.frame_rects.as_ref() {
                for rect in rects.iter().take(row.frame_count) {
                    if flat == index {
                        return Some(inset_rect(*rect, inset));
                    }
                    flat += 1;
                }
                continue;
            }
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = self.y_offset + row.row_index * self.frame_height;
                let cell = URect {
                    min: UVec2::new(x, y),
                    max: UVec2::new(x + self.frame_width, y + self.frame_height),
                };
                if flat == index {
                    return Some(inset_rect(cell, inset));
                }
                flat += 1;
            }
        }
        None
    }

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.row(anim).duration_secs
    }
}

/// Compute the atlas image extent (width, height) that covers every
/// cell, whether the spec carries per-frame rects (preferred) or
/// only grid metadata. The atlas must be at least as large as the
/// underlying PNG so URect coords don't overflow.
fn atlas_extent(spec: &CharacterSheetSpec) -> (u32, u32) {
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any_rect = false;
    for (_, row) in spec.rows.iter() {
        if let Some(rects) = row.frame_rects.as_ref() {
            for r in rects.iter().take(row.frame_count) {
                max_x = max_x.max(r.max.x);
                max_y = max_y.max(r.max.y);
                any_rect = true;
            }
        }
    }
    if any_rect {
        return (max_x, max_y);
    }
    // Grid fallback — same shape as the previous build_atlas extent
    // math (now informed by AUTHORED row_index, so dropped rows
    // don't shrink the y-coverage).
    let max_frames = spec
        .rows
        .iter()
        .map(|(_, row)| row.frame_count)
        .max()
        .unwrap_or(0) as u32;
    let max_row_index_plus_one = spec
        .rows
        .iter()
        .map(|(_, row)| row.row_index)
        .max()
        .map(|i| i + 1)
        .unwrap_or(0);
    let w = spec.label_width + max_frames * spec.frame_width;
    let h = spec.y_offset + max_row_index_plus_one * spec.frame_height;
    (w, h)
}

/// Shrink a cell by `inset` on every side so bilinear filtering at
/// the seam can't pull pixels from neighboring cells. Saturating
/// math keeps a tiny cell from inverting (min > max) on a pathological
/// inset.
fn inset_rect(r: URect, inset: u32) -> URect {
    URect {
        min: UVec2::new(r.min.x + inset, r.min.y + inset),
        max: UVec2::new(
            r.max.x.saturating_sub(inset).max(r.min.x + 1),
            r.max.y.saturating_sub(inset).max(r.min.y + 1),
        ),
    }
}


#[cfg(test)]
mod tests;
