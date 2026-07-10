//! **FB4a — the difficulty ladder, as data; and the humanity checks it can keep.**
//!
//! `docs/planning/engine/fighter-brain.md` §4: *"`FighterBrainProfile` (RON):
//! `reaction_ms` (L9 ≈ 150, L1 ≈ 500), `apm_cap`, `execution_noise` (timing/aim
//! jitter σ), `rollout_depth` / `rollout_k` (0 disables L3), `read_weight`
//! (opponent-model usage), `utility_weights`. Levels 1–9 are nine authored rows.
//! Games/demos ship their own rows — it's content."*
//!
//! ## The one humanity check that is now STRUCTURAL
//!
//! §3 asks a test to *"assert the delay buffer is on the ONLY read path"* and to
//! prove *"no same-tick perceive→act"*. FB1 built the buffer and said out loud
//! that nothing forced a brain through it.
//!
//! Nothing has to. [`crate::perception::Perceived`] has a private field, and only
//! [`crate::perception::DelayedPerception::perceive`] mints one. L1's `classify`
//! and L2's `generate_options` take a `Perceived`, so **a brain layer that wanted
//! to read the live world would have to edit `perception.rs` to name it.** A test
//! can be forgotten and a grep lint can be argued with; a type cannot.
//!
//! The one door is `Perceived::cheating`, whose name is the documentation. It is
//! for RL rigs, replay fixtures, and the brain layers' own unit tests.
//! [`FighterBrainProfile::delay`] never calls it, and
//! `no_shipped_profile_reacts_instantly` is why.
//!
//! ## What FB4 still owes
//!
//! - **The APM cap is DATA here, not enforcement.** *"Input-rate histograms within
//!   the APM cap"* needs a brain that emits inputs, and nothing above L2 does.
//! - **The ladder self-play rig** (level *n* beats *n−1* in ≥ 60% of headless
//!   matches) needs the same. It is also the instrument that calibrates
//!   [`super::options::UtilityWeights`] — §FB6 is explicit that the weights are not
//!   divined up front, and FB2 found the hole that will make the ladder say so.

use crate::perception::DelayedPerception;

use super::options::UtilityWeights;

/// One rung of the difficulty ladder. Content: a game ships its own rows.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct FighterBrainProfile {
    /// 1..=9. Only a label; the ordering lives in the numbers below.
    pub level: u8,
    /// How late the brain sees the world. **Never zero on a shipped row** (§1.3:
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
    /// **The only production path to a `Perceived`.** It never calls
    /// `Perceived::cheating`, so no shipped difficulty can read the live world —
    /// even a profile that authored `reaction_ms: 0` would get a zero-delay
    /// BUFFER, which still shows the previous tick's `observe` and still refuses a
    /// same-tick perceive→act on the frame a fight begins (the warm-up rule).
    pub fn delay(&self, tick_hz: f32) -> DelayedPerception {
        DelayedPerception::from_reaction_ms(self.reaction_ms, tick_hz)
    }

    /// Does this profile run L3? Below the budget, or before N3.1's restore exists,
    /// L2's scores act alone — L3 is an upgrade, never a dependency (§1).
    pub fn uses_rollouts(&self) -> bool {
        self.rollout_depth > 0 && self.rollout_k > 0
    }
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

    /// **The ladder's own well-formedness**, checkable without a single match.
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
mod tests {
    use super::*;

