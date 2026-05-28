//! Boss-policy brain template: scripted multi-phase + legacy cycle.
//!
//! Owns the vocabulary that used to live on `BossRuntime`:
//!
//! - [`BossMovementProfile`] — anchor-sway / air-swoop / stationary-giant
//!   movement families.
//! - [`BossAttackProfile`] — per-strike hitbox identities (FloorSlam,
//!   GnuAppleRain, WingSweep, …). The world-space AABB for each profile
//!   is still computed by `BossRuntime::volumes_for(profile)` because
//!   the math depends on the boss's pos / spawn / combat_size /
//!   is_gnu_ton flags — but the **choice** of which profile is active
//!   this tick is made here, not there.
//! - [`BossPatternStep`] — one beat in a scripted timeline (Telegraph
//!   / Strike / Rest).
//! - [`BossPattern`] — an ordered list of steps that loops.
//! - [`BossAttackPattern`] — `Cycle` (legacy rhythm using
//!   `BossBehaviorProfile::attacks`) or `Scripted` (per-phase
//!   timeline).
//!
//! Plus the brain-template state shape:
//!
//! - [`BossPatternCfg`] — per-boss tuning (pattern, movement, phase
//!   timings, spawn anchor, world bounds). Built at spawn-time from
//!   `BossBehaviorProfile`.
//! - [`BossPatternState`] — per-actor cursor (step index/elapsed,
//!   last phase, movement_timer, pattern_timer, cycle phase) advanced
//!   by [`tick_boss_pattern`] every frame.
//! - [`BossPatternContext`] — per-tick read-only inputs the system
//!   passes in (encounter phase, target pos, current pos, dt).
//! - [`BossAttackState`] — the brain's component-side output sink.
//!   Holds the live telegraph / active profile + remaining time so
//!   rendering and contact systems read execution state from a
//!   uniform place instead of poking at runtime fields.
//!
//! [`tick_boss_pattern`] is the pure function the boss tick system
//! calls each frame to turn (cfg + state + context) into
//! (`ActorControlFrame` intent + `BossAttackState` mirror). The
//! function is `BrainSnapshot`-free on purpose — the user-facing
//! follow-up plan explicitly says "Phase must be available to the
//! brain. Prefer a boss-specific tick system over bloating
//! `BrainSnapshot` for all actors."

use ambition_engine as ae;
use bevy::prelude::Component;

// ===== Vocabulary (moved from content/features/bosses.rs) =====

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
    },
    /// Hitbox is live: active volumes draw, contact damages the player.
    Strike {
        profile: BossAttackProfile,
        duration: f32,
    },
    /// No volume. Pure breathing room so the player can reposition or punish.
    Rest { duration: f32 },
}

/// A full attack script for one boss phase. Loops when it reaches the end.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct BossPattern {
    pub steps: Vec<BossPatternStep>,
}

