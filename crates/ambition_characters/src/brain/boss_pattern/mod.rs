//! Boss-policy brain template.
//!
//! This module defines the content-free vocabulary for scripted boss brains:
//! movement profiles, attack profiles, pattern steps, looping/cyclic attack
//! schedules, per-boss tuning, per-actor cursors, and transient attack intent.
//!
//! [`tick_boss_pattern`] turns [`BossPatternCfg`] + [`BossPatternState`] +
//! [`BossPatternContext`] into an [`crate::actor::control::ActorControlFrame`]
//! plus [`BossAttackIntent`]. The matching move owns the execution timeline and
//! projects [`BossAttackState`]. The boss tick is separate from [`BrainSnapshot`]
//! because bosses need encounter phase, arena bounds, spawn anchors, and other
//! boss-specific context that should not bloat every actor snapshot.
//!
//! The named boss roster lives in content; this crate only defines reusable
//! behavior vocabulary such as "floor slam" or "debris rain".

#![allow(unused_imports)]
use ambition_engine_core as ae;
use bevy::prelude::Component;

// ===== Vocabulary =====

/// Movement family for a live boss actor. Encounter phases decide *when* a boss
/// is active; this profile decides how the authored actor moves while active.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossMovementProfile {
    /// Existing grounded/hovering sentinel feel: stay near the authored spawn,
    /// sway horizontally, and chase the player a little without abandoning the
    /// arena anchor.
    AnchorSway {
        x_radius: f32,
        y_bob: f32,
        x_frequency: f32,
        y_frequency: f32,
        chase_scale: f32,
        chase_limit: f32,
        speed: f32,
    },
    /// Wide airborne arcs for ship/bird-like bosses. Keeps a stable home anchor
    /// but spends more of the fight sweeping across it.
    AirSwoop {
        x_radius: f32,
        y_radius: f32,
        x_frequency: f32,
        y_frequency: f32,
        chase_scale: f32,
        chase_limit: f32,
        speed: f32,
    },
    /// Stationary giant: the entity barely moves — only a slow breath-like
    /// sway. The hands and head do the attacking via hitbox volumes computed
    /// relative to spawn; the entity itself stays nearly fixed so the large
    /// background body sprite reads as immovable.
    StationaryGiant {
        sway_amplitude: f32,
        sway_frequency: f32,
        speed: f32,
    },
}

/// Frame/route policy used by a boss movement profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossMovementFramePolicy {
    /// Move on an authored world-arena lateral lane. This deliberately uses
    /// world X / fixed arena floor semantics; it should not rotate with the
    /// controlled actor or a local acceleration frame.
    WorldArenaLateral,
    /// Move freely in authored world-arena XY space.
    WorldArenaPlanar,
}

impl BossMovementProfile {
    /// Where the movement profile wants the boss to be this tick, in
    /// world space. Pure function of (profile, spawn anchor,
    /// movement_timer, target).
    pub fn target(&self, spawn: ae::Vec2, movement_timer: f32, target_pos: ae::Vec2) -> ae::Vec2 {
        let anchor_to_player = target_pos - spawn;
        match *self {
            Self::AnchorSway {
                x_radius,
                y_bob,
                x_frequency,
                y_frequency,
                chase_scale,
                chase_limit,
                ..
            } => {
                let chase = (anchor_to_player.x * chase_scale).clamp(-chase_limit, chase_limit);
                ae::Vec2::new(
                    spawn.x + (movement_timer * x_frequency).sin() * x_radius + chase,
                    spawn.y - (movement_timer * y_frequency).sin().abs() * y_bob,
                )
            }
            Self::AirSwoop {
                x_radius,
                y_radius,
                x_frequency,
                y_frequency,
                chase_scale,
                chase_limit,
                ..
            } => {
                let chase = (anchor_to_player.x * chase_scale).clamp(-chase_limit, chase_limit);
                ae::Vec2::new(
                    spawn.x + (movement_timer * x_frequency).sin() * x_radius + chase,
                    spawn.y + (movement_timer * y_frequency).sin() * y_radius - y_radius * 0.35,
                )
            }
            Self::StationaryGiant {
                sway_amplitude,
                sway_frequency,
                ..
            } => {
                // Minimal sway around spawn — the GNU-ton body stays nearly fixed.
                let _ = anchor_to_player; // giant ignores player for movement
                ae::Vec2::new(
                    spawn.x + (movement_timer * sway_frequency).sin() * sway_amplitude,
                    spawn.y,
                )
            }
        }
    }

    /// Max speed (px/s) the profile is willing to move at this tick.
    pub fn speed(&self) -> f32 {
        match *self {
            Self::AnchorSway { speed, .. }
            | Self::AirSwoop { speed, .. }
            | Self::StationaryGiant { speed, .. } => speed,
        }
    }

    /// Movement policy for the profile's authored route.
    pub fn frame_policy(&self) -> BossMovementFramePolicy {
        match *self {
            Self::AnchorSway {
                y_bob, y_frequency, ..
            } => {
                if y_bob.abs() <= f32::EPSILON && y_frequency.abs() <= f32::EPSILON {
                    BossMovementFramePolicy::WorldArenaLateral
                } else {
                    BossMovementFramePolicy::WorldArenaPlanar
                }
            }
            Self::StationaryGiant { .. } => BossMovementFramePolicy::WorldArenaLateral,
            Self::AirSwoop { .. } => BossMovementFramePolicy::WorldArenaPlanar,
        }
    }

    /// True when this movement profile is explicitly locked to an authored
    /// world-arena lateral lane. Macro Approach / Retreat can otherwise
    /// introduce a vertical component by steering toward the player's center or
    /// a retreat anchor. Smirking Behemoth authors this as
    /// `AnchorSway(y_bob: 0, y_frequency: 0)`: it should slide along fixed
    /// arena X like the YHTBTR boss, never rise or sink toward the player.
    pub fn world_arena_lateral_only(&self) -> bool {
        self.frame_policy() == BossMovementFramePolicy::WorldArenaLateral
    }
}

