//! Catalog entry + preset shapes. Deserialize-only mirrors of the
//! `Brain` / `ActionSet` configs in `crate::brain`. Kept separate from
//! the runtime types so:
//!
//! 1. Brain cfgs can keep non-Deserialize fields (per-actor `state`,
//!    `Vec<f32>` history buffers) without leaking serde into the
//!    tick path.
//! 2. RON authoring follows a stable, documented shape that doesn't
//!    move when an unrelated runtime detail changes.

use ambition_engine_core as ae;
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
#[derive(Clone, Debug, Deserialize, PartialEq)]
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

/// Surface-momentum motion feel, authored on the catalog row (Q21). The
/// gameplay-side **mirror** of the serde-free kernel struct
/// [`ae::MomentumParams`](ambition_engine_core::MomentumParams):
/// the kernel stays serde-free (its doc's contract), so this Deserialize twin
/// lives here and hydrates via [`to_kernel`](MomentumParamsSpec::to_kernel).
///
/// Every field carries a `#[serde(default = ...)]` matching the kernel's
/// `Default` value-for-value, so authored RON omits whatever it doesn't tune —
/// `momentum: Some(())` alone yields the kernel defaults. A character row that
/// carries this field opts its body into `MotionModel::SurfaceMomentum` (the
/// surface-follower solver); a row without it stays on the axis-swept path.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct MomentumParamsSpec {
    #[serde(default = "md_ground_accel")]
    pub ground_accel: f32,
    #[serde(default = "md_brake")]
    pub brake: f32,
    #[serde(default = "md_friction")]
    pub friction: f32,
    #[serde(default = "md_slope_factor")]
    pub slope_factor: f32,
    #[serde(default = "md_top_speed")]
    pub top_speed: f32,
    #[serde(default = "md_air_accel")]
    pub air_accel: f32,
    #[serde(default = "md_jump_speed")]
    pub jump_speed: f32,
    #[serde(default = "md_stick_factor")]
    pub stick_factor: f32,
    #[serde(default = "md_min_stick_speed")]
    pub min_stick_speed: f32,
}

// Per-field defaults, read straight off the kernel `Default` so the two never
// drift (the kernel is the single source of truth for the feel baseline).
fn md_ground_accel() -> f32 {
    ae::MomentumParams::default().ground_accel
}
fn md_brake() -> f32 {
    ae::MomentumParams::default().brake
}
fn md_friction() -> f32 {
    ae::MomentumParams::default().friction
}
fn md_slope_factor() -> f32 {
    ae::MomentumParams::default().slope_factor
}
fn md_top_speed() -> f32 {
    ae::MomentumParams::default().top_speed
}
fn md_air_accel() -> f32 {
    ae::MomentumParams::default().air_accel
}
fn md_jump_speed() -> f32 {
    ae::MomentumParams::default().jump_speed
}
fn md_stick_factor() -> f32 {
    ae::MomentumParams::default().stick_factor
}
fn md_min_stick_speed() -> f32 {
    ae::MomentumParams::default().min_stick_speed
}

impl Default for MomentumParamsSpec {
    fn default() -> Self {
        // Mirrors `ae::MomentumParams::default()` field-for-field.
        Self {
            ground_accel: md_ground_accel(),
            brake: md_brake(),
            friction: md_friction(),
            slope_factor: md_slope_factor(),
            top_speed: md_top_speed(),
            air_accel: md_air_accel(),
            jump_speed: md_jump_speed(),
            stick_factor: md_stick_factor(),
            min_stick_speed: md_min_stick_speed(),
        }
    }
}

impl MomentumParamsSpec {
    /// Hydrate into the serde-free kernel struct the surface solver consumes.
    pub fn to_kernel(&self) -> ae::MomentumParams {
        ae::MomentumParams {
            ground_accel: self.ground_accel,
            brake: self.brake,
            friction: self.friction,
            slope_factor: self.slope_factor,
            top_speed: self.top_speed,
            air_accel: self.air_accel,
            jump_speed: self.jump_speed,
            stick_factor: self.stick_factor,
            min_stick_speed: self.min_stick_speed,
        }
    }
}

/// A named movement/combat capability preset, authored on the catalog row.
///
/// The gameplay-side **mirror** of the engine's [`ae::AbilitySet`] presets: the
/// kernel owns the actual bool set; this Deserialize twin names one of the
/// blessed presets so an authored RON row stays a single word
/// (`abilities: Some(Basic)`) instead of twenty-six booleans.
///
/// A character row that carries this field spawns its PLAYABLE body with that
/// capability set instead of the session's shared `EditableAbilitySet` — the
/// per-character analogue of [`momentum`](CharacterCatalogEntry::momentum). A row
/// without it (the default) keeps the shared sandbox set, so every existing
/// character is untouched. This is how a restricted-kit demo character (classic
/// run + jump, no blink/dash/wall/fly) is authored without forcing the whole
/// multi-game host into that reduced kit.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum AbilityKitSpec {
    /// Classic first-room kit: run + (variable) jump + reset, nothing else.
    /// [`ae::AbilitySet::basic()`](ambition_engine_core::AbilitySet::basic).
    Basic,
    /// The engine's deliberately sane initial endgame subset.
    /// [`ae::AbilitySet::sane_subset()`](ambition_engine_core::AbilitySet::sane_subset).
    SaneSubset,
    /// Every currently implemented verb.
    /// [`ae::AbilitySet::sandbox_all()`](ambition_engine_core::AbilitySet::sandbox_all).
    SandboxAll,
}