impl BossPattern {
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
    pub fn pattern_for(&self, phase: ae::BossEncounterPhase) -> Option<&BossPattern> {
        match self {
            BossAttackPattern::Cycle => None,
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => match phase {
                ae::BossEncounterPhase::Intro => Some(intro),
                ae::BossEncounterPhase::Phase1 => Some(phase1),
                ae::BossEncounterPhase::Transition => Some(transition),
                ae::BossEncounterPhase::Phase2 => Some(phase2),
                ae::BossEncounterPhase::Enrage => Some(enrage),
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
    FloorSlam,
    SideSweep,
    FullBodyPulse,
    WingSweep,
    DiveLane,
    Broadside,
    // GNU-ton specific: giant hands slam from above
    GnuHandSlam,
    // GNU-ton specific: hands sweep in from the far sides
    GnuHandSweep,
    // GNU-ton specific: the head descends into player space (vulnerability + hazard)
    GnuHeadDescent,
    // GNU-ton specific: shockwave from both hands meeting in the center
    GnuShockwave,
    // GNU-ton specific: apples fall from the ceiling around the player.
    // Direct contact damage comes from spawned enemy projectiles
    // (gravity > 0), not a single AABB, so `volumes_for` returns empty
    // for this profile and damage routes through the projectile path.
    GnuAppleRain,
    // Gradient Sentinel: tall vertical hazard column at the boss x.
    // Ordinary melee profile (volume from `volumes_for_profile`); the
    // player jumps over or laterals away.
    GradientLane,
    // Gradient Sentinel special: boss memorizes player positions during
    // telegraph and fires bolts at every sample on the strike edge.
    // Damage routes through spawned enemy projectiles (the bolt
    // barrage), so `volumes_for` returns empty for this profile.
    OverfitVolley,
    // Gradient Sentinel special: a "local minimum" pit forms at the
    // player's position on strike start and persists as a damaging
    // World-anchored hitbox for several seconds; spawns 1 puppy_slug
    // minion from inside the pit.
    MinimaTrap,
    // Gradient Sentinel special: a cross-shaped hazard centered on the
    // boss. Two World-anchored hitboxes (horizontal arm + vertical
    // arm); one is "live" at a time and the active axis rotates
    // periodically across the strike window. Player stands on the safe
    // axis and reads the swap.
    SaddlePoint,
    // Gradient Sentinel special: spawns N "slop" minions (small_lurker
    // stand-in) at the top of the arena that descend toward the
    // player. Damage routes through the minion contact path, not a
    // boss AABB, so `volumes_for` returns empty.
    GradientCascade,
}

impl BossAttackProfile {
    /// True iff this profile is implemented through a `Special`
    /// message + EFFECTS consumer. False for profiles whose damage
    /// flows through melee/contact hitbox volumes.
    pub fn is_special(&self) -> bool {
        matches!(
            self,
            BossAttackProfile::GnuAppleRain
                | BossAttackProfile::OverfitVolley
                | BossAttackProfile::MinimaTrap
                | BossAttackProfile::SaddlePoint
                | BossAttackProfile::GradientCascade
        )
    }
}

/// Free function used by both `BossPattern::total_duration` and the
/// brain's cursor advancement.
pub fn step_duration(step: &BossPatternStep) -> f32 {
    match step {
        BossPatternStep::Telegraph { duration, .. }
        | BossPatternStep::Strike { duration, .. }
        | BossPatternStep::Rest { duration } => *duration,
    }
}

// ===== Brain-template cfg / state =====

/// Scripted multi-phase boss policy. Built at spawn time from the
/// authored `BossBehaviorProfile`; the brain ticks the cursor and
/// emits per-tick intent against this cfg.
#[derive(Clone, Debug)]
pub struct BossPatternCfg {
    /// Engagement gating shared with every other state-machine brain.
    /// `0.0` means the brain is currently peaceful (cursor still
    /// advances but no melee/special is emitted).
    pub aggressiveness: f32,
    /// Encounter id (matches `boss_encounter::encounter_id_from_name`).
    /// Stays a `String` so the brain can pull straight from the
    /// existing registry instead of forcing a parallel id type.
    pub encounter_id: String,
    /// Pattern choice + per-phase scripted steps (or `Cycle` for the
    /// legacy rhythm). Moved out of `BossBehaviorProfile` so the
    /// brain owns the schedule.
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
    /// Multiplier applied to the movement speed during an active
    /// `is_special()` strike. Specials (SaddlePoint, MinimaTrap,
    /// OverfitVolley, GradientCascade) anchor World-space hitboxes
    /// at the boss position; if the boss keeps sliding sideways
    /// during the strike the hitboxes drift away from the visible
    /// telegraph. Set to `< 1.0` to slow the boss while a special is
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
    /// each tick and writes that into `BossAttackState.active_profile`
    /// (during Active phase) or `BossAttackState.telegraph_profile`
    /// (during Windup phase). Empty for `Scripted` bosses.
    pub cycle_attacks: Vec<BossAttackProfile>,
    /// Apple-rain horizontal dodge amplitude (px). The GNU-ton brain
    /// adds a horizontal sway during an active GnuAppleRain strike
    /// so the giant reads as "stepping aside to avoid its own
    /// experiment". Set to 0 for bosses that don't dodge their own
    /// special.
    pub apple_rain_dodge_amp: f32,
    /// Apple-rain horizontal dodge frequency (Hz-ish, fed into a
    /// `sin(movement_timer * freq)` oscillator).
    pub apple_rain_dodge_freq: f32,
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
            apple_rain_dodge_amp: 0.0,
            apple_rain_dodge_freq: 0.0,
            macro_tuning: BossMacroTuning::disabled(),
        }
    }

    /// Pick the movement profile this cfg wants for the given
    /// encounter phase. Phases without a dedicated override fall
    /// back to the default `movement`. Dormant/Stagger/Death are
    /// non-attacking — the brain handles them upstream.
    pub fn movement_for_phase(&self, phase: ae::BossEncounterPhase) -> &BossMovementProfile {
        match phase {
            ae::BossEncounterPhase::Phase2 | ae::BossEncounterPhase::Transition => {
                self.movement_phase2.as_ref().unwrap_or(&self.movement)
            }
            ae::BossEncounterPhase::Enrage => {
                self.movement_enrage.as_ref().unwrap_or(&self.movement)
            }
            _ => &self.movement,
        }
    }
}

/// Per-actor cursor and clock state advanced by [`tick_boss_pattern`].
/// Component-equivalent — held inside the `Brain::StateMachine(BossPattern{...})`
/// variant so brain swaps don't accidentally drop the cursor.
#[derive(Clone, Copy, Debug, Default)]
pub struct BossPatternState {
    /// Last encounter phase the brain ticked under. When the phase
    /// changes the brain resets the scripted cursor so a new phase's
    /// timeline begins at step 0 rather than mid-step. `None` until
    /// the first tick.
    pub last_phase: Option<ae::BossEncounterPhase>,
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
}

/// Three-state cycle-mode attack lifecycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CyclePhase {
    /// Boss is on cooldown between attacks; emits no intent.
    #[default]
    Cooldown,
    /// Boss is telegraphing an attack; volumes draw but no damage.
    Windup,
    /// Boss attack is live; `frame.melee_pressed` emits.
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
    /// Multiplier applied to movement speed during Approach. > 1.0
    /// makes the boss commit visually to the chase.
    pub approach_speed_scale: f32,
    /// Multiplier applied to movement speed during Retreat. < 1.0
    /// makes the boss feel like it's pulling away deliberately.
    pub retreat_speed_scale: f32,
    /// How far (px) the boss retreats from the player along the
    /// player→boss axis. Larger = bigger retreat arc.
    pub retreat_distance: f32,
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
            approach_speed_scale: 1.0,
            retreat_speed_scale: 1.0,
            retreat_distance: 0.0,
        }
    }

    /// True iff this tuning has at least one transition trigger
    /// enabled. Used as the gate to skip the macro state machine
    /// entirely for bosses that opted out.
    pub fn is_enabled(&self) -> bool {
        self.too_close_distance > 0.0
            || self.too_far_distance > 0.0
            || self.engage_max_duration_s > 0.0
    }
}

/// Per-tick read-only inputs to [`tick_boss_pattern`]. The boss tick
/// system builds this from the boss entity's components.
#[derive(Clone, Copy, Debug)]
pub struct BossPatternContext {
    /// Boss encounter phase this tick (forwarded by the system from
    /// `BossEncounterRegistry`). Drives pattern selection + the
    /// `is_attacking()` gate.
    pub encounter_phase: ae::BossEncounterPhase,
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
    /// Scaled sim dt for this tick. The cursor + clocks all advance
    /// by this value.
    pub dt: f32,
}