/// One beat in a scripted boss attack timeline. Patterns built from these
/// steps give each boss a memorizable rhythm — explicit rest beats let the
/// player read the telegraph, react, and then learn the sequence over time.
/// Bosses without a scripted pattern fall back to the older
/// `attack_cooldown`-driven cycle through `BossBehaviorProfile::attacks`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossPatternStep {
    /// Boss is winding up: telegraph volumes draw, no damage yet.
    Telegraph {
        profile: BossAttackProfile,
        duration: f32,
        /// **BD3 — the authored anticipation.** `#[serde(default)]`, so every
        /// pre-BD3 row parses unchanged. `None` means "this attack telegraphs by
        /// duration alone", which §3 rule 5 counts as *no telegraph identity*.
        #[serde(default)]
        telegraph: Option<TelegraphSpec>,
    },
    /// Hitbox is live: active volumes draw, contact damages the player.
    Strike {
        profile: BossAttackProfile,
        duration: f32,
    },
    /// No volume. Pure breathing room so the player can reposition or punish.
    Rest { duration: f32 },
    /// **BD1 — conditional selection.** Roll ONCE, the moment the timeline is
    /// resolved, and splice the winning arm's steps in place of this one. Arms
    /// with a `when` bucket that does not hold this tick are ineligible; among the
    /// rest the roll is weighted. Zero duration: control flow, not a beat.
    ///
    /// The roll happens at RESOLUTION, not at the cursor, so a fight's timeline is
    /// a concrete list of beats before the first tick of it runs. That is what
    /// lets BD5's validator integrate a pass's threat, and what keeps the ticker's
    /// cursor arithmetic honest — a zero-duration step at the cursor is a foot-gun.
    Select { table: Vec<WeightedArm> },
    /// **BD1 — stance entry.** Jump to the named stance's steps, remembering where
    /// to come back to. Zero duration. Reaching the end of a stance returns to the
    /// step after the `Stance` that entered it.
    Stance { id: String },
}

/// **BD3 — the telegraph a player READS.** `docs/planning/engine/boss-design.md`
/// §1: *"a `telegraph` presentation event on pattern/move rows (pose row, flash,
/// sfx cue — combat-model CM5's event channel) so anticipation is AUTHORED per
/// attack, and the validator (§3) can SEE it."*
///
/// The duration of a wind-up says how LONG the player has. This says what they
/// are looking at. §3 rule 5 — *"distinct attacks must differ in telegraph (pose
/// row OR cue)"* — is a statement about THIS type, and cannot be made about a
/// duration: two attacks that both wind up for 1.2 s are not thereby
/// distinguishable, and a fight in which every attack looks the same is
/// unreadable however generous its timings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
pub struct TelegraphSpec {
    /// The animation row the boss holds while winding up. The strongest signal,
    /// because it is on screen the whole time.
    #[serde(default)]
    pub pose: Option<String>,
    /// A CM5 sfx cue id, played once on the telegraph's rising edge.
    #[serde(default)]
    pub cue: Option<String>,
    /// A CM5 vfx effect id, spawned once on the rising edge.
    #[serde(default)]
    pub vfx: Option<String>,
}

impl TelegraphSpec {
    /// Does this spec say anything at all? An all-`None` spec is authored noise
    /// and reads as absent — the validator treats it as no telegraph identity.
    pub fn is_authored(&self) -> bool {
        self.pose.is_some() || self.cue.is_some() || self.vfx.is_some()
    }

    /// §3 rule 5's IDENTITY: what makes this telegraph distinguishable from
    /// another. Two attacks may share a `vfx` flourish; they may not share both a
    /// pose and a cue, because then nothing on screen tells them apart.
    pub fn identity(&self) -> (Option<&str>, Option<&str>) {
        (self.pose.as_deref(), self.cue.as_deref())
    }
}

/// One arm of a [`BossPatternStep::Select`] table.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct WeightedArm {
    /// Relative weight among the ELIGIBLE arms. `<= 0` never wins.
    pub weight: f32,
    /// `None` = always eligible. `Some(bucket)` = eligible only while the bucket
    /// holds. A table whose arms are all ineligible resolves to nothing (the
    /// `Select` becomes zero beats), which is a legal, deliberate "do nothing here".
    #[serde(default)]
    pub when: Option<SituationBucket>,
    /// The sub-sequence spliced in when this arm wins. May itself contain
    /// `Select`/`Stance` steps; resolution recurses, depth-limited.
    pub steps: Vec<BossPatternStep>,
}

/// The CLOSED situation vocabulary a `Select` arm can gate on, computed from the
/// boss's existing context (`BossPatternContext`) — never from a private query.
///
/// Closed on purpose (`docs/planning/engine/boss-design.md` §1 BD1: *"No scripting
/// language — three enum arms"*). A bucket an authoring agent cannot name is a
/// bucket BD5's validator cannot reason about.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum SituationBucket {
    /// The target is within [`PLAYER_NEAR_PX`] of the boss.
    PlayerNear,
    /// The target is beyond [`PLAYER_NEAR_PX`].
    PlayerFar,
    /// The target is above the boss in the world frame (`+y` is down).
    PlayerAbove,
    /// The target is on the side the boss is NOT facing.
    PlayerBehind,
    /// The boss's own health fraction (`0..=1`) is below this.
    HpBelow(f32),
}

/// The near/far split, in world px. A per-game tuning number that lives here as a
/// const until a SECOND game needs a different one — grow, don't mint (§4b.3).
/// 220px is a little over two body-widths of the shipped bosses: close enough that
/// a sweep reaches, far enough that a rain has somewhere to land.
pub const PLAYER_NEAR_PX: f32 = 220.0;

/// What makes an [`InterruptRule`] fire.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum InterruptTrigger {
    /// The boss took at least `min_damage` in one tick.
    OnHitTaken { min_damage: i32 },
    /// The encounter just entered this phase (a rising edge, once per entry).
    OnPhaseEnter { phase: BossEncounterPhase },
    /// Every `every_s` seconds of pattern time.
    OnTimer { every_s: f32 },
}

/// A rule that yanks the boss out of its timeline and into a named stance.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct InterruptRule {
    pub on: InterruptTrigger,
    /// Minimum seconds between two firings of THIS rule. `0` means every chance.
    pub cooldown_s: f32,
    /// The stance id to enter. An id with no stance is a no-op and warns once at
    /// validation time (BD5); the ticker ignores it rather than panicking mid-fight.
    pub enter: String,
}

