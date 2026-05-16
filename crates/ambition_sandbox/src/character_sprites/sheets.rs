//! Sprite-sheet specs for every character target plus per-spec
//! geometry helpers (`sprite_render_size`, `feet_anchor_for`,
//! `build_character_sprite`).
//!
//! The frame counts, durations, label widths, and `feet_anchor_y`
//! values are kept in sync with `tools/ambition_sprite2d_renderer`
//! output. After regenerating sheets, mirror the new YAML headers +
//! body_metrics here. When the runtime gains a YAML loader for the
//! `body_metrics` field, these constants can be removed.

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::anim::CharacterAnim;
use super::assets::CharacterSpriteAsset;

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for one of the generated sheets.
///
/// Frames are 128x128 with a per-row label strip on the left whose width
/// differs between targets. Rows are sparse and ordered exactly as the
/// generator emits them, so a sandbag can list only idle/hit/death while the
/// player can still list the full movement/combat set.
///
/// Tuning fields (`collision_scale`, `feet_anchor_y`, `frame_sample_inset`)
/// live per-spec so each target can be tuned without touching globals —
/// the prior version used module-level constants which forced identical
/// scale/anchor across robot and goblin even though their rendered bodies
/// occupy different fractions of the 128px frame.
#[derive(Clone, Copy, Debug)]
pub struct CharacterSheetSpec {
    pub label_width: u32,
    /// Pixel offset from the top of the sheet PNG before the first row.
    /// Used to share one PNG across multiple sprite specs that each take
    /// a different row block — e.g. the lab-props sheet has 8 props
    /// stacked vertically, each addressed by its own const with
    /// `y_offset = N * frame_height`. Defaults to 0 for sheets whose
    /// row 0 starts at the top of the image.
    pub y_offset: u32,
    /// Per-frame width in source-image pixels. The generator now crops
    /// each sheet to the union of opaque-pixel bboxes across every frame,
    /// so this is *not* always 128 anymore — robot is 120, goblin 121.
    pub frame_width: u32,
    pub frame_height: u32,
    pub rows: &'static [(CharacterAnim, AnimRow)],
    /// Multiplier applied to the entity's collision-box max dimension to
    /// derive the rendered sprite's height. Width is derived from the
    /// cropped frame's aspect ratio so the character isn't squashed.
    pub collision_scale: f32,
    /// Sprite anchor y (normalized; negative shifts the sprite up so feet
    /// land near the collision-box bottom).
    pub feet_anchor_y: f32,
    /// Pixel inset on every URect to prevent bilinear filtering from
    /// pulling neighboring frame pixels at the seam.
    pub frame_sample_inset: u32,
}

pub const ROBOT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    // The new directional-attack rows extend the union-bbox crop back
    // out to 128×128 (overhead swings + spinning aerials reach the
    // canvas edges). Re-confirm against the regenerated manifest after
    // any animation edit that widens the silhouette envelope.
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.065,
            },
        ),
        // Hover / free-flight pose with jet flames at the feet. Sits
        // after Dash because the robot config lists `hover` after
        // `dash`; PNG row order is the source of truth, so any
        // reorder here must match a regenerated sheet.
        (
            CharacterAnim::Fly,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.078,
            },
        ),
        // Held ledge-grab dangle.
        (
            CharacterAnim::LedgeGrab,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.100,
            },
        ),
        // ── Traversal polish rows (appended; PNG row order matches
        // `configs/robot.yaml`).
        (
            CharacterAnim::DashStartup,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.050,
            },
        ),
        (
            CharacterAnim::LandHard,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::LandRecovery,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::WallGrab,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::LedgeClimb,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.100,
            },
        ),
        (
            CharacterAnim::LedgeGetup,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::FloatGlide,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        // ── Directional sword attacks (Marth/Lucina shapes).
        (
            CharacterAnim::AttackSide,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AttackUp,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AttackDown,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AirNeutral,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.060,
            },
        ),
        (
            CharacterAnim::AirForward,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::AirBack,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::AirDown,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.070,
            },
        ),
        (
            CharacterAnim::AirUp,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.320,
    frame_sample_inset: 1,
};

