//! Stage 4 — difficulty filter.
//!
//! Modulates the chosen [`SpecificAction`] based on the actor's
//! [`DifficultyProfile`]:
//!
//! - `reaction_delay_s` (informational; consumed by the observation
//!   builder, not here)
//! - `commit_probability` — dice roll against a per-actor RNG seed;
//!   on failure the chosen action degrades to `Idle`. Lets easier
//!   enemies "think about it" without committing every tick.
//! - `accuracy` — jitters the aim vector on `MeleeAttack` /
//!   `RangedAttack`. `1.0` = no jitter; `0.0` = uniform-random.
//! - `mash_speed_hz` (informational; the EFFECTS-stage cooldown
//!   gate is the authority for actual swing rate).

use crate::engine_core as ae;

use super::action::SpecificAction;
use super::SmashState;

/// Per-actor difficulty tuning. Authored today via
/// [`SmashCfg::difficulty`]; an upcoming pass lifts this into
/// `enemy_archetypes.ron` so designers can tune per-archetype
/// without code edits.
#[derive(Clone, Copy, Debug)]
pub struct DifficultyProfile {
    /// Seconds of observation lag — handled by the snapshot
    /// builder, not the filter. Carried here for completeness so
    /// downstream visualization can read one struct.
    #[allow(dead_code, reason = "reaction delay surfaces upstream of the filter today")]
    pub reaction_delay_s: f32,
    /// `[0.0, 1.0]` — probability of committing the chosen action
    /// this tick. Lower = drops more attempts to Idle.
    pub commit_probability: f32,
    /// `[0.0, 1.0]` — `1.0` = no aim jitter; lower values jitter
    /// the attack axis proportionally. Applied to MeleeAttack /
    /// RangedAttack only.
    pub accuracy: f32,
    /// Hz — informational, for downstream cooldown / mashing
    /// systems to consult.
    #[allow(dead_code, reason = "consumer lives in the EFFECTS-stage cooldown gate")]
    pub mash_speed_hz: f32,
}

impl DifficultyProfile {
    pub const EASY: Self = Self {
        reaction_delay_s: 0.30,
        commit_probability: 0.55,
        accuracy: 0.65,
        mash_speed_hz: 1.0,
    };
    pub const MEDIUM: Self = Self {
        reaction_delay_s: 0.15,
        commit_probability: 0.85,
        accuracy: 0.85,
        mash_speed_hz: 1.4,
    };
    pub const HARD: Self = Self {
        reaction_delay_s: 0.05,
        commit_probability: 0.98,
        accuracy: 0.98,
        mash_speed_hz: 2.0,
    };
}

/// Apply the difficulty filter to a chosen action. Mutates the
/// actor's RNG seed so consecutive ticks produce different rolls.
pub fn apply_difficulty(
    action: SpecificAction,
    profile: &DifficultyProfile,
    state: &mut SmashState,
) -> SpecificAction {
    // Idle / movement-only actions skip the filter — there's nothing
    // to drop or jitter, and movement should always commit so the
    // actor doesn't visibly freeze mid-step.
    let is_action = matches!(
        action,
        SpecificAction::MeleeAttack { .. }
            | SpecificAction::RangedAttack { .. }
            | SpecificAction::Special
            | SpecificAction::Jump
            | SpecificAction::DoubleJump
            | SpecificAction::Dodge { .. }
            | SpecificAction::Shield
    );
    if !is_action {
        return action;
    }
    // --- Commit roll ---
    if profile.commit_probability < 1.0 {
        let roll = roll_unit(state);
        if roll > profile.commit_probability {
            return SpecificAction::Idle;
        }
    }
    // --- Accuracy jitter (aim vectors only) ---
    if profile.accuracy < 1.0 {
        match action {
            SpecificAction::MeleeAttack { dir } => SpecificAction::MeleeAttack {
                dir: jitter_dir(dir, profile.accuracy, state),
            },
            SpecificAction::RangedAttack { dir } => SpecificAction::RangedAttack {
                dir: jitter_dir(dir, profile.accuracy, state),
            },
            other => other,
        }
    } else {
        action
    }
}