/// A full attack script for one boss phase. Loops when it reaches the end.
///
/// `stances` and `interrupts` are `#[serde(default)]`, so **every existing
/// `boss_profiles.ron` row parses unchanged** (byte-parity, as BD1's sketch
/// requires). A pattern with neither behaves exactly as it did before BD1.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct BossPattern {
    pub steps: Vec<BossPatternStep>,
    /// Named sub-sequences, entered via [`BossPatternStep::Stance`] or an
    /// [`InterruptRule`]. A `BTreeMap`, not the sketch's `HashMap`: the ticker
    /// only ever `get`s by id, but a validator and a trace both WALK it, and
    /// ADR 0023 bans std-hash iteration anywhere the sim can observe the order.
    #[serde(default)]
    pub stances: std::collections::BTreeMap<String, Vec<BossPatternStep>>,
    #[serde(default)]
    pub interrupts: Vec<InterruptRule>,
}

impl BossPattern {
    /// Total time of the phase's OWN steps, with control-flow steps counting zero
    /// and `Select` counting its heaviest arm — the worst case a designer should
    /// budget against. Stances are not included: they are entered, not scheduled.
    pub fn total_duration(&self) -> f32 {
        self.steps.iter().map(step_duration).sum()
    }
}

/// How a boss decides which attack hitbox is active each frame.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossAttackPattern {
    /// Legacy cycle: rotate through `BossBehaviorProfile::attacks` using the
    /// flat windup / active / cooldown durations on the profile. Cheap, but
    /// every attack uses the same rhythm.
    Cycle,
    /// Scripted timeline keyed off `BossEncounterPhase`. Each phase carries
    /// its own ordered list of telegraph / strike / rest beats. Missing
    /// phases fall back to `phase1`.
    Scripted {
        intro: BossPattern,
        phase1: BossPattern,
        transition: BossPattern,
        phase2: BossPattern,
        enrage: BossPattern,
    },
}

impl BossAttackPattern {
    pub fn pattern_for(&self, phase: BossEncounterPhase) -> Option<&BossPattern> {
        match self {
            BossAttackPattern::Cycle => None,
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => match phase {
                BossEncounterPhase::Intro => Some(intro),
                BossEncounterPhase::Phase1 => Some(phase1),
                BossEncounterPhase::Transition => Some(transition),
                BossEncounterPhase::Phase2 => Some(phase2),
                BossEncounterPhase::Enrage => Some(enrage),
                // Dormant / Stagger / Death don't run patterns; the caller
                // already skips attacks in those phases.
                _ => Some(phase1),
            },
        }
    }
}

/// Attack hitbox identity emitted by a `BossPatternStep`. The concrete
/// world-space AABB for each profile is computed by
/// `BossRuntime::volumes_for(&BossAttackProfile)` because the math
/// reads boss pos / spawn / combat_size / is_gnu_ton; this enum is
/// pure data so the brain can pick a profile without touching the
/// runtime.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossAttackProfile {
    /// A body-mounted GEOMETRY strike. The `String` is the strike **key**
    /// (snake_case, e.g. `"floor_slam"`): it selects the move's body-local
    /// hitbox rects from the strike-geometry table — a built-in engine default
    /// (`floor_slam`, `side_sweep`, `full_body_pulse`, `wing_sweep`,
    /// `dive_lane`, `broadside`, `hand_slam`, `hand_sweep`, `head_descent`,
    /// `converging_shockwave`, `hazard_column`) OR the boss's RON
    /// `strike_geometry` override. Damage flows through the moveset's
    /// `HitVolume`s. A new geometry strike is a new key + authored rects, with
    /// NO edit to this enum: the old per-variant `FloorSlam`/`SideSweep`/… all
    /// collapsed into this carrier, their geometry moved to the keyed table.
    Strike(String),
    /// A content-defined SPECIAL. The `String` is the technique **key**
    /// (snake_case, e.g. `"overfit_volley"`); a content-owned *Technique*
    /// recognizes it, reads its own params, and emits the effects. Damage
    /// routes through whatever that technique spawns (projectiles /
    /// World-anchored hitboxes / minions), so a `Special` carries no
    /// body-mounted geometry. A new game adds a boss special by registering a
    /// content technique under a new key — no edit to this enum. (The old
    /// per-boss `DebrisRain`/`MemorizedVolley`/… collapsed here earlier.)
    Special(String),
}

/// The engine's built-in geometry strike keys — the default strike-geometry
/// vocabulary a `BossAttackProfile::Strike` selects from. A move id in this set
/// is a geometry `Strike`; any other id is a content-technique `Special` (used
/// by [`BossAttackProfile::from_move_id`] to reconstruct a profile from a
/// content-free move id). The strike RECTS for these keys live in the
/// gameplay-core strike-geometry table; this crate holds only the key
/// vocabulary the brain authors against.
pub const BUILTIN_STRIKE_KEYS: &[&str] = &[
    "floor_slam",
    "side_sweep",
    "full_body_pulse",
    "wing_sweep",
    "dive_lane",
    "broadside",
    "hand_slam",
    "hand_sweep",
    "head_descent",
    "converging_shockwave",
    "hazard_column",
];

impl BossAttackProfile {
    /// True iff this profile is a content-Technique `Special`. Both variants
    /// run through the shared move timeline: specials sustain a typed effect,
    /// while geometry strikes carry body-mounted hitbox volumes.
    pub fn is_special(&self) -> bool {
        matches!(self, BossAttackProfile::Special(_))
    }

    /// The content technique key for a `Special` profile, else `None`.
    pub fn special_key(&self) -> Option<&str> {
        match self {
            BossAttackProfile::Special(key) => Some(key.as_str()),
            BossAttackProfile::Strike(_) => None,
        }
    }

    /// The moveset move id this profile binds to — its key (for BOTH a `Strike`
    /// and a `Special`, the id IS the key). The boss attack trigger
    /// (`trigger_boss_attack_moves`) looks the active profile's move up by this
    /// id, so EVERY boss strike (geometry AND special) runs through the SAME
    /// moveset runtime as an actor's swing (fable review §A1: the moveset is the
    /// boss's melee system too).
    pub fn move_id(&self) -> String {
        match self {
            BossAttackProfile::Strike(key) | BossAttackProfile::Special(key) => key.clone(),
        }
    }

