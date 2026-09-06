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

use ambition_platformer2d_core as ae;

use super::smash::{SmashCfg, SmashState};
use super::snapshot::BrainSnapshot;

// ===== Top-level state-machine variant =====

/// A reusable AI policy + its per-actor runtime state.
#[derive(Clone, Debug)]
pub enum StateMachineCfg {
    /// No motion. Used by static NPCs and sandbag-style targets.
    StandStill,
    /// `aggressiveness` controls engagement.
    Patrol { cfg: PatrolCfg, state: PatrolState },
    /// Move forward in `actor_facing`. Drives the puppy slug today. Surface
    /// wrapping belongs to the crawler motion model; a simple walker's choice to
    /// reverse at a semantic side contact belongs to this autonomous policy.
    Wanderer { cfg: WandererCfg },
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
    ChargeCrash {
        cfg: ChargeCrashCfg,
        state: ChargeCrashState,
    },
    /// Scripted multi-phase boss policy. The cfg + state live in
    /// `brain/boss_pattern/mod.rs`; this variant carries them but the
    /// real tick driver is `tick_boss_brains_system` in
    /// `ambition_boss_encounter/src/ecs/tick.rs` (see the dispatch fn below).
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
    /// The fighter brain that PLAYS (FB4b §13). L1 classify → L2 options →
    /// L3 rollout → a held control frame, on a human cadence with an APM ceiling
    /// and execution noise.
    ///
    /// A `StateMachineCfg` variant rather than a new `Brain` arm on purpose: this
    /// is where the dispatcher already threads `Option<&WorldView>`, which the
    /// delay buffer needs, and where the snapshot cursor already rewinds per-arm
    /// state. Every field of `FighterState` gates behaviour, so all of it is
    /// rollback state — the derive-memo rule applied before the desync rather
    /// than after it.
    Fighter {
        cfg: Box<super::fighter::FighterCfg>,
        state: Box<super::fighter::FighterState>,
    },
    /// Lively flyer: peaceful (perch/fly/walk/land-by-player) or hostile
    /// (stalk/dive/recover), selected by `cfg.aggressiveness`.
    Aerial { cfg: AerialCfg, state: AerialState },
    /// Drives the PLAYER's own movement verbs (run/jump/dash/fly) in a cycle —
    /// proves a brain can control a full player body through the shared
    /// `ActorControlFrame`. Peaceful (it only moves).
    PlayerDemo {
        cfg: PlayerDemoCfg,
        state: PlayerDemoState,
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
            Self::ChargeCrash { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::BossPattern { cfg, .. } => cfg.aggressiveness > 0.0,
            // A fighter is in a MATCH. There is no peaceful fighter brain: the
            // thing it exists to do is fight somebody who agreed to it.
            Self::Fighter { .. } => true,
            // Smash brain is always hostile by construction — peaceful
            // archetypes don't use it (they get Patrol / Wanderer
            // instead). If we add a peaceful Smash variant later, this
            // gate moves into `SmashCfg`.
            Self::Smash { .. } => true,
            Self::Aerial { cfg, .. } => cfg.aggressiveness > 0.0,
            Self::PlayerDemo { .. } => false,
        }
    }
}

impl StateMachineCfg {
    /// What perception this brain needs supplied (ADR 0034, increment 1).
    ///
    /// ⛔⛔ EXHAUSTIVE, AND THE ARMS ARE EVIDENCE, NOT TASTE. Each classification
    /// below is what that arm actually READS, established by grepping the
    /// capability — `target_pos`, `target_alive`, `target_delta_local`,
    /// `to_character_ai_snapshot` — rather than the tick function's signature.
    /// Classifying by signature would have put `MeleeBrute` in `None`: it never
    /// sees a `WorldView` and never names `target_pos`, and it steers entirely
    /// by the belief, through `to_character_ai_snapshot`'s
    /// `player_pos: self.target_delta_local()`.
    ///
    /// A new variant is a compile error here rather than a body that silently
    /// stops being told where its foe is.
    pub fn perception_requirement(&self) -> crate::perception::PerceptionRequirement {
        use crate::perception::PerceptionRequirement as Need;
        match self {
            // `tick_stand_still(out)` takes no snapshot AT ALL — the strongest
            // evidence available, and the case increment 1 exists for.
            Self::StandStill => Need::None,
            // Steers by wall contact and its own facing; names no target.
            Self::Wanderer { .. } => Need::None,
            // A scripted demo puppet: its own clock, no foe.
            Self::PlayerDemo { .. } => Need::None,

            // Reads the belief directly (`target_pos` / `target_alive`).
            Self::Patrol { .. }
            | Self::Skirmisher { .. }
            | Self::Sniper { .. }
            | Self::ChargeCrash { .. }
            | Self::Aerial { .. } => Need::TargetBelief,
            // Reads it through `to_character_ai_snapshot`.
            Self::MeleeBrute { .. } => Need::TargetBelief,
            // `tick.rs` copies `target_pos` into the boss pattern's own snapshot.
            // A boss is omniscient by POLICY, which is a question about what
            // fills the belief, not about whether it needs one.
            Self::BossPattern { .. } => Need::TargetBelief,

            // The only two arms that take a `&WorldView`.
            Self::Smash { .. } | Self::Fighter { .. } => Need::TacticalWorld,
        }
    }
}

