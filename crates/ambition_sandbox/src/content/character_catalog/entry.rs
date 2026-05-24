//! Catalog entry + preset shapes. Deserialize-only mirrors of the
//! `Brain` / `ActionSet` configs in `crate::brain`. Kept separate from
//! the runtime types so:
//!
//! 1. Brain cfgs can keep non-Deserialize fields (per-actor `state`,
//!    `Vec<f32>` history buffers) without leaking serde into the
//!    tick path.
//! 2. RON authoring follows a stable, documented shape that doesn't
//!    move when an unrelated runtime detail changes.

use serde::Deserialize;
use std::collections::BTreeMap;

/// What tier a character occupies in the Hall of Characters and other
/// gallery rooms. Drives layout: `MainHall` characters get standard
/// 128 px slots; `Basement` characters get the wide 256 px slots.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum CharacterTier {
    MainHall,
    Basement,
}

/// Footprint hint. Today it only influences gallery layout; the
/// runtime physics footprint still comes from the sheet spec.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum CharacterBodyKind {
    Standard,
    Wide,
    Floating,
    Crawler,
}

/// Optional composition layer for multi-part sprites (bosses, etc.).
/// Dormant scaffolding — the renderer still emits a composed sheet
/// today, so the runtime ignores this field. Reserved for future
/// layered-render work without breaking the catalog schema.
#[allow(
    dead_code,
    reason = "Reserved for future layered-rendering of multi-part sprites; ships as schema-stable scaffolding so adding composition to a catalog entry is forwards-compatible."
)]
#[derive(Clone, Debug, Deserialize)]
pub struct CompositionLayer {
    pub id: String,
    pub layer: i32,
    pub anchor_px: (f32, f32),
}

/// One character entry in `character_catalog.ron`.
#[allow(
    dead_code,
    reason = "Public catalog schema; future consumers (Hall layout generator, dialogue UI, faction-aware spawn rules) read tier / body_kind / composition / tags. Today the validator + sprite loader use a subset."
)]
#[derive(Clone, Debug, Deserialize)]
pub struct CharacterCatalogEntry {
    /// Human-facing label (UI, dialogue, debug overlays).
    pub display_name: String,
    /// Sprite-sheet image path, relative to the sandbox asset root.
    pub spritesheet: String,
    /// Sprite-sheet RON manifest path, relative to the sandbox asset
    /// root. Today the manifest carries grid/frame info; future
    /// catalog work moves animation timing here too.
    pub manifest: String,
    /// Gallery tier. Drives hall placement.
    pub tier: CharacterTier,
    /// Footprint hint. Drives slot sizing.
    pub body_kind: CharacterBodyKind,
    /// Optional layered composition (multi-part sprites). `None` for
    /// single-part characters.
    #[serde(default)]
    pub composition: Option<Vec<CompositionLayer>>,
    /// Name of the preset in `brain_presets` to apply by default.
    pub default_brain: String,
    /// Name of the preset in `action_set_presets` to apply by default.
    pub default_action_set: String,
    /// Free-form tags. Tooling filters by these (e.g. the hall
    /// generator uses `tags = ["boss"]` to fence basement entries).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Deserialize-only mirror of `brain::StateMachineCfg`. Variant
/// names match `StateMachineCfg`; fields match the corresponding
/// `*Cfg` struct field-for-field. The catalog stores the preset
/// shape (cfg only — no per-actor `state`); resolver code constructs
/// the runtime `Brain` by pairing the preset with a default `state`.
///
/// `Patrol` uses `spawn_local_x` rather than `spawn_x` to make
/// explicit that the value is an offset from the NPC's spawn
/// position, not a world-space coordinate. The resolver adds the
/// NPC's actual spawn-X at runtime.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum BrainPreset {
    StandStill,
    Patrol {
        spawn_local_x: f32,
        radius: f32,
        speed: f32,
        aggressiveness: f32,
        aggro_radius: f32,
        attack_range: f32,
    },
    Wanderer {
        speed: f32,
        climb_walls: bool,
        chatter_threshold: u8,
        chatter_window_s: f32,
        chatter_pause_s: f32,
        aggressiveness: f32,
    },
    MeleeBrute {
        aggressiveness: f32,
        aggro_radius: f32,
        attack_range: f32,
        chase_speed: f32,
    },
    Skirmisher {
        aggressiveness: f32,
        aggro_radius: f32,
        standoff_px: f32,
        strafe_speed: f32,
        fire_cooldown_s: f32,
    },
    Sniper {
        aggressiveness: f32,
        aggro_radius: f32,
        fire_cooldown_s: f32,
    },
    BossPattern {
        aggressiveness: f32,
        encounter_id: String,
    },
}

/// Locomotion style. Mirrors `brain::action_set::MoveStyleSpec`.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub enum MoveStylePreset {
    #[default]
    Walk,
    WalkHeavy,
    Hop,
    Strafe,
    Slither,
    Float,
}

/// Mirrors `brain::action_set::MeleeActionSpec` — each variant
/// carries its own windup/active/recover timing.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum MeleePreset {
    Swipe {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
    Lunge {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
        step_px: f32,
    },
    Slam {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
        hop_height_px: f32,
    },
    Bite {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
    PunchWeak {
        windup_s: f32,
        active_s: f32,
        recover_s: f32,
        damage: i32,
        reach_px: f32,
    },
}

/// Mirrors `brain::action_set::RangedActionSpec`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum RangedPreset {
    Rock { speed: f32, damage: i32 },
    Arrow { speed: f32, damage: i32 },
    Pistol { speed: f32, damage: i32 },
    Bolt { speed: f32, damage: i32 },
}

/// Mirrors `brain::action_set::SpecialActionSpec`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum SpecialPreset {
    BubbleShield,
    BossSpotlight,
}

/// Action-set preset (capability bundle). Each character points at
/// one of these by name in its `default_action_set` field.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ActionSetPreset {
    #[serde(default)]
    pub move_style: MoveStylePreset,
    #[serde(default)]
    pub melee: Option<MeleePreset>,
    #[serde(default)]
    pub ranged: Option<RangedPreset>,
    #[serde(default)]
    pub special: Option<SpecialPreset>,
}

/// Top-level RON shape: brain presets + action-set presets + the
/// character map keyed by `character_id`.
#[derive(Clone, Debug, Deserialize)]
pub struct CharacterCatalogData {
    pub brain_presets: BTreeMap<String, BrainPreset>,
    pub action_set_presets: BTreeMap<String, ActionSetPreset>,
    pub characters: BTreeMap<String, CharacterCatalogEntry>,
}
