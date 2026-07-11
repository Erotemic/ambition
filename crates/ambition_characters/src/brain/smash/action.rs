//! Stage 3 — specific action choice.
//!
//! Given a [`BroadMode`] + the actor's capability mask
//! ([`ActionSet`]), pick a concrete [`SpecificAction`] to commit
//! this tick. Actions whose required capability is absent from the
//! ActionSet are silently dropped (the actor falls through to a
//! safer alternative — e.g. a slug with no jump just walks where a
//! goblin would jump).

use ambition_engine_core as ae;

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
            // Vertical gap: jump to close the vertical distance when the target
            // is meaningfully above (against gravity). Framed against local
            // gravity (`to_target_up`), not screen `-y`, so it is correct under
            // any orientation (I10). Trigger on the ground for the first jump, OR
            // mid-air if a double-jump is still available AND we're not already
            // rising fast (`self_vel_up < 50`). The rising-gate prevents the
            // brain from double-jumping mid-ascent and wasting the jump.
            if obs.to_target_up() > cfg.vertical_chase_min {
                if obs.self_on_ground {
                    return SpecificAction::Jump;
                }
                if obs.self_air_jumps_remaining > 0 && obs.self_vel_up() < 50.0 {
                    return SpecificAction::DoubleJump;
                }
            }
            // Run direction along the gravity-perpendicular SIDE axis (the body
            // consumes `locomotion.x` as a local-side scalar), so the actor walks
            // toward the target under ANY gravity orientation — not just screen-down.
            // Byte-identical to `to_target_x` under screen-down gravity. Uses the
            // HELD facing inside the alignment deadzone, so the run sign doesn't
            // rapid-flip when the target stacks on the gravity axis (e.g. a foe
            // airborne directly up-gravity) — the per-frame side-to-side flip fix.
            let dir = obs.side_face_toward_target();
            SpecificAction::Walk {
                dir: dir * (cfg.chase_speed / cfg.chase_speed.max(1.0)),
            }
        }
        BroadMode::Retreat => {
            // Move directly away from target, along the local side axis (I10).
            let dir = -obs.side_face_toward_target();
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
                // Directional pick, framed against local gravity (I10):
                //   - Target meaningfully above me: up-attack (toward `up_axis`).
                //   - I'm above the target, airborne: down-air (toward `down`).
                //   - Otherwise: forward swing along the gravity-perpendicular
                //     side axis toward the target.
                // `to_target_up > 0` ⇒ target above; `< 0` ⇒ target below.
                let up_amt = obs.to_target_up();
                let i_am_above_target = up_amt < -28.0 && !obs.self_on_ground;
                let target_above = up_amt > 28.0;
                if i_am_above_target {
                    return SpecificAction::MeleeAttack { dir: obs.down };
                }
                if target_above {
                    return SpecificAction::MeleeAttack { dir: obs.up_axis() };
                }
                // A melee swing aims at the foe's ACTUAL side (tight deadzone), even
                // at point-blank cross-up range inside the facing/run alignment band —
                // otherwise the swing would face the held direction and whiff when the
                // foe crosses to the other side. Facing can hold; the strike tracks.
                let toward_side = signum_or(obs.to_target_side(), obs.self_facing);
                return SpecificAction::MeleeAttack {
                    dir: obs.side_axis() * toward_side,
                };
            }
            // Out of swing range or on cooldown — close the rest of
            // the way at chase speed.
            if obs.distance_to_target > cfg.attack_range {
                // Jump-to-close-vertical-gap (single or double).
                // Same gravity-framed gate as Approach.
                if obs.to_target_up() > cfg.vertical_chase_min {
                    if obs.self_on_ground {
                        return SpecificAction::Jump;
                    }
                    if obs.self_air_jumps_remaining > 0 && obs.self_vel_up() < 50.0 {
                        return SpecificAction::DoubleJump;
                    }
                }
                let dir = obs.side_face_toward_target();
                return SpecificAction::Walk { dir };
            }
            // In range but on cooldown — hold ground, face target.
            //
            // NOTE (§A1 subsumption): the moveset is the special EXECUTOR and the
            // capability is on `ActionSet.special`, so wiring the autonomous brain to
            // choose `SpecificAction::Special` here is a one-line change. It's LEFT
            // OFF deliberately: a naive "fire special whenever melee recharges" cadence
            // spams the move and crowds out the damage-triggered regroup kit (it broke
            // the duel-arena regroup-dash). The autonomous special CADENCE is a feel/AI
            // tuning pass for Jon against the landed system (needs a real special
            // cooldown / spacing gate). Possession already fires the special via
            // `special_pressed` → the moveset today.
            SpecificAction::Idle
        }
        BroadMode::Reposition => {
            // Anti-clump:
            //   - "Front" actor (closer to target than the centroid
            //     of nearby allies): keep approaching by walking
            //     along `away_dir.x` — which, for an actor in front,
            //     points TOWARD the target.
            //   - "Back" actor (further from target than the
            //     centroid): hold position rather than retreat back
            //     to spawn. The front engages first; once it cycles
            //     into cooldown or moves, the back can re-evaluate.
            //
            // This prevents the back-goblin-retreats-forever
            // oscillation that "always walk along away_dir" produces.
            if obs.crowding.away_dir.length_squared() < 0.05 {
                // Allies stacked exactly on top — no usable direction.
                return SpecificAction::Idle;
            }
            let away_dir_x = signum_or(obs.crowding.away_dir.x, 0.0);
            let toward_target_x = signum_or(obs.to_target_x, 0.0);
            if away_dir_x.abs() < 0.001 || toward_target_x.abs() < 0.001 {
                return SpecificAction::Idle;
            }
            if away_dir_x.signum() == toward_target_x.signum() {
                // Walking AWAY from the centroid coincidentally walks
                // TOWARD the target → we're the front actor. Push
                // through and engage.
                SpecificAction::Walk { dir: away_dir_x }
            } else {
                // Walking away from the centroid would walk AWAY from
                // the target → we're the back actor. Hold the line.
                SpecificAction::Idle
            }
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
mod tests;
