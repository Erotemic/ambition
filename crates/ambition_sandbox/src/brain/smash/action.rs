//! Stage 3 — specific action choice.
//!
//! Given a [`BroadMode`] + the actor's capability mask
//! ([`ActionSet`]), pick a concrete [`SpecificAction`] to commit
//! this tick. Actions whose required capability is absent from the
//! ActionSet are silently dropped (the actor falls through to a
//! safer alternative — e.g. a slug with no jump just walks where a
//! goblin would jump).

use ambition_engine as ae;

use super::super::action_set::ActionSet;
use super::mode::BroadMode;
use super::observation::ObservationFrame;
use super::SmashCfg;

/// Local replacement for the `SignumOr` trait that lives in
/// `content::features::util` with restrictive visibility. The brain
/// module can't see that trait, so we inline the one-line helper.
fn signum_or(x: f32, fallback: f32) -> f32 {
    if x.abs() < 0.001 {
        fallback
    } else {
        x.signum()
    }
}

/// Concrete action the actor will commit this tick. Each variant
/// carries the parameters [`emit_inputs`] needs to translate into
/// an `ActorControlFrame`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    dead_code,
    reason = "vocab seeded ahead of consumer wiring; smash brain unlocks variants gradually"
)]
pub enum SpecificAction {
    /// Hold position; emit neutral movement.
    Idle,
    /// Walk along the x-axis. `dir` is signed `[-1, 1]`.
    Walk { dir: f32 },
    /// Dash burst — same direction as `Walk` but at higher speed.
    Dash { dir: f32 },
    /// Press jump (single press edge). Vertical motion handled by
    /// the player-side physics; the brain just emits the edge.
    Jump,
    /// Press jump while already airborne (double jump). Falls back
    /// to a regular jump if the actor isn't off the ground; the
    /// integration layer will ignore it.
    DoubleJump,
    /// Spawn the actor's melee attack in `dir`. `dir` axis-aligned;
    /// `(1, 0)` = forward swing, `(-1, 0)` = back-air, `(0, -1)` =
    /// up-tilt, etc. The ActionSet's `MeleeActionSpec` decides the
    /// concrete swing shape; the brain just commits intent.
    MeleeAttack { dir: ae::Vec2 },
    /// Spawn a ranged projectile in `dir`. Requires ActionSet
    /// `.ranged.is_some()`.
    RangedAttack { dir: ae::Vec2 },
    /// Trigger the actor's special. Resolved by the actor's
    /// `SpecialActionSpec`.
    Special,
    /// Shield (player-only today). Reserved.
    Shield,
    /// Spot/air dodge in `dir`. Reserved.
    Dodge { dir: ae::Vec2 },
}

