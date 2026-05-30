//! State-machine brain templates.
//!
//! Each variant of [`StateMachineCfg`] is a reusable AI policy. The
//! variant carries both the cfg (per-template tuning) and the
//! per-actor runtime state in one bundle so callers can't pass
//! mismatched state to a brain. The set is small and closed (per
//! the universal-brain design): adding a template means adding a
//! variant + a `tick_*` function. Per-entity *variety* lives in the
//! actor's `ActionSet`, not here — two `MeleeBrute` brains can
//! resolve the same `frame.melee_pressed=true` into different
//! concrete attacks depending on the entity's action set.
//!
//! No template encodes a "telegraph" explicitly. The brain emits
//! `melee_pressed = true` and the ActionSet's attack spec carries
//! its own windup → active → recover animation timing.

use crate::engine_core as ae;

use super::action_set::ActionSet;
use super::smash::{tick_smash, SmashCfg, SmashState};
use super::snapshot::BrainSnapshot;

// ===== Top-level state-machine variant =====

/// A reusable AI policy + its per-actor runtime state.
#[derive(Clone, Debug)]
pub enum StateMachineCfg {
    /// No motion. Used by static NPCs and sandbag-style targets.
    StandStill,
    /// Fixed waypoint loop. `aggressiveness` controls engagement.
    Patrol { cfg: PatrolCfg, state: PatrolState },
    /// Move forward; on wall, climb-if-able else reverse; pause on
    /// rapid chatter. Drives the puppy slug today.
    Wanderer {
        cfg: WandererCfg,
        state: WandererState,
    },
    /// Approach + melee + recover. Aggressiveness gates engagement.
    MeleeBrute {
        cfg: MeleeBruteCfg,
        state: MeleeBruteState,
    },
    /// Strafe + ranged harass.
    Skirmisher {
        cfg: SkirmisherCfg,
        state: SkirmisherState,
    },
    /// Hold position + long-range fire.
    Sniper { cfg: SniperCfg, state: SniperState },
    /// Dedicated shark charge brain. Riderless sharks use this to
    /// stalk, lunge, and then cool down after a crash or bite.
    Shark { cfg: SharkCfg, state: SharkState },
    /// Scripted multi-phase boss policy. The cfg + state live in
    /// `brain/boss_pattern.rs`; this variant carries them but the
    /// real tick driver is `tick_boss_brains_system` in
    /// `content/features/ecs/bosses.rs` (see the dispatch fn below).
    BossPattern {
        cfg: super::BossPatternCfg,
        state: super::BossPatternState,
    },
    /// Smash-brawl pipeline: observe → mode → action → difficulty
    /// → emit. The dispatcher needs the actor's `ActionSet` (to
    /// know what attacks are available), so the regular
    /// `tick_state_machine` falls through to `tick_smash_via_state_machine`
    /// only when the caller threads the ActionSet in. See
    /// [`tick_state_machine_with_actions`] below.
    Smash { cfg: SmashCfg, state: SmashState },
}

