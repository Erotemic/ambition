//! Stage 2 — broad mode choice.
//!
//! Converts the [`ObservationFrame`] into a single [`BroadMode`].
//! Sticky (hysteresis) so the brain doesn't oscillate when distance
//! hovers at a threshold — a chosen mode must dwell at least
//! [`MODE_MIN_DWELL_S`] before another mode can take over (except
//! for hard overrides like stun / out-of-range → Idle).

use super::observation::ObservationFrame;
use super::{SmashCfg, SmashState};

/// Top-level "what should I do right now" decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BroadMode {
    /// No active engagement — patrol / wait. Default.
    #[default]
    Idle,
    /// Close distance to the target.
    Approach,
    /// Create distance from the target (player too close).
    Retreat,
    /// In melee/range window — commit an attack.
    Engage,
    /// Anti-clump: too many allies stacked up; sidestep to spread
    /// out. Higher priority than Approach so a swarm visibly fans
    /// out rather than piling on.
    Reposition,
    /// Off-stage / suspended over a gap. Today a stub —
    /// `TerrainAwareness.off_stage` is always false until the
    /// snapshot builder learns about ledges.
    Recover,
}

/// Minimum time a mode must dwell before another mode can replace
/// it. Hard overrides (stun, target dead, target out of aggro)
/// bypass this gate.
pub const MODE_MIN_DWELL_S: f32 = 0.18;

/// Choose this tick's broad mode. Mutates `state` to track the
/// hysteresis window (via `mode_dwell_s`, which the caller in
/// `tick_smash` advances each tick by `snapshot.dt`).
pub fn choose_mode(obs: &ObservationFrame, cfg: &SmashCfg, state: &mut SmashState) -> BroadMode {
    // --- Hard overrides (bypass hysteresis) ---
    if !obs.self_alive || obs.stun_remaining > 0.0 {
        return commit(state, BroadMode::Idle);
    }
    if !obs.target_alive {
        return commit(state, BroadMode::Idle);
    }
    if obs.terrain.off_stage {
        return commit(state, BroadMode::Recover);
    }
    if obs.distance_to_target > cfg.aggro_radius {
        return commit(state, BroadMode::Idle);
    }

    // --- Candidate mode ---
    //
    // Priority order:
    //   1. Engage (in swing range — commit the attack regardless of
    //      crowding or body overlap; clumping at the moment of impact
    //      is fine and melee actors should not back away from a free
    //      point-blank swing)
    //   2. Retreat (too close to target, but outside the actual attack
    //      band — useful for non-melee / future defensive Smash configs)
    //   3. Reposition (crowded with a nearby ally — sidestep before
    //      converging)
    //   4. Engage hold band (in engage_distance but not attack_range)
    //   5. Approach (default — close the gap)
    //
    // Note: Engage outranks Retreat and Reposition so a provoked cove
    // pirate already in swing range doesn't maintain distance instead
    // of swinging. Ranged held-item actors should use Skirmisher /
    // Sniper brains; Smash is the grounded melee brawler policy.
    let candidate = if obs.distance_to_target <= cfg.attack_range && !obs.self_attacking {
        BroadMode::Engage
    } else if obs.distance_to_target < cfg.too_close_distance {
        BroadMode::Retreat
    } else if obs.crowding.pressure >= cfg.crowding_threshold {
        BroadMode::Reposition
    } else if obs.distance_to_target <= cfg.engage_distance {
        BroadMode::Engage
    } else {
        BroadMode::Approach
    };

    // --- Hysteresis: stick with last mode unless it's been at
    // least MODE_MIN_DWELL_S since we entered it. Bypassed when:
    //   - the candidate matches what we already chose (no-op),
    //   - the candidate is Engage (don't delay an attack), or
    //   - the current state is Idle (no real prior commitment, so
    //     a fresh actor entering aggro should be able to commit
    //     to Approach/Retreat/Reposition immediately rather than
    //     spend the dwell window standing still).
    if candidate == state.mode {
        return state.mode;
    }
    if state.mode == BroadMode::Idle {
        return commit(state, candidate);
    }
    if state.mode_dwell_s < MODE_MIN_DWELL_S && candidate != BroadMode::Engage {
        return state.mode;
    }
    commit(state, candidate)
}