pub fn tick_simple_state_machine(
    sm: &mut StateMachineCfg,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) -> bool {
    if !snapshot.alive {
        // Dead actors emit a neutral frame regardless of brain. Write
        // explicitly so a pre-poisoned `out` doesn't leak through.
        *out = crate::actor::control::ActorControlFrame::neutral();
        return true;
    }
    match sm {
        StateMachineCfg::StandStill => tick_stand_still(out),
        StateMachineCfg::Patrol { cfg, state } => tick_patrol(cfg, state, snapshot, out),
        StateMachineCfg::Wanderer { cfg } => tick_wanderer(cfg, snapshot, out),
        StateMachineCfg::MeleeBrute { cfg, state } => tick_melee_brute(cfg, state, snapshot, out),
        StateMachineCfg::Skirmisher { cfg, state } => tick_skirmisher(cfg, state, snapshot, out),
        StateMachineCfg::Sniper { cfg, state } => tick_sniper(cfg, state, snapshot, out),
        StateMachineCfg::ChargeCrash { cfg, state } => tick_charge_crash(cfg, state, snapshot, out),
        StateMachineCfg::Aerial { cfg, state } => tick_aerial(cfg, state, snapshot, out),
        StateMachineCfg::PlayerDemo { cfg, state } => tick_player_demo(cfg, state, snapshot, out),
        // ⚠ NAMED, not a `_` arm. A new variant has to come here and say which
        // side of the split it is on, instead of silently becoming somebody
        // else's problem at runtime.
        StateMachineCfg::BossPattern { .. }
        | StateMachineCfg::Smash { .. }
        | StateMachineCfg::Fighter { .. } => return false,
    }
    true
}

// ===== StandStill =====

fn tick_stand_still(out: &mut crate::actor::control::ActorControlFrame) {
    *out = crate::actor::control::ActorControlFrame::neutral();
}

// ===== Patrol =====

/// Patrol speed for NPCs (px/s). Slightly slower than the standard
/// enemy patrol speed so peaceful NPCs read as casual rather than
/// alert. Owned by the brain (its consumer); content re-exports it
/// for authoring-side reference.
pub const NPC_PATROL_SPEED: f32 = 60.0;

/// Authored world-lane route for a simple patroller.
///
/// This is intentionally world/environment space: peaceful NPCs pace along an
/// authored hallway lane, not the controlled actor's current local side axis.
/// Combat decisions can still be controlled-actor-local; the route anchor is a
/// separate authored-world concept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoredWorldPatrolLane {
    /// Center of the lane on world X.
    pub center_x: f32,
    /// Half-width of the lane (px). 0.0 = pinned to center.
    pub radius_px: f32,
}

impl AuthoredWorldPatrolLane {
    pub const fn new(center_x: f32, radius_px: f32) -> Self {
        Self {
            center_x,
            radius_px,
        }
    }

    pub fn signed_offset(self, world_pos: ae::Vec2) -> f32 {
        world_pos.x - self.center_x
    }

    pub fn facing_after_bounds(self, world_pos: ae::Vec2, current_facing: f32) -> f32 {
        let from_center = self.signed_offset(world_pos);
        if from_center > self.radius_px {
            -1.0
        } else if from_center < -self.radius_px {
            1.0
        } else {
            current_facing
        }
    }
}

/// Hostility is controlled separately — a hostile Patrol brain still emits melee_pressed when
/// in range and can flip facing to chase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatrolCfg {
    /// World/environment route lane. This is route-space, not local side.
    pub lane: AuthoredWorldPatrolLane,
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
    /// Peaceful NPC default. Speed is [`NPC_PATROL_SPEED`] so the
    /// brain-driven Patrol gait matches what the pre-brain
    /// `NpcRuntime::update` used.
    pub const NPC_DEFAULT: Self = Self {
        lane: AuthoredWorldPatrolLane::new(0.0, 64.0),
        speed: NPC_PATROL_SPEED,
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
    pub mode: crate::actor::ai::CharacterAiMode,
}

fn local_target_side(snapshot: &BrainSnapshot) -> f32 {
    snapshot.target_delta_local().x
}

// the two named conversions this module already routed everything through.
// Typing them is what makes the naming convention enforceable rather than a
// habit: a caller cannot reach world space except by passing through here.
fn frame_to_world(snapshot: &BrainSnapshot, local: ae::LocalAxes) -> ae::WorldVec2 {
    ae::WorldVec2(snapshot.acceleration_frame().to_world(local.vec()))
}

fn frame_to_local(snapshot: &BrainSnapshot, world: ae::WorldVec2) -> ae::LocalAxes {
    ae::LocalAxes::from_vec(snapshot.acceleration_frame().to_local(world.vec()))
}

