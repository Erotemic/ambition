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
// `ae` is used both by `maybe_substitute_ranged` (the ranged-verb emit) and the
// tests, so the import is no longer test-gated.
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
    /// Seconds until the actor's *ranged* verb is off cooldown. The
    /// shared projectile pool doesn't rate-limit enemy fire, so the
    /// ranged cadence lives here in brain state: decremented each
    /// tick, gated against `> 0`, and reset to [`RANGED_COOLDOWN_S`]
    /// when the brain commits a ranged shot. Melee keeps using the
    /// integration-side attack cooldown (`attack_cooldown_remaining`).
    pub ranged_cooldown_remaining: f32,
}

/// Ranged-verb cadence (seconds). A ranged-capable Smash actor fires
/// at most once per this interval at mid-range, then closes for the
/// melee finish. Module-level for now — promote to [`SmashCfg`] if
/// archetypes ever want distinct ranged tempos.
const RANGED_COOLDOWN_S: f32 = 1.1;

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
    // Tick the ranged cadence down (clamped at 0) before this frame's
    // verb selection can re-arm it.
    state.ranged_cooldown_remaining = (state.ranged_cooldown_remaining - snapshot.dt).max(0.0);
    let obs = observe(snapshot);
    let mode = choose_mode(&obs, cfg, state);
    let action = choose_action(&obs, mode, cfg, actions);
    // Verb selection by range (the player/enemy unification flex): a
    // ranged-capable actor closing on a mid-range target fires ranged
    // on its own cadence before committing to the melee finish.
    // Substituted *before* difficulty so the shot inherits the same
    // accuracy jitter / commit roll as a melee swing.
    let action = maybe_substitute_ranged(action, &obs, mode, cfg, actions, state);
    let action = apply_difficulty(action, &cfg.difficulty, state);
    emit_inputs(action, &obs, out);
}

/// Replace a *closing* action (`Walk`/`Idle` toward the target) with a
/// ranged shot when the actor has a ranged verb, is at mid-range
/// (inside aggro, outside melee reach), is approaching/holding (not
/// retreating), isn't mid-swing, and the ranged cadence is ready.
/// Re-arms the cadence on commit. Melee swings already in reach and
/// retreats are never overridden — the actor still closes for the
/// melee finish once the shot lands.
fn maybe_substitute_ranged(
    action: SpecificAction,
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    actions: &ActionSet,
    state: &mut SmashState,
) -> SpecificAction {
    if actions.ranged.is_none()
        || obs.self_attacking
        || state.ranged_cooldown_remaining > 0.0
    {
        return action;
    }
    let closing = matches!(action, SpecificAction::Walk { .. } | SpecificAction::Idle);
    let approaching = matches!(mode, BroadMode::Approach | BroadMode::Engage);
    let in_band = obs.distance_to_target > cfg.attack_range
        && obs.distance_to_target <= cfg.aggro_radius;
    if !(closing && approaching && in_band) {
        return action;
    }
    state.ranged_cooldown_remaining = RANGED_COOLDOWN_S;
    let dir_x = if obs.to_target_x.abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_x.signum()
    };
    SpecificAction::RangedAttack {
        dir: ae::Vec2::new(dir_x, 0.0),
    }
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
    fn melee_smash_swings_when_target_is_point_blank() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet {
            melee: Some(crate::brain::MeleeActionSpec::Swipe(
                crate::brain::SwipeSpec::STRIKER_DEFAULT,
            )),
            ..ActionSet::peaceful()
        };
        // 20px is inside STRIKER_DEFAULT.too_close_distance, but a
        // melee-capable Smash actor should take the point-blank swing
        // instead of backing away forever. This pins the cove-pirate
        // regression where provoked NPCs approached, then held range
        // without ever swinging when the player was beside them.
        let snap = snap_with_target_at_x(20.0);
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(frame.melee_pressed, "point-blank melee actor should swing");
    }

    /// Difficulty profile that always commits and never jitters, so the
    /// ranged-cadence tests are deterministic regardless of rng seed.
    fn crisp_striker_cfg() -> SmashCfg {
        SmashCfg {
            difficulty: DifficultyProfile {
                reaction_delay_s: 0.0,
                commit_probability: 1.0,
                accuracy: 1.0,
                ..DifficultyProfile::HARD
            },
            ..SmashCfg::STRIKER_DEFAULT
        }
    }

    fn ranged_actions() -> ActionSet {
        ActionSet {
            ranged: Some(crate::brain::RangedActionSpec::Rock {
                speed: 300.0,
                damage: 2,
            }),
            ..ActionSet::peaceful()
        }
    }

    #[test]
    fn ranged_capable_actor_fires_at_mid_range() {
        // A Smash actor with a ranged verb, at mid-range (inside aggro
        // 460, outside melee reach 56), fires ranged rather than silently
        // walking closer — the player/enemy "verb selection by range" flex.
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ranged_actions();
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(frame.fire.is_some(), "ranged actor should fire at mid-range");
        assert!(!frame.melee_pressed, "should not also melee at mid-range");
        assert!(
            state.ranged_cooldown_remaining > 0.0,
            "ranged cadence armed after firing"
        );
    }

    #[test]
    fn melee_takes_precedence_over_ranged_in_reach() {
        // With BOTH verbs, a point-blank target gets the melee swing,
        // not a ranged shot — ranged only substitutes for *closing*
        // actions outside melee range.
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ActionSet {
            melee: Some(crate::brain::MeleeActionSpec::Swipe(
                crate::brain::SwipeSpec::STRIKER_DEFAULT,
            )),
            ..ranged_actions()
        };
        let snap = snap_with_target_at_x(20.0); // inside attack_range
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(frame.melee_pressed, "in-reach actor swings");
        assert!(frame.fire.is_none(), "does not fire ranged in melee reach");
    }

    #[test]
    fn ranged_cadence_gates_back_to_back_shots() {
        // Immediately after a shot the cadence blocks another; the actor
        // closes (walks) instead. Once the cooldown elapses it fires again.
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ranged_actions();
        let mut snap = snap_with_target_at_x(300.0);
        snap.dt = 0.2;

        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame);
        assert!(frame.fire.is_some(), "first tick fires");

        let mut frame2 = crate::actor_control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, &mut frame2);
        assert!(frame2.fire.is_none(), "still on cooldown → no second shot");
        assert!(
            frame2.desired_vel.x > 0.0,
            "closes toward target while the ranged verb reloads"
        );

        // Advance past the cadence; it fires again.
        let mut fired_again = false;
        for _ in 0..((RANGED_COOLDOWN_S / snap.dt) as usize + 2) {
            let mut f = crate::actor_control::ActorControlFrame::neutral();
            tick_smash(&cfg, &mut state, &actions, &snap, &mut f);
            if f.fire.is_some() {
                fired_again = true;
                break;
            }
        }
        assert!(fired_again, "fires again once the cadence elapses");
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