fn commit(state: &mut SmashState, mode: BroadMode) -> BroadMode {
    if state.mode != mode {
        state.mode = mode;
        state.mode_dwell_s = 0.0;
    }
    mode
}

#[cfg(test)]
mod tests {
    use super::super::observation::CrowdingSignal;
    use super::*;
    use crate::engine_core as ae;

    fn obs_at(distance_x: f32) -> ObservationFrame {
        ObservationFrame {
            self_pos: ae::Vec2::ZERO,
            self_vel: ae::Vec2::ZERO,
            self_facing: 1.0,
            self_on_ground: true,
            self_alive: true,
            self_attacking: false,
            self_air_jumps_remaining: 0,
            attack_cooldown_remaining: 0.0,
            stun_remaining: 0.0,
            target_pos: ae::Vec2::new(distance_x, 0.0),
            target_alive: true,
            to_target_x: distance_x,
            to_target_y: 0.0,
            distance_to_target: distance_x.abs(),
            crowding: CrowdingSignal::default(),
            terrain: Default::default(),
            sim_time: 1.0,
            dt: 1.0 / 60.0,
        }
    }

    #[test]
    fn idles_outside_aggro() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let obs = obs_at(2000.0);
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Idle);
    }

    #[test]
    fn approaches_at_long_range() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let obs = obs_at(300.0); // outside engage_distance (70)
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Approach);
    }

    #[test]
    fn engages_inside_attack_range() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let obs = obs_at(40.0); // inside attack_range (56)
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Engage);
    }

    #[test]
    fn engages_when_point_blank_and_in_attack_range() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let obs = obs_at(20.0); // inside both too_close (30) and attack_range (56)
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Engage);
    }

    #[test]
    fn retreats_when_too_close_but_not_in_attack_range() {
        let mut cfg = SmashCfg::STRIKER_DEFAULT;
        cfg.attack_range = 10.0;
        cfg.too_close_distance = 30.0;
        let mut state = SmashState::default();
        let obs = obs_at(20.0);
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Retreat);
    }

    #[test]
    fn repositions_when_crowded() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let mut obs = obs_at(300.0);
        obs.crowding.pressure = 1.0; // way over 0.65 threshold
        obs.crowding.away_dir = ae::Vec2::new(-1.0, 0.0);
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Reposition,);
    }

    #[test]
    fn hysteresis_prevents_approach_to_retreat_flip_within_dwell() {
        // Pin the contract: once committed to Approach, a brief dip
        // below `too_close_distance` (into the Retreat band, NOT swing
        // range) should not immediately flip to Retreat. Engage is the
        // only candidate that bypasses dwell. STRIKER_DEFAULT keeps
        // `too_close_distance` inside `attack_range` so a point-blank
        // pirate engages instead of retreating, which leaves no Retreat
        // band — so carve an explicit one the way
        // `retreats_when_too_close_but_not_in_attack_range` does.
        let mut cfg = SmashCfg::STRIKER_DEFAULT;
        cfg.attack_range = 10.0;
        cfg.too_close_distance = 30.0;
        let mut state = SmashState::default();
        let obs = obs_at(300.0);
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Approach);
        // Simulate caller's per-tick `mode_dwell_s += dt` ahead of
        // the second call: 0.1 s < MODE_MIN_DWELL_S = 0.18.
        state.mode_dwell_s = 0.1;
        let obs2 = obs_at(20.0);
        let chosen = choose_mode(&obs2, &cfg, &mut state);
        assert_eq!(chosen, BroadMode::Approach, "should stick within dwell");
    }

    #[test]
    fn hysteresis_does_not_block_engage_transition() {
        // Engage is a hard exit from the dwell window — the brain
        // shouldn't waste a strike opportunity for hysteresis.
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let _ = choose_mode(&obs_at(300.0), &cfg, &mut state);
        assert_eq!(state.mode, BroadMode::Approach);
        state.mode_dwell_s = 0.05;
        let chosen = choose_mode(&obs_at(40.0), &cfg, &mut state);
        assert_eq!(chosen, BroadMode::Engage);
    }

    #[test]
    fn stun_forces_idle() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let mut obs = obs_at(40.0); // would be Engage
        obs.stun_remaining = 0.5;
        assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Idle);
    }
}