    /// The inverse of [`move_id`](Self::move_id): recover the profile a live
    /// boss move belongs to from its move id. A key in [`BUILTIN_STRIKE_KEYS`]
    /// (the engine's default geometry vocabulary) is a `Strike`; any other id is
    /// a content-technique `Special(key)` (its move id IS the key). This lets a
    /// `BossAttackState` PROJECTION derive which profile a `MovePlayback`
    /// represents without threading the profile through the content-free move
    /// runtime.
    ///
    /// `from_move_id(p.move_id()) == p` for every profile whose key is a built-in
    /// strike or a non-colliding special (pinned by `move_id_round_trips`). A
    /// `Special` whose key happens to equal a built-in strike key resolves to the
    /// `Strike` — the same documented collision the old variant table had, which
    /// no content authors. (A wholly new content geometry key not in the built-in
    /// set projects as `Special` in the READ-MODEL only; its damage still flows
    /// through the authored `Strike` move, so this is cosmetic — refine if a game
    /// needs custom geometry keys visible in the projected telegraph.)
    pub fn from_move_id(id: &str) -> BossAttackProfile {
        if BUILTIN_STRIKE_KEYS.contains(&id) {
            BossAttackProfile::Strike(id.to_string())
        } else {
            BossAttackProfile::Special(id.to_string())
        }
    }
}

/// Free function used by both `BossPattern::total_duration` and the
/// brain's cursor advancement.
/// A step's authored duration. Control-flow steps (`Select`, `Stance`) take no
/// time; `Select` reports its HEAVIEST arm so `total_duration` is a worst-case
/// budget rather than a lie. The ticker never sees either — `resolve_timeline`
/// removes `Select` and `Stance` is consumed as a jump.
pub fn step_duration(step: &BossPatternStep) -> f32 {
    match step {
        BossPatternStep::Telegraph { duration, .. }
        | BossPatternStep::Strike { duration, .. }
        | BossPatternStep::Rest { duration } => *duration,
        BossPatternStep::Stance { .. } => 0.0,
        BossPatternStep::Select { table } => table
            .iter()
            .map(|arm| arm.steps.iter().map(step_duration).sum::<f32>())
            .fold(0.0, f32::max),
    }
}

// ===== Brain-template cfg / state =====

/// Scripted multi-phase boss policy. Built at spawn time from the
/// authored `BossBehaviorProfile`; the brain ticks the cursor and
/// emits per-tick intent against this cfg.
#[derive(Clone, Debug)]
pub struct BossPatternCfg {
    /// Engagement gating shared with every other state-machine brain.
    /// `0.0` means the brain is currently peaceful: its cursor still
    /// advances, but it emits no [`BossAttackIntent`].
    pub aggressiveness: f32,
    /// Encounter id (matches `boss_encounter::encounter_id_from_name`).
    /// Stays a `String` so the brain can pull straight from the
    /// existing registry instead of forcing a parallel id type.
    pub encounter_id: String,
    /// Pattern choice plus per-phase scripted steps, or `Cycle` for the classic
    /// roster rhythm. The brain owns the schedule.
    pub pattern: BossAttackPattern,
    /// Movement profile (anchor sway / air swoop / stationary giant).
    /// Tells the brain how to fill `frame.desired_vel` each tick.
    /// Used as the fallback for any phase whose dedicated override
    /// (`movement_phase2`, `movement_enrage`) is `None`.
    pub movement: BossMovementProfile,
    /// Per-phase movement overrides. `None` means "use `movement`
    /// during this phase." Lets a single boss escalate from a slow
    /// anchored sway in phase 1 to a wide AirSwoop in phase 2, or to
    /// a faster aggressive AnchorSway in enrage — without bloating
    /// `BossMovementProfile` itself into a phase-aware variant.
    pub movement_phase2: Option<BossMovementProfile>,
    pub movement_enrage: Option<BossMovementProfile>,
    /// Multiplier applied to movement speed during any active strike.
    /// Content techniques may anchor world-space effects
    /// at the boss position; if the boss keeps sliding sideways
    /// during the strike the hitboxes drift away from the visible
    /// telegraph. Set to `< 1.0` to slow the boss while a strike is
    /// committed. `1.0` keeps the legacy behavior.
    pub strike_speed_scale: f32,
    /// World-space anchor the movement profile sways around. Captured
    /// from `BossRuntime::spawn` at spawn time so the brain doesn't
    /// have to query the runtime.
    pub spawn: ae::Vec2,
    /// Combat collision size (used for the soft world-bounds clamp on
    /// `desired_vel`). Captured from `BossRuntime::combat_size`.
    pub combat_size: ae::Vec2,
    /// Cycle-mode windup duration (seconds). Used by `BossAttackPattern::Cycle`
    /// to time the windup → active transition. Built from
    /// `BossBehaviorProfile::attack_windup.max(0.01)`.
    pub cycle_attack_windup: f32,
    /// Cycle-mode active hit-window duration (seconds). Built from
    /// `BossBehaviorProfile::attack_active.max(combat_tuning.boss_attack_active).max(0.01)`.
    pub cycle_attack_active: f32,
    /// Cycle-mode cooldown duration (seconds). Built from
    /// `BossBehaviorProfile::attack_cooldown.max(0.05)`.
    pub cycle_attack_cooldown: f32,
    /// Cycle-mode rotation of attack profiles. The brain picks
    /// `cycle_attacks[(pattern_timer / cycle_attack_cooldown).floor() % len]`
    /// each tick and writes that into [`BossAttackIntent::active_profile`]
    /// (during Active) or [`BossAttackIntent::telegraph_profile`]
    /// (during Windup). Empty for `Scripted` bosses.
    pub cycle_attacks: Vec<BossAttackProfile>,
    /// Self-dodge horizontal amplitude (px). A boss that dodges its OWN
    /// strike adds a horizontal sway during the active window so it reads as
    /// "stepping aside to avoid its own experiment" (GNU-ton weaves out of its
    /// apple rain). Set to 0 for bosses that hold their ground. Authored as
    /// boss DATA (`BossBehaviorProfile::self_dodge`); the engine names no boss.
    pub self_dodge_amp: f32,
    /// Self-dodge horizontal frequency (Hz-ish, fed into a
    /// `sin(movement_timer * freq)` oscillator).
    pub self_dodge_freq: f32,
    /// Chase/engage/retreat macro tuning. Use
    /// [`BossMacroTuning::disabled`] for legacy behavior (boss
    /// stays in `Engage` permanently and movement = movement
    /// profile). Set non-zero thresholds to opt into the
    /// chase/retreat dance.
    pub macro_tuning: BossMacroTuning,
}