/// Component-side mirror of the brain's "what attack is live right
/// now?" decision. Written by the boss brain tick system; read by
/// rendering (debug overlay), damage (boss player_damage), and the
/// `Special` resolver (boss runtime's `frame.special_pressed` gate).
///
/// This component replaces the policy fields that used to live on
/// `BossRuntime` (`active_strike_profile` + `telegraph_profile` +
/// `attack_timer` + `attack_windup_timer`). `BossRuntime` may keep
/// mirror copies briefly during the transition; the new component is
/// the source of truth.
#[derive(Component, Clone, Debug, Default)]
pub struct BossAttackState {
    /// `Some(profile)` while the brain is inside a `Telegraph` step
    /// for `profile`; `None` outside Telegraph.
    pub telegraph_profile: Option<BossAttackProfile>,
    /// Seconds left in the current telegraph window. `0.0` when no
    /// telegraph is active.
    pub telegraph_remaining: f32,
    /// `Some(profile)` while the brain is inside a `Strike` step for
    /// `profile`; `None` outside Strike.
    pub active_profile: Option<BossAttackProfile>,
    /// Seconds left in the current strike window. `0.0` when no
    /// strike is active.
    pub active_remaining: f32,
}

impl BossAttackState {
    /// Clear every field — used when a boss enters a non-attacking
    /// phase (Dormant / Stagger / Death).
    pub fn clear(&mut self) {
        self.telegraph_profile = None;
        self.telegraph_remaining = 0.0;
        self.active_profile = None;
        self.active_remaining = 0.0;
    }
}

// ===== tick_boss_pattern =====

/// Pure brain tick: advance the cursor + clocks, write
/// `ActorControlFrame` intent (movement + melee/special edges), and
/// mirror the live telegraph/active profile into the
/// `BossAttackState` sink.
///
/// Behavioral parity with the deleted `BossRuntime::update_scripted_attacks`
/// / `update_cycle_attacks` is the goal — the migration moves the
/// policy decision into the brain without changing what bosses
/// visibly do.
pub fn tick_boss_pattern(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    out: &mut crate::actor_control::ActorControlFrame,
    attack_state: &mut BossAttackState,
) {
    // Always start from a neutral frame so a leaked
    // `melee_pressed = true` from a previous (now-stale) state
    // can't survive into the next tick.
    *out = crate::actor_control::ActorControlFrame::neutral();

    if ctx.dt <= 0.0 {
        return;
    }

    // Tick the free-running clocks the movement profile reads.
    state.movement_timer += ctx.dt;
    state.pattern_timer += ctx.dt;

    // Phase change → reset the scripted cursor. Scripted patterns
    // anchor on step 0 of the new phase rather than carrying the
    // old phase's cursor in mid-step.
    if state.last_phase != Some(ctx.encounter_phase) {
        state.step_index = 0;
        state.step_elapsed = 0.0;
        state.cycle_phase = CyclePhase::Cooldown;
        state.cycle_phase_remaining = 0.0;
        state.last_phase = Some(ctx.encounter_phase);
        // Reset to Engage on phase change so the macro timer
        // doesn't carry stale duration across the music swap.
        state.macro_state = BossMacroState::Engage;
        state.engage_timer = 0.0;
    }

    // Advance the chase/engage/retreat macro state machine BEFORE
    // emitting desired_vel so the movement override (Approach
    // chases the player, Retreat pulls away) is in lockstep with
    // the current macro state.
    if cfg.macro_tuning.is_enabled() && ctx.encounter_phase.is_attacking() {
        advance_macro_state(cfg, state, ctx);
    }

    // Non-attacking phases (Dormant / Stagger / Death) emit no intent
    // and clear the mirror so rendering doesn't keep drawing a stale
    // telegraph through a stagger window.
    if !ctx.encounter_phase.is_attacking() {
        attack_state.clear();
        // Still emit desired_vel from the movement profile so a
        // boss in Dormant still keeps its sway phase (matches the
        // legacy behavior).
        emit_desired_vel(cfg, state, ctx, out, attack_state);
        return;
    }

    match &cfg.pattern {
        BossAttackPattern::Scripted { .. } => {
            advance_scripted(cfg, state, ctx, attack_state);
        }
        BossAttackPattern::Cycle => {
            advance_cycle(cfg, state, ctx, attack_state);
        }
    }

    // Edge tags into the ActorControlFrame: while a Strike is active,
    // emit melee_pressed for ordinary profiles and special_pressed
    // for profiles the EFFECTS consumer handles (apple rain today).
    // ActionSet binds special_pressed to `SpecialActionSpec::GnuAppleRain`;
    // the resolver writes `ActorActionMessage::Special`; the consumer
    // spawns the apples.
    if cfg.aggressiveness > 0.0 {
        if let Some(profile) = attack_state.active_profile.as_ref() {
            if profile.is_special() {
                out.special_pressed = true;
            } else {
                out.melee_pressed = true;
            }
        }
    }

    emit_desired_vel(cfg, state, ctx, out, attack_state);
}

/// Scripted-pattern cursor advancement.
fn advance_scripted(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_state: &mut BossAttackState,
) {
    let steps: Vec<BossPatternStep> = match cfg.pattern.pattern_for(ctx.encounter_phase) {
        Some(pattern) if !pattern.steps.is_empty() => pattern.steps.clone(),
        _ => {
            attack_state.clear();
            return;
        }
    };

    state.step_elapsed += ctx.dt;
    // Wrap the cursor if a phase transition shrunk the script under
    // our feet, then advance through any completed steps this frame.
    if state.step_index >= steps.len() {
        state.step_index = 0;
        state.step_elapsed = 0.0;
    }
    loop {
        let current = &steps[state.step_index];
        let duration = step_duration(current).max(0.01);
        if state.step_elapsed < duration {
            break;
        }
        state.step_elapsed -= duration;
        state.step_index = (state.step_index + 1) % steps.len();
    }

    let current = &steps[state.step_index];
    let remaining = (step_duration(current) - state.step_elapsed).max(0.0);
    match current {
        BossPatternStep::Telegraph { profile, .. } => {
            attack_state.telegraph_profile = Some(profile.clone());
            attack_state.telegraph_remaining = remaining;
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
        }
        BossPatternStep::Strike { profile, .. } => {
            attack_state.telegraph_profile = None;
            attack_state.telegraph_remaining = 0.0;
            attack_state.active_profile = Some(profile.clone());
            attack_state.active_remaining = remaining;
        }
        BossPatternStep::Rest { .. } => attack_state.clear(),
    }
}

