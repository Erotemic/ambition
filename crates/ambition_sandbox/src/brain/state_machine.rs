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

use ambition_engine as ae;

use super::snapshot::BrainSnapshot;

// ===== Top-level state-machine variant =====

/// A reusable AI policy + its per-actor runtime state.
#[derive(Clone, Debug)]
pub enum StateMachineCfg {
    /// No motion. Used by static NPCs and sandbag-style targets.
    StandStill,
    /// Fixed waypoint loop. `aggressiveness` controls engagement.
    Patrol {
        cfg: PatrolCfg,
        state: PatrolState,
    },
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
    Sniper {
        cfg: SniperCfg,
        state: SniperState,
    },
    /// Scripted multi-phase boss policy — looked up by encounter id.
    BossPattern {
        cfg: BossPatternCfg,
        state: BossPatternState,
    },
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
            Self::BossPattern { cfg, .. } => cfg.aggressiveness > 0.0,
        }
    }
}

/// Tick a state-machine brain: read the snapshot, mutate the brain's
/// own state, and write the abstract intent into `out`.
pub fn tick_state_machine(
    sm: &mut StateMachineCfg,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    if !snapshot.alive {
        // Dead actors emit a neutral frame regardless of brain.
        return;
    }
    match sm {
        StateMachineCfg::StandStill => tick_stand_still(out),
        StateMachineCfg::Patrol { cfg, state } => tick_patrol(cfg, state, snapshot, out),
        StateMachineCfg::Wanderer { cfg, state } => tick_wanderer(cfg, state, snapshot, out),
        StateMachineCfg::MeleeBrute { cfg, state } => tick_melee_brute(cfg, state, snapshot, out),
        StateMachineCfg::Skirmisher { cfg, state } => tick_skirmisher(cfg, state, snapshot, out),
        StateMachineCfg::Sniper { cfg, state } => tick_sniper(cfg, state, snapshot, out),
        StateMachineCfg::BossPattern { cfg, state } => tick_boss_pattern(cfg, state, snapshot, out),
    }
}

// ===== StandStill =====

fn tick_stand_still(out: &mut ae::ActorControlFrame) {
    *out = ae::ActorControlFrame::neutral();
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
        aggro_radius: 80.0,  // talk radius for peaceful patrol
        attack_range: 0.0,
    };
}

/// Per-actor Patrol runtime state.
#[derive(Clone, Copy, Debug, Default)]
pub struct PatrolState {
    /// Most recently evaluated mode. Cached so HUD / animation
    /// systems can read it without re-evaluating.
    pub mode: ae::CharacterAiMode,
}