/// Autonomous "turn away from the wall I am walking into" policy.
///
/// The movement kernel publishes the local-side contact normal; this helper
/// interprets that fact for simple autonomous walkers. A human-controlled body
/// or a fighter brain never calls this helper, so collision cannot silently
/// override its facing intent.
fn wall_turn_facing(snapshot: &BrainSnapshot) -> Option<f32> {
    if !snapshot.turns_at_walls {
        return None;
    }
    let wall_normal = snapshot.side_contact_normal?;
    let facing = snapshot.actor_facing.signum_or(1.0);
    (wall_normal.abs() > 0.5 && wall_normal * facing < 0.0).then_some(wall_normal.signum())
}

fn tick_patrol(
    cfg: &PatrolCfg,
    state: &mut PatrolState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    let ai = crate::actor::ai::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        true,
    ));
    state.mode = ai.mode;
    *out = crate::actor::control::ActorControlFrame::neutral();
    match ai.intent {
        crate::actor::ai::CharacterAiIntent::Hold => {
            // Player in talk range or otherwise hold position.
            // Face toward target if any.
            if snapshot.target_alive {
                let side = local_target_side(snapshot);
                if side.abs() > 4.0 {
                    out.facing = side.signum();
                }
            }
        }
        crate::actor::ai::CharacterAiIntent::Patrol => {
            // Bounce within the authored world lane, with a semantic side
            // contact taking precedence over the geometric lane bound. Both are
            // steering decisions owned by this brain.
            let facing = wall_turn_facing(snapshot).unwrap_or_else(|| {
                cfg.lane
                    .facing_after_bounds(snapshot.actor_pos, snapshot.actor_facing)
            });
            out.facing = facing;
            out.locomotion = snapshot.locomotion_for(ae::LocalAxes::new(facing * cfg.speed, 0.0));
        }
        crate::actor::ai::CharacterAiIntent::Chase { direction_side } => {
            // Only triggers when `aggressiveness > 0` — peaceful
            // patrollers' aggro_radius gates as "talk", which the
            // evaluator returns as Hold for `attack_range = 0`.
            // For aggressive patrol we close the distance.
            if cfg.aggressiveness > 0.0 {
                out.locomotion =
                    snapshot.locomotion_for(ae::LocalAxes::new(direction_side * cfg.speed, 0.0));
                out.facing = direction_side.signum_or(snapshot.actor_facing);
            } else {
                // Peaceful patroller in "Chase" mode = HOLD. The
                // npc semantics: "player is close, face them".
                let side = local_target_side(snapshot);
                if side.abs() > 4.0 {
                    out.facing = side.signum();
                }
            }
        }
        crate::actor::ai::CharacterAiIntent::Attack { direction_side } => {
            if cfg.aggressiveness > 0.0 {
                out.facing = direction_side.signum_or(snapshot.actor_facing);
                out.melee_pressed = snapshot.attack_cooldown_remaining <= 0.0;
            }
        }
    }
}

// ===== Wanderer =====

/// Forward-motion brain: emits locomotion in its chosen facing. Drives the
/// puppy slug today. A crawler body still wraps surfaces in the movement kernel;
/// a simple grounded walker may turn away from a real semantic side contact here
/// when its authored autonomous-steering policy enables that behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WandererCfg {
    /// Forward speed (px/s).
    pub speed: f32,
    /// Aggressiveness gate. `0.0` for the puppy slug; positive
    /// values would make a hostile Wanderer that triggers melee
    /// when in range of `target_pos`.
    pub aggressiveness: f32,
}

impl WandererCfg {
    /// Puppy slug defaults — slither forward; the crawler body owns walls.
    pub const PUPPY_SLUG_DEFAULT: Self = Self {
        speed: 36.0,
        aggressiveness: 0.0,
    };
}

fn tick_wanderer(
    cfg: &WandererCfg,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    // Surface wrapping is movement physics; choosing to reverse a simple
    // walker is autonomous control policy.
    out.facing = wall_turn_facing(snapshot).unwrap_or_else(|| snapshot.actor_facing.signum_or(1.0));
    out.locomotion = snapshot.locomotion_for(ae::LocalAxes::new(out.facing * cfg.speed, 0.0));
}

// ===== MeleeBrute =====

/// Approach + melee + recover. The brain decides WHEN to attack;
/// the ActionSet decides WHAT the attack looks like.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub mode: crate::actor::ai::CharacterAiMode,
}

