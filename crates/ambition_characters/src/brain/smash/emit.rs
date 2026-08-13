//! Stage 5 — emit inputs.
//!
//! Translates a [`SpecificAction`] into the matching
//! [`crate::actor::control::ActorControlFrame`] fields. This is the only stage that
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
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::Walk { dir } => {
            let signed_dir = signum_or(dir, 0.0);
            // Walk = a throttle of the brawler's dash-grade top speed; the body's
            // tuning owns the px/s scale. (jitter-free here; intent is the throttle)
            out.locomotion =
                ae::LocalAxes::new(signed_dir * (WALK_SPEED_PX_S / DASH_SPEED_PX_S), 0.0);
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
            out.locomotion = ae::LocalAxes::new(signed_dir, 0.0);
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
            out.locomotion = ae::LocalAxes::new(toward * (WALK_SPEED_PX_S / DASH_SPEED_PX_S), 0.0);
        }
        SpecificAction::Special => {
            out.special_pressed = true;
        }
        SpecificAction::Shield => {
            // ⛔⛔ **"no engine-side input bit yet" WAS WRONG, and it had been
            // wrong for a while** (corrected 2026-08-13). `shield_held` is a
            // field on the very struct this function writes to, and it is the
            // live path a player's guard takes — `shield_held` → `resolve_shield`
            // (`avatar/starting_character.rs`). The comment described the world
            // when the variant was added and nobody re-read it, so P5.38 recorded
            // `Shield` as having "zero producers" while the reason it had none
            // was assumed to be downstream.
            //
            // ⚠ a body that cannot shield is not harmed by this: the ability mask
            // gates the verb (`AbilitySet::shield`), so holding the bit on a body
            // without a guard raises nothing.
            out.shield_held = true;
            out.locomotion = ae::LocalAxes::ZERO;
        }
        SpecificAction::Dodge { .. } => {
            // ⚠ **still genuinely reserved, and for the stated reason**: there is
            // no dodge bit on `ActorControlFrame`. A dodge reaches a body through
            // the dash buffer, which is the coupling P5.38 already records — a
            // body owning `dodge` never dashes because `apply_dodge` claims that
            // buffer first. Emitting a dash here would ride the same defect.
            out.locomotion = ae::LocalAxes::ZERO;
        }
    }
}

#[cfg(test)]
mod tests;
