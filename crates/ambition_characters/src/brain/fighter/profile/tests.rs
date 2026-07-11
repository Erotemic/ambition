//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