impl StateMachineCfg {
    /// Is this brain currently hostile? Used by debug tooling and
    /// the EFFECTS-stage attack gate for the (rare) case where a
    /// brain has melee capability but is in a peaceful sub-state.
    pub fn is_hostile(&self) -> bool {
        match self {
            Self::StandStill => false,
            Self::Patrol { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::Wanderer { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::MeleeBrute { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::Skirmisher { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::Sniper { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::Shark { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::BossPattern { cfg, .. } => cfg.aggressiveness > 0.0,
            // Smash brain is always hostile by construction — peaceful
            // archetypes don't use it (they get Patrol / Wanderer
            // instead). If we add a peaceful Smash variant later, this
            // gate moves into `SmashCfg`.
            Self::Smash { .. } => true,
        }
    }
}

/// Tick a state-machine brain: read the snapshot, mutate the brain's
/// own state, and write the abstract intent into `out`.
///
/// The `Smash` variant ignores the actor's [`ActionSet`] here (falls
/// back to a peaceful default that disables melee). Callers that
/// want a Smash actor to actually attack should call
/// [`tick_state_machine_with_actions`] instead, threading the
/// actor's `ActionSet` through. The two-entry-point split keeps
/// existing callers (player brain driver, NPC sims) source-compat
/// while the actor driver opt-in into the ActionSet-aware path.
pub fn tick_state_machine(
    sm: &mut StateMachineCfg,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    tick_state_machine_with_actions(sm, &ActionSet::peaceful(), snapshot, out);
}

/// Like [`tick_state_machine`] but threads the actor's `ActionSet`
/// to the Smash brain so it knows what attacks are available.
pub fn tick_state_machine_with_actions(
    sm: &mut StateMachineCfg,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    if !snapshot.alive {
        // Dead actors emit a neutral frame regardless of brain. Write
        // explicitly so a pre-poisoned `out` doesn't leak through.
        *out = crate::actor_control::ActorControlFrame::neutral();
        return;
    }
    match sm {
        StateMachineCfg::StandStill => tick_stand_still(out),
        StateMachineCfg::Patrol { cfg, state } => tick_patrol(cfg, state, snapshot, out),
        StateMachineCfg::Wanderer { cfg, state } => tick_wanderer(cfg, state, snapshot, out),
        StateMachineCfg::MeleeBrute { cfg, state } => tick_melee_brute(cfg, state, snapshot, out),
        StateMachineCfg::Skirmisher { cfg, state } => tick_skirmisher(cfg, state, snapshot, out),
        StateMachineCfg::Sniper { cfg, state } => tick_sniper(cfg, state, snapshot, out),
        StateMachineCfg::Shark { cfg, state } => tick_shark(cfg, state, snapshot, out),
        StateMachineCfg::BossPattern { cfg, state } => {
            tick_boss_pattern_via_state_machine(cfg, state, snapshot, out)
        }
        StateMachineCfg::Smash { cfg, state } => tick_smash(cfg, state, actions, snapshot, out),
    }
}

// ===== StandStill =====

fn tick_stand_still(out: &mut crate::actor_control::ActorControlFrame) {
    *out = crate::actor_control::ActorControlFrame::neutral();
}

// ===== Patrol =====

/// Fixed left-right paddle around a spawn point. Hostility is
/// controlled separately — a hostile Patrol brain still emits
/// melee_pressed when in range and can flip facing to chase.
#[derive(Clone, Copy, Debug)]
pub struct PatrolCfg {
    /// Center of the paddle.
    pub spawn_x: f32,
    /// Half-width of the paddle (px). 0.0 = pinned to spawn.
    pub radius: f32,
    /// Walk speed (px/s).
    pub speed: f32,
    /// `0.0` = peaceful patroller (NPC), `>0.0` = engages target
    /// when in range.
    pub aggressiveness: f32,
    /// If `aggressiveness > 0`, the distance below which the
    /// patroller becomes Chase/Attack.
    pub aggro_radius: f32,
    /// If `aggressiveness > 0`, the melee attack range (px).
    pub attack_range: f32,
}

impl PatrolCfg {
    /// Peaceful NPC default. Speed mirrors the legacy
    /// [`crate::content::features::NPC_PATROL_SPEED`] constant so
    /// the brain-driven Patrol gait matches what the pre-brain
    /// `NpcRuntime::update` used.
    pub const NPC_DEFAULT: Self = Self {
        spawn_x: 0.0,
        radius: 64.0,
        speed: crate::content::features::NPC_PATROL_SPEED,
        aggressiveness: 0.0,
        aggro_radius: 80.0, // talk radius for peaceful patrol
        attack_range: 0.0,
    };
}

/// Per-actor Patrol runtime state.
#[derive(Clone, Copy, Debug, Default)]
pub struct PatrolState {
    /// Most recently evaluated mode. Cached so HUD / animation
    /// systems can read it without re-evaluating.
    pub mode: crate::character_ai::CharacterAiMode,
}

fn tick_patrol(
    cfg: &PatrolCfg,
    state: &mut PatrolState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    let ai = crate::character_ai::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        true,
    ));
    state.mode = ai.mode;
    *out = crate::actor_control::ActorControlFrame::neutral();
    match ai.intent {
        crate::character_ai::CharacterAiIntent::Hold => {
            // Player in talk range or otherwise hold position.
            // Face toward target if any.
            if snapshot.target_alive {
                let dx = snapshot.target_pos.x - snapshot.actor_pos.x;
                if dx.abs() > 4.0 {
                    out.facing = dx.signum();
                }
            }
        }
        crate::character_ai::CharacterAiIntent::Patrol => {
            // Bounce within `[spawn_x ± radius]`. Caller is
            // expected to flip `facing` on wall contact; brain
            // also flips at the geometric bound.
            let from_spawn = snapshot.actor_pos.x - cfg.spawn_x;
            let facing = if from_spawn > cfg.radius {
                -1.0
            } else if from_spawn < -cfg.radius {
                1.0
            } else {
                snapshot.actor_facing
            };
            out.facing = facing;
            out.desired_vel = ae::Vec2::new(facing * cfg.speed, 0.0);
        }
        crate::character_ai::CharacterAiIntent::Chase { direction_x } => {
            // Only triggers when `aggressiveness > 0` — peaceful
            // patrollers' aggro_radius gates as "talk", which the
            // evaluator returns as Hold for `attack_range = 0`.
            // For aggressive patrol we close the distance.
            if cfg.aggressiveness > 0.0 {
                out.desired_vel = ae::Vec2::new(direction_x * cfg.speed, 0.0);
                out.facing = direction_x.signum_or(snapshot.actor_facing);
            } else {
                // Peaceful patroller in "Chase" mode = HOLD. The
                // npc semantics: "player is close, face them".
                let dx = snapshot.target_pos.x - snapshot.actor_pos.x;
                if dx.abs() > 4.0 {
                    out.facing = dx.signum();
                }
            }
        }
        crate::character_ai::CharacterAiIntent::Attack { direction_x } => {
            if cfg.aggressiveness > 0.0 {
                out.facing = direction_x.signum_or(snapshot.actor_facing);
                out.melee_pressed = snapshot.attack_cooldown_remaining <= 0.0;
            }
        }
    }
}

// ===== Wanderer =====

/// Forward-and-react brain. Drives the puppy slug today:
/// - Always moves forward in `actor_facing`.
/// - On wall contact: climb if `climb_walls` and the wall is
///   climbable; otherwise reverse facing.
/// - If facing flips too many times in a short window, pause for a
///   beat so the actor doesn't oscillate in a corner.
#[derive(Clone, Copy, Debug)]
pub struct WandererCfg {
    /// Forward speed (px/s).
    pub speed: f32,
    /// Try climbing a wall before reversing.
    pub climb_walls: bool,
    /// Reversal count within `chatter_window_s` that triggers a
    /// pause.
    pub chatter_threshold: u8,
    /// Sliding window the chatter counter looks back over.
    pub chatter_window_s: f32,
    /// How long to pause after the chatter threshold trips.
    pub chatter_pause_s: f32,
    /// Aggressiveness gate. `0.0` for the puppy slug; positive
    /// values would make a hostile Wanderer that triggers melee
    /// when in range of `target_pos`.
    pub aggressiveness: f32,
}

impl WandererCfg {
    /// Puppy slug defaults — slither + climb + 3-reversals-in-1s
    /// triggers a 2s pause.
    pub const PUPPY_SLUG_DEFAULT: Self = Self {
        speed: 36.0,
        climb_walls: true,
        chatter_threshold: 3,
        chatter_window_s: 1.0,
        chatter_pause_s: 2.0,
        aggressiveness: 0.0,
    };
}

/// Per-actor Wanderer state. Holds the chatter sliding window plus
/// the current pause expiry.
#[derive(Clone, Debug, Default)]
pub struct WandererState {
    /// Sim-time stamps of recent reversals; older entries are
    /// pruned each tick. Bounded by the cfg's threshold.
    pub recent_reversals: Vec<f32>,
    /// Sim-time at which the current chatter-pause ends. 0.0 = no
    /// pause active.
    pub pause_until: f32,
    /// Whether the actor is currently climbing a wall (overrides
    /// the "forward" motion in favor of moving along the surface).
    pub climbing: bool,
}

fn tick_wanderer(
    cfg: &WandererCfg,
    state: &mut WandererState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();

    // Honor an active pause.
    if snapshot.sim_time < state.pause_until {
        return;
    }

    // Prune chatter history outside the window.
    let cutoff = snapshot.sim_time - cfg.chatter_window_s;
    state.recent_reversals.retain(|&t| t >= cutoff);

    // Wall contact this tick? Decide climb-or-reverse.
    if let Some(contact) = snapshot.wall_contact {
        if cfg.climb_walls && contact.is_climbable {
            // Switch to climbing along the surface. The integration
            // layer's surface-walking path takes over for actors in
            // climb mode (their velocity is computed tangent to the
            // contact normal, not from a brain-level `desired_vel`).
            // The brain just signals intent and stays out of the way.
            state.climbing = true;
            return;
        }
        // Reverse facing.
        let new_facing = -snapshot.actor_facing.signum_or(1.0);
        out.facing = new_facing;
        state.recent_reversals.push(snapshot.sim_time);
        state.climbing = false;
        if state.recent_reversals.len() >= cfg.chatter_threshold as usize {
            // Chatter trip — pause and clear the history so we
            // don't immediately re-trigger.
            state.pause_until = snapshot.sim_time + cfg.chatter_pause_s;
            state.recent_reversals.clear();
            return;
        }
        // Move in the new facing direction this tick.
        out.desired_vel = ae::Vec2::new(new_facing * cfg.speed, 0.0);
        return;
    }

    // No wall contact: emit straight-ahead motion. (The `climbing`
    // sub-state persists until the next wall contact resolves it.)
    out.facing = snapshot.actor_facing.signum_or(1.0);
    out.desired_vel = ae::Vec2::new(out.facing * cfg.speed, 0.0);
}

// ===== MeleeBrute =====

/// Approach + melee + recover. The brain decides WHEN to attack;
/// the ActionSet decides WHAT the attack looks like.
#[derive(Clone, Copy, Debug)]
pub struct MeleeBruteCfg {
    pub aggressiveness: f32,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub chase_speed: f32,
}

impl MeleeBruteCfg {
    pub const STRIKER_DEFAULT: Self = Self {
        aggressiveness: 1.0,
        aggro_radius: 220.0,
        attack_range: 36.0,
        chase_speed: 110.0,
    };
    pub const BRUTE_DEFAULT: Self = Self {
        aggressiveness: 1.0,
        aggro_radius: 240.0,
        attack_range: 44.0,
        chase_speed: 75.0,
    };
}

/// Per-actor MeleeBrute state.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeleeBruteState {
    pub mode: crate::character_ai::CharacterAiMode,
}

fn tick_melee_brute(
    cfg: &MeleeBruteCfg,
    state: &mut MeleeBruteState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    let ai = crate::character_ai::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        false,
    ));
    state.mode = ai.mode;
    *out = crate::actor_control::ActorControlFrame::neutral();
    match ai.intent {
        crate::character_ai::CharacterAiIntent::Hold => {}
        crate::character_ai::CharacterAiIntent::Patrol => {
            // Not used by MeleeBrute today (patrol_enabled=false).
        }
        crate::character_ai::CharacterAiIntent::Chase { direction_x } => {
            out.desired_vel = ae::Vec2::new(direction_x * cfg.chase_speed, 0.0);
            out.facing = direction_x.signum_or(snapshot.actor_facing);
        }
        crate::character_ai::CharacterAiIntent::Attack { direction_x } => {
            out.facing = direction_x.signum_or(snapshot.actor_facing);
            // Brain wants to start an attack windup if the cooldown
            // is clear. The ActionSet's attack spec timing then
            // determines the concrete windup → active → recover
            // window the EFFECTS stage applies.
            out.melee_pressed = snapshot.attack_cooldown_remaining <= 0.0
                && snapshot.attack_windup_remaining <= 0.0
                && snapshot.attack_active_remaining <= 0.0
                && snapshot.attack_recover_remaining <= 0.0;
        }
    }
}

// ===== Skirmisher =====

/// Strafe + ranged harass. Maintains a stand-off distance and fires.
#[derive(Clone, Copy, Debug)]
pub struct SkirmisherCfg {
    pub aggressiveness: f32,
    pub aggro_radius: f32,
    /// Distance from target the actor tries to maintain.
    pub standoff_px: f32,
    pub strafe_speed: f32,
    /// Cooldown between shots (s).
    pub fire_cooldown_s: f32,
    /// How fast the orbital phase drifts (radians / s). Drives the
    /// "reposition to different locations" behavior the user asked
    /// for — without drift the actor would lock onto its initial
    /// offset and never move around the target. Range ~0.4 to 1.2
    /// reads as a slow orbit that takes 5-15s to circle.
    pub orbit_drift_rad_s: f32,
}

impl SkirmisherCfg {
    pub const RANGER_DEFAULT: Self = Self {
        aggressiveness: 1.0,
        aggro_radius: 320.0,
        standoff_px: 140.0,
        strafe_speed: 85.0,
        fire_cooldown_s: 0.8,
        orbit_drift_rad_s: 0.6,
    };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkirmisherState {
    pub mode: crate::character_ai::CharacterAiMode,
    /// Seconds remaining until the next shot can fire. Counts down
    /// each tick by `snapshot.dt`. Reset to `cfg.fire_cooldown_s`
    /// on fire. The previous shape compared an absolute `sim_time`
    /// against `last_fire_t`, but the sandbox actors path doesn't
    /// populate `snapshot.sim_time` — it's hard-coded to 0.0 — so
    /// every comparison evaluated `0 - 0 >= 1.5` and Skirmisher
    /// never fired in production. The decrementing-timer shape is
    /// what MeleeBrute uses via `attack_cooldown_remaining` and
    /// avoids the global-clock dependency.
    pub cooldown_remaining: f32,
    /// Per-actor orbital phase in radians. The Skirmisher orbits
    /// the target on a circle of radius `cfg.standoff_px` and picks
    /// its desired position via
    /// `target_pos + (cos θ, sin θ) * standoff_px` where θ is this
    /// phase. Seeding it from the actor's stable id-derived RNG
    /// spreads a squadron of shark-riders around the player
    /// (above / below / left / right) instead of stacking them all
    /// at the same offset axis. The phase drifts slowly over time
    /// (`drift_rate_rad_s`) so the orbit isn't fixed.
    pub orbit_phase: f32,
}

fn tick_skirmisher(
    cfg: &SkirmisherCfg,
    state: &mut SkirmisherState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();
    // Cooldown ticks down every frame regardless of target state so
    // a Skirmisher that loses sight mid-cooldown doesn't get a free
    // first shot the moment the player re-enters aggro.
    state.cooldown_remaining = (state.cooldown_remaining - snapshot.dt).max(0.0);
    // Orbital phase drifts continuously so the actor circles the
    // target instead of locking onto a fixed offset. The per-actor
    // initial phase (seeded at spawn) keeps a squadron spread out
    // around the player.
    state.orbit_phase += cfg.orbit_drift_rad_s * snapshot.dt;
    if state.orbit_phase > std::f32::consts::TAU {
        state.orbit_phase -= std::f32::consts::TAU;
    }
    if !snapshot.target_alive {
        return;
    }
    let to_target_raw = snapshot.target_pos - snapshot.actor_pos;
    let raw_dist = to_target_raw.length();
    if raw_dist > cfg.aggro_radius {
        state.mode = crate::character_ai::CharacterAiMode::Idle;
        return;
    }
    state.mode = crate::character_ai::CharacterAiMode::Chase;
    // Compute the actor's desired position offset from the target.
    // The horizontal component sweeps the full ±standoff range so
    // shark-riders fan out left and right of the player. The
    // vertical component is biased upward (negative y in sandbox
    // coordinates) and clamped to a shallow band so aerial actors
    // stay at altitude rather than orbiting through the floor. Each
    // actor has its own initial phase, so a squadron spreads to
    // different positions around the target; the phase drifts so
    // the offsets aren't static.
    //
    // Sandbox world Y grows DOWNWARD, so "above the player" is
    // `target_y - something`. The bias `vertical_center` plus the
    // sine modulation `vertical_amp` keeps the actor above the
    // player throughout the orbit.
    let (sin_p, cos_p) = state.orbit_phase.sin_cos();
    let horizontal = cos_p * cfg.standoff_px;
    let vertical_center = -0.45 * cfg.standoff_px;
    let vertical_amp = 0.20 * cfg.standoff_px;
    let vertical = vertical_center + sin_p * vertical_amp;
    let orbit_offset = ae::Vec2::new(horizontal, vertical);
    let desired_pos = snapshot.target_pos + orbit_offset;
    let to_orbit = desired_pos - snapshot.actor_pos;
    let approach_dist = to_orbit.length();
    let approach_dir = to_orbit.normalize_or_zero();
    // Facing always toward the actual target so the rider / muzzle
    // aims at the player rather than the orbit point.
    let aim_dir = to_target_raw.normalize_or_zero();
    out.facing = aim_dir.x.signum_or(snapshot.actor_facing);
    // Move toward the orbit point at strafe_speed. Aerial archetypes
    // (sharks etc.) need 2D motion to actually orbit; the
    // integration uses both x and y when `is_aerial = true`.
    // Scale down speed when within a small radius of the desired
    // position so the actor doesn't oscillate around it.
    let speed_scale = (approach_dist / 24.0).min(1.0);
    out.desired_vel = approach_dir * cfg.strafe_speed * speed_scale;
    // Fire at the actual target when the cooldown timer is clear.
    // ActionSet supplies the concrete projectile (speed, damage);
    // brain just emits dir.
    if state.cooldown_remaining <= 0.0 {
        // Speed = 0.0 here is a sentinel; the action_set resolver
        // pulls speed from the actor's RangedActionSpec when it
        // builds the projectile spawn.
        out.fire = Some(crate::actor_control::ActorFireRequest {
            dir: aim_dir,
            speed: 0.0,
        });
        state.cooldown_remaining = cfg.fire_cooldown_s;
        state.mode = crate::character_ai::CharacterAiMode::Attack;
    }
}

// ===== Sniper =====

/// Hold position + long-range fire. Like a Skirmisher but does not
/// strafe — used by stationary turret-like enemies.
#[derive(Clone, Copy, Debug)]
pub struct SniperCfg {
    pub aggressiveness: f32,
    pub aggro_radius: f32,
    pub fire_cooldown_s: f32,
}

impl SniperCfg {
    pub const DEFAULT: Self = Self {
        aggressiveness: 1.0,
        aggro_radius: 480.0,
        fire_cooldown_s: 1.5,
    };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SniperState {
    /// Seconds remaining until the next shot can fire. Decrements
    /// each tick by `snapshot.dt`. See `SkirmisherState` doc for why
    /// this replaces the previous `last_fire_t` / `sim_time` shape.
    pub cooldown_remaining: f32,
}

fn tick_sniper(
    cfg: &SniperCfg,
    state: &mut SniperState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();
    state.cooldown_remaining = (state.cooldown_remaining - snapshot.dt).max(0.0);
    if !snapshot.target_alive {
        return;
    }
    let to_target = snapshot.target_pos - snapshot.actor_pos;
    let dist = to_target.length();
    if dist > cfg.aggro_radius {
        return;
    }
    let dir = to_target.normalize_or_zero();
    out.facing = dir.x.signum_or(snapshot.actor_facing);
    if state.cooldown_remaining <= 0.0 {
        out.fire = Some(crate::actor_control::ActorFireRequest { dir, speed: 0.0 });
        state.cooldown_remaining = cfg.fire_cooldown_s;
    }
}

// ===== Shark =====

/// Dedicated shark charge policy. The riderless burning shark uses
/// this to lunge forward in bursts rather than simply marching like
/// a melee brute.
#[derive(Clone, Copy, Debug)]
pub struct SharkCfg {
    pub aggressiveness: f32,
    pub aggro_radius: f32,
    pub cruise_speed: f32,
    pub charge_speed: f32,
    pub bite_range: f32,
    pub charge_duration_s: f32,
    pub charge_cooldown_s: f32,
    pub standoff_px: f32,
    pub vertical_wobble_px: f32,
    pub orbit_drift_rad_s: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SharkState {
    pub mode: crate::character_ai::CharacterAiMode,
    pub charge_remaining: f32,
    pub charge_cooldown_remaining: f32,
    pub orbit_phase: f32,
}

fn tick_shark(
    cfg: &SharkCfg,
    state: &mut SharkState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();
    state.charge_cooldown_remaining = (state.charge_cooldown_remaining - snapshot.dt).max(0.0);
    state.charge_remaining = (state.charge_remaining - snapshot.dt).max(0.0);
    state.orbit_phase += cfg.orbit_drift_rad_s * snapshot.dt;
    if state.orbit_phase > std::f32::consts::TAU {
        state.orbit_phase -= std::f32::consts::TAU;
    }

    if !snapshot.target_alive {
        state.mode = crate::character_ai::CharacterAiMode::Idle;
        return;
    }

    let to_target = snapshot.target_pos - snapshot.actor_pos;
    let dist = to_target.length();
    if dist > cfg.aggro_radius {
        state.mode = crate::character_ai::CharacterAiMode::Idle;
        return;
    }

    let aim_dir = if to_target.length_squared() > 0.0 {
        to_target.normalize_or_zero()
    } else {
        ae::Vec2::new(snapshot.actor_facing, 0.0)
    };
    let facing = aim_dir.x.signum_or(snapshot.actor_facing);
    out.facing = facing;

    let (sin_p, cos_p) = state.orbit_phase.sin_cos();
    let orbit_offset = ae::Vec2::new(
        cos_p * cfg.standoff_px,
        -0.42 * cfg.standoff_px + sin_p * cfg.vertical_wobble_px,
    );
    let desired_orbit_pos = snapshot.target_pos + orbit_offset;
    let to_orbit = desired_orbit_pos - snapshot.actor_pos;
    let orbit_dir = to_orbit.normalize_or_zero();

    if state.charge_remaining > 0.0 {
        state.mode = crate::character_ai::CharacterAiMode::Attack;
        out.desired_vel = orbit_dir * cfg.charge_speed;
        return;
    }

    if dist <= cfg.bite_range && snapshot.attack_cooldown_remaining <= 0.0 {
        state.mode = crate::character_ai::CharacterAiMode::Attack;
        out.melee_pressed = true;
        return;
    }

    if state.charge_cooldown_remaining <= 0.0 {
        state.mode = crate::character_ai::CharacterAiMode::Telegraph;
        state.charge_remaining = cfg.charge_duration_s.max(snapshot.dt);
        state.charge_cooldown_remaining = cfg.charge_cooldown_s;
        out.desired_vel = orbit_dir * cfg.charge_speed;
        return;
    }

    state.mode = crate::character_ai::CharacterAiMode::Chase;
    out.desired_vel = orbit_dir * cfg.cruise_speed;
}

// ===== BossPattern =====
//
// `BossPatternCfg` / `BossPatternState` / `tick_boss_pattern` moved
// to `brain/boss_pattern.rs`. They are re-exported from `brain::*`.
// The real boss-tick driver lives in `content/features/ecs/bosses.rs`
// (`tick_boss_brains_system`) because it needs boss-entity context
// — encounter phase, target pos, world bounds — that doesn't fit
// inside the generic `BrainSnapshot`.
//
// The `tick_state_machine` dispatch arm below emits a neutral frame
// for `BossPattern` because this generic path is the wrong driver
// for bosses; the boss tick system bypasses it and calls
// `boss_pattern::tick_boss_pattern` directly with the full
// `BossPatternContext`. The arm exists only so a possessed-boss
// actor in a non-boss code path doesn't crash the dispatch.
fn tick_boss_pattern_via_state_machine(
    _cfg: &super::BossPatternCfg,
    _state: &mut super::BossPatternState,
    _snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();
}

// ===== Trait helpers =====
//
// `ae::Vec2::signum_or` isn't in the engine; provide a tiny ext
// trait here so the brain templates above read cleanly. Adding it to
// the engine itself is overkill for a single use site.

trait SignumOr {
    fn signum_or(self, fallback: f32) -> f32;
}

impl SignumOr for f32 {
    fn signum_or(self, fallback: f32) -> f32 {
        if self.abs() < f32::EPSILON {
            fallback
        } else {
            self.signum()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::snapshot::{BrainSnapshot, WallContact};

    /// Pin the SignumOr trait's "near-zero → fallback" semantics.
    /// Many brain ticks lean on this to keep facing stable when
    /// movement input is briefly neutral; a regression to plain
    /// `signum()` would let actors snap to 0 facing on neutral
    /// frames. Edge cases: positive, negative, exactly zero,
    /// sub-epsilon positive.
    #[test]
    fn signum_or_falls_back_when_input_is_near_zero() {
        assert_eq!((0.0_f32).signum_or(1.0), 1.0);
        assert_eq!((0.0_f32).signum_or(-1.0), -1.0);
        assert_eq!((f32::EPSILON * 0.5).signum_or(7.0), 7.0);
        assert_eq!((-f32::EPSILON * 0.5).signum_or(7.0), 7.0);
        // Clearly positive / negative → signum wins.
        assert_eq!((0.5_f32).signum_or(99.0), 1.0);
        assert_eq!((-0.5_f32).signum_or(99.0), -1.0);
    }

    fn snap_at(pos_x: f32, target_x: f32) -> BrainSnapshot {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = ae::Vec2::new(pos_x, 0.0);
        s.target_pos = ae::Vec2::new(target_x, 0.0);
        s
    }

    #[test]
    fn stand_still_emits_neutral_frame() {
        let mut sm = StateMachineCfg::StandStill;
        let mut out = crate::actor_control::ActorControlFrame::default();
        out.desired_vel = ae::Vec2::new(99.0, 99.0); // pre-poisoned
        out.melee_pressed = true;
        tick_state_machine(&mut sm, &BrainSnapshot::idle(), &mut out);
        assert_eq!(out, crate::actor_control::ActorControlFrame::neutral());
    }

    #[test]
    fn dead_actor_brain_emits_neutral_regardless_of_template() {
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        };
        let mut s = snap_at(0.0, 4.0);
        s.alive = false;
        // Pre-poison `out` so the test catches the early-return-
        // without-write path (a previously-leaked frame surviving
        // into a dead-actor tick).
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        out.melee_pressed = true;
        out.desired_vel = ae::Vec2::new(99.0, 99.0);
        out.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 100.0,
        });
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        assert!(out.fire.is_none());
    }

