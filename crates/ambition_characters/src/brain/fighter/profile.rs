//! Data-driven fighter-brain difficulty profiles.
//!
//! Profiles author reaction delay, input-rate limits, execution noise, rollout
//! depth/breadth, read weight, and utility weights for ladder levels. Live brain
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
    /// How hard the brain leans on the opponent model (FB5). `0` ignores it.
    pub read_weight: f32,
    pub utility_weights: UtilityWeights,
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

    /// The engine's default rung for a level, so a catalog row can say
    /// `Fighter { level: 5 }` and get a brain.
    ///
    /// this is a FLOOR, not the ladder. A game that cares ships its own nine
    /// rows (`FighterBrainLadder::from_ron`) and this is never consulted; what it
    /// exists for is that authoring a fighter should not require authoring a
    /// difficulty curve first. Before it, the rig was unreachable from content at
    /// all — which is a worse default than an imperfect one.
    ///
    /// The shape follows §1.3: reaction shrinks and APM grows with level, and
    /// level 9 is small numbers, never zero — a frame-perfect CPU is not a
    /// hard opponent, it is a different game.
    pub fn for_level(level: u8) -> Self {
        let level = level.clamp(1, 9);
        let t = (level - 1) as f32 / 8.0;
        Self {
            level,
            // 500ms at level 1 down to 150ms at level 9 — the two numbers §1.3
            // names, linearly between.
            reaction_ms: 500.0 - t * 350.0,
            // A human ceiling. 120 APM is a relaxed player, 420 is a very fast
            // one; nothing here approaches a machine's.
            apm_cap: 120.0 + t * 300.0,
            // Execution noise SHRINKS with level but never reaches zero, for the
            // same reason reaction does not.
            execution_noise: 0.45 - t * 0.35,
            // L3 is an upgrade, not a dependency (§1): the lower rungs act on
            // L2's scores alone, which is also what keeps a four-fighter match
            // affordable.
            rollout_depth: if level >= 6 { 12 } else { 0 },
            rollout_k: if level >= 6 { 4 } else { 0 },
            read_weight: t * 0.6,
            utility_weights: UtilityWeights::default(),
        }
    }

    /// Does this profile run L3? Below the budget, or before N3.1's restore exists,
    /// L2's scores act alone — L3 is an upgrade, never a dependency (§1).
    pub fn uses_rollouts(&self) -> bool {
        self.rollout_depth > 0 && self.rollout_k > 0
    }
}

/// The game's authored ladder, where the sim can reach it.
///
/// a resource rather than an argument, because of where the pack lives.
/// The prepared content pack is `game/ambition_content`, ABOVE the monolith that
/// builds most brains — so the monolith cannot fetch the ladder, and the game has
/// to hand it down. Absent means this game shipped no rows and
/// [`FighterBrainProfile::for_level`] applies, which is the engine's stated rule.
///
/// config, not state. A rewind restores a brain; it never rebuilds the
/// ladder that brain was constructed from, so this is deliberately not rollback
/// state — the same argument that puts write-once construction data outside the
/// snapshot.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq)]
pub struct AuthoredFighterLadder(pub FighterBrainLadder);

/// The one place a level becomes a profile.
///
/// the engine's rule was written in a doc comment and consulted nowhere, so
/// it did not hold. [`FighterBrainProfile::for_level`] says of itself: *"this
/// is a FLOOR, not the ladder. A game that cares ships its own nine rows
/// (`FighterBrainLadder::from_ron`) and this is never consulted."* Ambition ships
/// the nine rows, and both production call sites called the floor anyway —
/// because a rule about which of two sources wins cannot be enforced by the
/// source that loses. This function IS the rule.
pub fn profile_for_level(level: u8, ladder: Option<&FighterBrainLadder>) -> FighterBrainProfile {
    ladder
        .and_then(|ladder| ladder.level(level))
        .cloned()
        .unwrap_or_else(|| FighterBrainProfile::for_level(level))
}

/// A game's ladder: nine rows, level 1 through 9.
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