impl AbilityKitSpec {
    /// Hydrate into the engine capability set the movement kernel consumes.
    pub fn to_ability_set(self) -> ae::AbilitySet {
        match self {
            Self::Basic => ae::AbilitySet::basic(),
            Self::SaneSubset => ae::AbilitySet::sane_subset(),
            Self::SandboxAll => ae::AbilitySet::sandbox_all(),
        }
    }
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

/// Where a character's PLAYABLE action/combat kit comes from when the home box
/// WEARS it — the catalog, or the host game's code.
///
/// This exists to separate two concepts that are NOT the same: "which row is the
/// content default" and "whose kit does the playable body use". The general case
/// is [`Authored`](Self::Authored): the worn kit IS the row's `default_action_set`
/// (a demo speedster, a pirate, a goblin — a body earns its moveset from the same
/// catalog row that names its sprite and brain). The exception is
/// [`HostCode`](Self::HostCode): a protagonist whose combat abilities are a
/// runtime-mutable code concern — an [`crate`]-external `AbilitySet` / progression
/// / dev-toggle system — rather than static catalog data. Such a row keeps the
/// body's code-built kit; its `default_action_set` still exists (for its Hall
/// pedestal or when it is spawned as an NPC) but does NOT define the playable kit.
///
/// A standalone game whose default protagonist wants its OWN authored profile
/// simply leaves this `Authored` (the default). Only a game that layers a code
/// kit on its protagonist (Ambition's robot) marks that row `HostCode`. This
/// selector does not replace the body's movement/progression `AbilitySet`; it
/// owns the ActionSet, derived moveset, and direct combat adjuncts such as the
/// host charge-projectile capability.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum PlayableKitSource {
    /// The worn action/combat kit is the row's `default_action_set`.
    #[default]
    Authored,
    /// The worn body uses its host-code-built action/combat kit; the catalog
    /// action set does not define the playable kit for this row.
    HostCode,
}

/// One character entry in `character_catalog.ron`.
#[allow(
    dead_code,
    reason = "Public catalog schema; future consumers (Hall layout generator, dialogue UI, faction-aware spawn rules) read tier / body_kind / composition / tags. Today the validator + sprite loader use a subset."
)]
#[derive(Clone, Debug, Deserialize, PartialEq)]
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
    /// Whose action/combat kit the PLAYABLE body uses when the home box wears
    /// this character:
    /// the catalog's `default_action_set` ([`PlayableKitSource::Authored`], the
    /// default) or the host game's code-built kit ([`PlayableKitSource::HostCode`],
    /// a protagonist with a runtime `AbilitySet`/progression kit). Defaults to
    /// `Authored`, so every existing standalone row wears its own authored kit;
    /// only a host protagonist marks itself `HostCode`. See [`PlayableKitSource`].
    #[serde(default)]
    pub playable_kit: PlayableKitSource,
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
    /// Surface-momentum motion feel (Q21 / S2). `Some` opts this character's
    /// body into `MotionModel::SurfaceMomentum` — the surface-follower solver
    /// (slopes, loops, momentum) — whether it is spawned as an NPC or WORN by
    /// the player. `None` (the default) keeps the body on the axis-swept path,
    /// so every existing character is untouched.
    #[serde(default)]
    pub momentum: Option<MomentumParamsSpec>,
    /// Playable capability set (run / jump / blink / dash / wall / fly / …).
    /// `Some` overrides the session's shared `EditableAbilitySet` for a body that
    /// WEARS this character — the per-character analogue of [`momentum`](Self::momentum).
    /// `None` (the default) keeps the shared sandbox set, so every existing row is
    /// untouched. A restricted-kit demo character authors e.g. `Some(Basic)` here
    /// (classic run + jump) instead of forcing the whole host into that kit. See
    /// [`AbilityKitSpec`].
    #[serde(default)]
    pub abilities: Option<AbilityKitSpec>,
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
///
/// Keep the open `Special(String)` hatch here too: the catalog is an
/// authoring surface, so it must be able to reach every content-defined
/// technique the runtime action set can emit.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum SpecialPreset {
    Special(String),
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
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CharacterCatalogData {
    pub brain_presets: BTreeMap<String, BrainPreset>,
    pub action_set_presets: BTreeMap<String, ActionSetPreset>,
    pub characters: BTreeMap<String, CharacterCatalogEntry>,
}

#[cfg(test)]
mod momentum_spec_tests {
    use super::*;

    #[test]
    fn omitted_fields_inherit_the_kernel_defaults() {
        // Authoring only what it tunes (Sanic's fast profile) leaves every
        // other field at the kernel `Default` — the Q21 contract.
        let spec: MomentumParamsSpec =
            ron::from_str("(ground_accel: 900.0, top_speed: 1200.0, jump_speed: 700.0)")
                .expect("partial momentum spec should deserialize");
        let k = spec.to_kernel();
        let d = ae::MomentumParams::default();
        assert_eq!(k.ground_accel, 900.0, "tuned field wins");
        assert_eq!(k.top_speed, 1200.0);
        assert_eq!(k.jump_speed, 700.0);
        // Untouched fields match the kernel baseline value-for-value.
        assert_eq!(k.brake, d.brake);
        assert_eq!(k.friction, d.friction);
        assert_eq!(k.slope_factor, d.slope_factor);
        assert_eq!(k.air_accel, d.air_accel);
        assert_eq!(k.stick_factor, d.stick_factor);
        assert_eq!(k.min_stick_speed, d.min_stick_speed);
    }

    #[test]
    fn empty_spec_is_the_kernel_default() {
        let spec: MomentumParamsSpec = ron::from_str("()").expect("empty spec ok");
        assert_eq!(spec.to_kernel(), ae::MomentumParams::default());
    }
}