    /// Nine rows, monotone in every axis. Reaction falls, APM rises, noise falls.
    const LADDER: &str = r#"[
        (level: 1, reaction_ms: 500.0, apm_cap: 60.0,  execution_noise: 0.40, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.1, kill_potential: 0.0, stage_risk: -0.1)),
        (level: 2, reaction_ms: 450.0, apm_cap: 90.0,  execution_noise: 0.35, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.2, kill_potential: 0.0, stage_risk: -0.2)),
        (level: 3, reaction_ms: 400.0, apm_cap: 120.0, execution_noise: 0.30, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.3, kill_potential: 0.1, stage_risk: -0.3)),
        (level: 4, reaction_ms: 350.0, apm_cap: 160.0, execution_noise: 0.25, rollout_depth: 0, rollout_k: 0, read_weight: 0.1, utility_weights: (reach_fit: 1.0, frame_advantage: 0.4, kill_potential: 0.2, stage_risk: -0.4)),
        (level: 5, reaction_ms: 300.0, apm_cap: 200.0, execution_noise: 0.20, rollout_depth: 0, rollout_k: 0, read_weight: 0.2, utility_weights: (reach_fit: 1.0, frame_advantage: 0.5, kill_potential: 0.3, stage_risk: -0.5)),
        (level: 6, reaction_ms: 260.0, apm_cap: 240.0, execution_noise: 0.16, rollout_depth: 0, rollout_k: 0, read_weight: 0.3, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.6)),
        (level: 7, reaction_ms: 220.0, apm_cap: 280.0, execution_noise: 0.12, rollout_depth: 0, rollout_k: 0, read_weight: 0.5, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.7)),
        (level: 8, reaction_ms: 185.0, apm_cap: 320.0, execution_noise: 0.08, rollout_depth: 0, rollout_k: 0, read_weight: 0.7, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.8)),
        (level: 9, reaction_ms: 150.0, apm_cap: 360.0, execution_noise: 0.05, rollout_depth: 0, rollout_k: 0, read_weight: 1.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.8)),
    ]"#;

    fn ladder() -> FighterBrainLadder {
        FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses")
    }

    #[test]
    fn a_nine_rung_ladder_parses_and_is_well_formed() {
        let l = ladder();
        assert_eq!(l.rungs().len(), 9);
        assert_eq!(l.problems(), Vec::<String>::new());
        assert_eq!(l.level(9).unwrap().reaction_ms, 150.0);
        assert_eq!(l.level(1).unwrap().reaction_ms, 500.0);
    }

    /// **§1.3, as a check rather than a wish.** *"Level 9 = small numbers, never
    /// zero."* A shipped profile that reacts instantly is a cheating CPU wearing a
    /// difficulty's name, and the `problems()` list says so at startup rather than
    /// after a self-play run.
    #[test]
    fn no_shipped_profile_reacts_instantly() {
        for r in ladder().rungs() {
            assert!(r.reaction_ms > 0.0, "level {}", r.level);
        }

        // Reach in the only way a test can: rebuild the RON with a zeroed row.
        let bad = LADDER.replace("reaction_ms: 150.0", "reaction_ms: 0.0");
        let cheat = FighterBrainLadder::from_ron(&bad).unwrap();
        let problems = cheat.problems();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("never reacts instantly")),
            "{problems:?}"
        );
    }

    /// The ladder's monotonicity is checkable BEFORE a single match. Every one of
    /// these would otherwise surface as "the levels do not order correctly" after
    /// hours of self-play.
    #[test]
    fn a_non_monotone_ladder_is_caught_at_startup_not_after_a_self_play_run() {
        for (needle, patch, expect) in [
            ("reaction_ms: 150.0", "reaction_ms: 260.0", "reacts slower"),
            ("apm_cap: 360.0", "apm_cap: 100.0", "may act less often"),
            ("execution_noise: 0.05", "execution_noise: 0.9", "sloppier"),
        ] {
            let bad = LADDER.replace(needle, patch);
            let l = FighterBrainLadder::from_ron(&bad).unwrap();
            assert!(
                l.problems().iter().any(|p| p.contains(expect)),
                "patching `{needle}` -> `{patch}` should report `{expect}`: {:?}",
                l.problems()
            );
        }
    }

    /// The ladder's endpoints convert to the delay buffers §5 names: 150 ms → 9
    /// ticks at 60 Hz, 500 ms → 30. And every rung's buffer is a REAL buffer, not
    /// a pass-through.
    #[test]
    fn every_rung_gets_a_real_delay_buffer() {
        let l = ladder();
        assert_eq!(l.level(9).unwrap().delay(60.0).delay_ticks(), 9);
        assert_eq!(l.level(1).unwrap().delay(60.0).delay_ticks(), 30);
        for r in l.rungs() {
            assert!(
                r.delay(60.0).delay_ticks() > 0,
                "level {} would see the live world",
                r.level
            );
        }
    }

    /// A rung that reacts faster gets a shallower buffer. This is the reaction-time
    /// distribution check §3 asks for, in the form it can take before a rig exists:
    /// the ONLY thing that decides how late a brain sees the world is `reaction_ms`.
    #[test]
    fn the_buffer_depth_is_monotone_in_the_reaction_time() {
        let l = ladder();
        let depths: Vec<usize> = l
            .rungs()
            .iter()
            .map(|r| r.delay(60.0).delay_ticks())
            .collect();
        for w in depths.windows(2) {
            assert!(w[1] <= w[0], "depths not monotone: {depths:?}");
        }
        assert!(
            depths[0] > depths[8],
            "level 9 must react faster than level 1"
        );
    }

    /// **L3 is an upgrade, never a dependency** (§1). The whole shipped ladder runs
    /// with rollouts OFF, because N3.1's `restore` does not exist yet — and every
    /// rung still plays, on L2's scores alone.
    #[test]
    fn the_whole_ladder_degrades_gracefully_without_l3() {
        for r in ladder().rungs() {
            assert!(!r.uses_rollouts(), "level {} expects L3", r.level);
        }
    }
}
