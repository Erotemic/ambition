//! Data-driven fighter-brain difficulty profiles.
//!
//! Profiles author reaction delay, input-rate limits, execution noise, rollout
//! depth/breadth, and utility weights for ladder levels. Live brain
//! layers consume [`crate::perception::Perceived`], which can only be produced by
//! delayed perception in normal gameplay; the explicit `Perceived::cheating`
//! constructor is reserved for rigs/tests. Shipped profiles never use zero
//! reaction delay.

use crate::perception::DelayedPerception;

use super::options::UtilityWeights;

/// One rung of the difficulty ladder. Content: a game ships its own rows.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct FighterBrainProfile {
    /// 1..=9. Only a label; the ordering lives in the numbers below.
    pub level: u8,
    /// How late the brain sees the world. Never zero on a shipped row (§1.3:
    /// *"Level 9 = small numbers, never zero"*), which is what makes the CPU's
    /// skill prediction rather than reflex.
    pub reaction_ms: f32,
    /// Actions per minute the brain may emit. Data today; enforcement is FB4's rig.
    pub apm_cap: f32,
    /// Timing/aim jitter σ. Data today.
    pub execution_noise: f32,
    /// L3 rollout horizon in ticks. `0` disables L3 entirely, which is the
    /// graceful degradation §1 promises: below the budget, or before N3.1's
    /// restore lands, L2's scores act alone.
    pub rollout_depth: u32,
    /// How many candidate options L3 rolls out. `0` with `rollout_depth = 0`.
    pub rollout_k: u32,
    pub utility_weights: UtilityWeights,
    /// Run the tactical intent (`intent.rs`): hold a spacing band outside the
    /// foe's threat, and arm a punish when its commitment leaves a window.
    /// Off on every shipped rung until the experiment's gate
    /// (`docs/planning/engine/fighter-tactical-intent.md`) says otherwise.
    #[serde(default)]
    pub tactics: bool,
}

impl FighterBrainProfile {
    /// The perception buffer this profile's reaction latency implies.
    ///
    /// The only production path to a `Perceived`. It never calls
    /// `Perceived::cheating`, so no shipped difficulty can read the live world —
    /// even a profile that authored `reaction_ms: 0` would get a zero-delay
    /// BUFFER, which still shows the previous tick's `observe` and still refuses a
    /// same-tick perceive→act on the frame a fight begins (the warm-up rule).
    pub fn delay(&self, tick_hz: f32) -> DelayedPerception {
        DelayedPerception::from_reaction_ms(self.reaction_ms, tick_hz)
    }

    /// The engine's profile for a fighter in a room that no game gave a
    /// ladder: ONE profile at every level. `level` is kept as a label, so a
    /// ladder declared later finds the rung.
    ///
    /// What "CPU level N" means is a game's decision (Q88): Smash's pack states
    /// nine rows, and another game must not inherit them as a hidden curve.
    /// The values are the middle of the human range §1.3 names (reaction 500 ms
    /// to 150 ms, APM 120 to 420), never zero, and with no L3 rollout, which no
    /// shipped rung runs.
    pub fn engine_default(level: u8) -> Self {
        Self {
            level: level.clamp(1, 9),
            reaction_ms: 325.0,
            apm_cap: 270.0,
            execution_noise: 0.275,
            rollout_depth: 0,
            rollout_k: 0,
            utility_weights: UtilityWeights::default(),
            tactics: false,
        }
    }

    /// Does this profile run L3? Below the budget, or before N3.1's restore exists,
    /// L2's scores act alone — L3 is an upgrade, never a dependency (§1).
    pub fn uses_rollouts(&self) -> bool {
        self.rollout_depth > 0 && self.rollout_k > 0
    }
}

/// The one place a level becomes a profile: the game's rung when its ladder
/// has one, else [`FighterBrainProfile::engine_default`].
///
/// A rule about which of two sources wins cannot be enforced by the source
/// that loses, so the rule is this function and not a doc comment on the
/// default.
pub fn profile_for_level(level: u8, ladder: Option<&FighterBrainLadder>) -> FighterBrainProfile {
    ladder
        .and_then(|ladder| ladder.level(level))
        .cloned()
        .unwrap_or_else(|| FighterBrainProfile::engine_default(level))
}