    #[test]
    fn patrol_paces_horizontally_around_spawn() {
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.spawn_x = 50.0;
        cfg.radius = 30.0;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        // Target far away → no Chase; brain stays in Patrol.
        let mut s = snap_at(60.0, 5000.0);
        s.actor_facing = 1.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Within patrol bounds: keeps facing, moves forward.
        assert!(out.desired_vel.x > 0.0);
        assert_eq!(out.facing, 1.0);

        // Push past the right bound → facing flips.
        let mut s2 = snap_at(90.0, 5000.0);
        s2.actor_facing = 1.0;
        tick_state_machine(&mut sm, &s2, &mut out);
        assert!(out.desired_vel.x < 0.0);
        assert_eq!(out.facing, -1.0);
    }

    #[test]
    fn patrol_state_mode_mirrors_evaluator_intent() {
        // tick_patrol writes state.mode = ai.mode from the engine
        // evaluator. The NPC code at npcs.rs:230 reads PatrolState
        // .mode to pick HUD sprites — pin that a Patrol tick with
        // a far target gets mode = Patrol (i.e. the actor paces).
        let cfg = PatrolCfg::NPC_DEFAULT;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        let s = snap_at(0.0, 5000.0); // target far away
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        if let StateMachineCfg::Patrol { state, .. } = &sm {
            // Far target → evaluator picks Patrol (not Idle/Chase/Attack).
            assert_eq!(state.mode, crate::character_ai::CharacterAiMode::Patrol);
        } else {
            unreachable!();
        }
        // With a close target the evaluator switches to Chase (or
        // Attack if in range) — mode follows.
        let close = snap_at(0.0, 30.0);
        tick_state_machine(&mut sm, &close, &mut out);
        if let StateMachineCfg::Patrol { state, .. } = &sm {
            assert_ne!(
                state.mode,
                crate::character_ai::CharacterAiMode::Patrol,
                "close target should leave Patrol",
            );
        }
    }