/// Pick the action for the chosen mode. Pure function of `obs +
/// mode + cfg + actions`. The brain's randomized / time-gated
/// modulation lives in stage 4 (difficulty).
pub fn choose_action(
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    actions: &ActionSet,
) -> SpecificAction {
    if obs.self_attacking {
        // Mid-swing: the brain commits to letting the swing finish.
        // Movement during windup/active/recover is owned by the
        // ActionSet's animation timing; the brain emits neutral
        // intent.
        return SpecificAction::Idle;
    }
    match mode {
        BroadMode::Idle => SpecificAction::Idle,
        BroadMode::Approach => {
            let dir = signum_or(obs.to_target_x, obs.self_facing);
            SpecificAction::Walk { dir: dir * (cfg.chase_speed / cfg.chase_speed.max(1.0)) }
        }
        BroadMode::Retreat => {
            // Move directly away from target.
            let dir = signum_or(-obs.to_target_x, -obs.self_facing);
            SpecificAction::Walk { dir }
        }
        BroadMode::Engage => {
            // Inside the engage band — commit a melee swing if we
            // have one and the cooldown is clear; otherwise hold
            // position and look threatening.
            if actions.melee.is_some()
                && obs.distance_to_target <= cfg.attack_range
                && obs.attack_cooldown_remaining <= 0.0
            {
                // Forward swing along the target axis. The ActionSet
                // owns the actual hitbox shape; the brain just
                // commits the axis.
                let axis_x = signum_or(obs.to_target_x, obs.self_facing);
                return SpecificAction::MeleeAttack {
                    dir: ae::Vec2::new(axis_x, 0.0),
                };
            }
            // Out of swing range or on cooldown — close the rest of
            // the way at chase speed.
            if obs.distance_to_target > cfg.attack_range {
                let dir = signum_or(obs.to_target_x, obs.self_facing);
                return SpecificAction::Walk { dir };
            }
            // In range but on cooldown — hold ground, face target.
            SpecificAction::Idle
        }
        BroadMode::Reposition => {
            // Anti-clump: sidestep along the crowding `away_dir`.
            // Fall back to a perpendicular-to-target movement if
            // the crowd signal didn't pick a direction.
            let dir = if obs.crowding.away_dir.length_squared() > 0.05 {
                signum_or(obs.crowding.away_dir.x, 0.0)
            } else {
                // Pick "away from the player" as a tie-breaker so a
                // single isolated actor still spreads in a sensible
                // direction.
                signum_or(-obs.to_target_x, 0.0)
            };
            // If we have a `Dash` available (ActionSet doesn't model
            // it yet) we'd use it here under severe crowding. For
            // now, just walk away.
            SpecificAction::Walk { dir }
        }
        BroadMode::Recover => {
            // Stub: walk toward the target's x as a "return to
            // stage" pseudo-recovery until ledge data is wired.
            let dir = signum_or(obs.to_target_x, 0.0);
            SpecificAction::Walk { dir }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observation::CrowdingSignal;
    use super::super::SmashCfg;
    use super::*;
    use crate::brain::action_set::{MeleeActionSpec, SwipeSpec};

    fn obs_at(distance_x: f32, attacking: bool) -> ObservationFrame {
        ObservationFrame {
            self_pos: ae::Vec2::ZERO,
            self_vel: ae::Vec2::ZERO,
            self_facing: 1.0,
            self_on_ground: true,
            self_alive: true,
            self_attacking: attacking,
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
    fn approach_picks_walk_toward_target() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet::peaceful();
        let act = choose_action(&obs_at(300.0, false), BroadMode::Approach, &cfg, &actions);
        match act {
            SpecificAction::Walk { dir } => assert!(dir > 0.0),
            other => panic!("expected Walk, got {other:?}"),
        }
        let act = choose_action(&obs_at(-300.0, false), BroadMode::Approach, &cfg, &actions);
        match act {
            SpecificAction::Walk { dir } => assert!(dir < 0.0),
            other => panic!("expected Walk left, got {other:?}"),
        }
    }

    #[test]
    fn engage_with_melee_in_range_emits_attack() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let act = choose_action(&obs_at(40.0, false), BroadMode::Engage, &cfg, &actions);
        assert!(
            matches!(act, SpecificAction::MeleeAttack { .. }),
            "got {act:?}"
        );
    }

    #[test]
    fn engage_without_melee_capability_does_not_attack() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet::peaceful(); // no melee
        let act = choose_action(&obs_at(40.0, false), BroadMode::Engage, &cfg, &actions);
        assert!(!matches!(act, SpecificAction::MeleeAttack { .. }));
    }

    #[test]
    fn engage_on_cooldown_holds_instead_of_attacking() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let mut obs = obs_at(40.0, false);
        obs.attack_cooldown_remaining = 0.5;
        let act = choose_action(&obs, BroadMode::Engage, &cfg, &actions);
        assert_eq!(act, SpecificAction::Idle, "got {act:?}");
    }

    #[test]
    fn retreat_walks_away_from_target() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet::peaceful();
        // Target to the right (positive x) → retreat = walk left.
        let act = choose_action(&obs_at(20.0, false), BroadMode::Retreat, &cfg, &actions);
        match act {
            SpecificAction::Walk { dir } => assert!(dir < 0.0),
            other => panic!("expected Walk left, got {other:?}"),
        }
    }

    #[test]
    fn reposition_uses_away_dir() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet::peaceful();
        let mut obs = obs_at(300.0, false);
        obs.crowding.away_dir = ae::Vec2::new(-1.0, 0.0); // crowd is to the right; we should sidestep left
        let act = choose_action(&obs, BroadMode::Reposition, &cfg, &actions);
        match act {
            SpecificAction::Walk { dir } => assert!(dir < 0.0, "got {dir}"),
            other => panic!("expected Walk, got {other:?}"),
        }
    }

    #[test]
    fn mid_swing_emits_idle_regardless_of_mode() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let obs = obs_at(40.0, true); // self_attacking = true
        for mode in [
            BroadMode::Approach,
            BroadMode::Retreat,
            BroadMode::Engage,
            BroadMode::Reposition,
        ] {
            let act = choose_action(&obs, mode, &cfg, &actions);
            assert_eq!(act, SpecificAction::Idle, "mode={mode:?}");
        }
    }
}
