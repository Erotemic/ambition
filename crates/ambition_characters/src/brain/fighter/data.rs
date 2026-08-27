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
use crate::brain::fighter::rollout::ShadowTuning;
use crate::perception::DelayedPerception;

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

    pub(super) fn interval(&self) -> u32 {
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
    pub(super) fn may_press(self, cap: f32, tick_hz: f32) -> bool {
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
