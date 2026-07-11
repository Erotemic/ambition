//! **FB2 — L2, the option generator and utility scorer.**
//!
//! `docs/planning/engine/fighter-brain.md` §1:
//!
//! > *"L2 — Option generator + utility scorer: per state, enumerate legal options
//! > from DATA — movement verbs from the body's capability mask, and attacks from
//! > the frame-data table (CM7 …) — the brain knows its moveset the way a player
//! > who read the frame data does, and automatically understands any NEW character
//! > it's put in. Score = authored utility features (range vs. option reach, frame
//! > advantage, kill potential at victim's damage meter, stage position risk) with
//! > per-difficulty weights."*
//!
//! Pure. Every input is the [`Perceived`] view the no-cheat contract allows, plus the
//! body's own kit and its difficulty's [`UtilityWeights`].
//!
//! ## The four features, and why each is a fact about the VIEW
//!
//! - **`reach_fit`** — does this attack's `reach` match the gap to the opponent?
//!   A jab at three body-lengths scores nothing, and neither does a lunge in
//!   someone's face. This is what makes the brain *understand a new character*:
//!   the reach comes from CM7's frame data, not from a table someone typed.
//! - **`frame_advantage`** — will this attack's `startup_s` beat what the opponent
//!   is already committed to (`phase_remaining`)? Positive means it lands first.
//!   A player who read the frame data knows this number; so does the brain.
//! - **`kill_potential`** — the victim's `damage_frac`. In a smash-percent game a
//!   move's value is not its damage but who it can end.
//! - **`stage_risk`** — how little stage is behind ME. Committing to a long
//!   recovery near a blastzone is how a level-9 CPU dies to a level-3 one.
//!
//! ## What is NOT here
//!
//! **The weights are not tuned.** §FB6: *"Scoring weights are NOT divined up
//! front: v1 weights are authored starting values, then FB4's ladder self-play
//! monotonicity gate is the calibration instrument (adjust until levels order
//! correctly)."* [`UtilityWeights::v1`] is that starting value, and it is a
//! starting value, not a claim.
//!
//! **The decision cadence is not here.** §5: *"rebuilt per decision tick (not per
//! frame — decide at ~10–20 Hz gated by reaction latency, hold intents between
//! decisions)."* The latency lives on `FighterBrainProfile.reaction_ms`, which is
//! FB4's; L2 is a pure function that a decision tick calls.
//!
//! ## A gap in §1's four features, found by building them
//!
//! **None of the four reads a move's POWER.** `kill_potential` is the *victim's*
//! meter; `reach_fit` and `frame_advantage` are geometry and timing;
//! `stage_risk` is about me. So at any weights, given a punish window that BOTH a
//! jab and a smash fit, the jab wins — it is faster and therefore has more frame
//! advantage, and nothing prices the smash's payoff. A level-9 CPU that always
//! jabs its punishes is not a level-9 CPU.
//!
//! CM7's [`MoveFrameData`] carries no damage or knockback either, so L2 could not
//! price it even if a fifth feature existed. Two ways out, and this slice takes
//! neither on its own authority: derive `max_damage`/`max_knockback` into
//! `MoveFrameData` (a pure derivation over the Active volumes, like `reach`) and
//! add an `expected_payoff = damage × landing_chance` feature; or let FB4's ladder
//! discover that the weights cannot order the levels and force the question. §FB6
//! is explicit that *"scoring weights are NOT divined up front … FB4's ladder
//! self-play monotonicity gate is the calibration instrument"*, so the second is
//! the doctrine's own answer. **Recorded here rather than fixed by inventing a
//! fifth weight nobody has calibrated.**

use ambition_entity_catalog::MoveFrameData;

use super::situation::{is_punishable, Situation};
use crate::perception::Perceived;

/// One movement verb the body can attempt. Derived from `SelfView`'s capability
/// mask — the body-enforced floor (invariant I3), so the brain can only propose
/// what the body could accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementVerb {
    /// Close the gap on the ground.
    Approach,
    /// Open the gap on the ground.
    Retreat,
    Jump,
    Dash,
    Shield,
    Blink,
    /// Toward the stage's center. The only verb `Recovery` cares about.
    Recover,
}

/// A scored movement option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveOption {
    pub verb: MovementVerb,
    pub score: f32,
}

