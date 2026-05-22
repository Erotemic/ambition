//! Sprite-sheet specs for every character target plus per-spec
//! geometry helpers (`sprite_render_size`, `feet_anchor_for`,
//! `build_character_sprite`).
//!
//! # Source of truth
//!
//! Generator output (`tools/ambition_sprite2d_renderer`) writes a
//! `*_spritesheet.ron` next to each PNG. The RON manifest is the
//! canonical record for everything the generator *knows*: per-frame
//! sizing, animation rows, foot anchor. The `*_SHEET` statics in this
//! file are thin `LazyLock<CharacterSheetSpec>` wrappers that pull
//! those values from the RON on first access and combine them with
//! the gameplay-tuning fields the generator can't infer
//! (`collision_scale`, `frame_sample_inset`, `y_offset`). There is no
//! hand-typed copy of frame width / height / row counts — those would
//! drift the moment a generator re-runs.
//!
//! Callsites are unchanged from the old const era: `PIRATE_SHEET`
//! still dereferences to `&CharacterSheetSpec` via `LazyLock`'s
//! `Deref` impl. Tables that previously stored specs by value (e.g.
//! `NPC_SPRITE_REGISTRY`) now hold `&'static LazyLock<CharacterSheetSpec>`
//! references.

use std::sync::LazyLock;

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::anim::CharacterAnim;
use super::assets::CharacterSpriteAsset;
use super::registry::{NormPoint, SheetRecord};

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
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
struct SheetTuning {
    collision_scale: f32,
    feet_anchor_y_override: Option<f32>,
    frame_sample_inset: u32,
    y_offset: u32,
}

impl SheetTuning {
    const fn new(collision_scale: f32, frame_sample_inset: u32) -> Self {
        Self {
            collision_scale,
            feet_anchor_y_override: None,
            frame_sample_inset,
            y_offset: 0,
        }
    }

    const fn with_y_offset(mut self, y_offset: u32) -> Self {
        self.y_offset = y_offset;
        self
    }

    const fn with_feet_anchor_y(mut self, feet_anchor_y: f32) -> Self {
        self.feet_anchor_y_override = Some(feet_anchor_y);
        self
    }
}

/// Load + cache a `CharacterSheetSpec` from its RON manifest, combined
/// with the static gameplay tuning. Panics if the RON is missing or
/// malformed — that's a hard project invariant (the test
/// `every_spritesheet_ron_parses_into_sheet_record` catches it before
/// runtime).
fn load_spec(sprite_id: &str, tuning: &SheetTuning) -> CharacterSheetSpec {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/sprites")
        .join(format!("{sprite_id}_spritesheet.ron"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("load_spec({sprite_id}): read {}: {e}", path.display()));
    let record: SheetRecord = ron::from_str(&text)
        .unwrap_or_else(|e| panic!("load_spec({sprite_id}): parse {}: {e}", path.display()));
    spec_from_record(&record, tuning)
}

fn spec_from_record(record: &SheetRecord, tuning: &SheetTuning) -> CharacterSheetSpec {
    let rows: Vec<(CharacterAnim, AnimRow)> = record
        .rows
        .iter()
        .filter_map(|row| {
            let anim = CharacterAnim::from_name(&row.animation)?;
            Some((
                anim,
                AnimRow {
                    frame_count: row.frame_count as usize,
                    duration_secs: row.duration_secs,
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
        y_offset: tuning.y_offset,
        frame_width: record.frame_width,
        frame_height: record.frame_height,
        rows,
        collision_scale: tuning.collision_scale,
        feet_anchor_y,
        frame_sample_inset: tuning.frame_sample_inset,
    }
}

const ROBOT_TUNING: SheetTuning = SheetTuning::new(2.1, 1);
pub static ROBOT_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("robot", &ROBOT_TUNING));

/// Player-specific compact robot sheet. Rendered from
/// `tools/ambition_sprite2d_renderer/configs/player_robot.yaml`
/// (`archetype: player_compact`). Shares the same row order as
/// `ROBOT_SHEET` so animation indexing is identical — only the
/// per-frame geometry + anchor differ to match the shrunk
/// silhouette.
const PLAYER_ROBOT_TUNING: SheetTuning = SheetTuning::new(1.35, 1);
pub static PLAYER_ROBOT_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("player_robot", &PLAYER_ROBOT_TUNING));

const GOBLIN_TUNING: SheetTuning = SheetTuning::new(2.1, 1);
pub static GOBLIN_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("goblin", &GOBLIN_TUNING));

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
const ABSURD_GENERAL_TUNING: SheetTuning = SheetTuning::new(1.15, 2);
pub static ABSURD_GENERAL_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("absurd_general", &ABSURD_GENERAL_TUNING));

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
const BURNING_FLYING_SHARK_TUNING: SheetTuning = SheetTuning::new(1.4, 1);
pub static BURNING_FLYING_SHARK_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("burning_flying_shark", &BURNING_FLYING_SHARK_TUNING));