/// Player-specific compact robot sheet. Rendered from
/// `tools/ambition_sprite2d_renderer/configs/player_robot.yaml`
/// (`archetype: player_compact`). Shares the same row order as
/// `ROBOT_SHEET` so animation indexing is identical — only the
/// per-frame geometry + anchor differ to match the shrunk
/// silhouette.
pub const PLAYER_ROBOT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    // Union-bbox crop of the compact player sheet (was 128×128 source).
    // Re-confirm against the regenerated manifest after any animation
    // edit that widens the silhouette envelope.
    frame_width: 128,
    frame_height: 125,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::Fly,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.078,
            },
        ),
        (
            CharacterAnim::LedgeGrab,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.100,
            },
        ),
        (
            CharacterAnim::DashStartup,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.050,
            },
        ),
        (
            CharacterAnim::LandHard,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::LandRecovery,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::WallGrab,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::LedgeClimb,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.100,
            },
        ),
        (
            CharacterAnim::LedgeGetup,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::FloatGlide,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::AttackSide,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AttackUp,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AttackDown,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.065,
            },
        ),
        (
            CharacterAnim::AirNeutral,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.060,
            },
        ),
        (
            CharacterAnim::AirForward,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::AirBack,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::AirDown,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.070,
            },
        ),
        (
            CharacterAnim::AirUp,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.062,
            },
        ),
    ],
    // The compact silhouette fills more of its (smaller) frame than the
    // base robot does, so a smaller `collision_scale` keeps the rendered
    // sprite close in size to the 30×48 collider rather than oversizing
    // the placeholder. Tuned against the player_robot manifest's
    // body_pixel_bbox (≈62×89 inside the 128×125 crop).
    collision_scale: 1.35,
    // Manifest reports feet_anchor_norm.y = -0.340.
    feet_anchor_y: -0.340,
    frame_sample_inset: 1,
};

pub const GOBLIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    // After the gen2d union-bbox crop the goblin sheet is 121x127.
    frame_width: 121,
    frame_height: 127,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.065,
            },
        ),
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.350,
    frame_sample_inset: 1,
};

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
pub const ABSURD_GENERAL_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 116,
    y_offset: 0,
    // Pitch values: each frame's content is 120×116, but the
    // generator reserves 4 extra pixels on the right and bottom
    // edges for inter-frame padding. Sampling at the pitch with
    // `frame_sample_inset: 2` keeps bilinear filtering inside the
    // frame interior even with the wider gap.
    frame_width: 124,
    frame_height: 120,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    // The General's body_pixel_bbox covers ~95% of the 116-tall
    // frame (the whole "uniformed officer" silhouette is in-frame),
    // so we want the rendered quad height to be barely larger than
    // the LDtk-authored collision box. Robot/Goblin sit around 2.1
    // because their generator leaves big transparent margins; the
    // General has almost no margin so 1.1 keeps the silhouette on
    // scale with other characters.
    collision_scale: 1.15,
    // Body metrics from the generator: feet_pixel.y = 113 in a
    // 116-tall frame → normalized −0.474 from frame center. Match
    // that here so the General's boots land on the alcove floor
    // instead of hovering above it.
    feet_anchor_y: -0.474,
    frame_sample_inset: 2,
};

// ─────────────────────────────────────────────────────────────────
// Toon-target NPC sheets — share the generator's 4-px inter-frame
// padding (col_pitch = content_w + 4, row_pitch = content_h + 4) and
// `feet_anchor_norm.y ≈ -0.47` from `body_metrics`. We declare only
// `Idle` here; rows added later (Walk/Talk) need to land at PNG row
// indices 1, 2, … in order, since `build_atlas` walks rows
// sequentially. `collision_scale ≈ 1 / (body_h / row_pitch)` keeps
// the silhouette scaled to the LDtk collision box.
// ─────────────────────────────────────────────────────────────────

/// Pirate Admiral / Pirate Raider — both ship the same generator
/// layout (idle / walk / slash / taunt / hurt / death; 128×128
/// frames; feet_anchor_norm.y ≈ -0.375). They share one sheet spec
/// because the layout is identical even though the rendered art
/// differs. Two filenames; one indexing contract.
pub const PIRATE_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.085,
            },
        ),
        (
            CharacterAnim::Taunt,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.100,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
    ],
    // Generator leaves enough headroom that 1.6 lines the silhouette
    // up roughly to the LDtk collision box (matches the goblin/robot
    // ballpark for similarly-sized characters).
    collision_scale: 1.6,
    // From both pirate manifests: feet_anchor_norm.y = -0.375.
    feet_anchor_y: -0.375,
    frame_sample_inset: 1,
};

/// Architect — hub research / ADR-explainer NPC.
pub const ARCHITECT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 116,
    y_offset: 0,
    // body_metrics frame=97×114, +4px padding both axes → 101×118.
    frame_width: 101,
    frame_height: 118,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.465,
    frame_sample_inset: 2,
};

/// Kernel Guide — onboarding NPC at the hub spawn area.
pub const KERNEL_GUIDE_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 116,
    y_offset: 0,
    // body_metrics frame=89×97, +4px padding → 93×101.
    frame_width: 93,
    frame_height: 101,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.469,
    frame_sample_inset: 2,
};