/// One attack the body's kit can throw, with the frame data a player who read the
/// tables would know.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackOption {
    pub move_id: String,
    pub frames: MoveFrameData,
    pub score: f32,
    /// The features that produced `score`, so a failing ladder run can be read
    /// rather than guessed at. `Σ weight_i · feature_i` is `score` by construction.
    pub features: Features,
}

/// The four features, unweighted, each in a bounded range so a weight is a
/// comparable number rather than a unit conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Features {
    /// `0..=1`. 1 when the attack's reach exactly spans the gap.
    pub reach_fit: f32,
    /// `-1..=1`. Positive when `startup_s` beats the opponent's commitment.
    pub frame_advantage: f32,
    /// `0..=1`. The victim's accumulated damage fraction.
    pub kill_potential: f32,
    /// `0..=1`. 1 when I am against a blastzone. **Costed, not rewarded** — its
    /// weight is negative in [`UtilityWeights::v1`].
    pub stage_risk: f32,
}

impl Features {
    fn dot(&self, w: &UtilityWeights) -> f32 {
        self.reach_fit * w.reach_fit
            + self.frame_advantage * w.frame_advantage
            + self.kill_potential * w.kill_potential
            + self.stage_risk * w.stage_risk
    }
}

/// Per-difficulty scoring weights. Content in the end (`FighterBrainProfile`'s
/// `utility_weights`); a struct here so L2 stays pure.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct UtilityWeights {
    pub reach_fit: f32,
    pub frame_advantage: f32,
    pub kill_potential: f32,
    /// Negative: stage risk is a cost.
    pub stage_risk: f32,
}

impl UtilityWeights {
    /// **v1 starting values, not tuned.** FB4's ladder self-play monotonicity gate
    /// is the calibration instrument (§FB6). Reach dominates because an attack
    /// that cannot touch the opponent has no other virtue.
    pub fn v1() -> Self {
        Self {
            reach_fit: 1.0,
            frame_advantage: 0.6,
            kill_potential: 0.4,
            stage_risk: -0.8,
        }
    }
}

impl Default for UtilityWeights {
    fn default() -> Self {
        Self::v1()
    }
}

/// One attack the caller's kit offers. The caller resolves these from the body's
/// moveset; L2 never queries anything.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackCandidate {
    pub move_id: String,
    pub frames: MoveFrameData,
}

/// L2's working set for one decision tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionSet {
    /// Scored movement verbs, best first.
    pub movement: Vec<MoveOption>,
    /// Scored attacks, best first. **Empty in [`Situation::Recovery`]**: a body
    /// past the blastzone has exactly one problem.
    pub attacks: Vec<AttackOption>,
}

impl OptionSet {
    pub fn best_attack(&self) -> Option<&AttackOption> {
        self.attacks.first()
    }

    pub fn best_movement(&self) -> Option<MoveOption> {
        self.movement.first().copied()
    }
}

/// How far past its own reach an attack is still worth considering. Beyond this
/// the fit is zero rather than negative — an attack that misses by a mile and one
/// that misses by two are equally useless, and letting the feature go negative
/// would let a big negative reach_fit be bought back by kill potential.
const REACH_TOLERANCE: f32 = 2.0;