/// Pirate Admiral / Pirate Raider — both ship the same generator
/// layout (idle / walk / slash / taunt / hurt / death; 128×128
/// frames; feet_anchor_norm.y ≈ -0.375). They share one sheet spec
/// because the layout is identical even though the rendered art
/// differs. Two filenames; one indexing contract.
const PIRATE_TUNING: SheetTuning = SheetTuning::new(1.6, 1);
pub static PIRATE_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("pirate_admiral", &PIRATE_TUNING));

/// Architect — hub research / ADR-explainer NPC.
const ARCHITECT_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static ARCHITECT_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("architect", &ARCHITECT_TUNING));

/// Kernel Guide — onboarding NPC at the hub spawn area.
const KERNEL_GUIDE_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static KERNEL_GUIDE_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("kernel_guide", &KERNEL_GUIDE_TUNING));

/// Vault Keeper — persistence / save-seed NPC in the basement.
const VAULT_KEEPER_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static VAULT_KEEPER_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("vault_keeper", &VAULT_KEEPER_TUNING));

/// Interdimensional gate ring — the standing stone arch that frames
/// a portal. Two authored rows in
/// `interdimensional_gate_ring_spritesheet.yaml`:
/// - Row 0 = `idle` (8 frames × 140ms) — the always-on slow rotation
/// - Row 1 = `spin` (12 frames × 85ms) — the faster boot-spin
///
/// We borrow `CharacterAnim::Walk` as the semantic slot for the
/// `spin` row (same pattern as [`GATE_PORTAL_SHEET`]). The
/// [`crate::rooms::sync_portal_ring_rotation_system`] requests
/// `Walk` while `PortalPhase::Opening` is active and falls back to
/// `Idle` otherwise.
const GATE_RING_TUNING: SheetTuning = SheetTuning::new(1.00, 2);
pub static GATE_RING_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("interdimensional_gate_ring", &GATE_RING_TUNING));

/// Interdimensional gate portal — the shimmering surface inside the
/// ring. Three rows authored in the source PNG
/// (`interdimensional_gate_portal_spritesheet.yaml`): `opening`
/// (8 frames × 80ms = 640ms one-shot), `stable` (8 × 110ms looping),
/// `closing` (8 × 80ms one-shot). The portal's [`crate::rooms::PortalPhase`]
/// state machine drives which row to play; this spec borrows
/// existing `CharacterAnim` variants as semantic slots
/// (Idle=opening so the default boot is visible, Walk=stable for
/// the steady "ready" loop, Run=closing for the shutdown
/// one-shot). The runtime's
/// [`crate::rooms::sync_portal_sprite_animation`] system calls
/// `CharacterAnimator::request(...)` with the right variant on
/// phase change.
const GATE_PORTAL_TUNING: SheetTuning = SheetTuning::new(1.00, 2);
pub static GATE_PORTAL_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("interdimensional_gate_portal", &GATE_PORTAL_TUNING));

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

// Lab-prop sheets are special: 8 different props share one PNG
// (`creator_lab_props_spritesheet.png`) addressed by y_offset, and the
// RON manifest for that PNG isn't shaped like the per-character RONs
// (it stores props in a `props:` map keyed by prop id, not row-ordered
// animation rows). Until the generator emits a per-prop RON or the
// runtime grows a multi-prop manifest reader, these stay hand-typed.
// The single-row idle animation is identical across all 8 props.
fn lab_prop_sheet(y_offset: u32) -> CharacterSheetSpec {
    CharacterSheetSpec {
        label_width: 160,
        y_offset,
        frame_width: 128,
        frame_height: 128,
        rows: vec![(
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.140,
            },
        )],
        collision_scale: 1.00,
        feet_anchor_y: -0.500,
        frame_sample_inset: 2,
    }
}