impl BossPatternCfg {
    /// Build a default cfg for testing — peaceful, empty pattern,
    /// stationary-giant movement. Call sites that need a real boss
    /// build their own from `BossBehaviorProfile` at spawn time.
    pub fn neutral_test() -> Self {
        Self {
            aggressiveness: 0.0,
            encounter_id: String::new(),
            pattern: BossAttackPattern::Cycle,
            movement: BossMovementProfile::StationaryGiant {
                sway_amplitude: 0.0,
                sway_frequency: 0.0,
                speed: 0.0,
            },
            movement_phase2: None,
            movement_enrage: None,
            strike_speed_scale: 1.0,
            spawn: ae::Vec2::ZERO,
            combat_size: ae::Vec2::new(100.0, 100.0),
            cycle_attack_windup: 0.5,
            cycle_attack_active: 0.2,
            cycle_attack_cooldown: 0.5,
            cycle_attacks: Vec::new(),
            self_dodge_amp: 0.0,
            self_dodge_freq: 0.0,
            macro_tuning: BossMacroTuning::disabled(),
        }
    }

    /// The boss body's authored **special repertoire** — the ordered,
    /// deduplicated list of `(profile, strike-window seconds)` this boss can
    /// perform, derived from its authored pattern. This is the boss's CAPABILITY
    /// (what its body can do when commanded), independent of the autonomous
    /// POLICY (the pattern schedule that chooses *when*). The scripted brain
    /// drives these profiles on its own timeline; a possessing human controller
    /// maps input onto the SAME list (unified-actors I2/I7: possession grants the
    /// body's full kit, nothing special-cased). Scripted bosses contribute their
    /// `Strike` profiles across every phase; `Cycle` bosses contribute
    /// `cycle_attacks`; the strike window is the authored per-step duration (or
    /// `cycle_attack_active`), floored so a zero can't make a strike instant.
    pub fn special_repertoire(&self) -> Vec<(BossAttackProfile, f32)> {
        let mut out: Vec<(BossAttackProfile, f32)> = Vec::new();
        let mut push = |profile: &BossAttackProfile, duration: f32| {
            if !out.iter().any(|(p, _)| p == profile) {
                out.push((profile.clone(), duration.max(0.05)));
            }
        };
        match &self.pattern {
            BossAttackPattern::Cycle => {
                for profile in &self.cycle_attacks {
                    push(profile, self.cycle_attack_active);
                }
            }
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => {
                for pattern in [intro, phase1, transition, phase2, enrage] {
                    for step in &pattern.steps {
                        if let BossPatternStep::Strike { profile, duration } = step {
                            push(profile, *duration);
                        }
                    }
                }
            }
        }
        out
    }

    /// First-seen TELEGRAPH window (seconds) per attack profile — the pre-strike
    /// windup a scripted `Telegraph` step authors before its paired `Strike`.
    /// Mirrors [`special_repertoire`](Self::special_repertoire) (strike windows)
    /// so the boss's moveset move can span the whole telegraph→strike as ONE
    /// timeline: the move's Active window opens at `telegraph_seconds`, encoding
    /// the offset the projected `BossAttackState.active_elapsed` needs (E53). A
    /// profile with no authored telegraph (a bare `Strike`, or `Cycle` with no
    /// windup) is absent → the caller treats it as `0.0`. `Cycle` bosses telegraph
    /// with `cycle_attack_windup` for every rotation profile.
    pub fn telegraph_windows(&self) -> Vec<(BossAttackProfile, f32)> {
        let mut out: Vec<(BossAttackProfile, f32)> = Vec::new();
        let mut push = |profile: &BossAttackProfile, duration: f32| {
            if duration > 0.0 && !out.iter().any(|(p, _)| p == profile) {
                out.push((profile.clone(), duration));
            }
        };
        match &self.pattern {
            BossAttackPattern::Cycle => {
                for profile in &self.cycle_attacks {
                    push(profile, self.cycle_attack_windup);
                }
            }
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => {
                for pattern in [intro, phase1, transition, phase2, enrage] {
                    for step in &pattern.steps {
                        if let BossPatternStep::Telegraph {
                            profile, duration, ..
                        } = step
                        {
                            push(profile, *duration);
                        }
                    }
                }
            }
        }
        out
    }

    /// Pick the movement profile this cfg wants for the given
    /// encounter phase. Phases without a dedicated override fall
    /// back to the default `movement`. Dormant/Stagger/Death are
    /// non-attacking — the brain handles them upstream.
    pub fn movement_for_phase(&self, phase: BossEncounterPhase) -> &BossMovementProfile {
        match phase {
            BossEncounterPhase::Phase2 | BossEncounterPhase::Transition => {
                self.movement_phase2.as_ref().unwrap_or(&self.movement)
            }
            BossEncounterPhase::Enrage => self.movement_enrage.as_ref().unwrap_or(&self.movement),
            _ => &self.movement,
        }
    }
}

/// Per-actor cursor and clock state advanced by [`tick_boss_pattern`].
/// Component-equivalent — held inside the `Brain::StateMachine(BossPattern{...})`
/// variant so brain swaps don't accidentally drop the cursor.
///
/// The cursor is durable simulation state. [`BossAttackIntent`] is only the
/// transient output cache required by the universal `Brain::tick` seam: the ECS
/// boss adapter copies it out each tick, and [`BossAttackState`] is projected
/// separately from the authoritative live move. The intent cache is therefore
/// deliberately omitted from rollback snapshots.
#[derive(Clone, Debug, Default)]
pub struct BossPatternState {
    /// Last encounter phase the brain ticked under. When the phase
    /// changes the brain resets the scripted cursor so a new phase's
    /// timeline begins at step 0 rather than mid-step. `None` until
    /// the first tick.
    pub last_phase: Option<BossEncounterPhase>,
    /// Cursor into the active scripted pattern's `steps`. Cycle-mode
    /// patterns leave this at 0.
    pub step_index: usize,
    /// Seconds spent in the current scripted step. Reset on step
    /// advance.
    pub step_elapsed: f32,
    /// Free-running clock the movement profile reads to seed its
    /// sin() oscillator. Advances by `dt` each tick the brain runs.
    pub movement_timer: f32,
    /// Free-running clock the cycle-mode pattern reads to pick which
    /// attack profile is current (`pattern_timer / cycle_attack_cooldown`).
    /// Advances by `dt` each tick.
    pub pattern_timer: f32,
    /// Cycle-mode phase the brain is currently in. Scripted patterns
    /// leave this at `CyclePhase::Cooldown` and ignore it.
    pub cycle_phase: CyclePhase,
    /// Seconds remaining in the current cycle phase. Drained by
    /// `dt`; transition to the next cycle phase happens at 0.
    pub cycle_phase_remaining: f32,
    /// High-level chase/engage/retreat state. Defaults to `Engage`.
    pub macro_state: BossMacroState,
    /// Seconds spent in the current `Engage` window. Reset to 0
    /// when a non-Engage state is exited; drives the periodic
    /// `engage_max_duration_s` retreat trigger.
    pub engage_timer: f32,
    /// Tiny deterministic RNG state used only by optional probabilistic
    /// idle attack gates. Zero means "seed from cfg on first roll."
    pub rng_seed: u64,
    /// Transient per-tick attack request emitted by [`tick_boss_pattern`].
    /// This is not an execution timeline and is not snapshotted: the live move
    /// owns execution, while the ECS [`BossAttackState`] is projected from it.
    pub attack_intent: BossAttackIntent,

