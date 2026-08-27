//! Stage 5 — emit inputs.
//!
//! Translates a [`SpecificAction`] into the matching
//! [`ambition_characters::actor::control::ActorControlFrame`] fields. This is the only stage that
//! knows the integration pipeline's frame schema — everything
//! upstream stays vocabulary-pure.

use ambition_platformer2d_core as ae;

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

/// Full-throttle reference speed (px/s) — the denominator that turns
/// [`WALK_SPEED_PX_S`] into a fraction of a body's own top speed. It is NOT a
/// speed the emitter commands: `Sprint` sends throttle `1.0` and the body's
/// tuning decides what that is worth.
#[allow(dead_code, reason = "consumer arrives with the sprint-action upgrade")]
const SPRINT_SPEED_PX_S: f32 = 260.0;

/// Translate the chosen action into ActorControlFrame fields.
/// Overwrites `out` (caller must reset to neutral first if it
/// matters; today `tick_smash` does that at the top).
pub fn emit_inputs(
    action: SpecificAction,
    obs: &ObservationFrame,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
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
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::Walk { dir } => {
            let signed_dir = signum_or(dir, 0.0);
            // Walk = a partial throttle of the body's own top speed; its
            // tuning owns the px/s scale. (jitter-free here; intent is the throttle)
            out.locomotion =
                ae::LocalAxes::new(signed_dir * (WALK_SPEED_PX_S / SPRINT_SPEED_PX_S), 0.0);
            if signed_dir.abs() > 0.001 {
                out.facing = signed_dir;
            }
        }
        SpecificAction::Sprint { dir } => {
            // CLOSING IS LOCOMOTION. Full throttle against the body's own top
            // speed — the same surface `Walk` uses at a partial throttle, and the
            // whole difference between the two.
            //
            // The brain would have been asking to evade at the exact moment it meant to run.
            // And the burst it was reaching for is gone from this game's vocabulary anyway.
            //
            // a driver that genuinely wants the discrete burst must ask
            // `resolve_burst_maneuver` what a press would MEAN on this body
            // first; nothing in the smash brain does, which is why dropping the
            // press is the correct shape and not a lost capability.
            let signed_dir = signum_or(dir, 0.0);
            out.locomotion = ae::LocalAxes::new(signed_dir, 0.0);
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
            out.attack_axis = ae::LocalAxes::from_vec(dir);
            // Face along the attack axis (x component).
            let axis_x = dir.x;
            if axis_x.abs() > 0.001 {
                out.facing = signum_or(axis_x, out.facing);
            }
        }
        SpecificAction::RangedAttack { dir } => {
            if dir.length_squared() > 1e-6 {
                out.fire = Some(
                    ambition_characters::actor::control::ActorFireRequest::controlled_body_local(
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
            out.locomotion =
                ae::LocalAxes::new(toward * (WALK_SPEED_PX_S / SPRINT_SPEED_PX_S), 0.0);
        }
        SpecificAction::Special => {
            out.special_pressed = true;
        }
        SpecificAction::Shield => {
            // a body that cannot shield is not harmed by this: the ability mask
            // gates the verb (`AbilitySet::shield`), so holding the bit on a body
            // without a guard raises nothing.
            out.shield_held = true;
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::Grab => {
            // One press, one attempt. The authored grab move owns how long the
            // attempt stays live, so there is nothing to hold here.
            out.grab_pressed = true;
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::CaptureAttack { forward } => {
            // the ORDINARY attack press. `trigger_moveset_moves` reads the
            // capture context and turns it into a pummel or a throw; a brain
            // that reached for a capture-specific verb here would be the
            // CPU-only road this design exists without.
            out.melee_pressed = true;
            // MIRRORED BY FACING, and it was not. `attack_axis` is a
            // STICK — `attack_dir_from_axis` computes `axis.x * facing` — so a
            // bare `+x` means *forward* only while the body faces right. A
            // left-facing captor asking for a forward throw was asking for a
            // BACK throw, which a fighter that authors no back throw simply
            // cannot perform: it would have stood there holding its captive.
            // The same double-mirror cost George his side special once already,
            // and the note on `aim_the_stick` is where that is written down.
            //
            // a PARTIAL deflection, for the second reason that note gives: a
            // stick shoved to 1.0 reads as a flick, and a flick left armed turns
            // the next ordinary attack into an accidental smash.
            out.attack_axis = if forward {
                ae::LocalAxes::new(
                    ambition_characters::actor::attack_gesture::TILT_DEFLECTION
                        * if obs.self_facing < 0.0 { -1.0 } else { 1.0 },
                    0.0,
                )
            } else {
                ae::LocalAxes::ZERO
            };
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::CaptureStruggle => {
            // The same press a person mashes. A captive cannot walk, so there
            // is no locomotion to state beyond neutral.
            out.melee_pressed = true;
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::Dodge { .. } => {
            // The remaining requirement is the general one — this arm must ask
            // `resolve_burst_maneuver` (perception already carries the answer as
            // `ObservationFrame:burst`) and emit only when it says a dodge, instead of pressing
            // and hoping. Slice-sized work of its own; don't hand it the press without the
            // question.
            out.locomotion = ae::LocalAxes::ZERO;
        }
    }
}

#[cfg(test)]
mod tests;