fn tick_melee_brute(
    cfg: &MeleeBruteCfg,
    state: &mut MeleeBruteState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    let ai = crate::actor::ai::evaluate_character_ai_output(snapshot.to_character_ai_snapshot(
        cfg.aggro_radius,
        cfg.attack_range,
        false,
    ));
    state.mode = ai.mode;
    *out = crate::actor::control::ActorControlFrame::neutral();
    match ai.intent {
        crate::actor::ai::CharacterAiIntent::Hold => {}
        crate::actor::ai::CharacterAiIntent::Patrol => {
            // Not used by MeleeBrute today (patrol_enabled=false).
        }
        crate::actor::ai::CharacterAiIntent::Chase { direction_side } => {
            out.locomotion =
                snapshot.locomotion_for(ae::LocalAxes::new(direction_side * cfg.chase_speed, 0.0));
            out.facing = direction_side.signum_or(snapshot.actor_facing);
        }
        crate::actor::ai::CharacterAiIntent::Attack { direction_side } => {
            out.facing = direction_side.signum_or(snapshot.actor_facing);
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub mode: crate::actor::ai::CharacterAiMode,
    /// Counts down each tick by `snapshot.dt`. Reset to `cfg.fire_cooldown_s` on fire. The
    /// previous shape compared an absolute `sim_time` against `last_fire_t`, but the sandbox
    /// actors path doesn't populate `snapshot.sim_time` — it's hard-coded to 0.0 — so every
    /// comparison evaluated `0 - 0 >= 1.5` and Skirmisher never fired in production. The
    /// decrementing-timer shape is what MeleeBrute uses via `attack_cooldown_remaining` and
    /// avoids the global-clock dependency.
    pub cooldown_remaining: f32,
    /// Per-actor orbital phase in radians. The Skirmisher orbits the target on a circle of
    /// radius `cfg.standoff_px` and picks its desired position via `target_pos + (cos θ, sin θ)
    /// * standoff_px` where θ is this phase. Seeding it from the actor's stable id-derived RNG
    /// spreads a squadron of shark-riders around the player (above / below / left / right)
    /// instead of stacking them all at the same offset axis.
    pub orbit_phase: f32,
}

fn tick_skirmisher(
    cfg: &SkirmisherCfg,
    state: &mut SkirmisherState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    // Cooldown ticks down every frame regardless of target state so
    // a Skirmisher that loses sight mid-cooldown doesn't get a free
    // first shot the moment the player re-enters aggro.
    state.cooldown_remaining = (state.cooldown_remaining - snapshot.dt).max(0.0);
    // The per-actor initial phase (seeded at spawn) keeps a squadron spread out around the
    // player.
    state.orbit_phase += cfg.orbit_drift_rad_s * snapshot.dt;
    if state.orbit_phase > std::f32::consts::TAU {
        state.orbit_phase -= std::f32::consts::TAU;
    }
    if !snapshot.target_alive {
        return;
    }
    let to_target_raw = snapshot.target_pos - snapshot.actor_pos;
    let to_target_local = snapshot.target_delta_local();
    let raw_dist = to_target_raw.length();
    if raw_dist > cfg.aggro_radius {
        state.mode = crate::actor::ai::CharacterAiMode::Idle;
        return;
    }
    state.mode = crate::actor::ai::CharacterAiMode::Chase;
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
    let side_offset = cos_p * cfg.standoff_px;
    let away_from_feet_center = -0.45 * cfg.standoff_px;
    let away_from_feet_amp = 0.20 * cfg.standoff_px;
    let down_offset = away_from_feet_center + sin_p * away_from_feet_amp;
    let orbit_offset = frame_to_world(snapshot, ae::LocalAxes::new(side_offset, down_offset));
    let desired_pos = snapshot.target_pos + orbit_offset.vec();
    let to_orbit = desired_pos - snapshot.actor_pos;
    let approach_dist = to_orbit.length();
    let approach_dir = to_orbit.normalize_or_zero();
    // Facing always toward the actual target so the rider / muzzle
    // aims at the player rather than the orbit point.
    let aim_dir = to_target_raw.normalize_or_zero();
    out.facing = to_target_local.x.signum_or(snapshot.actor_facing);
    // Move toward the orbit point at strafe_speed. Aerial archetypes
    // (sharks etc.) need 2D motion to actually orbit; the
    // integration uses both x and y when `is_aerial = true`.
    // Scale down speed when within a small radius of the desired
    // position so the actor doesn't oscillate around it.
    let speed_scale = (approach_dist / 24.0).min(1.0);
    out.velocity_target = ae::WorldVec2(approach_dir * cfg.strafe_speed * speed_scale);
    out.velocity_target = apply_flying_separation(out.velocity_target, cfg.strafe_speed, snapshot);
    // Fire at the actual target when the cooldown timer is clear.
    // ActionSet supplies the concrete projectile (speed, damage);
    // brain just emits dir.
    if state.cooldown_remaining <= 0.0 {
        // Speed = 0.0 here is a sentinel; the action_set resolver
        // pulls speed from the actor's RangedActionSpec when it
        // builds the projectile spawn.
        out.fire = Some(crate::actor::control::ActorFireRequest::world_space(
            aim_dir, 0.0,
        ));
        state.cooldown_remaining = cfg.fire_cooldown_s;
        state.mode = crate::actor::ai::CharacterAiMode::Attack;
    }
}

// ===== Sniper =====

/// Hold position + long-range fire. Like a Skirmisher but does not
/// strafe — used by stationary turret-like enemies.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Decrements each tick by `snapshot.dt`.
    pub cooldown_remaining: f32,
}

fn tick_sniper(
    cfg: &SniperCfg,
    state: &mut SniperState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    state.cooldown_remaining = (state.cooldown_remaining - snapshot.dt).max(0.0);
    if !snapshot.target_alive {
        return;
    }
    let to_target = snapshot.target_pos - snapshot.actor_pos;
    let to_target_local = snapshot.target_delta_local();
    let dist = to_target.length();
    if dist > cfg.aggro_radius {
        return;
    }
    let dir = to_target.normalize_or_zero();
    out.facing = to_target_local.x.signum_or(snapshot.actor_facing);
    if state.cooldown_remaining <= 0.0 {
        out.fire = Some(crate::actor::control::ActorFireRequest::world_space(
            dir, 0.0,
        ));
        state.cooldown_remaining = cfg.fire_cooldown_s;
    }
}

// ===== ChargeCrash =====

/// Dedicated shark charge policy. The riderless burning shark uses
/// this to lunge forward in bursts rather than simply marching like
/// a melee brute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeCrashCfg {
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
pub struct ChargeCrashState {
    pub mode: crate::actor::ai::CharacterAiMode,
    pub charge_remaining: f32,
    pub charge_cooldown_remaining: f32,
    pub orbit_phase: f32,
}

fn tick_charge_crash(
    cfg: &ChargeCrashCfg,
    state: &mut ChargeCrashState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    state.charge_cooldown_remaining = (state.charge_cooldown_remaining - snapshot.dt).max(0.0);
    state.charge_remaining = (state.charge_remaining - snapshot.dt).max(0.0);
    state.orbit_phase += cfg.orbit_drift_rad_s * snapshot.dt;
    if state.orbit_phase > std::f32::consts::TAU {
        state.orbit_phase -= std::f32::consts::TAU;
    }

    if !snapshot.target_alive {
        state.mode = crate::actor::ai::CharacterAiMode::Idle;
        return;
    }

    let to_target = snapshot.target_pos - snapshot.actor_pos;
    let to_target_local = snapshot.target_delta_local();
    let dist = to_target.length();
    if dist > cfg.aggro_radius {
        state.mode = crate::actor::ai::CharacterAiMode::Idle;
        return;
    }

    // The shark steers by `orbit_dir` (below), not a direct aim vector — the old
    // `aim_dir` chase heading was superseded by the orbit-standoff model and left
    // an unused binding. `facing` still comes from the target's local-frame side.
    let facing = to_target_local.x.signum_or(snapshot.actor_facing);
    out.facing = facing;

    let (sin_p, cos_p) = state.orbit_phase.sin_cos();
    let orbit_offset = frame_to_world(
        snapshot,
        ae::LocalAxes::new(
            cos_p * cfg.standoff_px,
            -0.42 * cfg.standoff_px + sin_p * cfg.vertical_wobble_px,
        ),
    );
    let desired_orbit_pos = snapshot.target_pos + orbit_offset.vec();
    let to_orbit = desired_orbit_pos - snapshot.actor_pos;
    let orbit_dir = to_orbit.normalize_or_zero();

    if state.charge_remaining > 0.0 {
        state.mode = crate::actor::ai::CharacterAiMode::Attack;
        out.velocity_target = apply_flying_separation(
            ae::WorldVec2(orbit_dir * cfg.charge_speed),
            cfg.charge_speed,
            snapshot,
        );
        return;
    }

    if dist <= cfg.bite_range && snapshot.attack_cooldown_remaining <= 0.0 {
        state.mode = crate::actor::ai::CharacterAiMode::Attack;
        out.melee_pressed = true;
        return;
    }

    if state.charge_cooldown_remaining <= 0.0 {
        state.mode = crate::actor::ai::CharacterAiMode::Telegraph;
        state.charge_remaining = cfg.charge_duration_s.max(snapshot.dt);
        state.charge_cooldown_remaining = cfg.charge_cooldown_s;
        out.velocity_target = apply_flying_separation(
            ae::WorldVec2(orbit_dir * cfg.charge_speed),
            cfg.charge_speed,
            snapshot,
        );
        return;
    }

    state.mode = crate::actor::ai::CharacterAiMode::Chase;
    out.velocity_target = apply_flying_separation(
        ae::WorldVec2(orbit_dir * cfg.cruise_speed),
        cfg.cruise_speed,
        snapshot,
    );
}

fn apply_flying_separation(
    desired_vel: ae::WorldVec2,
    base_speed: f32,
    snapshot: &BrainSnapshot,
) -> ae::WorldVec2 {
    let Some(crowding) = snapshot.crowding else {
        return desired_vel;
    };
    if crowding.same_faction_count == 0 || crowding.away_dir.length_squared() <= f32::EPSILON {
        return desired_vel;
    }
    let pressure = crowding.pressure.clamp(0.0, 1.0);
    // `crowding.away_dir` is a world direction, so the separation it builds is a
    // world velocity and adds to a world one.
    let separation =
        ae::WorldVec2(crowding.away_dir.normalize_or_zero() * base_speed * (1.25 + pressure));
    let blended = desired_vel + separation;
    let max_speed = base_speed * (1.45 + pressure * 0.35);
    let speed = blended.length();
    if speed > max_speed && speed > 0.0 {
        blended * (max_speed / speed)
    } else {
        blended
    }
}

// ===== Aerial =====
//
// A lively flying brain with two faces selected by `aggressiveness`:
//   - peaceful (0.0): a bird that feels ALIVE — it flits between airborne
//     perches and ground spots, dwells/hops, and when the player comes near
//     it drops down beside them to be talked to (like a grounded NPC).
//   - hostile (>0): an aerial dive-bomber — stalks to an altitude above its
//     target, dives, pecks on contact, then peels off to recover.
//
// Pure + DETERMINISTIC: every "random" choice is hashed from `sim_time` + the
// spawn anchor, so the whole performance reproduces in a headless test (no RNG,
// no frame-timing dependence beyond `dt`). The actor must be gravity-free
// (enemy `is_aerial`, or a `Floating` NPC) so `velocity_target` drives 2D flight.

/// Deterministic pseudo-random in `[0, 1)` from a float seed (the classic
/// fract-of-a-big-sine hash). Keeps the brain reproducible without an RNG.
fn aerial_hash01(seed: f32) -> f32 {
    let x = (seed * 12.9898 + 7.137).sin() * 43758.5453;
    x - x.floor()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AerialCfg {
    /// `0.0` = lively peaceful bird; `>0.0` = aerial dive attacker.
    pub aggressiveness: f32,
    /// Wander / reposition speed (px/s).
    pub cruise_speed: f32,
    /// Attack dive speed (px/s).
    pub dive_speed: f32,
    /// Peaceful: drop-beside-player "talk" radius. Hostile: unused gate today.
    pub aggro_radius: f32,
    /// Melee reach (px) for the dive peck.
    pub attack_range: f32,
    /// How far the bird ranges from its anchor (px); also the dive altitude.
    pub roam_radius: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AerialPhase {
    /// Lively: airborne dwell on a high perch.
    #[default]
    Perch,
    /// Lively: in transit to the current waypoint.
    Fly,
    /// Lively: small hops along the ground.
    Walk,
    /// Hostile: repositioning to an altitude above the target.
    Stalk,
    /// Hostile: committed dive at the target.
    Dive,
    /// Hostile: peeling off + climbing after a dive.
    Recover,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AerialState {
    pub phase: AerialPhase,
    /// Sim time at which the current dwell/recover ends.
    pub phase_until: f32,
    /// Current fly-to target (lively).
    pub waypoint: ae::Vec2,
    /// Home anchor — captured from the actor's position on the first tick, so
    /// the brain needs no spawn coordinate threaded through construction. For a
    /// lively bird `anchor.y` is also the ground/perch reference it lands on.
    pub anchor: ae::Vec2,
    /// Cached mode for HUD / animation.
    pub mode: crate::actor::ai::CharacterAiMode,
    /// Lazily seeded on first tick (anchor/waypoint/phase need real values).
    pub initialized: bool,
}

fn aerial_pick_waypoint(
    cfg: &AerialCfg,
    state: &mut AerialState,
    now: f32,
    frame: ae::AccelerationFrame,
) {
    let anchor = state.anchor;
    let h1 = aerial_hash01(now * 0.37 + anchor.x * 0.13);
    let h2 = aerial_hash01(now * 0.71 + anchor.y * 0.17 + 3.3);
    let dx = (h1 - 0.5) * 2.0 * cfg.roam_radius;
    // ~60% airborne perches, ~40% ground stops, so it mixes flight + walking.
    let airborne = h2 > 0.4;
    let dy = if airborne {
        -(0.3 + h2 * 0.7) * cfg.roam_radius
    } else {
        0.0
    };
    state.waypoint = anchor + frame.to_world(ae::Vec2::new(dx, dy));
}

fn tick_aerial(
    cfg: &AerialCfg,
    state: &mut AerialState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    let pos = snapshot.actor_pos;
    let now = snapshot.sim_time;

    if cfg.aggressiveness > 0.0 {
        tick_aerial_hostile(cfg, state, snapshot, out, pos, now);
    } else {
        tick_aerial_lively(cfg, state, snapshot, out, pos, now);
    }

    // Face the target if engaged, else the direction of travel.
    let face_side = if snapshot.target_alive {
        frame_to_local(snapshot, ae::WorldVec2(snapshot.target_pos - pos)).x
    } else {
        frame_to_local(snapshot, out.velocity_target).x
    };
    if face_side.abs() > 4.0 {
        out.facing = face_side.signum();
    }
}

fn tick_aerial_lively(
    cfg: &AerialCfg,
    state: &mut AerialState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
    pos: ae::Vec2,
    now: f32,
) {
    use crate::actor::ai::CharacterAiMode;

    // Player-near: drop down beside the player at their feet level and hold,
    // so they can strike up a conversation — same "stop and face" feel as a
    // grounded patrol NPC, but the bird flies down to do it.
    if snapshot.target_alive {
        let to_target = snapshot.target_pos - pos;
        let to_target_local = frame_to_local(snapshot, ae::WorldVec2(to_target));
        if to_target.length() < cfg.aggro_radius {
            state.mode = CharacterAiMode::Idle;
            state.initialized = false; // re-roll a fresh leg once the player leaves
            let side = if to_target_local.x >= 0.0 { -1.0 } else { 1.0 };
            let perch = snapshot.target_pos
                + frame_to_world(snapshot, ae::LocalAxes::new(side * 30.0, 0.0)).vec();
            let delta = perch - pos;
            out.velocity_target = ae::WorldVec2(if delta.length() > 6.0 {
                delta.normalize_or_zero() * cfg.cruise_speed
            } else {
                ae::Vec2::ZERO
            });
            return;
        }
    }

    if !state.initialized {
        state.initialized = true;
        state.anchor = pos;
        aerial_pick_waypoint(cfg, state, now, snapshot.acceleration_frame());
        state.phase = AerialPhase::Fly;
    }

    match state.phase {
        AerialPhase::Fly => {
            state.mode = CharacterAiMode::Patrol;
            let delta = state.waypoint - pos;
            if delta.length() <= 10.0 {
                // Arrived: dwell. A high waypoint → perch; a ground one → walk.
                let airborne =
                    frame_to_local(snapshot, ae::WorldVec2(state.waypoint - state.anchor)).y < -8.0;
                state.phase = if airborne {
                    AerialPhase::Perch
                } else {
                    AerialPhase::Walk
                };
                let dwell = 1.1 + aerial_hash01(now + state.anchor.x) * 1.7;
                state.phase_until = now + dwell;
                out.velocity_target = ae::WorldVec2::ZERO;
            } else {
                out.velocity_target = ae::WorldVec2(delta.normalize_or_zero() * cfg.cruise_speed);
            }
        }
        AerialPhase::Walk => {
            state.mode = CharacterAiMode::Patrol;
            // Little ground hops: a slow back-and-forth drift.
            let hop = (now * 2.4).sin() * cfg.cruise_speed * 0.3;
            out.velocity_target = frame_to_world(snapshot, ae::LocalAxes::new(hop, 0.0));
            if now >= state.phase_until {
                aerial_pick_waypoint(cfg, state, now, snapshot.acceleration_frame());
                state.phase = AerialPhase::Fly;
            }
        }
        AerialPhase::Perch => {
            state.mode = CharacterAiMode::Patrol;
            out.velocity_target = ae::WorldVec2::ZERO;
            if now >= state.phase_until {
                aerial_pick_waypoint(cfg, state, now, snapshot.acceleration_frame());
                state.phase = AerialPhase::Fly;
            }
        }
        // Hostile phases can't occur on a peaceful bird; reset defensively.
        _ => state.phase = AerialPhase::Fly,
    }
}

fn tick_aerial_hostile(
    cfg: &AerialCfg,
    state: &mut AerialState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
    pos: ae::Vec2,
    now: f32,
) {
    use crate::actor::ai::CharacterAiMode;

    if !state.initialized {
        state.initialized = true;
        state.anchor = pos;
        state.phase = AerialPhase::Stalk;
    }

    if !snapshot.target_alive {
        // No prey: loiter near the captured anchor.
        state.mode = CharacterAiMode::Patrol;
        let delta = state.anchor - pos;
        out.velocity_target = ae::WorldVec2(if delta.length() > 12.0 {
            delta.normalize_or_zero() * cfg.cruise_speed
        } else {
            ae::Vec2::ZERO
        });
        return;
    }

    let target = snapshot.target_pos;
    let to_t = target - pos;
    let dist = to_t.length();
    let altitude = cfg.roam_radius.max(80.0);

    match state.phase {
        AerialPhase::Stalk => {
            state.mode = CharacterAiMode::Chase;
            // Climb to a point above the target, then commit to a dive.
            let anchor =
                target + frame_to_world(snapshot, ae::LocalAxes::new(0.0, -altitude)).vec();
            let delta = anchor - pos;
            out.velocity_target = apply_flying_separation(
                ae::WorldVec2(delta.normalize_or_zero() * cfg.cruise_speed),
                cfg.cruise_speed,
                snapshot,
            );
            let actor_from_target = frame_to_local(snapshot, ae::WorldVec2(pos - target));
            let lined_up = actor_from_target.y < -altitude * 0.5
                && actor_from_target.x.abs() < cfg.attack_range * 2.5;
            if lined_up && snapshot.attack_cooldown_remaining <= 0.0 {
                state.phase = AerialPhase::Dive;
                state.mode = CharacterAiMode::Telegraph;
            }
        }
        AerialPhase::Dive => {
            state.mode = CharacterAiMode::Attack;
            out.velocity_target = ae::WorldVec2(to_t.normalize_or_zero() * cfg.dive_speed);
            if dist <= cfg.attack_range && snapshot.attack_cooldown_remaining <= 0.0 {
                out.melee_pressed = true;
            }
            // Hit, or dropped below the target → peel off and recover.
            if dist <= cfg.attack_range
                || frame_to_local(snapshot, ae::WorldVec2(pos - target)).y > 8.0
            {
                state.phase = AerialPhase::Recover;
                state.phase_until = now + 1.1;
            }
        }
        AerialPhase::Recover => {
            state.mode = CharacterAiMode::Chase;
            let away_local = ae::LocalAxes::new(
                frame_to_local(snapshot, ae::WorldVec2(pos - target))
                    .x
                    .signum_or(1.0),
                -1.0,
            )
            .normalize_or_zero();
            let away = frame_to_world(snapshot, away_local);
            out.velocity_target =
                apply_flying_separation(away * cfg.cruise_speed, cfg.cruise_speed, snapshot);
            if now >= state.phase_until {
                state.phase = AerialPhase::Stalk;
            }
        }
        // Lively phases can't occur on a hostile bird; reset defensively.
        _ => state.phase = AerialPhase::Stalk,
    }
}

// ===== PlayerDemo =====
//
// A brain that drives the PLAYER's own movement verbs — run, jump, dash, fly —
// in a repeating cycle. It exists to PROVE the universal-brain seam: an entity
// carrying the player movement clusters + this brain is driven through the exact
// same `update_player_control_with_clusters` integration the human player uses,
// with no player-specific code path. It emits `jump_pressed` / `burst_pressed` /
// `fly_toggle_pressed` on the shared [`ActorControlFrame`] — byte-identical to a
// human pressing those buttons. The clock comes from `snapshot.sim_time`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerDemoCfg {
    /// Horizontal run AXIS intent in `[-1, 1]` (NOT px/s), written straight to
    /// `locomotion`. Every self-locomoting brain — player and enemy alike — now
    /// emits normalized `locomotion` intent and the integrator scales by the
    /// body's `max_run_speed`; the old px/s-velocity-vs-axis dual meaning of
    /// `desired_vel` is gone.
    pub run_axis: f32,
    /// Seconds spent in each verb phase before cycling to the next.
    pub phase_secs: f32,
}

impl Default for PlayerDemoCfg {
    fn default() -> Self {
        Self {
            run_axis: 1.0,
            phase_secs: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerDemoPhase {
    /// Walk to the right.
    #[default]
    Run,
    /// Jump (and keep moving).
    Jump,
    /// Ground dash.
    Dash,
    /// Toggle fly on and climb; toggles off again on exit.
    Fly,
}

impl PlayerDemoPhase {
    fn next(self) -> Self {
        match self {
            Self::Run => Self::Jump,
            Self::Jump => Self::Dash,
            Self::Dash => Self::Fly,
            Self::Fly => Self::Run,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerDemoState {
    pub phase: PlayerDemoPhase,
    pub phase_until: f32,
    /// Whether the demo currently has fly toggled on (so it can toggle off when
    /// it leaves the Fly phase).
    pub fly_on: bool,
    pub initialized: bool,
}

fn tick_player_demo(
    cfg: &PlayerDemoCfg,
    state: &mut PlayerDemoState,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    let now = snapshot.sim_time;

    if !state.initialized {
        state.initialized = true;
        state.phase = PlayerDemoPhase::Run;
        state.phase_until = now + cfg.phase_secs;
    }

    let mut just_entered = false;
    if now >= state.phase_until {
        state.phase = state.phase.next();
        state.phase_until = now + cfg.phase_secs;
        just_entered = true;
    }

    let run = cfg.run_axis;
    match state.phase {
        PlayerDemoPhase::Run => {
            out.facing = 1.0;
            out.locomotion = ae::LocalAxes::new(run, 0.0);
        }
        PlayerDemoPhase::Jump => {
            out.facing = 1.0;
            out.locomotion = ae::LocalAxes::new(run, 0.0);
            // Rising edge on entry; sustain the hold for a variable-height jump.
            out.jump_pressed = just_entered;
            out.jump_held = true;
        }
        PlayerDemoPhase::Dash => {
            out.facing = 1.0;
            out.locomotion = ae::LocalAxes::new(run, 0.0);
            out.burst_pressed = just_entered;
        }
        PlayerDemoPhase::Fly => {
            // Toggle fly ON when entering the phase, then climb (engine `+y` is
            // down, so up is negative). Forward drift too, to read as flight.
            if just_entered && !state.fly_on {
                out.fly_toggle_pressed = true;
                state.fly_on = true;
            }
            out.facing = 1.0;
            out.locomotion = ae::LocalAxes::new(run * 0.4, -1.0);
        }
    }

    // Toggle fly back OFF whenever we're not in the Fly phase, so the body
    // falls + walks again (a controller turning the ability off near ground).
    if !matches!(state.phase, PlayerDemoPhase::Fly) && state.fly_on {
        out.fly_toggle_pressed = true;
        state.fly_on = false;
    }
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
mod tests;