    // ── BD1: control flow ────────────────────────────────────────────────────
    /// The RESOLVED timeline the cursor walks: the active step list with every
    /// `Select` already rolled away. Rebuilt on phase change, on stance
    /// enter/leave, and each time the cursor loops — so a `Select` rolls once per
    /// pass, which is what "roll once when reached" means for a looping script.
    ///
    /// Empty until the first tick resolves it.
    pub timeline: Vec<BossPatternStep>,
    /// Where to come back to. Pushed on `Stance` entry or an interrupt; popped
    /// when the stance's timeline runs out. A stack, not a slot, so a stance may
    /// enter another stance without losing the way home.
    pub stance_stack: Vec<StanceReturn>,
    /// The stance the boss is inside, `None` at the phase's own timeline.
    pub stance: Option<String>,
    /// Seconds until each `interrupts[i]` may fire again. Parallel to the rule
    /// list; resized on phase change.
    pub interrupt_cooldowns: Vec<f32>,
    /// Seconds accumulated toward each `OnTimer` rule's `every_s`.
    pub interrupt_timers: Vec<f32>,
    /// The boss's HP as of the previous tick, so `OnHitTaken` can read the drop.
    /// `None` before the first tick — a boss cannot have been hit before it existed.
    pub last_hp: Option<i32>,
}

/// A saved cursor: where a `Stance` (or an interrupt) interrupted the timeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StanceReturn {
    /// The timeline to restore.
    pub timeline: Vec<BossPatternStep>,
    /// The stance name that timeline belonged to (`None` = the phase's own).
    pub stance: Option<String>,
    /// The step to resume AT (already advanced past the `Stance` marker).
    pub step_index: usize,
    /// Elapsed within that step. An interrupt keeps it, so a boss yanked out of a
    /// telegraph resumes the telegraph where it left off rather than restarting.
    pub step_elapsed: f32,
}

/// Three-state cycle-mode attack lifecycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CyclePhase {
    /// Boss is on cooldown between attacks; emits no intent.
    #[default]
    Cooldown,
    /// Boss is telegraphing an attack; volumes draw but no damage.
    Windup,
    /// Boss attack is live; the active profile is emitted as intent.
    Active,
}

/// High-level "what is the boss doing right now?" state, layered
/// over the scripted attack schedule. The schedule still ticks
/// independently; the macro state decides where the boss *wants*
/// to be in the arena so the fight has a chase/disengage rhythm:
///
/// - [`Engage`] — default. Movement uses the per-phase
///   [`BossMovementProfile`].
/// - [`Approach`] — boss closes distance to the player; movement
///   target = player position, speed scaled up. Triggered when the
///   player has run too far away or the boss has been in Engage
///   too long.
/// - [`Retreat`] — boss pulls back from the player; movement
///   target = a retreat anchor on the opposite side of the arena.
///   Triggered when the player is too close (anti-cornering) or
///   periodically so the player sees the boss "prepare something"
///   and wants to chase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BossMacroState {
    Engage,
    Approach {
        remaining_s: f32,
    },
    Retreat {
        remaining_s: f32,
        retreat_pos: ae::Vec2,
    },
}

impl Default for BossMacroState {
    fn default() -> Self {
        Self::Engage
    }
}

impl BossMacroState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Engage => "engage",
            Self::Approach { .. } => "approach",
            Self::Retreat { .. } => "retreat",
        }
    }
}

/// Tuning knobs for the macro state machine. Held inside
/// [`BossPatternCfg`] so each boss can author its own
/// engagement-distance feel. Bosses that don't need a chase/retreat
/// dance leave these at the zero defaults — the state machine then
/// permanently stays in `Engage` and the legacy "always move via
/// movement profile" behavior holds.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct BossMacroTuning {
    /// Distance (px) below which the boss flees the player to
    /// avoid cornering. Set to 0 to disable the too-close trigger.
    pub too_close_distance: f32,
    /// Distance (px) above which the boss commits to chasing the
    /// player. Set to 0 to disable the too-far trigger.
    pub too_far_distance: f32,
    /// Target distance (px) the boss tries to settle at during
    /// Approach — once within this radius the boss returns to
    /// Engage.
    pub engage_distance: f32,
    /// Seconds the boss spends in Approach before automatically
    /// returning to Engage (cap so a player who keeps running
    /// doesn't keep the boss in chase forever).
    pub approach_duration_s: f32,
    /// Seconds the boss spends in Retreat. Long enough to feel like
    /// preparation; short enough that the player gets a real
    /// "chase" window before the next engage.
    pub retreat_duration_s: f32,
    /// Max seconds in Engage before the boss force-triggers a
    /// Retreat (the "preparing something" beat). 0 disables the
    /// periodic retreat.
    pub engage_max_duration_s: f32,
    /// Horizontal clearance (px) the brain preserves between the boss
    /// body and the nearest solid/blink-wall tile in the direction it
    /// wants to move. This is a brain-level intent clamp only; the
    /// kinematic sweep remains the hard authority downstream.
    #[serde(default = "default_front_wall_standoff")]
    pub front_wall_standoff: f32,
    /// Per-second chance that an idle scripted boss advances out of a
    /// Rest beat into its next attack once that Rest beat's minimum
    /// duration has elapsed. 0 keeps fully deterministic scripts.
    #[serde(default)]
    pub idle_attack_chance_per_second: f32,
    /// If true, Engage holds the current body position instead of
    /// returning to the movement profile's spawn/sway target. Useful
    /// for contact bosses whose macro layer is the only movement
    /// policy: Approach closes distance, Engage idles/fires in place.
    #[serde(default)]
    pub hold_position_while_engaged: bool,
    /// Multiplier applied to movement speed during Approach. > 1.0
    /// makes the boss commit visually to the chase.
    pub approach_speed_scale: f32,
    /// Multiplier applied to movement speed during Retreat. < 1.0
    /// makes the boss feel like it's pulling away deliberately.
    pub retreat_speed_scale: f32,
    /// How far (px) the boss retreats from the player along the
    /// player→boss axis. Larger = bigger retreat arc.
    pub retreat_distance: f32,
    /// If true, the boss suppresses Telegraph/Strike actions while
    /// Approach or Retreat is active. Useful for YHTBTR-style bosses
    /// that only choose idle/attack once they have reached their
    /// preferred standoff range.
    #[serde(default)]
    pub suppress_attacks_while_moving: bool,
}

