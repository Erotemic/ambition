//! THE FIGHTER BRAIN'S DATA, pinned here by the orphan rule.
//!
//! ⛔⛔ `Brain`'s snapshot encoder lives in this crate (the orphan rule binds it
//! to whoever owns `Brain`) and `ambition_combat` depends on this crate, so this
//! crate can never name combat. Every type that encoder reads is therefore
//! pinned to `ambition_characters` and cannot follow the behaviour up, however
//! the behaviour is carved. For the Fighter arm the encoder projects EVERY field
//! of `FighterState` — so `FighterCfg`, `ApmLedger`, `PendingAttack`,
//! `FighterState` and `FoeSample` are all pinned, and they are all here.
//!
//! ⚠ NEVER SPELL THE IMPL HEADER IN PROSE. `check_absence_contracts.py` finds
//! encoded types by regexing raw source for the header that binds the snapshot
//! trait to a type, and it does not strip comments — writing it here invents a
//! wire-format entry and turns the contract red. Describe it instead.
//!
//! ⭐ THE MODULE EXISTS TO BE PROVED. If anything below reaches into this
//! subtree's behaviour — `tick_fighter`, `decide`, `refine_by_rollout` — this
//! file stops compiling.
//!
//! ⚠ `AttackVerb`, `HabitModel`, `FighterBrainProfile` and `ShadowTuning` are
//! pinned too and are NOT here: each is declared beside its own behaviour in
//! `options.rs`, `habit.rs`, `profile.rs` and `rollout.rs`. Same shape the smash
//! subtree left behind for a second slice — see D168.
//!
//! ⭐ `FighterState::new` came WITH the data rather than staying with the tick.
//! It is behaviour ON data and had to pick a side; the catalog resolver that
//! calls it is the argument for this one.

use crate::actor::control::ActorControlFrame;
use crate::brain::fighter::habit::HabitModel;
use crate::brain::fighter::profile::FighterBrainProfile;
use crate::perception::DelayedPerception;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::hit_response::HitResponseTuning;

/// Sim rate the cadence and reaction conversions assume when nothing says
/// otherwise. The rig takes `tick_hz` explicitly everywhere it matters; this is
/// only the default a config is built with.
pub const DEFAULT_TICK_HZ: f32 = 60.0;

/// How often the brain thinks, in ticks. §5's 10–20 Hz at a 60 Hz sim.
pub const DEFAULT_DECISION_INTERVAL_TICKS: u32 = 5;

/// The immutable half: who this fighter is and how it is allowed to play.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterCfg {
    pub profile: FighterBrainProfile,
    pub tuning: ShadowTuning,
    /// Ticks between decisions. `0` is coerced to 1 — a brain that decides
    /// "every zero ticks" is a divide-by-zero dressed as a config.
    pub decision_interval_ticks: u32,
    pub tick_hz: f32,
}

impl FighterCfg {
    pub fn new(profile: FighterBrainProfile) -> Self {
        Self {
            profile,
            tuning: ShadowTuning::default(),
            decision_interval_ticks: DEFAULT_DECISION_INTERVAL_TICKS,
            tick_hz: DEFAULT_TICK_HZ,
        }
    }

    pub fn interval(&self) -> u32 {
        self.decision_interval_ticks.max(1)
    }
}

/// The APM ledger: a RATE, not a log.
///
/// Two integers, because the question is "presses per minute so far" and a
/// history of press times answers it no better while being unbounded state a
/// rollback has to carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApmLedger {
    pub presses: u32,
    pub elapsed_ticks: u32,
}

impl ApmLedger {
    /// Actions per minute so far. Zero elapsed ticks is zero APM rather than a
    /// division by zero — a brain that has not lived yet has not acted.
    pub fn apm(self, tick_hz: f32) -> f32 {
        if self.elapsed_ticks == 0 || tick_hz <= 0.0 {
            return 0.0;
        }
        let minutes = self.elapsed_ticks as f32 / (tick_hz * 60.0);
        if minutes <= 0.0 {
            return 0.0;
        }
        self.presses as f32 / minutes
    }

