//! **FB5 — the opponent model: the "reads".**
//!
//! `docs/planning/engine/fighter-brain.md` §1:
//!
//! > *"Opponent modeling (the 'reads'): a small frequency memory over the
//! > opponent's observed choices in bucketed situations (tech direction, ledge
//! > option, approach habit) with decay. Level-9 reads = sampling the model; lower
//! > levels ignore it. Bounded, inspectable, and it's the honest version of what
//! > human top players do."*
//!
//! Honest is the operative word. The model observes only what the view already
//! showed — *what the opponent DID, in a situation the brain could name* — and
//! never what they are about to do. A brain that reads you is not a brain that can
//! see your controller.
//!
//! ## Bounded and inspectable, by construction
//!
//! - **Bounded**: `(Situation, Choice)` is a small closed product — 5 × N — so the
//!   whole model is a fixed-size table, not a growing history. Nothing to prune.
//! - **Inspectable**: [`HabitModel::frequency`] answers *"how often, out of what?"*
//!   in one call, and [`HabitModel::rows`] walks the whole thing in a stable order.
//! - **Deterministic**: a `BTreeMap`, not the sketch's `HashMap`. §5's sketch notes
//!   the counts are "read-only lookups, determinism-safe", which is true of the
//!   LOOKUP and false of any iteration — and a trace, a snapshot, and FB6's
//!   rollouts all iterate. ADR 0023 bans std-hash iteration where the sim can
//!   observe the order.
//!
//! ## Decay is what makes it a READ rather than a census
//!
//! An opponent who spot-dodged nine times and then stopped is not a spot-dodger. A
//! plain count says otherwise forever. [`HabitModel::observe`] decays every row of
//! the situation it saw before crediting the choice, so a habit fades at a rate the
//! difficulty chooses and a recent switch outweighs an old pattern.

use std::collections::BTreeMap;

use super::situation::Situation;

/// What the opponent was observed to DO in a situation. A closed vocabulary,
/// because §1 says *"no scripting language"* about the boss patterns and the same
/// discipline applies here: a choice an authoring agent cannot name is a choice
/// L2 cannot price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Choice {
    /// Closed the distance.
    Approach,
    /// Opened it.
    Retreat,
    Jump,
    Attack,
    Shield,
    /// Did nothing readable. Recorded, because "they wait" is itself a habit.
    Wait,
}

impl Choice {
    /// Every choice, in a fixed order. Iterating this — rather than the model's
    /// keys — is how a caller enumerates options without depending on what has been
    /// observed so far.
    pub const ALL: [Choice; 6] = [
        Choice::Approach,
        Choice::Retreat,
        Choice::Jump,
        Choice::Attack,
        Choice::Shield,
        Choice::Wait,
    ];
}

/// Decayed frequency counts over `(Situation, Choice)`.
///
/// `Default` is an empty model, which predicts nothing and whose every frequency
/// is the uniform prior. A brain with `read_weight = 0` (levels 1–3) never asks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HabitModel {
    counts: BTreeMap<(Situation, Choice), f32>,
    /// Multiplied into a situation's rows each time it is observed. `1.0` never
    /// forgets; `0.0` remembers only the last thing. `0.9` is a read.
    pub decay: f32,
}

impl HabitModel {
    /// `decay` is clamped to `0..=1`. A model that grew its counts would eventually
    /// saturate an `f32` and stop learning, silently.
    pub fn new(decay: f32) -> Self {
        Self {
            counts: BTreeMap::new(),
            decay: decay.clamp(0.0, 1.0),
        }
    }

    /// The opponent, in `situation`, did `choice`.
    ///
    /// Decays every row of THAT situation before crediting, so the counts stay a
    /// recency-weighted frequency rather than a lifetime census. Situations the
    /// opponent was not in do not decay — being edge-guarded rarely does not make
    /// what they do there any less known.
    pub fn observe(&mut self, situation: Situation, choice: Choice) {
        for c in Choice::ALL {
            if let Some(count) = self.counts.get_mut(&(situation, c)) {
                *count *= self.decay;
            }
        }
        *self.counts.entry((situation, choice)).or_insert(0.0) += 1.0;
    }

    /// Raw decayed count. Mostly for a trace; [`frequency`](Self::frequency) is the
    /// number a brain reasons with.
    pub fn count(&self, situation: Situation, choice: Choice) -> f32 {
        self.counts
            .get(&(situation, choice))
            .copied()
            .unwrap_or(0.0)
    }

    /// `count / total_for_situation`, or a **uniform prior** when the opponent has
    /// never been seen in this situation.
    ///
    /// The prior matters: a model that returned `0.0` for the unseen would tell a
    /// level-9 brain that its opponent will *never* shield out of a juggle, on the
    /// evidence of never having juggled them. Ignorance is not knowledge of absence.
    pub fn frequency(&self, situation: Situation, choice: Choice) -> f32 {
        let total: f32 = Choice::ALL.iter().map(|c| self.count(situation, *c)).sum();
        if total <= 0.0 {
            return 1.0 / Choice::ALL.len() as f32;
        }
        self.count(situation, choice) / total
    }

    /// The choice the opponent most often makes here, and how often — `None` when
    /// they have never been seen in this situation.
    ///
    /// Ties break on the `Choice` order, so a read is a function of the evidence
    /// and not of insertion history (ADR 0023).
    pub fn read(&self, situation: Situation) -> Option<(Choice, f32)> {
        let total: f32 = Choice::ALL.iter().map(|c| self.count(situation, *c)).sum();
        if total <= 0.0 {
            return None;
        }
        Choice::ALL
            .iter()
            .map(|c| (*c, self.count(situation, *c)))
            .max_by(|a, b| {
                a.1.partial_cmp(&b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.0.cmp(&a.0)) // earlier `Choice` wins a tie
            })
            .map(|(c, n)| (c, n / total))
    }

    /// How much a brain with this `read_weight` should shade its scoring toward the
    /// read: `read_weight × (frequency − uniform)`, so an opponent who does the
    /// expected thing exactly as often as chance contributes nothing at all.
    ///
    /// Level 1–3 pass `read_weight = 0` and get zero, exactly as §1 says: *"Level-9
    /// reads = sampling the model; lower levels ignore it."*
    pub fn read_bonus(&self, situation: Situation, choice: Choice, read_weight: f32) -> f32 {
        let uniform = 1.0 / Choice::ALL.len() as f32;
        read_weight * (self.frequency(situation, choice) - uniform)
    }

    /// Every non-empty row, in a stable order. What a trace prints and what FB6's
    /// rollouts sample a predicted policy from.
    pub fn rows(&self) -> impl Iterator<Item = ((Situation, Choice), f32)> + '_ {
        self.counts.iter().map(|(k, v)| (*k, *v))
    }

    /// Rows observed. Bounded by `5 × 6`, always.
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

#[cfg(test)]
mod tests;
