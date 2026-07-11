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
        // A relentless fighter (committed duelist) never disengages while its foe
        // lives — it CHASES at distance instead of idling out, so the bout can't go
        // inert just because the two drifted (or were flung by gravity) apart. An
        // ambient enemy idles as before once the target leaves its sensing radius.
        return commit(
            state,
            if cfg.relentless {
                BroadMode::Approach
            } else {
                BroadMode::Idle
            },
        );
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
mod tests;