    /// Whether one more press would still be under `cap`.
    ///
    /// Asked BEFORE the press, so the cap is a ceiling the brain never crosses
    /// rather than one it crosses and then sits above. A non-positive cap means
    /// "uncapped" — a fixture that wants a frame-perfect brain says `0.0` rather
    /// than an enormous number.
    pub fn may_press(self, cap: f32, tick_hz: f32) -> bool {
        if cap <= 0.0 {
            return true;
        }
        let would_be = ApmLedger {
            presses: self.presses + 1,
            elapsed_ticks: self.elapsed_ticks.max(1),
        };
        would_be.apm(tick_hz) <= cap
    }
}

/// A press the brain has committed to, and the press it is.
///
/// this was a bare `Option<u32>` — a delay with no memory of what it was
/// delaying. The scored move was chosen, the
/// count matured, and a NEUTRAL melee edge came out; `trigger_moveset_moves`
/// then resolved whatever the default gesture maps to, which for a body with
/// directional variants is the plain jab whatever the brain had picked.
///
/// The binding rides the jitter because the jitter is EXECUTION noise — a human
/// who decides to up-tilt and presses two frames late still up-tilts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingAttack {
    /// Ticks remaining before the press is emitted. Counts DOWN.
    pub ticks: u32,
    /// What to press when it matures.
    pub binding: super::options::AttackBinding,
    /// How long to keep Attack down after the press — the charge, decided by
    /// the situation that chose the move rather than by the one it matures in,
    /// because the opening being paid for is the one that was read.
    /// `0` for everything that is not a smash.
    pub hold_ticks: u32,
}

/// The mutable half. Every field decides what the brain does next, so every
/// field is rollback state.
///
/// No `PartialEq`: `DelayedPerception` holds a `VecDeque<WorldView>` and does not
/// derive it, and comparing two brains by their perception buffers is not a
/// question anything asks. Rollback compares the SNAPSHOT bytes, not the struct.
#[derive(Clone, Debug)]
pub struct FighterState {
    /// What the brain is allowed to see. The ONLY read path — `Perceived` can be
    /// minted nowhere else, which is what makes "the delay buffer is on the only
    /// read path" a type fact rather than a test.
    pub perception: DelayedPerception,
    pub habits: HabitModel,
    /// The intent emitted every tick between decisions. A human's hand does not
    /// go neutral because they stopped thinking.
    pub held: ActorControlFrame,
    /// Ticks until the next decision. Counts DOWN.
    pub ticks_until_decision: u32,
    pub apm: ApmLedger,
    /// A press the brain has committed to, `Some(ticks_until_press)`. The delay
    /// is the execution noise: a human who decides to jab does not jab on the
    /// same frame every time.
    pub pending_press: Option<PendingAttack>,
    /// The noise stream. Advanced only when a sample is consumed.
    pub noise: u64,
    /// The foe as it was at the LAST decision, so the next one can name what the
    /// foe did in between and feed the habit model.
    pub last_foe: Option<FoeSample>,
    /// Ticks the committed button still has to stay DOWN. Counts down, and it
    /// alone decides the sustain, so no button can latch.
    pub charge_hold_ticks: u32,
    /// WHICH button [`Self::charge_hold_ticks`] is holding.
    ///
    /// A held Attack is a smash charge or a string continuation; a held Special
    /// is a charge shot. Same counter, same rule, different field on the frame —
    /// and both fields are written every tick from this pair, so switching from
    /// one to the other cannot leave the old one stuck down.
    pub charge_hold_gesture: ambition_entity_catalog::ChargeGesture,
}

