//! Smash-brawl brain template — SSBM Subspace-Emissary feel.
//!
//! Each tick the brain runs a 5-stage pipeline:
//!
//! 1. **Observe**: snapshot the world into an [`ObservationFrame`]
//!    (self + target + stage + crowding + hazards).
//! 2. **Choose broad mode**: pick a [`BroadMode`] (Approach / Retreat
//!    / Engage / Reposition / Recover / Idle) with hysteresis so the
//!    actor doesn't oscillate.
//! 3. **Choose specific action**: pick a [`SpecificAction`] from the
//!    mode's allowed vocabulary, gated by the actor's [`ActionSet`]
//!    capability mask.
//! 4. **Apply difficulty filter**: reaction delay, commit
//!    probability, aim accuracy. Easier enemies "see late" and drop
//!    actions; harder enemies commit + aim cleanly.
//! 5. **Emit inputs**: translate the action into an
//!    [`crate::actor_control::ActorControlFrame`] the integration pipeline consumes.
//!
//! Every stage is a pure function of the previous one's output plus
//! [`SmashCfg`] / [`SmashState`]. This makes the pipeline trivially
//! unit-testable and keeps the brain backend swappable — a future
//! RL policy can replace any single stage without touching the
//! others.

use super::action_set::ActionSet;
use super::snapshot::BrainSnapshot;
#[cfg(test)]
use crate::engine_core as ae;

pub mod action;
pub mod difficulty;
pub mod emit;
pub mod mode;
pub mod observation;

pub use action::{choose_action, SpecificAction};
pub use difficulty::{apply_difficulty, DifficultyProfile};
pub use emit::emit_inputs;
pub use mode::{choose_mode, BroadMode};
pub use observation::{observe, CrowdingSignal, ObservationFrame, TerrainAwareness};

/// Tuning knobs for a [`StateMachineCfg::Smash`] brain. Per-actor
/// state lives in [`SmashState`]. Designer-facing today — eventually
/// migrates to data so per-archetype variants live in
/// `enemy_archetypes.ron`.
#[derive(Clone, Copy, Debug)]
pub struct SmashCfg {
    /// Maximum sensing distance (px). Outside this radius the brain
    /// idles regardless of target presence.
    pub aggro_radius: f32,
    /// Distance the brain tries to settle at while in `Engage`.
    /// Slightly outside `attack_range` so the actor has room to
    /// burst forward into an attack.
    pub engage_distance: f32,
    /// Concrete melee attack range (px). When the target is closer
    /// than this AND the actor has melee capability, `Engage` emits
    /// a melee attempt. Authoritative — replaces the old hardcoded
    /// melee-engage range.
    pub attack_range: f32,
    /// Distance below which the actor retreats to avoid being
    /// pinned against a wall by the target.
    pub too_close_distance: f32,
    /// Movement speed while in Approach / Chase (px/s).
    pub chase_speed: f32,
    /// Movement speed while in Retreat / Reposition (px/s).
    pub retreat_speed: f32,
    /// Crowding pressure (from same-faction allies) that triggers
    /// `Reposition` mode. `0.0` disables.
    pub crowding_threshold: f32,
    /// Difficulty profile applied at stage 4.
    pub difficulty: DifficultyProfile,
}

impl SmashCfg {
    /// "Standard melee striker" tuning — humanoid grunt that
    /// approaches, swings, and steps back. Used by MediumStriker,
    /// SmallSkitter, SmallLurker, PirateRaider.
    pub const STRIKER_DEFAULT: Self = Self {
        aggro_radius: 460.0,
        engage_distance: 70.0,
        attack_range: 56.0,
        too_close_distance: 30.0,
        chase_speed: 170.0,
        retreat_speed: 130.0,
        crowding_threshold: 0.65,
        difficulty: DifficultyProfile::MEDIUM,
    };
    /// Heavy brute tuning — slower, longer reach, less retreat.
    pub const BRUTE_DEFAULT: Self = Self {
        aggro_radius: 380.0,
        engage_distance: 90.0,
        attack_range: 70.0,
        too_close_distance: 24.0,
        chase_speed: 118.0,
        retreat_speed: 80.0,
        crowding_threshold: 0.55,
        difficulty: DifficultyProfile::MEDIUM,
    };
}

/// Per-actor runtime state for the Smash brain.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmashState {
    /// Mode active last tick. Used by the hysteresis check in
    /// `choose_mode` so the brain doesn't flip Approach⇄Retreat
    /// when distance hovers at the threshold.
    pub mode: BroadMode,
    /// Seconds the current mode has been active. Incremented each
    /// tick from `snapshot.dt`; reset to 0 on mode change. Compared
    /// against `MODE_MIN_DWELL_S` for hysteresis.
    pub mode_dwell_s: f32,
    /// Random seed for difficulty jitter (commit probability,
    /// reaction delay variance). Set once at first tick from the
    /// actor id; survives reset_to_spawn via spawn-time init.
    pub rng_seed: u64,
}

/// Tick the Smash brain pipeline. Pure function modulo `state`
/// (which the difficulty stage mutates for its RNG advance + the
/// mode stage mutates for hysteresis bookkeeping).
pub fn tick_smash(
    cfg: &SmashCfg,
    state: &mut SmashState,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor_control::ActorControlFrame,
) {
    *out = crate::actor_control::ActorControlFrame::neutral();
    if !snapshot.alive {
        state.mode = BroadMode::Idle;
        return;
    }
    // Advance the dwell accumulator before any mode-flip check.
    state.mode_dwell_s += snapshot.dt;
    let obs = observe(snapshot);
    let mode = choose_mode(&obs, cfg, state);
    let action = choose_action(&obs, mode, cfg, actions);
    let action = apply_difficulty(action, &cfg.difficulty, state);
    emit_inputs(action, &obs, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap_with_target_at_x(target_x: f32) -> BrainSnapshot {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = ae::Vec2::new(0.0, 0.0);
        s.target_pos = ae::Vec2::new(target_x, 0.0);
        s.actor_on_ground = true;
        s.target_alive = true;
        s
    }

    #[test]
    fn idles_when_target_out_of_range() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let snap = snap_with_target_at_x(2000.0);
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert_eq!(
            frame.desired_vel.x, 0.0,
            "actor outside aggro_radius should not move"
        );
        assert!(!frame.melee_pressed);
    }

    #[test]
    fn approaches_when_target_in_aggro_but_out_of_attack() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        // Target at 300 px — inside aggro (460), outside engage (70).
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(
            frame.desired_vel.x > 0.0,
            "actor should approach a target to its right; got vel={:?}",
            frame.desired_vel,
        );
    }

    #[test]
    fn dead_actor_emits_neutral_frame() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let mut snap = snap_with_target_at_x(100.0);
        snap.alive = false;
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        // Pre-poison: if `tick_smash` early-returns without writing,
        // the assertion below would catch a leak from the caller's
        // pre-existing frame state.
        frame.melee_pressed = true;
        frame.desired_vel = ae::Vec2::new(999.0, 999.0);
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(!frame.melee_pressed, "dead actor must not emit melee");
        assert_eq!(frame.desired_vel, ae::Vec2::ZERO);
    }
}