fn default_front_wall_standoff() -> f32 {
    48.0
}

impl BossMacroTuning {
    /// Disabled tuning — the boss permanently stays in `Engage`.
    /// Returned for bosses that don't carry their own macro tuning
    /// so the existing fights don't change behavior.
    pub fn disabled() -> Self {
        Self {
            too_close_distance: 0.0,
            too_far_distance: 0.0,
            engage_distance: 0.0,
            approach_duration_s: 0.0,
            retreat_duration_s: 0.0,
            engage_max_duration_s: 0.0,
            front_wall_standoff: 0.0,
            idle_attack_chance_per_second: 0.0,
            hold_position_while_engaged: false,
            approach_speed_scale: 1.0,
            retreat_speed_scale: 1.0,
            retreat_distance: 0.0,
            suppress_attacks_while_moving: false,
        }
    }

    /// True iff this tuning has at least one transition trigger
    /// enabled. Used as the gate to skip the macro state machine
    /// entirely for bosses that opted out.
    pub fn is_enabled(&self) -> bool {
        self.too_close_distance > 0.0
            || self.too_far_distance > 0.0
            || self.engage_max_duration_s > 0.0
            || self.contact_chase_mode()
    }

    /// Contact bosses like Smirking Behemoth do not want a preferred
    /// distance ring: if they are not blocked and the player is not
    /// horizontally overlapping them yet, they should keep closing until
    /// collision stops them. This mode is intentionally opt-in through the
    /// existing macro knobs rather than a boss-id branch.
    pub fn contact_chase_mode(&self) -> bool {
        self.too_close_distance <= 0.0
            && self.approach_duration_s > 0.0
            && self.hold_position_while_engaged
    }
}

/// Per-tick read-only inputs to [`tick_boss_pattern`]. The boss tick
/// system builds this from the boss entity's components.
#[derive(Default, Clone, Copy, Debug)]
pub struct BossPatternContext {
    /// Boss encounter phase this tick (forwarded by the system from
    /// `BossEncounterRegistry`). Drives pattern selection + the
    /// `is_attacking()` gate.
    pub encounter_phase: BossEncounterPhase,
    /// Boss's current authoritative world position. Read by the
    /// movement profile's velocity computation.
    pub actor_pos: ae::Vec2,
    /// Target position the boss is interested in (typically the
    /// primary player). Drives the movement profile's chase math.
    pub target_pos: ae::Vec2,
    /// World size (px). Used for the soft `desired_vel` clamp so the
    /// brain doesn't ask the boss to walk off the map. Real collision
    /// is still enforced by `step_kinematic` downstream.
    pub world_size: ae::Vec2,
    /// Distance from the boss body to the first blocking wall tile in
    /// the horizontal direction of the player, if one is in probe range.
    /// `None` means the approach lane is clear for this tick.
    pub front_wall_clearance: Option<f32>,
    /// Scaled sim dt for this tick. The cursor + clocks all advance
    /// by this value.
    pub dt: f32,
    /// The boss's own facing (`+1` right, `-1` left). BD1's
    /// [`SituationBucket::PlayerBehind`] reads it; `0.0` means "no opinion", and
    /// the bucket then never holds.
    pub actor_facing: f32,
    /// The boss's live HP. BD1's [`SituationBucket::HpBelow`] reads the fraction,
    /// and [`InterruptTrigger::OnHitTaken`] reads the DROP since last tick — which
    /// the brain remembers itself (`BossPatternState::last_hp`) rather than asking
    /// the damage system for a per-tick channel that does not exist. A boss's own
    /// health is the only evidence of a hit it actually needs.
    pub hp_current: i32,
    pub hp_max: i32,
}

impl BossPatternContext {
    /// `hp_current / hp_max`, clamped to `0..=1`. `1.0` when max is unknown, so a
    /// context that never learned the boss's health never trips `HpBelow`.
    pub fn hp_frac(&self) -> f32 {
        if self.hp_max <= 0 {
            return 1.0;
        }
        (self.hp_current as f32 / self.hp_max as f32).clamp(0.0, 1.0)
    }
}

/// Read-model projection of the authoritative live boss move. Written only by
/// the move-projection system; read by rendering, animation, geometry, and damage
/// consumers. The brain emits [`BossAttackIntent`] instead of writing this timing
/// state directly.
#[derive(Component, Clone, Debug, Default)]
pub struct BossAttackState {
    /// `Some(profile)` while the live move is in its telegraph/windup window;
    /// `None` outside that window.
    pub telegraph_profile: Option<BossAttackProfile>,
    /// Seconds left in the current telegraph window. `0.0` when no
    /// telegraph is active.
    pub telegraph_remaining: f32,
    /// Seconds elapsed in the current attack pose while telegraphing.
    /// Consumers use this to sample sprite-authored per-frame
    /// hit/hurt boxes without depending on presentation components.
    pub telegraph_elapsed: f32,
    /// BD3: authored anticipation associated with the live telegraph. Presentation
    /// reads this one read-model rather than re-walking the pattern. `None` when
    /// nothing is telegraphing or no anticipation metadata is available.
    pub telegraph_spec: Option<TelegraphSpec>,
    /// `Some(profile)` while the live move is in its active strike window;
    /// `None` outside that window.
    pub active_profile: Option<BossAttackProfile>,
    /// Seconds left in the current strike window. `0.0` when no
    /// strike is active.
    pub active_remaining: f32,
    /// Seconds elapsed in the current attack pose while striking. If
    /// the immediately-preceding scripted step was a Telegraph for
    /// the same profile, that telegraph duration is included so a
    /// non-looping visual row and its gameplay boxes stay continuous
    /// across Telegraph -> Strike.
    pub active_elapsed: f32,
}