pub static LAB_PROP_GENESIS_VAT: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(0));
#[allow(dead_code)]
pub static LAB_PROP_SPECIMEN_JAR: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(128));
pub static LAB_PROP_NEURAL_CONSOLE: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(256));
pub static LAB_PROP_RESONANCE_COIL: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(384));
pub static LAB_PROP_POWER_CORE: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(512));
pub static LAB_PROP_REPAIR_CRADLE: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(640));
#[allow(dead_code)]
pub static LAB_PROP_DRONE_CRADLE: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(768));
#[allow(dead_code)]
pub static LAB_PROP_PORTAL_CALIBRATOR: LazyLock<CharacterSheetSpec> = LazyLock::new(|| lab_prop_sheet(896));

/// Diagnostic Cart — the rail / gurney the player wakes on. Rendered
/// by the dedicated `intro_cart` tack-on target. 3 rows ship on disk
/// (idle / roll / jolt); only Idle wires here today. Frame size is
/// 192×128 (wider than tall — the cart is a prop, not a humanoid).
/// The cart authors as an NpcSpawn with `name: "Diagnostic Cart"` so
/// it picks up its sprite from `INTRO_NPC_SPRITE_REGISTRY` — same
/// path the other intro characters use. A dedicated `Prop` entity
/// type lands in a follow-up; for the v1 slice the NpcSpawn slot is
/// the lightest way to get a visible cart without engine churn.
const CART_TUNING: SheetTuning = SheetTuning::new(1.00, 2);
pub static CART_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("intro_cart", &CART_TUNING));

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
const NEWS_BOARD_TUNING: SheetTuning = SheetTuning::new(1.50, 2);
pub static NEWS_BOARD_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("news_board", &NEWS_BOARD_TUNING));

/// Creator — the researcher who wakes the player. Rendered by the
/// dedicated `creator` tack-on target (not the toon-side adapter), so
/// the sheet is wider (160×192) and starts after a 108px label column.
/// 4 animation rows ship on disk (idle/speak/gesture/walk); only Idle
/// is wired here today — when CharacterAnim grows a Talk variant,
/// the speak row at index 1 lands automatically because the renderer
/// looks the row up by enum discriminant.
const CREATOR_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static CREATOR_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("creator", &CREATOR_TUNING));

/// Fascist Enforcer — uniformed Nazi-dimension raid grunt. Toon-side
/// adapter render; the dedicated `fascist_enforcer` archetype reads
/// as "officer cap + storm uniform + rifle" so it's the correct
/// silhouette for the intro Nazi salvage guard (the Absurd General
/// placeholder was a satirical hub NPC, not a raid trooper).
const FASCIST_ENFORCER_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static FASCIST_ENFORCER_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("fascist_enforcer", &FASCIST_ENFORCER_TUNING));

/// Oiler — street mechanic / Eulerian gate-keeper NPC who finds the
/// player in the drain alley after the intro escape. Toon-side adapter
/// render; matches the Oiler review config (configs/review/oiler.yaml).
const OILER_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static OILER_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("oiler", &OILER_TUNING));

/// Erdish — wandering graph-theory eccentric. Toon-side adapter render;
/// matches the Erdish review config (configs/review/erdish.yaml).
const ERDISH_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static ERDISH_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("erdish", &ERDISH_TUNING));

/// Alice — unofficial cartographer. Toon-side adapter render; the
/// `alice_cryptographer` archetype reads as "cautious local with a
/// scarf and a sealed envelope". Matches the Alice review config
/// (configs/review/alice.yaml) and the
/// `alice_spritesheet.yaml`/`.png` pair that ships in
/// `crates/ambition_sandbox/assets/sprites/`.
const ALICE_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static ALICE_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("alice", &ALICE_TUNING));

