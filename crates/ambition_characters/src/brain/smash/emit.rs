//! Stage 5 — emit inputs.
//!
//! Translates a [`SpecificAction`] into the matching
//! [`crate::actor::control::ActorControlFrame`] fields. This is the only stage that
//! knows the integration pipeline's frame schema — everything
//! upstream stays vocabulary-pure.

use ambition_engine_core as ae;

use super::action::SpecificAction;
use super::observation::ObservationFrame;

/// Local sign-or-fallback helper — see action.rs for the rationale.
fn signum_or(x: f32, fallback: f32) -> f32 {
    if x.abs() < 0.001 {
        fallback
    } else {
        x.signum()
    }
}

/// Walk speed (px/s) the emitter sends when the brain commits
/// `Walk`. Should approximately match an enemy's chase speed.
/// Sandbox chase speeds today range ~100–225 px/s; the emit step
/// uses the action's own dir but defers actual speed to the
/// integration's `approach()` call against this magnitude.
const WALK_SPEED_PX_S: f32 = 170.0;

/// Dash speed (px/s) — higher burst movement, used by Reposition
/// under severe crowding (when authored) and by future
/// `BroadMode::Approach` upgrades.
#[allow(dead_code, reason = "consumer arrives with the dash-action upgrade")]
const DASH_SPEED_PX_S: f32 = 260.0;

