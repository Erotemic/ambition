//! A move that hangs its owner off a wire and lets a winch take her up.
//!
//! ⭐⭐ THE OTHER HALF OF `ambition_characters::smash_flyline`, and it is a
//! remarkably small file on purpose. It needs no collision view and picks no
//! destination: everything about being on a rope — the pendulum, the winch, the
//! stick that buys angular acceleration, the one release that writes one exit
//! velocity — is stated once in the movement kernel's `integrate_wire_clusters`.
//! This file only ties the knot.
//!
//! ⛔⛔ AND THAT IS THE ENTIRE POINT OF THE TECHNIQUE. The move it replaces ran
//! `apply_authored_teleports`, which emits `PLAYER_BLINK` at every transit — so
//! Jon's *"it should not get the teleport sound"* could never be answered by
//! editing a timeline, because the cue was never on one. A move that does not
//! run the teleport executor does not make the teleport's sound.

use bevy::prelude::*;

use ambition_characters::brain::{ActionRequest, ActorActionMessage, SpecialActionSpec};
use ambition_characters::smash_flyline::{FlylineParams, FLYLINE};
use ambition_platformer2d_core::{self as ae};

/// Recognise an authored flyline beat and put the body on the wire.
///
/// ⛔ IT RUNS WHERE EVERY OTHER `ActorActionMessage` CONSUMER RUNS, so a catch
/// authored on a move's timeline lands on the frame the move says.
///
/// ⛔ NOT A CLASS-B REMAP, and the contrast with the trapdoor beside it is the
/// reason to say so. That one PICKS A POSITION and writes it, which is exactly
/// what the remap ledger exists to record. This one writes no position at all:
/// the body travels continuously, through the same sweep as any other frame, and
/// an instrument watching for unexplained displacement will find none because
/// there is none. That is clause one of the ask — *"it is not a teleport"* —
/// stated in the shape of the code rather than in a comment.
pub fn apply_authored_flylines(
    mut actions: MessageReader<ActorActionMessage>,
    mut bodies: Query<(
        &ae::BodyKinematics,
        // ⭐ THE BODY'S OWN FRAME, because "which way is the sky" is not a
        // constant. The anchor is placed along this body's own up, so a stage
        // that rotates gravity rotates where the wire comes down from.
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &mut ae::movement::MotionModel,
    )>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != FLYLINE {
            continue;
        }
        let params: FlylineParams = match params.hydrate() {
            Ok(params) => params,
            Err(err) => {
                warn!("flyline params did not hydrate: {err}");
                continue;
            }
        };
        let Ok((kinematics, resolved_frame, mut motion_model)) = bodies.get_mut(message.actor)
        else {
            continue;
        };
        let at = kinematics.pos;
        // ⛔ THE WINCH SPEED IS DERIVED, NOT AUTHORED. `rise` and `lift_s` are
        // the two facts a designer has an opinion about — how far, and how long —
        // and a third authored number would be free to disagree with them. The
        // kernel reels at whatever rate makes those two true.
        //
        // ⭐⭐ AND THE RATE IT WANTS IS THE RAMP'S START, not the average. The
        // winch decelerates from this to `release_rise` so the release has no
        // step in it (see `integrate_wire_clusters`), and the area under a
        // linear ramp from `v0` to `v1` over `T` is `T·(v0+v1)/2` — so the `v0`
        // that still travels the authored `rise` is `2·rise/T − v1`.
        //
        // ⚠ CLAMPED AT `release_rise`, WHICH IS THE DEGENERATE AUTHORING. A move
        // asking to leave the wire faster than the average climb would need the
        // winch to ACCELERATE into the cut; the honest reading is that it has
        // over-authored the carry, and a flat rope is better than a rope that
        // speeds up at the top. The content test names the condition.
        let winch_speed = if params.lift_s > 0.0 {
            (2.0 * params.rise / params.lift_s - params.release_rise).max(params.release_rise)
        } else {
            0.0
        };
        let caught = ae::catch_the_wire(
            &mut motion_model,
            at,
            resolved_frame.get(),
            params.rope_length,
            params.lift_s,
            winch_speed,
            params.max_swing_deg.to_radians(),
            params.swing_accel,
            params.release_rise,
        );
        // ⚠ A BODY THAT COULD NOT TAKE THE WIRE MAKES NO SHOW OF IT. A
        // non-axis policy has no pendulum and refuses; drawing the rope and
        // banging the pulley anyway would advertise a lift that is not going to
        // happen.
        if !caught {
            continue;
        }
        // The look and the sound are the MOVE's, not this system's — the same
        // rule the authored teleport and trapdoor follow.
        //
        // ⛔ AND THIS IS THE CATCH, NOT THE WIRE. The rope itself is a persistent
        // object for the length of the lift and is drawn from the read model
        // (`rendering::flyline`); an FX-atlas row plays once and ends, which is
        // the distinction `rendering/submerged.rs` spells out for the trapdoor.
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: at,
            fx: ambition_vfx::fx::FxId::new(&params.vfx),
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        sfx.write_for(
            message.actor,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::SfxId::new(&params.sfx),
                pos: at,
            },
        );
    }
}

#[cfg(test)]
mod tests;