/// The observable facts about a foe that a habit is inferred from. Deliberately
/// small: a habit is a read of what somebody DOES, and everything here is
/// visible from across the stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoeSample {
    pub attacking: bool,
    pub on_ground: bool,
    pub shielding: bool,
    /// Signed: positive means the foe was closing the gap.
    pub closing: f32,
}

impl FighterState {
    pub fn new(cfg: &FighterCfg, seed: u64) -> Self {
        Self {
            perception: DelayedPerception::from_reaction_ms(cfg.profile.reaction_ms, cfg.tick_hz),
            habits: HabitModel::new(cfg.profile.read_weight.max(0.0)),
            held: ActorControlFrame::neutral(),
            ticks_until_decision: 0,
            apm: ApmLedger::default(),
            pending_press: None,
            noise: seed,
            last_foe: None,
            charge_hold_ticks: 0,
            charge_hold_gesture: ambition_entity_catalog::ChargeGesture::Smash,
        }
    }
}

// ⛔⛔ `ShadowTuning` CAME BACK FROM `rollout.rs` (D168, 2026-08-27) because
// `FighterCfg` names it BY VALUE, which makes it part of what the `Brain`
// snapshot encoder reaches — pinned to this crate by the orphan rule, whatever
// happens to the rollout engine that reads it.
//
// ⭐ THE ENGINE ITSELF DID NOT COME. `refine_by_rollout`, the shadow step and the
// scoring stay in `rollout.rs`, which is free to leave; what stays is the SHAPE
// they are configured by.

/// Public knowledge about the game a rollout runs in: physics constants and
/// the standard hit response. A player who has played ten minutes knows all of
/// these numbers by feel; passing them in keeps the module pure while letting
/// the decision tick supply the game's true values. [`ShadowTuning::default`]
/// is a reasonable platformer, good enough for fixtures and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowTuning {
    /// The victim-side hit response (launch + hitstun). The SAME kernel row
    /// shape `damage_apply` uses; the caller picks which feel row applies.
    pub response: HitResponseTuning,
    /// Gravity along `gravity_down`, engine units/s².
    pub gravity: f32,
    /// Grounded locomotion speed a driving fighter holds, units/s.
    pub ground_speed: f32,
    /// Dash speed, which is not a faster walk. The engine's dash SETS
    /// velocity outright (`abilities.rs`: `kinematics.vel = aim * dash_speed`)
    /// and does it airborne as readily as grounded.
    pub dash_speed: f32,
    /// How long that velocity is held (`DASH_TIME`). The dash is an impulse with
    /// a duration, not a sustained input, and the difference is what decides
    /// whether a dash near an edge is a step or a launch.
    pub dash_time: f32,
    /// Instant rise speed a predicted jump imparts, units/s.
    pub jump_speed: f32,
    /// How fast a GROUNDED body that stopped driving loses its speed, px/s².
    ///
    /// `ae::GROUND_FRICTION`, restated here for the same reason `dash_speed` is:
    /// the constant lives above this crate and is public knowledge rather than
    /// hidden state.
    pub ground_coast_decel: f32,
    /// Airborne lateral deceleration, matching `ae::AIR_FRICTION`.
    pub air_coast_decel: f32,
    /// Backward velocity applied by a melee swing (`ae::SLASH_RECOIL`); rollouts must
    /// include it because repeated attacks materially change trajectory.
    pub slash_recoil: f32,
    /// Reach assumed for the FOE's attacks. The view names their phase and
    /// clock but not their move, so their range is a model assumption —
    /// stated, authored, and calibrated by FB6e's fidelity instrument.
    pub assumed_foe_reach: f32,
    /// Damage assumed for the foe's generic predicted attack.
    pub assumed_foe_damage: i32,
    /// Startup/active timings for that generic predicted attack, seconds.
    pub assumed_foe_startup_s: f32,
    pub assumed_foe_active_s: f32,
}