/// Translate the chosen action into ActorControlFrame fields.
/// Overwrites `out` (caller must reset to neutral first if it
/// matters; today `tick_smash` does that at the top).
pub fn emit_inputs(
    action: SpecificAction,
    obs: &ObservationFrame,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    // Facing is set unconditionally toward the target (when one exists) so even Idle
    // mid-engagement faces the threat. Facing is a LOCAL +1/-1 (the body writes
    // `kin.facing`), so it tracks the gravity-perpendicular side sign toward the
    // target — correct under any gravity; byte-identical to `to_target_x` screen-down.
    // Held facing toward the target (gravity-perpendicular side sign). Uses the
    // alignment deadzone, so when the target stacks on the gravity axis the facing
    // HOLDS instead of flipping every frame — the rotated-gravity flip fix.
    let face_x = obs.side_face_toward_target();
    out.facing = face_x;

    match action {
        SpecificAction::Idle => {
            out.locomotion = ae::Vec2::ZERO;
        }
        SpecificAction::Walk { dir } => {
            let signed_dir = signum_or(dir, 0.0);
            // Walk = a throttle of the brawler's dash-grade top speed; the body's
            // tuning owns the px/s scale. (jitter-free here; intent is the throttle)
            out.locomotion = ae::Vec2::new(signed_dir * (WALK_SPEED_PX_S / DASH_SPEED_PX_S), 0.0);
            if signed_dir.abs() > 0.001 {
                out.facing = signed_dir;
            }
        }
        SpecificAction::Dash { dir } => {
            let signed_dir = signum_or(dir, 0.0);
            // Full-throttle locomotion is the body-agnostic floor (a body without
            // the dash capability still closes at its top walk speed). `dash_pressed`
            // is the intent edge the BODY turns into a burst when it has `can_dash`
            // (invariant I3): the brain attempts, the body owns the burst speed +
            // window + cooldown.
            out.locomotion = ae::Vec2::new(signed_dir, 0.0);
            out.dash_pressed = true;
            if signed_dir.abs() > 0.001 {
                out.facing = signed_dir;
            }
        }
        SpecificAction::Jump => {
            out.jump_pressed = true;
        }
        SpecificAction::DoubleJump => {
            // Engine doesn't track double-jump separately on actor
            // frames; emit a regular jump edge and let the
            // integration's double-jump gate decide.
            out.jump_pressed = true;
        }
        SpecificAction::MeleeAttack { dir } => {
            out.melee_pressed = true;
            out.attack_axis = dir;
            // Face along the attack axis (x component).
            let axis_x = dir.x;
            if axis_x.abs() > 0.001 {
                out.facing = signum_or(axis_x, out.facing);
            }
        }
        SpecificAction::RangedAttack { dir } => {
            if dir.length_squared() > 1e-6 {
                out.fire = Some(
                    crate::actor::control::ActorFireRequest::controlled_body_local(
                        dir,
                        // Speed routed through ActionSet at resolve time;
                        // emit a placeholder here.
                        0.0,
                    ),
                );
            }
            // Fire WHILE closing, not instead of closing: a ranged poke advances
            // toward the target (throwing the poke on the way in to the melee
            // finish) rather than camping at range. Without this the fighter
            // stands and pokes forever once it enters the ranged band — an
            // aggressive fighter keeps coming (the body, not a brain camp, paces
            // the shots; invariants I3/I4).
            let toward = obs.side_face_toward_target();
            out.locomotion = ae::Vec2::new(toward * (WALK_SPEED_PX_S / DASH_SPEED_PX_S), 0.0);
        }
        SpecificAction::Special => {
            out.special_pressed = true;
        }
        SpecificAction::Shield | SpecificAction::Dodge { .. } => {
            // Reserved — no engine-side input bit yet. Drop to Idle
            // so the actor doesn't visibly freeze in a "trying to
            // shield" pose.
            out.locomotion = ae::Vec2::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observation::CrowdingSignal;
    use super::*;

    fn obs_at(distance_x: f32) -> ObservationFrame {
        ObservationFrame {
            self_pos: ae::Vec2::ZERO,
            self_vel: ae::Vec2::ZERO,
            self_facing: 1.0,
            self_on_ground: true,
            self_aerial: false,
            self_alive: true,
            self_attacking: false,
            self_air_jumps_remaining: 0,
            attack_cooldown_remaining: 0.0,
            stun_remaining: 0.0,
            self_health_fraction: 1.0,
            target_pos: ae::Vec2::new(distance_x, 0.0),
            target_alive: true,
            to_target_x: distance_x,
            to_target_y: 0.0,
            distance_to_target: distance_x.abs(),
            down: ae::Vec2::new(0.0, 1.0),
            crowding: CrowdingSignal::default(),
            terrain: Default::default(),
            sim_time: 1.0,
            dt: 1.0 / 60.0,
        }
    }

    #[test]
    fn walk_emits_locomotion_along_dir() {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        emit_inputs(
            SpecificAction::Walk { dir: 1.0 },
            &obs_at(300.0),
            &mut frame,
        );
        assert!(frame.locomotion.x > 0.0);
        assert_eq!(frame.locomotion.y, 0.0);
        assert!(frame.facing > 0.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        emit_inputs(
            SpecificAction::Walk { dir: -1.0 },
            &obs_at(300.0),
            &mut frame,
        );
        assert!(frame.locomotion.x < 0.0);
        assert!(frame.facing < 0.0);
    }

    #[test]
    fn melee_attack_sets_melee_pressed_and_attack_axis() {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        emit_inputs(
            SpecificAction::MeleeAttack {
                dir: ae::Vec2::new(1.0, 0.0),
            },
            &obs_at(40.0),
            &mut frame,
        );
        assert!(frame.melee_pressed);
        assert_eq!(frame.attack_axis, ae::Vec2::new(1.0, 0.0));
        assert!(frame.facing > 0.0);
    }

    #[test]
    fn ranged_attack_sets_fire_with_dir() {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        emit_inputs(
            SpecificAction::RangedAttack {
                dir: ae::Vec2::new(0.0, -1.0),
            },
            &obs_at(200.0),
            &mut frame,
        );
        match frame.fire {
            Some(req) => {
                assert!((req.dir.y + 1.0).abs() < 1e-3);
                assert_eq!(req.dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
            }
            None => panic!("expected fire request"),
        }
    }

    #[test]
    fn jump_emits_jump_pressed_edge() {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        emit_inputs(SpecificAction::Jump, &obs_at(200.0), &mut frame);
        assert!(frame.jump_pressed);
    }

    #[test]
    fn idle_zeros_locomotion_but_keeps_facing() {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        // Target on the left → expect actor to face left.
        emit_inputs(SpecificAction::Idle, &obs_at(-200.0), &mut frame);
        assert_eq!(frame.locomotion, ae::Vec2::ZERO);
        assert!(frame.facing < 0.0, "facing should point at target");
    }
}