/// Bob — field cartographer. Toon-side adapter render; the
/// `bob_engineer` archetype is wider in the shoulders (engineer
/// silhouette) so the frame is correspondingly wider than Alice's.
/// Matches the Bob review config (configs/review/bob.yaml).
const BOB_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static BOB_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("bob", &BOB_TUNING));

/// Merchant Prototype — placeholder shopkeeper NPC.
const MERCHANT_PROTOTYPE_TUNING: SheetTuning = SheetTuning::new(1.10, 2);
pub static MERCHANT_PROTOTYPE_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("merchant_prototype", &MERCHANT_PROTOTYPE_TUNING));

// ─────────────────────────────────────────────────────────────────
// Robot-target faction-leader sheets. Tightly packed (no inter-frame
// padding), `feet_anchor_norm.y ≈ -0.328`, body fills ~83% of the
// row pitch → `collision_scale ≈ 1.20`.
// ─────────────────────────────────────────────────────────────────

/// Fretjaw — Goblin Cantina chieftain (faction leader of the
/// rowdy training-pit faction). Goblin-target generator output:
/// label_w=120, no inter-frame padding, body fills ~86% of the
/// 128-tall row.
const GOBLIN_CANTINA_CHIEFTAIN_TUNING: SheetTuning = SheetTuning::new(1.16, 1);
pub static GOBLIN_CANTINA_CHIEFTAIN_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("goblin_cantina_chieftain", &GOBLIN_CANTINA_CHIEFTAIN_TUNING));

/// Captain Pulse — Pulse Voyagers faction leader.
const PULSE_VOYAGER_CAPTAIN_TUNING: SheetTuning = SheetTuning::new(1.20, 1);
pub static PULSE_VOYAGER_CAPTAIN_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("pulse_voyager_captain", &PULSE_VOYAGER_CAPTAIN_TUNING));

/// Chadwick Disruptor III — Tech-Bros Basement faction leader.
const TECH_BRO_DISRUPTOR_TUNING: SheetTuning = SheetTuning::new(1.20, 1);
pub static TECH_BRO_DISRUPTOR_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("tech_bro_disruptor", &TECH_BRO_DISRUPTOR_TUNING));

const SANDBAG_TUNING: SheetTuning = SheetTuning::new(1.38, 1);
pub static SANDBAG_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("sandbag", &SANDBAG_TUNING));

/// Shadow Oni Leader / Shadow Duelist — both ship the same generator
/// layout (idle / walk / run / jump / fall / slash / hit / death /
/// blink_out / blink_in / dash; 128×128 frames, no inter-frame
/// padding, label_width = 100). Mirrors the PIRATE_SHEET pattern:
/// two filenames, one indexing contract. Both ninja manifests
/// report `feet_anchor_norm.y = -0.4921875`.
const NINJA_TUNING: SheetTuning = SheetTuning::new(1.5, 1);
pub static NINJA_SHEET: LazyLock<CharacterSheetSpec> = LazyLock::new(|| load_spec("ninja", &NINJA_TUNING));

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

    pub(super) fn row(&self, anim: CharacterAnim) -> AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        self.rows[idx].1
    }

    /// Build the atlas layout for this sheet. Accounts for `y_offset`
    /// so multiple specs can share one PNG (e.g. lab-props), each
    /// addressing its own row block.
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        let max_frames = self
            .rows
            .iter()
            .map(|(_, row)| row.frame_count)
            .max()
            .unwrap_or(0) as u32;
        let total_w = self.label_width + max_frames * self.frame_width;
        // `total_h` includes `y_offset` so the atlas image-size matches
        // the underlying PNG when this spec is for a sub-block of a
        // larger sheet (lab props use this — each prop spec y_offsets
        // into one shared PNG).
        let total_h = self.y_offset + self.rows.len() as u32 * self.frame_height;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (row_idx, (_, row)) in self.rows.iter().enumerate() {
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = self.y_offset + row_idx as u32 * self.frame_height;
                // Inset on every side so bilinear filtering at the frame
                // boundary cannot pull pixels from the next cell.
                let min = UVec2::new(x + inset, y + inset);
                let max = UVec2::new(x + self.frame_width - inset, y + self.frame_height - inset);
                layout.add_texture(URect { min, max });
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

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.row(anim).duration_secs
    }
}