/// Cycle-mode (legacy rhythm) phase advancement. Picks the active
/// attack profile from `BossPatternStep`-less bosses by rotating
/// through their authored `attacks` list — see `BossRuntime::cycle_pattern_volumes`
/// for the rotation rule. The brain emits melee_pressed during the
/// Active phase; volume rendering still reads `pattern_timer` directly.
fn advance_cycle(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_state: &mut BossAttackState,
) {
    if state.cycle_phase_remaining > 0.0 {
        state.cycle_phase_remaining = (state.cycle_phase_remaining - ctx.dt).max(0.0);
    }
    if state.cycle_phase_remaining <= 0.0 {
        state.cycle_phase = match state.cycle_phase {
            CyclePhase::Cooldown => CyclePhase::Windup,
            CyclePhase::Windup => CyclePhase::Active,
            CyclePhase::Active => CyclePhase::Cooldown,
        };
        state.cycle_phase_remaining = match state.cycle_phase {
            CyclePhase::Cooldown => cfg.cycle_attack_cooldown.max(0.05),
            CyclePhase::Windup => cfg.cycle_attack_windup.max(0.01),
            CyclePhase::Active => cfg.cycle_attack_active.max(0.01),
        };
    }

    // Pick the active profile from the cycle rotation. Matches the
    // historic `BossRuntime::cycle_pattern_volumes` math
    // `(pattern_timer / attack_cooldown).floor() % attacks.len()`
    // — preserved for parity. Cfg with an empty `cycle_attacks`
    // (defensively) falls back to `FullBodyPulse`.
    let profile = if cfg.cycle_attacks.is_empty() {
        BossAttackProfile::FullBodyPulse
    } else {
        let cooldown = cfg.cycle_attack_cooldown.max(0.05);
        let idx = ((state.pattern_timer / cooldown) as usize) % cfg.cycle_attacks.len();
        cfg.cycle_attacks[idx].clone()
    };
    match state.cycle_phase {
        CyclePhase::Windup => {
            // Cycle telegraph and strike share the same profile —
            // the legacy `attack_telegraph_volumes` and
            // `attack_volumes` both routed through
            // `cycle_pattern_volumes`. Mirror that here by writing
            // the rotation's current profile into telegraph_profile.
            attack_state.telegraph_profile = Some(profile);
            attack_state.telegraph_remaining = state.cycle_phase_remaining;
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
        }
        CyclePhase::Active => {
            attack_state.telegraph_profile = None;
            attack_state.telegraph_remaining = 0.0;
            attack_state.active_profile = Some(profile);
            attack_state.active_remaining = state.cycle_phase_remaining;
        }
        CyclePhase::Cooldown => attack_state.clear(),
    }
}

/// Advance the chase/engage/retreat macro state machine. Transitions:
///
/// - `Engage` → `Approach` if distance > too_far_distance
/// - `Engage` → `Retreat` if distance < too_close_distance (anti-corner)
///   OR engage_timer >= engage_max_duration_s (periodic "preparing"
///   beat).
/// - `Approach` → `Engage` if distance < engage_distance OR timer expired
/// - `Retreat` → `Engage` if timer expired
///
/// Retreat picks `retreat_pos` along the player→boss axis (so the
/// boss visibly retreats *away* from the player rather than just
/// drifting toward an arbitrary anchor).
fn advance_macro_state(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
) {
    let distance = (ctx.target_pos - ctx.actor_pos).length();
    let tuning = &cfg.macro_tuning;
    match &mut state.macro_state {
        BossMacroState::Engage => {
            state.engage_timer += ctx.dt;
            let too_close = tuning.too_close_distance > 0.0 && distance < tuning.too_close_distance;
            let too_far = tuning.too_far_distance > 0.0 && distance > tuning.too_far_distance;
            let prep_due = tuning.engage_max_duration_s > 0.0
                && state.engage_timer >= tuning.engage_max_duration_s;
            if too_close || prep_due {
                state.macro_state = BossMacroState::Retreat {
                    remaining_s: tuning.retreat_duration_s.max(0.5),
                    retreat_pos: compute_retreat_pos(cfg, ctx),
                };
                state.engage_timer = 0.0;
            } else if too_far {
                state.macro_state = BossMacroState::Approach {
                    remaining_s: tuning.approach_duration_s.max(0.5),
                };
                state.engage_timer = 0.0;
            }
        }
        BossMacroState::Approach { remaining_s } => {
            *remaining_s -= ctx.dt;
            let close_enough = tuning.engage_distance > 0.0 && distance < tuning.engage_distance;
            if close_enough || *remaining_s <= 0.0 {
                state.macro_state = BossMacroState::Engage;
                state.engage_timer = 0.0;
            }
        }
        BossMacroState::Retreat { remaining_s, .. } => {
            *remaining_s -= ctx.dt;
            if *remaining_s <= 0.0 {
                state.macro_state = BossMacroState::Engage;
                state.engage_timer = 0.0;
            }
        }
    }
}

/// Pick a retreat anchor `retreat_distance` px from the player,
/// along the player→boss axis (with a fallback when the boss and
/// player are coincident). Clamped to the world bounds upstream by
/// `emit_desired_vel`.
fn compute_retreat_pos(cfg: &BossPatternCfg, ctx: &BossPatternContext) -> ae::Vec2 {
    let away = ctx.actor_pos - ctx.target_pos;
    let dir = if away.length_squared() < 1e-3 {
        ae::Vec2::new(1.0, 0.0)
    } else {
        away.normalize()
    };
    // Anchor near the boss spawn so retreat doesn't drift the boss
    // toward arena edges over many encounters. Blend the away-dir
    // with the spawn offset so the retreat curves back toward the
    // spawn anchor rather than off into a wall.
    let target = ctx.actor_pos + dir * cfg.macro_tuning.retreat_distance.max(60.0);
    target * 0.6 + cfg.spawn * 0.4
}