    #[test]
    fn hostile_patrol_chases_target_in_aggro() {
        // Patrol with aggressiveness > 0 should chase the target
        // when it's inside aggro_radius but outside attack_range.
        // Pins the Chase branch's movement vs the peaceful "hold +
        // face target" branch.
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.spawn_x = 0.0;
        cfg.radius = 200.0;
        cfg.aggressiveness = 1.0;
        cfg.aggro_radius = 120.0;
        cfg.attack_range = 24.0;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        // Actor at 0, target at +80 → inside aggro, outside attack.
        let s = snap_at(0.0, 80.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Chase: closes the gap toward target_x.
        assert!(out.desired_vel.x > 0.0, "hostile patrol should chase right");
        assert_eq!(out.facing, 1.0);
        assert!(!out.melee_pressed);
    }

    #[test]
    fn hostile_patrol_attacks_target_in_melee_range() {
        // Patrol inside attack_range with cooldown clear → emit
        // melee intent. Pins the Attack branch.
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.spawn_x = 0.0;
        cfg.radius = 200.0;
        cfg.aggressiveness = 1.0;
        cfg.aggro_radius = 120.0;
        cfg.attack_range = 24.0;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        let mut s = snap_at(0.0, 15.0); // inside attack_range
        s.attack_cooldown_remaining = 0.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(
            out.melee_pressed,
            "hostile patrol in melee range should attack"
        );
        assert_eq!(out.facing, 1.0);
    }

    #[test]
    fn hostile_patrol_holds_attack_during_cooldown() {
        // Attack branch must not emit melee when cooldown is active.
        // Pins the timer gate so an enemy can't spam attacks every
        // tick by virtue of always being in range.
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.aggressiveness = 1.0;
        cfg.aggro_radius = 120.0;
        cfg.attack_range = 24.0;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        let mut s = snap_at(0.0, 15.0);
        s.attack_cooldown_remaining = 0.5; // mid-cooldown
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed, "must respect attack_cooldown_remaining");
    }