/// The boss's per-frame ATTACK INTENT — which profile it WANTS to fire this
/// frame, written by whatever drives the body (the autonomous `BossPattern` brain
/// or a possessing controller) and read by `trigger_boss_attack_moves` to START
/// the matching moveset move.
///
/// This is the INTENT half of the fable-review §A1 intent/projection split: the
/// trigger reads its intent from HERE, so [`BossAttackState`] is free to become a
/// pure PROJECTION of the live `MovePlayback` (`project_boss_attack_state_from_move`)
/// rather than doubling as both the brain's intent signal AND the read-model. A
/// `Telegraph` step sets `telegraph_profile` (play the move from its windup); a
/// `Strike`/possession step with no telegraph sets `active_profile` (start at the
/// strike edge). Cleared when the driver has no attack intent this frame.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BossAttackIntent {
    /// The profile whose move should play FROM ITS WINDUP (a telegraph edge).
    pub telegraph_profile: Option<BossAttackProfile>,
    /// The profile whose move should start AT THE STRIKE (no telegraph — a
    /// possession press or a `tel = 0` path).
    pub active_profile: Option<BossAttackProfile>,
}

impl BossAttackIntent {
    /// Clear both intents — the driver wants no attack this frame.
    pub fn clear(&mut self) {
        self.telegraph_profile = None;
        self.active_profile = None;
    }
}

impl BossAttackState {
    /// Clear every field — used when a boss enters a non-attacking
    /// phase (Dormant / Stagger / Death).
    pub fn clear(&mut self) {
        self.telegraph_profile = None;
        self.telegraph_spec = None;
        self.telegraph_remaining = 0.0;
        self.telegraph_elapsed = 0.0;
        self.active_profile = None;
        self.active_remaining = 0.0;
        self.active_elapsed = 0.0;
    }
}

/// The boss body's authored action repertoire — its CAPABILITY, persisted as a
/// **component** (not brain state) so it survives a brain swap. When a human
/// possesses a boss, its `Brain::StateMachine(BossPattern{..})` is transferred
/// away and stashed for restore; the pattern cfg is no longer reachable from the
/// boss tick, but this component keeps the boss's special list in scope so the
/// controller can still command the body's authored moves.
///
/// This is the boss analogue of an actor's `ActionSet`: capability is body data,
/// the brain is policy. Populated at spawn from
/// [`BossPatternCfg::special_repertoire`]; both the autonomous pattern and a
/// possessing controller drive the same profiles.
#[derive(Component, Clone, Debug, Default)]
pub struct BossCapability {
    /// `(profile, strike-window seconds)`, in first-seen order from the authored
    /// pattern. Empty for a boss with no authored strikes → possession maps to a
    /// no-op (the body simply has no special to command).
    pub specials: Vec<(BossAttackProfile, f32)>,
}

impl BossCapability {
    /// Derive the repertoire from a boss pattern cfg (call at spawn).
    pub fn from_cfg(cfg: &BossPatternCfg) -> Self {
        Self {
            specials: cfg.special_repertoire(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.specials.is_empty()
    }

    /// The move mapped to a controller "slot": `0` = attack / primary, wrapping
    /// around the repertoire so a boss with one move maps every control to it.
    /// `None` iff the boss authors no strikes.
    pub fn slot(&self, index: usize) -> Option<&(BossAttackProfile, f32)> {
        if self.specials.is_empty() {
            None
        } else {
            self.specials.get(index % self.specials.len())
        }
    }

    /// The boss's SIGNATURE special: the first content-technique `Special` profile
    /// in the repertoire (e.g. `echo_fan` / `apple_rain` / `overfit_volley`),
    /// regardless of where it sits among the geometry strikes. The possession
    /// special-button maps here so it fires a real boss special. `None` if the
    /// boss authors no content special (only geometry strikes).
    pub fn signature_special(&self) -> Option<&(BossAttackProfile, f32)> {
        self.specials.iter().find(|(p, _)| p.is_special())
    }
}

// ===== tick_boss_pattern =====

mod tick;
pub use tick::*;

/// BD1's three authored-logic atoms as pure functions: bucket evaluation,
/// weighted `Select` resolution, stance push/pop, and interrupt bookkeeping.
pub mod control_flow;

/// BD5's fight validator: `boss-design.md` §3's telegraph grammar and fairness
/// rules, run over authored data with per-game bands.
pub mod validator;

/// The boss SEED LIBRARY vocabulary (boss-design.md §2, slice BD4): attack
/// archetypes with a written design intent, fair-counter set, and measured
/// duration bands. The catalog itself is content.
pub mod seeds;

/// Where the boss is in the encounter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BossEncounterPhase {
    #[default]
    Dormant,
    /// Pre-fight intro: title card, boss roar, camera-pan.
    Intro,
    /// First phase of attacks.
    Phase1,
    /// Brief transition between Phase1 and Phase2 — boss is
    /// invulnerable, plays a tell. Patterns from neither phase fire.
    Transition,
    /// Second phase of attacks (faster patterns, more variety).
    Phase2,
    /// Boss is staggered and vulnerable to a punish window. Triggered
    /// by hitting a stagger HP threshold. Auto-recovers after a fixed
    /// duration.
    Stagger,
    /// Final low-HP phase: tighter, faster patterns. Visible "enraged"
    /// presentation cue.
    Enrage,
    /// Boss is dead, playing outro logic.
    Death,
}

impl BossEncounterPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Intro => "intro",
            Self::Phase1 => "phase1",
            Self::Transition => "transition",
            Self::Phase2 => "phase2",
            Self::Stagger => "stagger",
            Self::Enrage => "enrage",
            Self::Death => "death",
        }
    }

    pub fn boss_invulnerable(self) -> bool {
        matches!(
            self,
            Self::Dormant | Self::Intro | Self::Transition | Self::Death
        )
    }

    /// True while the boss should be running its attack patterns.
    /// Stagger is not an attacking phase.
    pub fn is_attacking(self) -> bool {
        matches!(self, Self::Phase1 | Self::Phase2 | Self::Enrage)
    }
}

#[cfg(test)]
mod tests;