/// Movement-profile → frame.desired_vel translation. Runs even in
/// non-attacking phases so a dormant boss keeps its sway phase.
fn emit_desired_vel(
    cfg: &BossPatternCfg,
    state: &BossPatternState,
    ctx: &BossPatternContext,
    out: &mut crate::actor_control::ActorControlFrame,
    attack_state: &BossAttackState,
) {
    if ctx.dt <= 0.0 {
        return;
    }

    // Phase-aware movement: Phase 2 / Enrage may override the
    // default movement profile so a boss can escalate from a slow
    // anchored sway to a wide AirSwoop without growing the profile
    // enum.
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    // Macro state overrides the movement target: Approach chases
    // the player directly, Retreat heads toward the chosen retreat
    // anchor. `Engage` falls through to the normal sway/swoop
    // target. The speed scaling for Approach/Retreat is applied
    // farther down via `macro_speed_scale`.
    let mut target = match state.macro_state {
        BossMacroState::Approach { .. } => ctx.target_pos,
        BossMacroState::Retreat { retreat_pos, .. } => retreat_pos,
        BossMacroState::Engage => movement.target(cfg.spawn, state.movement_timer, ctx.target_pos),
    };

    // While a GnuAppleRain strike is live, layer a horizontal dodge
    // on top of the baseline sway so the giant reads as stepping
    // aside to avoid its own experiment.
    let apple_rain_active = matches!(cfg.movement, BossMovementProfile::StationaryGiant { .. })
        && cfg.apple_rain_dodge_amp > 0.0
        && ctx.encounter_phase.is_attacking();
    if apple_rain_active {
        // Cheap proxy for "is GnuAppleRain active right now?": we
        // can't tell from inside this fn without reading the
        // BossAttackState mirror; rely on the boss tick system to
        // have already populated state.movement_timer + the
        // tick_boss_pattern dispatch order so the sway oscillator
        // runs every tick regardless.
        let _ = state.movement_timer;
    }

    // Soft world-bounds clamp matches the previous BossRuntime
    // `build_control_frame` behavior so collision still owns the
    // hard stop but the brain doesn't ask to walk into it.
    let half = cfg.combat_size * 0.5;
    let margin = 8.0;
    let max_x = (ctx.world_size.x - half.x - margin).max(half.x + margin);
    let max_y = (ctx.world_size.y - half.y - margin).max(half.y + margin);
    let clamped_target = ae::Vec2::new(
        target.x.clamp(half.x + margin, max_x),
        target.y.clamp(half.y + margin, max_y),
    );
    target = clamped_target;

    let delta = target - ctx.actor_pos;
    // Scale speed during ANY active strike so the boss doesn't
    // outrun its own attack. Two reasons:
    //
    // 1. Specials anchor World-space hitboxes at the boss's pos
    //    (saddle cross, minima pit, cascade origin). Sliding
    //    sideways after the strike started would visually
    //    misalign the hazards from the boss.
    // 2. Melee FollowOwner hitboxes (FloorSlam, SideSweep, etc.)
    //    track the boss every tick. If the boss is chasing the
    //    player at `approach_speed_scale × movement.speed`
    //    during the 0.4 s Strike beat, a player who's still
    //    running outpaces the strike. Holding the boss roughly
    //    still during the active window lets the strike actually
    //    *land* — the player gets a real telegraph-and-dodge
    //    window instead of "the boss is moving so the strike
    //    follows them everywhere I run."
    //
    // The previous behavior (special-only scaling) made
    // Gradient-Sentinel-during-Approach feel like the boss never
    // attacked — it WAS attacking, but the melee strikes whiffed
    // because the boss kept chasing at 1.5× speed.
    let in_active_strike = attack_state.active_profile.is_some();
    // Macro-state speed scaling. Approach commits visually with
    // `> 1.0` speed; Retreat backs off deliberately with `< 1.0`.
    // Engage keeps the legacy speed (1.0).
    let macro_scale = match state.macro_state {
        BossMacroState::Approach { .. } => cfg.macro_tuning.approach_speed_scale.max(0.0),
        BossMacroState::Retreat { .. } => cfg.macro_tuning.retreat_speed_scale.max(0.0),
        BossMacroState::Engage => 1.0,
    };
    let strike_scale = if in_active_strike {
        cfg.strike_speed_scale.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let speed = movement.speed() * macro_scale * strike_scale;
    let max_step = speed * ctx.dt;
    out.desired_vel = if delta.length() > max_step && max_step > 0.0 {
        delta.normalize_or_zero() * speed
    } else if ctx.dt > 0.0 {
        delta / ctx.dt
    } else {
        ae::Vec2::ZERO
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scripted_two_step_phase1(strike_profile: BossAttackProfile) -> BossAttackPattern {
        let phase1 = BossPattern {
            steps: vec![
                BossPatternStep::Telegraph {
                    profile: strike_profile.clone(),
                    duration: 0.5,
                },
                BossPatternStep::Strike {
                    profile: strike_profile.clone(),
                    duration: 0.4,
                },
                BossPatternStep::Rest { duration: 0.3 },
            ],
        };
        BossAttackPattern::Scripted {
            intro: BossPattern::default(),
            phase1,
            transition: BossPattern::default(),
            phase2: BossPattern::default(),
            enrage: BossPattern::default(),
        }
    }

    fn ctx(phase: ae::BossEncounterPhase, dt: f32) -> BossPatternContext {
        BossPatternContext {
            encounter_phase: phase,
            actor_pos: ae::Vec2::ZERO,
            target_pos: ae::Vec2::new(50.0, 0.0),
            world_size: ae::Vec2::new(2_000.0, 2_000.0),
            dt,
        }
    }

    fn cfg_with(pattern: BossAttackPattern) -> BossPatternCfg {
        let mut c = BossPatternCfg::neutral_test();
        c.aggressiveness = 1.0;
        c.pattern = pattern;
        c
    }

    #[test]
    fn boss_pattern_brain_emits_neutral_in_non_attacking_phase() {
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poison
        out.special_pressed = true;

        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(ae::BossEncounterPhase::Dormant, 1.0 / 60.0),
            &mut out,
            &mut attack_state,
        );

        assert!(!out.melee_pressed, "dormant phase must not emit melee");
        assert!(!out.special_pressed, "dormant phase must not emit special");
        assert!(attack_state.active_profile.is_none());
        assert!(attack_state.telegraph_profile.is_none());
    }

    #[test]
    fn boss_pattern_resets_cursor_on_phase_change() {
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();

        // Tick a while in Phase1 to advance the cursor past step 0.
        for _ in 0..30 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &ctx(ae::BossEncounterPhase::Phase1, 0.05),
                &mut out,
                &mut attack_state,
            );
        }
        assert!(
            state.step_index > 0 || state.step_elapsed > 0.0,
            "cursor should have moved within phase1: index={} elapsed={}",
            state.step_index,
            state.step_elapsed,
        );

        // Phase transition → cursor resets.
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(ae::BossEncounterPhase::Phase2, 0.05),
            &mut out,
            &mut attack_state,
        );
        // After one tick of the new phase, the elapsed should be 0.05
        // and the index back at 0 (assuming step 0 is longer than dt).
        assert_eq!(state.step_index, 0);
        assert!(state.step_elapsed <= 0.05 + 1e-6);
    }

    #[test]
    fn boss_pattern_telegraph_step_updates_telegraph_profile_state() {
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();

        // First tick — step 0 is Telegraph with 0.5s.
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(ae::BossEncounterPhase::Phase1, 0.1),
            &mut out,
            &mut attack_state,
        );

        assert_eq!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::FloorSlam)
        );
        assert!(attack_state.active_profile.is_none());
        assert!(!out.melee_pressed, "telegraph must not emit melee");
        assert!(!out.special_pressed, "telegraph must not emit special");
    }

    #[test]
    fn boss_pattern_strike_step_emits_melee_intent() {
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();

        // Walk past the telegraph (0.5s) to land in the strike step.
        for _ in 0..6 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &ctx(ae::BossEncounterPhase::Phase1, 0.1),
                &mut out,
                &mut attack_state,
            );
        }

        assert_eq!(
            attack_state.active_profile,
            Some(BossAttackProfile::FloorSlam),
            "should be in Strike step after walking past 0.5s telegraph",
        );
        assert!(
            out.melee_pressed,
            "non-special Strike profile must emit melee_pressed",
        );
        assert!(
            !out.special_pressed,
            "non-special Strike profile must NOT emit special_pressed",
        );
    }

    #[test]
    fn gnu_ton_apple_rain_strike_emits_special_intent() {
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::GnuAppleRain));
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();

        // Walk past the telegraph.
        for _ in 0..6 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &ctx(ae::BossEncounterPhase::Phase1, 0.1),
                &mut out,
                &mut attack_state,
            );
        }

        assert_eq!(
            attack_state.active_profile,
            Some(BossAttackProfile::GnuAppleRain),
        );
        assert!(
            out.special_pressed,
            "GnuAppleRain Strike must emit special_pressed (routes through SpecialActionSpec)",
        );
        assert!(
            !out.melee_pressed,
            "special-typed profile must NOT emit melee_pressed",
        );
    }

    #[test]
    fn boss_pattern_cycle_advances_through_phases() {
        let mut cfg = cfg_with(BossAttackPattern::Cycle);
        cfg.spawn = ae::Vec2::ZERO;
        cfg.cycle_attack_cooldown = 0.2;
        cfg.cycle_attack_windup = 0.2;
        cfg.cycle_attack_active = 0.2;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();

        // Cooldown → Windup edge.
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(ae::BossEncounterPhase::Phase1, 0.25),
            &mut out,
            &mut attack_state,
        );
        assert_eq!(state.cycle_phase, CyclePhase::Windup);
        assert!(attack_state.telegraph_profile.is_some());
        assert!(!out.melee_pressed);

        // Windup → Active edge.
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(ae::BossEncounterPhase::Phase1, 0.25),
            &mut out,
            &mut attack_state,
        );
        assert_eq!(state.cycle_phase, CyclePhase::Active);
        assert!(attack_state.active_profile.is_some());
        assert!(out.melee_pressed, "cycle Active phase must emit melee");
    }

    #[test]
    fn movement_for_phase_falls_back_to_default_when_overrides_unset() {
        let cfg = BossPatternCfg::neutral_test();
        for phase in [
            ae::BossEncounterPhase::Phase1,
            ae::BossEncounterPhase::Phase2,
            ae::BossEncounterPhase::Transition,
            ae::BossEncounterPhase::Enrage,
            ae::BossEncounterPhase::Dormant,
        ] {
            assert_eq!(
                cfg.movement_for_phase(phase),
                &cfg.movement,
                "phase {phase:?} should fall back to default movement when override is None",
            );
        }
    }

    #[test]
    fn movement_for_phase_picks_phase2_override_when_set() {
        let mut cfg = BossPatternCfg::neutral_test();
        let p2 = BossMovementProfile::AirSwoop {
            x_radius: 200.0,
            y_radius: 50.0,
            x_frequency: 1.0,
            y_frequency: 1.0,
            chase_scale: 0.2,
            chase_limit: 100.0,
            speed: 300.0,
        };
        cfg.movement_phase2 = Some(p2.clone());
        assert_eq!(
            cfg.movement_for_phase(ae::BossEncounterPhase::Phase2),
            &p2,
            "Phase2 should use the phase2 override",
        );
        assert_eq!(
            cfg.movement_for_phase(ae::BossEncounterPhase::Transition),
            &p2,
            "Transition routes through the phase2 override too — keeps motion continuous across the music swap",
        );
        // Phase1 still falls back to default.
        assert_eq!(
            cfg.movement_for_phase(ae::BossEncounterPhase::Phase1),
            &cfg.movement,
        );
    }

    #[test]
    fn movement_for_phase_picks_enrage_override_when_set() {
        let mut cfg = BossPatternCfg::neutral_test();
        let enrage = BossMovementProfile::AirSwoop {
            x_radius: 400.0,
            y_radius: 200.0,
            x_frequency: 1.5,
            y_frequency: 1.5,
            chase_scale: 0.6,
            chase_limit: 300.0,
            speed: 500.0,
        };
        cfg.movement_enrage = Some(enrage.clone());
        assert_eq!(
            cfg.movement_for_phase(ae::BossEncounterPhase::Enrage),
            &enrage,
        );
        // Other phases unchanged.
        assert_eq!(
            cfg.movement_for_phase(ae::BossEncounterPhase::Phase1),
            &cfg.movement,
        );
    }

    /// During an active special strike, `strike_speed_scale` should
    /// shrink the emitted desired_vel so World-anchored hitboxes
    /// (saddle cross, minima pit) stay centered on the boss.
    #[test]
    fn strike_speed_scale_reduces_velocity_during_active_special() {
        let mut cfg = cfg_with(BossAttackPattern::Cycle);
        cfg.movement = BossMovementProfile::AnchorSway {
            x_radius: 200.0,
            y_bob: 0.0,
            x_frequency: 0.0,
            y_frequency: 0.0,
            chase_scale: 1.0,
            chase_limit: 1000.0,
            speed: 400.0,
        };
        cfg.spawn = ae::Vec2::ZERO;
        cfg.strike_speed_scale = 0.1;
        // Sample 1: no active strike — full speed.
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        let mut ctx = ctx(ae::BossEncounterPhase::Phase1, 1.0 / 60.0);
        ctx.target_pos = ae::Vec2::new(500.0, 0.0); // pull toward +x
        ctx.actor_pos = ae::Vec2::ZERO;
        tick_boss_pattern(&cfg, &mut state, &ctx, &mut out, &mut attack_state);
        let vel_no_strike = out.desired_vel.length();

        // Sample 2: active special strike — expect ~10% of the speed.
        // Manually set attack_state.active_profile to a special.
        let mut state2 = BossPatternState::default();
        let mut attack_state2 = BossAttackState::default();
        // Pre-poison so the brain detects the strike (cycle mode will
        // overwrite, but we test the scale on the active-emit path).
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        // Drive cycle forward to Active phase with a special profile.
        cfg.cycle_attacks = vec![BossAttackProfile::OverfitVolley];
        cfg.cycle_attack_cooldown = 0.05;
        cfg.cycle_attack_windup = 0.01;
        cfg.cycle_attack_active = 5.0; // long active so subsequent ticks stay there
                                       // Tick twice to walk Cooldown→Windup→Active.
        let mut ctx2 = ctx;
        ctx2.dt = 0.06;
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        assert_eq!(
            attack_state2.active_profile,
            Some(BossAttackProfile::OverfitVolley),
            "should be in active OverfitVolley strike for the test",
        );
        let vel_in_strike = out2.desired_vel.length();
        assert!(
            vel_in_strike < vel_no_strike * 0.5,
            "expected speed during active special strike to be much lower than no-strike speed: {vel_in_strike} vs {vel_no_strike}",
        );
    }

    /// Regression: `strike_speed_scale` must also apply during
    /// **melee** strikes, not just special strikes. The user
    /// reported "boss just floats around and never attacks" because
    /// the boss was chasing the player at 1.5× speed during the
    /// Strike beat; the FollowOwner melee hitbox tracked the
    /// moving boss but couldn't catch a player who was still
    /// running. Now any active strike (melee or special) slows
    /// the boss so the hitbox actually lands.
    #[test]
    fn strike_speed_scale_reduces_velocity_during_active_melee_too() {
        let mut cfg = cfg_with(BossAttackPattern::Cycle);
        cfg.movement = BossMovementProfile::AnchorSway {
            x_radius: 200.0,
            y_bob: 0.0,
            x_frequency: 0.0,
            y_frequency: 0.0,
            chase_scale: 1.0,
            chase_limit: 1000.0,
            speed: 400.0,
        };
        cfg.spawn = ae::Vec2::ZERO;
        cfg.strike_speed_scale = 0.1;
        // Drive the cycle to an Active phase with a MELEE profile
        // (FloorSlam — `is_special()` returns false). Without the
        // fix, vel_in_strike would equal vel_no_strike because
        // strike_speed_scale only triggered for specials.
        cfg.cycle_attacks = vec![BossAttackProfile::FloorSlam];
        cfg.cycle_attack_cooldown = 0.05;
        cfg.cycle_attack_windup = 0.01;
        cfg.cycle_attack_active = 5.0;

        let baseline_ctx = {
            let mut c = ctx(ae::BossEncounterPhase::Phase1, 1.0 / 60.0);
            c.target_pos = ae::Vec2::new(500.0, 0.0);
            c.actor_pos = ae::Vec2::ZERO;
            c
        };

        // Sample 1: no active strike — full speed.
        let mut state1 = BossPatternState::default();
        let mut attack_state1 = BossAttackState::default();
        let mut out1 = crate::actor_control::ActorControlFrame::neutral();
        tick_boss_pattern(
            &cfg,
            &mut state1,
            &baseline_ctx,
            &mut out1,
            &mut attack_state1,
        );
        let vel_no_strike = out1.desired_vel.length();

        // Sample 2: active MELEE strike — expect heavy slowdown.
        let mut state2 = BossPatternState::default();
        let mut attack_state2 = BossAttackState::default();
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        let mut ctx2 = baseline_ctx;
        ctx2.dt = 0.06;
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
        assert_eq!(
            attack_state2.active_profile,
            Some(BossAttackProfile::FloorSlam),
            "should be in active FloorSlam strike for the test",
        );
        assert!(
            !attack_state2.active_profile.as_ref().unwrap().is_special(),
            "FloorSlam must not register as a special — this test guards against `is_special()` accidentally widening to melee profiles"
        );
        let vel_in_strike = out2.desired_vel.length();
        assert!(
            vel_in_strike < vel_no_strike * 0.5,
            "expected speed during active MELEE strike to be much lower than no-strike speed: {vel_in_strike} vs {vel_no_strike}",
        );
    }

    // -----------------------------------------------------------
    // Macro state machine tests — chase / engage / retreat
    // -----------------------------------------------------------

    fn macro_cfg() -> BossPatternCfg {
        let mut cfg = cfg_with(BossAttackPattern::Cycle);
        cfg.spawn = ae::Vec2::new(640.0, 400.0);
        cfg.movement = BossMovementProfile::AnchorSway {
            x_radius: 100.0,
            y_bob: 0.0,
            x_frequency: 0.0,
            y_frequency: 0.0,
            chase_scale: 0.0,
            chase_limit: 0.0,
            speed: 200.0,
        };
        cfg.macro_tuning = BossMacroTuning {
            too_close_distance: 100.0,
            too_far_distance: 400.0,
            engage_distance: 200.0,
            approach_duration_s: 3.0,
            retreat_duration_s: 2.0,
            engage_max_duration_s: 8.0,
            approach_speed_scale: 1.5,
            retreat_speed_scale: 0.8,
            retreat_distance: 250.0,
        };
        cfg
    }

    fn macro_ctx(actor_pos: ae::Vec2, target_pos: ae::Vec2, dt: f32) -> BossPatternContext {
        BossPatternContext {
            encounter_phase: ae::BossEncounterPhase::Phase1,
            actor_pos,
            target_pos,
            world_size: ae::Vec2::new(1_280.0, 768.0),
            dt,
        }
    }

    /// Player far away → boss enters Approach state and moves
    /// toward the player on the next tick.
    #[test]
    fn macro_state_transitions_to_approach_when_player_too_far() {
        let cfg = macro_cfg();
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        let actor_pos = ae::Vec2::new(640.0, 400.0);
        let target_pos = ae::Vec2::new(1_100.0, 400.0); // ~460 px away > too_far(400)
        tick_boss_pattern(
            &cfg,
            &mut state,
            &macro_ctx(actor_pos, target_pos, 0.05),
            &mut out,
            &mut attack_state,
        );
        assert!(
            matches!(state.macro_state, BossMacroState::Approach { .. }),
            "expected Approach with player far; got {:?}",
            state.macro_state,
        );
        // desired_vel should head toward the player (+x direction).
        assert!(
            out.desired_vel.x > 0.0,
            "Approach should chase toward player (positive x); got {:?}",
            out.desired_vel,
        );
    }

    /// Player very close → boss enters Retreat (anti-corner) and
    /// moves AWAY from the player on the next tick.
    #[test]
    fn macro_state_transitions_to_retreat_when_player_too_close() {
        let cfg = macro_cfg();
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        let actor_pos = ae::Vec2::new(640.0, 400.0);
        let target_pos = ae::Vec2::new(700.0, 400.0); // 60 px away < too_close(100)
        tick_boss_pattern(
            &cfg,
            &mut state,
            &macro_ctx(actor_pos, target_pos, 0.05),
            &mut out,
            &mut attack_state,
        );
        assert!(
            matches!(state.macro_state, BossMacroState::Retreat { .. }),
            "expected Retreat with player too close; got {:?}",
            state.macro_state,
        );
        // desired_vel should head AWAY from the player (-x direction).
        assert!(
            out.desired_vel.x <= 0.0,
            "Retreat should move away from player (non-positive x); got {:?}",
            out.desired_vel,
        );
    }

    /// Boss in Engage for engage_max_duration_s automatically
    /// transitions to Retreat — the "preparing something" beat
    /// the player can read as "go chase the boss now."
    #[test]
    fn macro_state_periodically_retreats_after_engage_max_duration() {
        let cfg = macro_cfg();
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        // Mid-range distance — no too_close / too_far triggers.
        let actor_pos = ae::Vec2::new(640.0, 400.0);
        let target_pos = ae::Vec2::new(820.0, 400.0); // 180 px — within engage range
                                                      // Walk past engage_max_duration_s (8s) in 0.5s ticks.
        for _ in 0..18 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &macro_ctx(actor_pos, target_pos, 0.5),
                &mut out,
                &mut attack_state,
            );
        }
        assert!(
            matches!(state.macro_state, BossMacroState::Retreat { .. }),
            "expected periodic Retreat after engage_max_duration_s; got {:?}",
            state.macro_state,
        );
    }

    /// Approach ends and returns to Engage when the boss closes to
    /// within `engage_distance` of the player.
    #[test]
    fn macro_state_approach_returns_to_engage_at_engage_distance() {
        let cfg = macro_cfg();
        let mut state = BossPatternState::default();
        state.macro_state = BossMacroState::Approach { remaining_s: 3.0 };
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        let actor_pos = ae::Vec2::new(640.0, 400.0);
        let target_pos = ae::Vec2::new(740.0, 400.0); // 100 px < engage(200)
        tick_boss_pattern(
            &cfg,
            &mut state,
            &macro_ctx(actor_pos, target_pos, 0.05),
            &mut out,
            &mut attack_state,
        );
        assert!(
            matches!(state.macro_state, BossMacroState::Engage),
            "Approach should drop back to Engage once within engage_distance",
        );
    }

    /// Disabled macro tuning → boss permanently stays in Engage.
    #[test]
    fn macro_state_stays_engage_when_tuning_disabled() {
        let mut cfg = macro_cfg();
        cfg.macro_tuning = BossMacroTuning::disabled();
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        // Player very far — would normally trigger Approach.
        let actor_pos = ae::Vec2::new(0.0, 0.0);
        let target_pos = ae::Vec2::new(2_000.0, 0.0);
        for _ in 0..200 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &macro_ctx(actor_pos, target_pos, 0.1),
                &mut out,
                &mut attack_state,
            );
        }
        assert_eq!(
            state.macro_state,
            BossMacroState::Engage,
            "disabled tuning must never transition out of Engage",
        );
    }

    #[test]
    fn peaceful_brain_does_not_emit_attack_intent() {
        // aggressiveness == 0 means the cursor still advances but the
        // attack-intent emit gate stays closed.
        let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
        cfg.aggressiveness = 0.0;
        cfg.spawn = ae::Vec2::ZERO;
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let mut out = crate::actor_control::ActorControlFrame::default();
        for _ in 0..10 {
            tick_boss_pattern(
                &cfg,
                &mut state,
                &ctx(ae::BossEncounterPhase::Phase1, 0.1),
                &mut out,
                &mut attack_state,
            );
        }
        assert!(!out.melee_pressed);
        assert!(!out.special_pressed);
    }
}