/// Vault Keeper — persistence / save-seed NPC in the basement.
pub const VAULT_KEEPER_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 116,
    y_offset: 0,
    // body_metrics frame=99×116, +4px padding → 103×120.
    frame_width: 103,
    frame_height: 120,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.474,
    frame_sample_inset: 2,
};

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

const LAB_PROP_ROWS: &[(CharacterAnim, AnimRow)] = &[(
    CharacterAnim::Idle,
    AnimRow {
        frame_count: 4,
        duration_secs: 0.140,
    },
)];

const fn lab_prop_sheet(y_offset: u32) -> CharacterSheetSpec {
    CharacterSheetSpec {
        label_width: 160,
        y_offset,
        frame_width: 128,
        frame_height: 128,
        rows: LAB_PROP_ROWS,
        collision_scale: 1.00,
        // Props sit on the floor by their bottom edge.
        feet_anchor_y: -0.500,
        frame_sample_inset: 2,
    }
}

pub const LAB_PROP_GENESIS_VAT: CharacterSheetSpec = lab_prop_sheet(0);
pub const LAB_PROP_SPECIMEN_JAR: CharacterSheetSpec = lab_prop_sheet(128);
pub const LAB_PROP_NEURAL_CONSOLE: CharacterSheetSpec = lab_prop_sheet(256);
pub const LAB_PROP_RESONANCE_COIL: CharacterSheetSpec = lab_prop_sheet(384);
pub const LAB_PROP_POWER_CORE: CharacterSheetSpec = lab_prop_sheet(512);
pub const LAB_PROP_REPAIR_CRADLE: CharacterSheetSpec = lab_prop_sheet(640);
pub const LAB_PROP_DRONE_CRADLE: CharacterSheetSpec = lab_prop_sheet(768);
pub const LAB_PROP_PORTAL_CALIBRATOR: CharacterSheetSpec = lab_prop_sheet(896);

/// Diagnostic Cart — the rail / gurney the player wakes on. Rendered
/// by the dedicated `intro_cart` tack-on target. 3 rows ship on disk
/// (idle / roll / jolt); only Idle wires here today. Frame size is
/// 192×128 (wider than tall — the cart is a prop, not a humanoid).
/// The cart authors as an NpcSpawn with `name: "Diagnostic Cart"` so
/// it picks up its sprite from `INTRO_NPC_SPRITE_REGISTRY` — same
/// path the other intro characters use. A dedicated `Prop` entity
/// type lands in a follow-up; for the v1 slice the NpcSpawn slot is
/// the lightest way to get a visible cart without engine churn.
pub const CART_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 112,
    y_offset: 0,
    frame_width: 192,
    frame_height: 128,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 6,
            duration_secs: 0.145,
        },
    )],
    collision_scale: 1.00,
    // The cart's bottom sits flush at y=128 (no overshoot from a
    // walking silhouette). Feet anchor at the bottom row, slight
    // upward bias so the player visually stands ON the cart top.
    feet_anchor_y: -0.500,
    frame_sample_inset: 2,
};

/// Creator — the researcher who wakes the player. Rendered by the
/// dedicated `creator` tack-on target (not the toon-side adapter), so
/// the sheet is wider (160×192) and starts after a 108px label column.
/// 4 animation rows ship on disk (idle/speak/gesture/walk); only Idle
/// is wired here today — when CharacterAnim grows a Talk variant,
/// the speak row at index 1 lands automatically because the renderer
/// looks the row up by enum discriminant.
pub const CREATOR_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 108,
    y_offset: 0,
    frame_width: 160,
    frame_height: 192,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 6,
            duration_secs: 0.145,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.453,
    frame_sample_inset: 2,
};

/// Fascist Enforcer — uniformed Nazi-dimension raid grunt. Toon-side
/// adapter render; the dedicated `fascist_enforcer` archetype reads
/// as "officer cap + storm uniform + rifle" so it's the correct
/// silhouette for the intro Nazi salvage guard (the Absurd General
/// placeholder was a satirical hub NPC, not a raid trooper).
pub const FASCIST_ENFORCER_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 112,
    y_offset: 0,
    // body_metrics frame=124×118, +4px padding → 128×122.
    frame_width: 128,
    frame_height: 122,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.474,
    frame_sample_inset: 2,
};

/// Oiler — street mechanic NPC who finds the player in the drain alley
/// after the intro escape. Toon-side adapter render; matches the Oiler
/// review config (configs/review/oiler.yaml).
pub const OILER_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 112,
    y_offset: 0,
    // body_metrics frame=79×100, +4px padding → 83×104.
    frame_width: 83,
    frame_height: 104,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.470,
    frame_sample_inset: 2,
};