/// Tiny LCG roll in `[0, 1)`. Stateful via `SmashState.rng_seed` so
/// each actor has an independent stream and ticks advance the seed
/// deterministically (replay-safe).
fn roll_unit(state: &mut SmashState) -> f32 {
    if state.rng_seed == 0 {
        // Seed-zero fallback: avoid the LCG fixed point. The driver
        // system pre-seeds from the actor id; this branch only
        // fires for state built with `default()`.
        state.rng_seed = 0xa5a5a5a5;
    }
    // Numerical Recipes LCG constants.
    state.rng_seed = state.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
    let n = (state.rng_seed >> 33) as u32; // top 31 bits
    (n as f32) / (u32::MAX >> 1) as f32
}

/// Apply uniform-random angular jitter to an axis-aligned aim
/// vector. `accuracy = 1.0` returns `dir` unchanged; smaller values
/// rotate the vector by up to `(1 - accuracy) * π/4` radians in
/// either direction.
fn jitter_dir(dir: ae::Vec2, accuracy: f32, state: &mut SmashState) -> ae::Vec2 {
    if dir.length_squared() < 1e-6 {
        return dir;
    }
    let max_angle = (1.0 - accuracy.clamp(0.0, 1.0)) * std::f32::consts::FRAC_PI_4;
    let roll = roll_unit(state) * 2.0 - 1.0; // -1 .. 1
    let angle = roll * max_angle;
    let (sin, cos) = angle.sin_cos();
    ae::Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_actions_skip_filter() {
        let mut state = SmashState {
            rng_seed: 42,
            ..Default::default()
        };
        let act = apply_difficulty(
            SpecificAction::Walk { dir: 1.0 },
            &DifficultyProfile::EASY,
            &mut state,
        );
        assert_eq!(act, SpecificAction::Walk { dir: 1.0 });
    }

    #[test]
    fn hard_difficulty_commits_attacks_almost_always() {
        let profile = DifficultyProfile::HARD;
        let mut committed = 0;
        let mut state = SmashState {
            rng_seed: 12345,
            ..Default::default()
        };
        for _ in 0..200 {
            let act = apply_difficulty(
                SpecificAction::MeleeAttack {
                    dir: ae::Vec2::new(1.0, 0.0),
                },
                &profile,
                &mut state,
            );
            if matches!(act, SpecificAction::MeleeAttack { .. }) {
                committed += 1;
            }
        }
        // 0.98 commit probability → ~196/200. Allow some slack.
        assert!(committed >= 180, "got {committed}/200 commits on HARD");
    }

    #[test]
    fn easy_difficulty_drops_some_attacks() {
        let profile = DifficultyProfile::EASY;
        let mut dropped = 0;
        let mut state = SmashState {
            rng_seed: 99,
            ..Default::default()
        };
        for _ in 0..200 {
            let act = apply_difficulty(
                SpecificAction::MeleeAttack {
                    dir: ae::Vec2::new(1.0, 0.0),
                },
                &profile,
                &mut state,
            );
            if matches!(act, SpecificAction::Idle) {
                dropped += 1;
            }
        }
        // 0.55 commit probability → ~45% drops. Allow slack.
        assert!(
            dropped > 40 && dropped < 150,
            "got {dropped}/200 drops on EASY"
        );
    }

    #[test]
    fn perfect_accuracy_preserves_aim_vector() {
        let profile = DifficultyProfile {
            accuracy: 1.0,
            commit_probability: 1.0,
            ..DifficultyProfile::HARD
        };
        let mut state = SmashState {
            rng_seed: 42,
            ..Default::default()
        };
        let act = apply_difficulty(
            SpecificAction::MeleeAttack {
                dir: ae::Vec2::new(1.0, 0.0),
            },
            &profile,
            &mut state,
        );
        match act {
            SpecificAction::MeleeAttack { dir } => {
                assert!((dir.x - 1.0).abs() < 1e-3);
                assert!(dir.y.abs() < 1e-3);
            }
            other => panic!("got {other:?}"),
        }
    }
}