/// Default shadow movement is derived from the engine's canonical [`ae::MovementTuning`];
/// opponent attack properties remain explicit model assumptions.
impl Default for ShadowTuning {
    /// The engine's canonical movement defaults, plus the foe assumptions the
    /// perception view genuinely cannot supply.
    fn default() -> Self {
        Self::for_body(&ae::MovementTuning::default())
    }
}

impl ShadowTuning {
    /// Predict this body from its [`ae::MovementTuning`]. Opponent range, damage, and
    /// timing remain assumptions because the observation does not expose the opponent's move.
    pub fn for_body(movement: &ae::MovementTuning) -> Self {
        Self {
            response: HitResponseTuning {
                knockback_x: 220.0,
                knockback_y: 260.0,
                hitstun_time: 0.35,
                hitstun_reference_launch:
                    ambition_platformer2d_core::hit_response::STANDARD_LAUNCH_SPEED,
                hitstun_max_scale: ambition_platformer2d_core::hit_response::MAX_HITSTUN_SCALE,
                // The shadow rollout predicts the victim's LAUNCH; hitlag is a
                // clock beat it does not simulate, so the row carries the
                // engine default rather than a second opinion about feel.
                hitlag_time: 0.070,
                di_max_angle: 0.0,
            },
            assumed_foe_reach: 60.0,
            assumed_foe_damage: 5,
            // The engine's standard enemy swing timings (combat events
            // vocabulary); restated here because those constants live above
            // this crate, and they are public knowledge, not hidden state.
            assumed_foe_startup_s: 0.36,
            assumed_foe_active_s: 0.20,
            ..Self::from_movement_only(movement)
        }
    }

    /// Re-derive the movement half from `movement`, keeping every assumption.
    ///
    /// The fold a decision uses: a config may carry authored foe assumptions or
    /// a tuned hit response, and only the body's own motion is replaced.
    pub fn with_movement(self, movement: &ae::MovementTuning) -> Self {
        Self {
            response: self.response,
            assumed_foe_reach: self.assumed_foe_reach,
            assumed_foe_damage: self.assumed_foe_damage,
            assumed_foe_startup_s: self.assumed_foe_startup_s,
            assumed_foe_active_s: self.assumed_foe_active_s,
            ..Self::from_movement_only(movement)
        }
    }

    /// The movement half alone. The assumption fields are placeholders here and
    /// are always overwritten by the two callers above — this exists so the
    /// mapping from a body's tuning onto the shadow's fields is written ONCE.
    fn from_movement_only(movement: &ae::MovementTuning) -> Self {
        Self {
            response: HitResponseTuning {
                knockback_x: 0.0,
                knockback_y: 0.0,
                hitstun_time: 0.0,
                hitstun_reference_launch:
                    ambition_platformer2d_core::hit_response::STANDARD_LAUNCH_SPEED,
                hitstun_max_scale: ambition_platformer2d_core::hit_response::MAX_HITSTUN_SCALE,
                // The shadow rollout predicts the victim's LAUNCH; hitlag is a
                // clock beat it does not simulate, so the row carries the
                // engine default rather than a second opinion about feel.
                hitlag_time: 0.070,
                di_max_angle: 0.0,
            },
            gravity: movement.gravity,
            ground_speed: movement.max_run_speed,
            dash_speed: movement.dash_speed,
            dash_time: movement.dash_time,
            jump_speed: movement.jump_speed,
            ground_coast_decel: movement.ground_friction,
            air_coast_decel: movement.air_friction,
            // Every melee press shoves the attacker along `-facing` by this
            // much, and the brain presses an attack on most decisions, so the
            // recoils RATCHET. It is the single biggest force acting on a
            // fighter's body and the model had no term for it at all.
            slash_recoil: movement.slash_recoil,
            assumed_foe_reach: 0.0,
            assumed_foe_damage: 0,
            assumed_foe_startup_s: 0.0,
            assumed_foe_active_s: 0.0,
        }
    }
}