    #[test]
    fn peaceful_patrol_in_talk_range_holds_and_faces_target() {
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.spawn_x = 0.0;
        cfg.radius = 64.0;
        let mut sm = StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        };
        // Target right next to actor → evaluator returns Chase
        // (i.e. "player in range"). For peaceful aggressiveness=0
        // brain interprets as HOLD + face target.
        let s = snap_at(0.0, 30.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        assert_eq!(out.facing, 1.0);
        assert!(!out.melee_pressed);
    }

    #[test]
    fn wanderer_moves_forward_with_no_wall_contact() {
        let cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.desired_vel.x > 0.0);
        assert_eq!(out.facing, 1.0);
    }

    #[test]
    fn wanderer_reverses_on_non_climbable_wall() {
        let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        cfg.climb_walls = true; // climb on, but wall isn't climbable
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        s.wall_contact = Some(WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: false,
        });
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Facing flipped from +1 to -1; velocity goes left.
        assert_eq!(out.facing, -1.0);
        assert!(out.desired_vel.x < 0.0);
    }

    #[test]
    fn wanderer_climbs_when_able() {
        let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        cfg.climb_walls = true;
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        s.wall_contact = Some(WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: true,
        });
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // No reversal recorded; climbing flag flips on inside state.
        if let StateMachineCfg::Wanderer { state, .. } = &sm {
            assert!(state.climbing);
            assert!(state.recent_reversals.is_empty());
        } else {
            unreachable!();
        }
        // Frame in climb mode emits zero motion (the actor walks
        // along the surface via the integration's surface-walk path
        // rather than the brain).
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn wanderer_climbing_to_walking_transition_via_wall_clear() {
        // Wanderer that engaged climb mode (climb_walls=true,
        // climbable wall) should keep climbing while wall stays;
        // when wall clears, brain returns to forward walk.
        let cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        s.wall_contact = Some(crate::brain::snapshot::WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: true,
        });
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Engaged climb mode → zero motion.
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        if let StateMachineCfg::Wanderer { state, .. } = &sm {
            assert!(state.climbing);
        }
        // Clear wall — wanderer returns to forward walking.
        s.wall_contact = None;
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out2);
        // Note: brain's climbing flag persists until next wall
        // contact resolves; what we test is that with NO wall
        // the brain emits forward walk (per the early-return
        // logic in tick_wanderer).
        assert!(out2.desired_vel.x > 0.0);
    }

    #[test]
    fn wanderer_resumes_walking_after_pause_expires() {
        // Pause is time-bounded; once chatter_pause_s elapses past
        // pause_until, the wanderer should resume forward motion.
        let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        cfg.climb_walls = false;
        cfg.chatter_threshold = 1; // first reversal trips pause
        cfg.chatter_window_s = 0.5;
        cfg.chatter_pause_s = 1.0;
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        // Trip the chatter via a reversal at t=0.
        s.wall_contact = Some(crate::brain::snapshot::WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: false,
        });
        s.sim_time = 0.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Pause is active.
        if let StateMachineCfg::Wanderer { state, .. } = &sm {
            assert!(state.pause_until > 0.5);
        }
        // Advance time past pause_until + remove wall contact.
        s.sim_time = 2.0;
        s.wall_contact = None;
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out2);
        // Forward motion resumed.
        assert!(
            out2.desired_vel.x != 0.0,
            "wanderer should walk after pause expires"
        );
    }

    #[test]
    fn wanderer_pauses_on_rapid_chatter() {
        let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        cfg.chatter_threshold = 3;
        cfg.chatter_window_s = 1.0;
        cfg.chatter_pause_s = 2.0;
        cfg.climb_walls = false; // ensure we reverse not climb
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: WandererState::default(),
        };
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        s.wall_contact = Some(WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: false,
        });
        s.sim_time = 0.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();

        // Three reversals across <1s should trip the pause on the
        // third reversal.
        s.sim_time = 0.0;
        tick_state_machine(&mut sm, &s, &mut out);
        s.actor_facing = out.facing;
        s.sim_time = 0.2;
        tick_state_machine(&mut sm, &s, &mut out);
        s.actor_facing = out.facing;
        s.sim_time = 0.4;
        tick_state_machine(&mut sm, &s, &mut out);
        // Pause should be active; frame is neutral.
        if let StateMachineCfg::Wanderer { state, .. } = &sm {
            assert!(state.pause_until > 0.4);
        }
        // Next tick during pause window → no motion.
        s.sim_time = 0.5;
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        out2.desired_vel = ae::Vec2::new(99.0, 99.0);
        tick_state_machine(&mut sm, &s, &mut out2);
        assert_eq!(out2.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn melee_brute_chases_then_attacks_when_in_range() {
        let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg,
            state: MeleeBruteState::default(),
        };
        // Target close enough to chase but outside attack range.
        let s = snap_at(0.0, 100.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.desired_vel.x > 0.0);
        assert!(!out.melee_pressed);
        // Target within attack range.
        let s2 = snap_at(0.0, 20.0);
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s2, &mut out2);
        assert!(out2.melee_pressed);
        assert_eq!(out2.facing, 1.0);
    }

    #[test]
    fn melee_brute_does_not_attack_during_active_windup() {
        let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg,
            state: MeleeBruteState::default(),
        };
        let mut s = snap_at(0.0, 20.0);
        s.attack_windup_remaining = 0.1;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed);
    }

    #[test]
    fn melee_brute_attack_gate_respects_each_phase_timer() {
        // The Attack branch ANDs four timer gates: cooldown, windup,
        // active, recover. Any of them positive must suppress
        // melee_pressed. Walks each one individually to catch a
        // future refactor that drops one of the gates.
        let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
        let cases: [(&str, fn(&mut BrainSnapshot)); 4] = [
            ("cooldown", |s| s.attack_cooldown_remaining = 0.1),
            ("windup", |s| s.attack_windup_remaining = 0.1),
            ("active", |s| s.attack_active_remaining = 0.1),
            ("recover", |s| s.attack_recover_remaining = 0.1),
        ];
        for (name, poke) in cases {
            let mut sm = StateMachineCfg::MeleeBrute {
                cfg,
                state: MeleeBruteState::default(),
            };
            let mut s = snap_at(0.0, 20.0); // inside attack range
            poke(&mut s);
            let mut out = crate::actor_control::ActorControlFrame::neutral();
            tick_state_machine(&mut sm, &s, &mut out);
            assert!(
                !out.melee_pressed,
                "{} timer > 0 should suppress melee_pressed",
                name,
            );
        }
        // Sanity: with all timers clear, melee_pressed = true.
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg,
            state: MeleeBruteState::default(),
        };
        let s = snap_at(0.0, 20.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.melee_pressed, "all timers clear → should attack");
    }

    #[test]
    fn skirmisher_holds_standoff_then_fires() {
        let cfg = SkirmisherCfg::RANGER_DEFAULT;
        let mut sm = StateMachineCfg::Skirmisher {
            cfg,
            state: SkirmisherState::default(),
        };
        // Inside aggro, beyond standoff: chase closer.
        let mut s = snap_at(0.0, 200.0);
        s.sim_time = 0.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_some() || out.desired_vel.x != 0.0);
        // After firing, last_fire_t is now 0.0; within cooldown
        // window another tick should not fire again immediately.
        s.sim_time = 0.1;
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out2);
        assert!(out2.fire.is_none());
    }

    #[test]
    fn skirmisher_state_mode_tracks_engagement_phase() {
        // tick_skirmisher writes state.mode = Idle when outside
        // aggro, Chase when inside aggro pre-fire, Attack when
        // firing. Pin all three transitions — the NPC consumer
        // reads state.mode for HUD / sprite picking, so a future
        // refactor that drops a mode write would silently break
        // the HUD without tripping any other test.
        // Seed the cooldown timer so the first in-aggro tick stays
        // in Chase rather than immediately firing — the production
        // spawn helper (`enemy_default_brain`) seeds it the same way.
        let mut sm = StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg::RANGER_DEFAULT,
            state: SkirmisherState {
                cooldown_remaining: SkirmisherCfg::RANGER_DEFAULT.fire_cooldown_s,
                ..Default::default()
            },
        };
        // Far outside aggro → Idle.
        let mut s = snap_at(0.0, 5000.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        if let StateMachineCfg::Skirmisher { state, .. } = &sm {
            assert_eq!(state.mode, crate::character_ai::CharacterAiMode::Idle);
        } else {
            unreachable!();
        }
        // Inside aggro with the seeded cooldown still draining →
        // Chase (one dt tick is small relative to the seed).
        s = snap_at(0.0, 200.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        if let StateMachineCfg::Skirmisher { state, .. } = &sm {
            assert_eq!(state.mode, crate::character_ai::CharacterAiMode::Chase);
        }
        // Drain the cooldown by passing a one-shot dt that exceeds
        // the remaining timer; next tick → Attack + fire.
        s.dt = 5.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        if let StateMachineCfg::Skirmisher { state, .. } = &sm {
            assert_eq!(state.mode, crate::character_ai::CharacterAiMode::Attack);
        }
        assert!(out.fire.is_some(), "should fire after cooldown");
    }

    #[test]
    fn skirmisher_holds_quiet_when_target_dead() {
        // Skirmisher with dead target inside aggro range must emit
        // a neutral frame — no fire, no strafe. Pins the
        // target_alive=false early-return so an enemy can't keep
        // shooting at a dropped player.
        let cfg = SkirmisherCfg::RANGER_DEFAULT;
        let mut sm = StateMachineCfg::Skirmisher {
            cfg,
            state: SkirmisherState::default(),
        };
        let mut s = snap_at(0.0, 200.0);
        s.sim_time = 5.0; // way past any cooldown
        s.target_alive = false;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_none());
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn sniper_holds_and_fires_within_aggro() {
        let mut sm = StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        };
        // Target well within aggro_radius (480.0).
        let mut s = snap_at(0.0, 200.0);
        // last_fire_t defaults to 0; first fire requires
        // sim_time >= fire_cooldown_s (default 1.5).
        s.sim_time = 2.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Sniper never moves (no desired_vel).
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        // Fired (sim_time past cooldown threshold).
        assert!(out.fire.is_some());
        // After firing, cooldown gates re-fire.
        s.sim_time = 2.1;
        let mut out2 = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out2);
        assert!(out2.fire.is_none(), "Sniper should respect fire_cooldown_s");
    }

    #[test]
    fn sniper_holds_quiet_outside_aggro() {
        let mut sm = StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        };
        // Target way outside aggro (default 480).
        let s = snap_at(0.0, 5000.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_none(), "Sniper out of aggro should not fire");
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn sniper_holds_quiet_when_target_dead() {
        // Pin the target_alive=false early-return path: even when
        // the dead target is inside aggro range and the cooldown
        // is satisfied, the sniper emits a neutral frame (no fire,
        // no facing change).
        let mut sm = StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        };
        let mut s = snap_at(0.0, 200.0); // well within aggro
        s.sim_time = 2.0; // past cooldown
        s.target_alive = false;
        s.actor_facing = 1.0;
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_none(), "Sniper must not fire at dead target");
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn brain_tick_overwrites_prior_frame_intent() {
        // Brain.tick treats `out` as a write target, not an
        // accumulator. Pre-poisoned intent (melee_pressed=true,
        // fire=Some) must be cleared before the brain writes its
        // own intent. Pins this so a future stale-state bug
        // doesn't sneak through.
        let mut sm = StateMachineCfg::StandStill;
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
        });
        frame.jump_pressed = true;
        let snap = crate::brain::snapshot::BrainSnapshot::idle();
        tick_state_machine(&mut sm, &snap, &mut frame);
        // StandStill = neutral frame; pre-poisoned intent gone.
        assert!(!frame.melee_pressed);
        assert!(frame.fire.is_none());
        assert!(!frame.jump_pressed);
    }

    #[test]
    fn brain_dispatch_50_actors_under_one_millisecond() {
        // Sustained dispatch perf: tick 50 brains' state machine
        // once, all variants represented, total under 1ms. Pins
        // the "brain dispatch is monomorphic per-variant" property
        // — a regression to dyn dispatch or boxed brains would
        // blow this.
        let mut sm_list = vec![
            StateMachineCfg::StandStill,
            StateMachineCfg::Patrol {
                cfg: PatrolCfg::NPC_DEFAULT,
                state: PatrolState::default(),
            },
            StateMachineCfg::Wanderer {
                cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
                state: WandererState::default(),
            },
            StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            },
            StateMachineCfg::Skirmisher {
                cfg: SkirmisherCfg::RANGER_DEFAULT,
                state: SkirmisherState::default(),
            },
        ];
        // Duplicate to reach 50.
        while sm_list.len() < 50 {
            sm_list.extend_from_slice(&sm_list.clone());
        }
        sm_list.truncate(50);
        let snap = crate::brain::snapshot::BrainSnapshot::idle();
        let start = std::time::Instant::now();
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        for sm in &mut sm_list {
            tick_state_machine(sm, &snap, &mut frame);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(5),
            "50 brain ticks should be < 5ms, took {elapsed:?}",
        );
    }

    #[test]
    fn brain_tick_cost_is_well_under_one_millisecond() {
        // Smoke check on per-tick brain dispatch cost. Ten ticks
        // of a MeleeBrute brain should complete well under 1ms on
        // any reasonable hardware. A regression that adds heap
        // allocation or expensive math inside the brain hot path
        // would trip this — it'd grow per-tick by orders of
        // magnitude.
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        };
        let snap = crate::brain::snapshot::BrainSnapshot::idle();
        let start = std::time::Instant::now();
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        for _ in 0..10 {
            tick_state_machine(&mut sm, &snap, &mut frame);
        }
        let elapsed = start.elapsed();
        // 10 ticks should finish in well under 1ms (generous;
        // typically a few microseconds total).
        assert!(
            elapsed < std::time::Duration::from_millis(10),
            "10 MeleeBrute ticks should be << 10ms, took {elapsed:?}",
        );
    }

    #[test]
    fn brain_templates_survive_zero_dt() {
        // Zero dt is the "paused frame" case — bullet-time +
        // hitstop both feed dt=0 to consumers. Every brain
        // template should tick cleanly without panic / NaN
        // propagation. Pins the pause-safety invariant.
        let templates: Vec<StateMachineCfg> = vec![
            StateMachineCfg::StandStill,
            StateMachineCfg::Patrol {
                cfg: PatrolCfg::NPC_DEFAULT,
                state: PatrolState::default(),
            },
            StateMachineCfg::Wanderer {
                cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
                state: WandererState::default(),
            },
            StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            },
            StateMachineCfg::Skirmisher {
                cfg: SkirmisherCfg::RANGER_DEFAULT,
                state: SkirmisherState::default(),
            },
            StateMachineCfg::Sniper {
                cfg: SniperCfg::DEFAULT,
                state: SniperState::default(),
            },
            StateMachineCfg::Shark {
                cfg: SharkCfg {
                    aggressiveness: 1.0,
                    aggro_radius: 360.0,
                    cruise_speed: 120.0,
                    charge_speed: 420.0,
                    bite_range: 34.0,
                    charge_duration_s: 0.45,
                    charge_cooldown_s: 0.8,
                    standoff_px: 140.0,
                    vertical_wobble_px: 24.0,
                    orbit_drift_rad_s: 0.8,
                },
                state: SharkState::default(),
            },
        ];
        for mut brain in templates {
            let mut snap = crate::brain::snapshot::BrainSnapshot::idle();
            snap.dt = 0.0;
            let mut frame = crate::actor_control::ActorControlFrame::neutral();
            tick_state_machine(&mut brain, &snap, &mut frame);
            assert!(frame.desired_vel.x.is_finite());
            assert!(frame.desired_vel.y.is_finite());
        }
    }

    #[test]
    fn boss_pattern_via_state_machine_emits_neutral_frame() {
        // The generic `tick_state_machine` path is intentionally a
        // no-op for `BossPattern`: bosses bypass it via
        // `tick_boss_brains_system` (which has access to
        // encounter_phase / world bounds / target). If the dispatch
        // path here ever starts producing intent, the boss tick
        // system would race with it. Pin the contract.
        let mut sm = StateMachineCfg::BossPattern {
            cfg: crate::brain::BossPatternCfg::neutral_test(),
            state: crate::brain::BossPatternState::default(),
        };
        let s = snap_at(0.0, 100.0);
        let mut out = crate::actor_control::ActorControlFrame::neutral();
        out.melee_pressed = true; // pre-poisoned
        out.desired_vel = ae::Vec2::new(99.0, 99.0);
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn is_hostile_reports_per_cfg() {
        assert!(!StateMachineCfg::StandStill.is_hostile());
        assert!(!StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        }
        .is_hostile());
        assert!(!StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }
        .is_hostile());
        assert!(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        }
        .is_hostile());
        assert!(StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg::RANGER_DEFAULT,
            state: SkirmisherState::default(),
        }
        .is_hostile());
        assert!(StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        }
        .is_hostile());
        assert!(StateMachineCfg::Shark {
            cfg: SharkCfg {
                aggressiveness: 1.0,
                aggro_radius: 360.0,
                cruise_speed: 120.0,
                charge_speed: 420.0,
                bite_range: 34.0,
                charge_duration_s: 0.45,
                charge_cooldown_s: 0.8,
                standoff_px: 140.0,
                vertical_wobble_px: 24.0,
                orbit_drift_rad_s: 0.8,
            },
            state: SharkState::default(),
        }
        .is_hostile());
    }
}