fn tick_patrol(
    cfg: &PatrolCfg,
    state: &mut PatrolState,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    let ai = ae::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        true,
    ));
    state.mode = ai.mode;
    *out = ae::ActorControlFrame::neutral();
    match ai.intent {
        ae::CharacterAiIntent::Hold => {
            // Player in talk range or otherwise hold position.
            // Face toward target if any.
            if snapshot.target_alive {
                let dx = snapshot.target_pos.x - snapshot.actor_pos.x;
                if dx.abs() > 4.0 {
                    out.facing = dx.signum();
                }
            }
        }
        ae::CharacterAiIntent::Patrol => {
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
        ae::CharacterAiIntent::Chase { direction_x } => {
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
        ae::CharacterAiIntent::Attack { direction_x } => {
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
    out: &mut ae::ActorControlFrame,
) {
    *out = ae::ActorControlFrame::neutral();

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
    pub mode: ae::CharacterAiMode,
}

fn tick_melee_brute(
    cfg: &MeleeBruteCfg,
    state: &mut MeleeBruteState,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    let ai = ae::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        false,
    ));
    state.mode = ai.mode;
    *out = ae::ActorControlFrame::neutral();
    match ai.intent {
        ae::CharacterAiIntent::Hold => {}
        ae::CharacterAiIntent::Patrol => {
            // Not used by MeleeBrute today (patrol_enabled=false).
        }
        ae::CharacterAiIntent::Chase { direction_x } => {
            out.desired_vel = ae::Vec2::new(direction_x * cfg.chase_speed, 0.0);
            out.facing = direction_x.signum_or(snapshot.actor_facing);
        }
        ae::CharacterAiIntent::Attack { direction_x } => {
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
}

impl SkirmisherCfg {
    pub const RANGER_DEFAULT: Self = Self {
        aggressiveness: 1.0,
        aggro_radius: 320.0,
        standoff_px: 140.0,
        strafe_speed: 85.0,
        fire_cooldown_s: 0.8,
    };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkirmisherState {
    pub mode: ae::CharacterAiMode,
    /// Sim-time of the last shot. Used with `fire_cooldown_s`.
    pub last_fire_t: f32,
}

fn tick_skirmisher(
    cfg: &SkirmisherCfg,
    state: &mut SkirmisherState,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    *out = ae::ActorControlFrame::neutral();
    if !snapshot.target_alive {
        return;
    }
    let to_target = snapshot.target_pos - snapshot.actor_pos;
    let dist = to_target.length();
    if dist > cfg.aggro_radius {
        state.mode = ae::CharacterAiMode::Idle;
        return;
    }
    state.mode = ae::CharacterAiMode::Chase;
    // Move toward the standoff distance.
    let dir = to_target.normalize_or_zero();
    out.facing = dir.x.signum_or(snapshot.actor_facing);
    let approach_sign = (dist - cfg.standoff_px).signum_or(0.0);
    out.desired_vel = ae::Vec2::new(dir.x * cfg.strafe_speed * approach_sign, 0.0);
    // Fire when the cooldown is clear. ActionSet supplies the
    // concrete projectile (speed, damage). Brain just emits dir.
    if snapshot.sim_time - state.last_fire_t >= cfg.fire_cooldown_s {
        // Speed = 0.0 here is a sentinel; the action_set resolver
        // pulls speed from the actor's RangedActionSpec when it
        // builds the projectile spawn.
        out.fire = Some(ae::ActorFireRequest { dir, speed: 0.0 });
        state.last_fire_t = snapshot.sim_time;
        state.mode = ae::CharacterAiMode::Attack;
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
    pub last_fire_t: f32,
}

fn tick_sniper(
    cfg: &SniperCfg,
    state: &mut SniperState,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    *out = ae::ActorControlFrame::neutral();
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
    if snapshot.sim_time - state.last_fire_t >= cfg.fire_cooldown_s {
        out.fire = Some(ae::ActorFireRequest { dir, speed: 0.0 });
        state.last_fire_t = snapshot.sim_time;
    }
}

// ===== BossPattern =====

/// Scripted multi-phase boss policy. The encounter id picks the
/// concrete phase schedule; the brain is a pointer + a tick cursor.
/// Today this is a placeholder — boss runtimes still drive
/// themselves via the existing `BossRuntime` pattern. Daytime
/// work migrates each boss onto a BossPattern brain and the
/// encounter id then keys into the existing
/// `BossEncounterRegistry`.
#[derive(Clone, Debug)]
pub struct BossPatternCfg {
    pub aggressiveness: f32,
    /// Encounter id (matches `boss_encounter::encounter_id_from_name`).
    /// Stays a String so it can pull straight from the existing
    /// registry instead of forcing a parallel id type.
    pub encounter_id: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BossPatternState {
    pub phase: u32,
    pub phase_elapsed: f32,
}

fn tick_boss_pattern(
    _cfg: &BossPatternCfg,
    _state: &mut BossPatternState,
    _snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    // Placeholder until the boss migration lands. A neutral frame
    // means a possessed BossPattern actor wouldn't fight — which is
    // safe behavior for the parallel-shape introduction in Chunk 2.
    *out = ae::ActorControlFrame::neutral();
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

    fn snap_at(pos_x: f32, target_x: f32) -> BrainSnapshot {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = ae::Vec2::new(pos_x, 0.0);
        s.target_pos = ae::Vec2::new(target_x, 0.0);
        s
    }

    #[test]
    fn stand_still_emits_neutral_frame() {
        let mut sm = StateMachineCfg::StandStill;
        let mut out = ae::ActorControlFrame::default();
        out.desired_vel = ae::Vec2::new(99.0, 99.0); // pre-poisoned
        out.melee_pressed = true;
        tick_state_machine(&mut sm, &BrainSnapshot::idle(), &mut out);
        assert_eq!(out, ae::ActorControlFrame::neutral());
    }

    #[test]
    fn dead_actor_brain_emits_neutral_regardless_of_template() {
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        };
        let mut s = snap_at(0.0, 4.0);
        s.alive = false;
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
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
        let mut out = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Facing flipped from +1 to -1; velocity goes left.
        assert_eq!(out.facing, -1.0);
        assert!(out.desired_vel.x < 0.0);
    }

    #[test]
    fn wanderer_climbs_when_able() {
        let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
        cfg.climb_walls = true;
        let mut state = WandererState::default();
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        s.wall_contact = Some(WallContact {
            normal: ae::Vec2::new(-1.0, 0.0),
            is_climbable: true,
        });
        let mut sm = StateMachineCfg::Wanderer {
            cfg,
            state: state.clone(),
        };
        let mut out = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();

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
        let mut out2 = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.desired_vel.x > 0.0);
        assert!(!out.melee_pressed);
        // Target within attack range.
        let s2 = snap_at(0.0, 20.0);
        let mut out2 = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(!out.melee_pressed);
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_some() || out.desired_vel.x != 0.0);
        // After firing, last_fire_t is now 0.0; within cooldown
        // window another tick should not fire again immediately.
        s.sim_time = 0.1;
        let mut out2 = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out2);
        assert!(out2.fire.is_none());
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        // Sniper never moves (no desired_vel).
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        // Fired (sim_time past cooldown threshold).
        assert!(out.fire.is_some());
        // After firing, cooldown gates re-fire.
        s.sim_time = 2.1;
        let mut out2 = ae::ActorControlFrame::neutral();
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
        let mut out = ae::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(out.fire.is_none(), "Sniper out of aggro should not fire");
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
    }

    #[test]
    fn boss_pattern_placeholder_ticks_to_neutral_frame() {
        // BossPattern is a placeholder until the boss EFFECTS-flip
        // lands. Ticking it should emit a neutral frame regardless
        // of target / sim_time — pin that contract so the
        // placeholder doesn't silently start producing intent.
        let mut sm = StateMachineCfg::BossPattern {
            cfg: BossPatternCfg {
                aggressiveness: 1.0,
                encounter_id: "test_boss".to_string(),
            },
            state: BossPatternState::default(),
        };
        let s = snap_at(0.0, 100.0);
        let mut out = ae::ActorControlFrame::neutral();
        out.melee_pressed = true; // pre-poisoned
        out.desired_vel = ae::Vec2::new(99.0, 99.0);
        tick_state_machine(&mut sm, &s, &mut out);
        // Placeholder writes a neutral frame, overwriting the
        // pre-poisoned values.
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
    }
}