/// **L2.** Enumerate and score every legal option for this tick.
///
/// `situation` is L1's answer, passed in rather than recomputed: the two layers
/// must agree about the tick, and a second `classify` call on a delayed view could
/// disagree with the first.
pub fn generate_options(
    view: Perceived<'_>,
    situation: Situation,
    kit: &[AttackCandidate],
    weights: &UtilityWeights,
) -> OptionSet {
    let me = &view.self_view;
    let foe = view.nearest_hostile();

    // Movement first: it is the only thing `Recovery` has.
    let mut movement = movement_options(&view, situation);
    sort_by_score_then_name(&mut movement, |m| (m.score, verb_order(m.verb)));

    if situation == Situation::Recovery || foe.is_none() {
        return OptionSet {
            movement,
            attacks: Vec::new(),
        };
    }
    let foe = foe.expect("checked");

    let gap = (foe.pos - me.pos).length();
    let stage_risk = {
        let half_stage = (view.stage.bounds.max - view.stage.bounds.min).length() * 0.5;
        if half_stage <= 0.0 {
            1.0
        } else {
            (1.0 - view.stage.distance_to_edge(me.pos) / half_stage).clamp(0.0, 1.0)
        }
    };
    // A committed opponent cannot answer for `phase_remaining` seconds. An
    // uncommitted one answers immediately, so any startup at all is a gamble.
    let their_commitment = if is_punishable(foe, me.gravity_down) {
        foe.phase_remaining
    } else {
        0.0
    };

    let mut attacks: Vec<AttackOption> = kit
        .iter()
        .map(|c| {
            let features = Features {
                reach_fit: reach_fit(c.frames.reach, gap),
                frame_advantage: frame_advantage(c.frames.startup_s, their_commitment),
                kill_potential: foe.damage_frac(),
                stage_risk,
            };
            AttackOption {
                move_id: c.move_id.clone(),
                frames: c.frames.clone(),
                score: features.dot(weights),
                features,
            }
        })
        .collect();
    // Ties break on the move id, so the best option is a function of the world and
    // not of the kit's declaration order (ADR 0023: no order-dependent decisions).
    attacks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.move_id.cmp(&b.move_id))
    });

    OptionSet { movement, attacks }
}

/// `1` when the attack's reach exactly spans the gap, falling to `0` as the miss
/// grows past [`REACH_TOLERANCE`] × reach. A zero-reach move (a buff, a summon)
/// has no fit at any distance and must be priced by its other features alone.
pub fn reach_fit(reach: f32, gap: f32) -> f32 {
    if reach <= 0.0 {
        return 0.0;
    }
    let miss = (gap - reach).abs();
    (1.0 - miss / (reach * REACH_TOLERANCE)).clamp(0.0, 1.0)
}

/// `+1` when the attack lands a full startup before the opponent can answer; `-1`
/// when it is a full startup too slow. Normalized by the startup so a slow move's
/// disadvantage is measured against its own commitment, not against a wall clock.
pub fn frame_advantage(startup_s: f32, their_commitment_s: f32) -> f32 {
    let scale = startup_s.max(0.01);
    ((their_commitment_s - startup_s) / scale).clamp(-1.0, 1.0)
}

/// Movement verbs the body's capability mask permits, scored by the situation.
///
/// The scores are coarse ON PURPOSE. §1 puts the interesting judgement in the
/// attack scorer and in L3's rollouts; movement's job at L2 is to express the
/// situation's ONE obligation — get back, get out, get in — so that a brain with
/// no L3 still plays a recognizable game.
fn movement_options(view: &crate::perception::WorldView, situation: Situation) -> Vec<MoveOption> {
    let me = &view.self_view;
    let mut out = Vec::new();
    let mut push = |verb: MovementVerb, score: f32| out.push(MoveOption { verb, score });

    match situation {
        Situation::Recovery => {
            push(MovementVerb::Recover, 1.0);
            if me.can_blink {
                push(MovementVerb::Blink, 0.9);
            }
            push(MovementVerb::Jump, 0.5);
        }
        Situation::Disadvantage => {
            if me.can_shield {
                push(MovementVerb::Shield, 0.8);
            }
            push(MovementVerb::Retreat, 0.7);
            if me.can_dash {
                push(MovementVerb::Dash, 0.6);
            }
            push(MovementVerb::Jump, 0.4);
        }
        Situation::EdgeGuard | Situation::Advantage => {
            push(MovementVerb::Approach, 0.8);
            if me.can_dash {
                push(MovementVerb::Dash, 0.7);
            }
            push(MovementVerb::Jump, 0.3);
        }
        Situation::Neutral => {
            push(MovementVerb::Approach, 0.5);
            push(MovementVerb::Retreat, 0.4);
            push(MovementVerb::Jump, 0.3);
            if me.can_dash {
                push(MovementVerb::Dash, 0.3);
            }
        }
    }
    out
}

fn verb_order(v: MovementVerb) -> MovementVerb {
    v
}

fn sort_by_score_then_name<T, K: Ord>(items: &mut [T], key: impl Fn(&T) -> (f32, K)) {
    items.sort_by(|a, b| {
        let (sa, ka) = key(a);
        let (sb, kb) = key(b);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ka.cmp(&kb))
    });
}

#[cfg(test)]
mod tests;
