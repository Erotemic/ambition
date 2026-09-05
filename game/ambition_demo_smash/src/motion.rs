//! THE SMASH RULESET'S COMMANDED-VELOCITY SEAM (ADR 0024, authority #4).
//!
//! ⭐⭐ ONE OPERATION, NOT THREE BARE WRITES. A steered bolt flying its caster
//! home, a launch plate throwing whoever stands on it, and a homing dash are the
//! same statement three times: *for this instant, the smash ruleset — not the
//! movement kernel — says what this body's velocity IS*. Naming it once means
//! the ownership argument is made once, in the place a reader will look for it,
//! instead of being retyped at each seam and drifting.
//!
//! ⛔⛔ AND IT IS WAIVED AT THE OPERATION RATHER THAN AT ITS CALLERS, which is
//! the shape `engine.velocity-writes-are-authority-only` already asks for in as
//! many words, on `intercept.rs`: *"Waived at the OPERATION rather than at its
//! callers, which is the point of having an operation: the next interception
//! adds no entry here."* ⇒ The next smash move that launches somebody calls this
//! and adds no policy entry; a new bare `kin.vel =` anywhere in this demo still
//! fails the guard and still has to argue for itself.
//!
//! ⛔ SET, NOT ADD, AND THAT IS THE WHOLE REASON THESE ARE NOT IMPULSES. An
//! impulse seam (`vel +=`, `AccelerationFrame::launch`) composes with whatever
//! the body arrived with. Every caller here needs the opposite: a plate that
//! added would throw a fast-falling body less far than a walking one, and a dash
//! that added would make a running start into a faster homing move. The genre's
//! expectation is that these OVERWRITE.
//!
//! ⚠ WHAT THIS GIVES UP, SAID PLAINLY: the commanded vector is WORLD-space, so
//! it does not rotate with a body's resolved frame. `AccelerationFrame::launch`
//! is the frame-aware operation and was the first thing tried here — it is
//! scalar and always throws AWAY FROM THE FEET, which would have deleted the
//! angled plate `PlaceSpringParams::launch` exists to author. ⇒ On a stage with
//! rotated gravity these would point the wrong way. No smash stage rotates
//! gravity; if one ever does, this is the one function to fix.

use ambition_platformer2d::engine_core as ae;

/// The smash ruleset states this body's velocity for this instant.
///
/// `why` is for the causal log only — it costs nothing and it is what turns
/// "somebody moved" into "the plate fired" when a replay disagrees.
pub fn command_body_velocity(kin: &mut ae::BodyKinematics, velocity: ae::Vec2, why: &str) {
    kin.vel = velocity;
    bevy::log::debug!(target: "ambition::moves", "commanded velocity: {why} -> {velocity:?}");
}
