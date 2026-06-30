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

/// Per-character sprite gameplay tuning, authored in the catalog row.
///
/// The generated `*_spritesheet.ron` manifest carries everything the
/// sprite RENDERER knows (frame grid, rows, feet anchor); these are
/// the gameplay-side knobs it can't infer. Rows without this field
/// use middle-of-the-road defaults (`collision_scale: 1.5`,
/// `frame_sample_inset: 1`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct SpriteTuningSpec {
    /// render_size = aabb_size * collision_scale (the sprite is drawn
    /// larger than the collision box so silhouettes read correctly).
    pub collision_scale: f32,
    /// Pixels trimmed from each frame edge when sampling, to drop
    /// generator border bleed.
    pub frame_sample_inset: u32,
    /// Override for the manifest's `feet_anchor_norm.y` when the
    /// generated anchor doesn't sit actors on the floor correctly.
    #[serde(default)]
    pub feet_anchor_y: Option<f32>,
}

/// An occasion on which a character may speak a one-line speech bubble.
/// Each variant maps to a named pool on [`CharacterBarks`]; the firing
/// system for that occasion picks (and rotates through) lines from the
/// matching pool. Heterogeneous by design — some are events (struck,
/// provoked), some are ambient states (idling, on display) — but the data
/// model is uniform so all of a character's voice lives in one place.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum BarkSituation {
    /// Struck in combat — a peaceful NPC's retaliation warning, or an
    /// enemy/boss yelping under a hit. Event-driven; rotates with strikes.
    OnHit,
    /// The moment a peaceful NPC crosses its hostility threshold and turns
    /// to fight. Event-driven; fires once.
    Provoked,
    /// Ambient muttering while idling — a peaceful NPC standing around, or a
    /// boss between strikes. Timer-driven; rotates.
    Idle,
    /// On display in the Hall of Characters: the character's fun, often
    /// self-aware gallery line. Timer-driven; rotates.
    Hall,
}

/// Per-character speech-bubble pools, one list per [`BarkSituation`]. All
/// pools default empty — an empty pool means "no authored line for that
/// occasion", and the firing system falls back (generic mob lines for
/// `OnHit` / `Provoked`, silence for `Idle` / `Hall`).
///
/// Authored in the catalog row so a character's voice lives with its
/// identity: every system that spawns the character — a room placement, the
/// peaceful→hostile flip, the Hall gallery — draws from the same lines.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct CharacterBarks {
    /// Lines when struck in combat. Rotates with strike count.
    #[serde(default)]
    pub on_hit: Vec<String>,
    /// Line(s) when a peaceful NPC turns hostile. Usually one.
    #[serde(default)]
    pub provoked: Vec<String>,
    /// Ambient idle muttering.
    #[serde(default)]
    pub idle: Vec<String>,
    /// Hall-of-Characters gallery lines (fun / self-aware).
    #[serde(default)]
    pub hall: Vec<String>,
}

impl CharacterBarks {
    /// The line pool for `situation` (possibly empty).
    pub fn pool(&self, situation: BarkSituation) -> &[String] {
        match situation {
            BarkSituation::OnHit => &self.on_hit,
            BarkSituation::Provoked => &self.provoked,
            BarkSituation::Idle => &self.idle,
            BarkSituation::Hall => &self.hall,
        }
    }

    /// Pick a line for `situation`, rotating by `rotation` so repeated barks
    /// cycle the pool. `None` when the pool is empty (caller falls back).
    pub fn pick(&self, situation: BarkSituation, rotation: u32) -> Option<&str> {
        let pool = self.pool(situation);
        if pool.is_empty() {
            return None;
        }
        Some(pool[(rotation as usize) % pool.len()].as_str())
    }
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
    /// Gameplay sprite tuning (collision scale / sample inset / feet
    /// anchor override). `None` = defaults. Replaces the old
    /// hardcoded `*_SHEET` statics in `character_sprites/sheets.rs`.
    #[serde(default)]
    pub sprite_tuning: Option<SpriteTuningSpec>,
    /// Speech-bubble lines for this character, keyed by occasion. Defaults
    /// to all-empty (silent). The single source of truth for a character's
    /// voice — supersedes the hardcoded `features::npcs` match tables and the
    /// `CombatBanterRegistry` content installers, which remain only as a
    /// fallback until every row is populated.
    #[serde(default)]
    pub barks: CharacterBarks,
    /// Yarn node id for this character's Hall-of-Characters conversation (the
    /// line shown when the player Inspects its pedestal). `None` = no hall
    /// dialogue; the pedestal is inspect-silent. Folded into the dialogue
    /// validator's known-id set so authored nodes are checked, and read by
    /// the hall generator to populate each pedestal's `dialogue_id`.
    #[serde(default)]
    pub hall_dialogue_id: Option<String>,
}

impl CharacterCatalogEntry {
    /// The sheet-manifest record key for this character: the manifest
    /// filename root (e.g. `sprites/pirate_admiral_spritesheet.ron`
    /// -> `pirate_admiral`). Multiple catalog ids that point at the SAME
    /// `manifest` path share one generated sheet (texture + record both);
    /// each character with its own art reads its own manifest. (Cross-id
    /// atlas borrowing was removed — it broke once sheets became
    /// per-frame alpha-trimmed: own texture + a foreign sheet's rects
    /// misaligned the animation.)
    pub fn manifest_target(&self) -> Option<&str> {
        let file = self.manifest.rsplit('/').next()?;
        file.strip_suffix("_spritesheet.ron")
    }
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
    /// Lively flyer (perch/fly/walk + land-by-player when peaceful; stalk/dive/
    /// recover when aggressive). `aggressiveness == 0` = peaceful bird.
    Aerial {
        aggressiveness: f32,
        cruise_speed: f32,
        dive_speed: f32,
        aggro_radius: f32,
        attack_range: f32,
        roam_radius: f32,
    },
    BossPattern {
        aggressiveness: f32,
        encounter_id: String,
    },
    /// Smash-brawl reactive fighter (observe → mode → action → difficulty
    /// → emit). The strong, never-cheats melee/zoner brain — it perceives
    /// only a `BrainSnapshot` and acts only through the actor's `ActionSet`,
    /// the same seam the player uses. Always hostile by construction; the
    /// encounter swaps this in when the player picks "challenge". The
    /// `difficulty` floats are the fairness knobs (reaction lag, commit
    /// probability, aim accuracy).
    Smash {
        aggro_radius: f32,
        engage_distance: f32,
        attack_range: f32,
        too_close_distance: f32,
        chase_speed: f32,
        retreat_speed: f32,
        crowding_threshold: f32,
        dash_to_close: bool,
        reaction_delay_s: f32,
        commit_probability: f32,
        accuracy: f32,
        mash_speed_hz: f32,
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