/// A game's ladder: nine rows, level 1 through 9.
///
/// A game declares it as a rule of the rooms it governs
/// (`DeclaredRules<FighterBrainLadder>`, read per brain by
/// `project_authored_fighter_ladder`): one binary hosts several games, and each
/// game's CPUs fight on that game's rows (Q88). A room that no game gave a
/// ladder keeps [`FighterBrainProfile::engine_default`].
///
/// Config, not state. A rewind restores a brain; it never rebuilds the ladder
/// that brain was constructed from, so this is deliberately not rollback state.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct FighterBrainLadder {
    rungs: Vec<FighterBrainProfile>,
}

impl FighterBrainLadder {
    pub fn from_ron(ron: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron)
    }

    /// Rungs in authored order.
    pub fn rungs(&self) -> &[FighterBrainProfile] {
        &self.rungs
    }

    /// The rungs, writable — for a TOOL that sweeps one authored field across
    /// the whole ladder before installing it.
    ///
    /// ⛔⛔ **THE ALTERNATIVE IS POKING LIVE BRAINS, AND THAT LOSES SILENTLY,
    /// EVERY TICK.** `project_authored_fighter_ladder` is a CONTINUOUS
    /// authority over `FighterCfg::profile`: it deliberately carries no change
    /// filter (no tick-based filter composes with a disabling component, so a
    /// candidate session's brains would keep the engine floor), which means it
    /// re-reads every fighter every tick and rewrites any profile that differs
    /// from its rung — rebuilding `FighterState` with it. ⇒ A second writer of a
    /// LIVE profile is reverted within one tick, and resets that fighter's
    /// pending press and habit state on every tick it keeps trying.
    ///
    /// ⚠ MEASURED 2026-09-21, not reasoned: `ladder-rig --ladder <shipped>` gave
    /// byte-identical bouts for `--apm 1` and `--apm 600`, and for every
    /// `--weight` value including `reach_fit=0` against `reach_fit=999`, because
    /// none of them survived a tick. The same flags on the ENGINE FLOOR — where
    /// no ladder resource exists and the projection returns early — moved every
    /// number. The rig's whole reason to exist on the shipped ladder was a
    /// no-op and nothing said so.
    ///
    /// ⇒ **When a ladder is installed, the ladder is the owner, and an override
    /// belongs here.** Callers that change ordering-relevant fields should ask
    /// [`Self::problems`] afterwards rather than assume the sweep kept the
    /// ladder well-formed.
    pub fn rungs_mut(&mut self) -> &mut [FighterBrainProfile] {
        &mut self.rungs
    }

    pub fn level(&self, level: u8) -> Option<&FighterBrainProfile> {
        self.rungs.iter().find(|r| r.level == level)
    }

    /// The ladder's own well-formedness, checkable without a single match.
    ///
    /// Every one of these is a way a ladder can be nonsense while every individual
    /// row looks fine, and every one of them would show up in a self-play run as
    /// "the levels do not order correctly" — after hours of matches, instead of at
    /// startup.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.rungs.len() != 9 {
            out.push(format!(
                "a ladder has 9 rungs, this has {}",
                self.rungs.len()
            ));
        }
        for (i, r) in self.rungs.iter().enumerate() {
            if r.level as usize != i + 1 {
                out.push(format!("rung {i} is labelled level {}", r.level));
            }
            if r.reaction_ms <= 0.0 {
                out.push(format!(
                    "level {}: reaction_ms is {} — a shipped difficulty never reacts \
                     instantly (§1.3)",
                    r.level, r.reaction_ms
                ));
            }
            if r.apm_cap <= 0.0 {
                out.push(format!("level {}: apm_cap must be positive", r.level));
            }
        }
        for pair in self.rungs.windows(2) {
            let (lo, hi) = (&pair[0], &pair[1]);
            if hi.reaction_ms > lo.reaction_ms {
                out.push(format!(
                    "level {} reacts slower than level {} ({}ms vs {}ms) — the ladder \
                     is not monotone in reaction",
                    hi.level, lo.level, hi.reaction_ms, lo.reaction_ms
                ));
            }
            if hi.apm_cap < lo.apm_cap {
                out.push(format!(
                    "level {} may act less often than level {}",
                    hi.level, lo.level
                ));
            }
            if hi.execution_noise > lo.execution_noise {
                out.push(format!(
                    "level {} is sloppier than level {}",
                    hi.level, lo.level
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