/// Erdish — wandering graph-theory eccentric. Toon-side adapter render;
/// matches the Erdish review config (configs/review/erdish.yaml).
pub const ERDISH_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 112,
    y_offset: 0,
    // body_metrics frame=87×112, +4px padding → 91×116.
    frame_width: 91,
    frame_height: 116,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.474,
    frame_sample_inset: 2,
};

/// Merchant Prototype — placeholder shopkeeper NPC.
pub const MERCHANT_PROTOTYPE_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 116,
    y_offset: 0,
    // body_metrics frame=83×98, +4px padding → 87×102.
    frame_width: 87,
    frame_height: 102,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.10,
    feet_anchor_y: -0.469,
    frame_sample_inset: 2,
};

// ─────────────────────────────────────────────────────────────────
// Robot-target faction-leader sheets. Tightly packed (no inter-frame
// padding), `feet_anchor_norm.y ≈ -0.328`, body fills ~83% of the
// row pitch → `collision_scale ≈ 1.20`.
// ─────────────────────────────────────────────────────────────────

/// Fretjaw — Goblin Cantina chieftain (faction leader of the
/// rowdy training-pit faction). Goblin-target generator output:
/// label_w=120, no inter-frame padding, body fills ~86% of the
/// 128-tall row.
pub const GOBLIN_CANTINA_CHIEFTAIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 120,
    y_offset: 0,
    frame_width: 114,
    frame_height: 128,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.16,
    feet_anchor_y: -0.352,
    frame_sample_inset: 1,
};

/// Captain Pulse — Pulse Voyagers faction leader.
pub const PULSE_VOYAGER_CAPTAIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 120,
    y_offset: 0,
    frame_width: 110,
    frame_height: 128,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.20,
    feet_anchor_y: -0.328,
    frame_sample_inset: 1,
};

/// Chadwick Disruptor III — Tech-Bros Basement faction leader.
pub const TECH_BRO_DISRUPTOR_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 120,
    y_offset: 0,
    frame_width: 111,
    frame_height: 128,
    rows: &[(
        CharacterAnim::Idle,
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        },
    )],
    collision_scale: 1.20,
    feet_anchor_y: -0.328,
    frame_sample_inset: 1,
};

pub const SANDBAG_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.112,
            },
        ),
    ],
    collision_scale: 1.38,
    feet_anchor_y: -0.438,
    frame_sample_inset: 1,
};

/// Shadow Oni Leader / Shadow Duelist — both ship the same generator
/// layout (idle / walk / run / jump / fall / slash / hit / death /
/// blink_out / blink_in / dash; 128×128 frames, no inter-frame
/// padding, label_width = 100). Mirrors the PIRATE_SHEET pattern:
/// two filenames, one indexing contract. Both ninja manifests
/// report `feet_anchor_norm.y = -0.4921875`.
pub const NINJA_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    y_offset: 0,
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.068,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.058,
            },
        ),
    ],
    // Ninja silhouettes fill nearly the full 128px frame height (body
    // bbox h = 128 in both manifests), so a smaller scale than the
    // robot/goblin (2.1) keeps the rendered sprite roughly proportional
    // to the LDtk collision box.
    collision_scale: 1.5,
    feet_anchor_y: -0.492,
    frame_sample_inset: 1,
};

/// Per-target sprite render size. The generator's character occupies only
/// part of the 128×128 frame, so the rendered quad must be larger than
/// the collision box for the visible body to roughly match the hitbox.
///
/// TODO(gen2d-collision-aware): teach the generator to write
/// `body_pixel_extent` + `feet_y_pixel` into the spritesheet YAML and
/// load them at runtime, replacing these per-spec constants with values
/// derived from each sheet's actual rendered body. The per-spec tuning
/// already isolates the override per target so the migration is local.
pub fn sprite_render_size(spec: CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, 1.0)
}

/// Render-size helper with an additional presentation-only scale.
///
/// The collision box remains gameplay authority; this scale is only for
/// placeholder sprites while final art is still in flux.
pub fn sprite_render_size_scaled(
    spec: CharacterSheetSpec,
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

pub fn player_placeholder_render_size(spec: CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, PLAYER_PLACEHOLDER_VISUAL_SCALE)
}

/// Sprite anchor that places the rendered character's feet on the bottom
/// of the collision box (rather than at its centre).
pub fn feet_anchor_for(spec: CharacterSheetSpec, collision: Vec2) -> Anchor {
    feet_anchor_for_render_size(spec, collision, sprite_render_size(spec, collision))
}

/// Sprite anchor for an explicit render size. This keeps the feet planted when
/// presentation-only scaling makes the sprite larger than its collider.
pub fn feet_anchor_for_render_size(
    spec: CharacterSheetSpec,
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
    build_character_sprite_with_render_size(asset, sprite_render_size(asset.spec, collision))
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
