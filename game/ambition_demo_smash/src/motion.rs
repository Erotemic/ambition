//! The smash ruleset's commanded-velocity seam (ADR 0024, authority #4).
//!
//! One operation, not three bare writes. A steered bolt flying its caster home,
//! a launch plate and a homing dash all say: for this instant, the smash
//! ruleset, not the movement kernel, sets this body's velocity. The ownership
//! argument is made once, here.
//!
//! The waiver is on this operation, not on its callers, as
//! `engine.velocity-writes-are-authority-only` asks. A new smash move that
//! launches a body calls this and adds no policy entry; a new bare `kin.vel =`
//! in this demo still fails the guard.
//!
//! Set, not add, so these are not impulses. An impulse (`vel +=`,
//! `AccelerationFrame::launch`) composes with the body's current velocity. A
//! plate that added would throw a fast-falling body less far, and a dash that
//! added would be faster from a running start.
//!
//! Limit: the commanded vector is world-space and does not rotate with a body's
//! resolved frame. `AccelerationFrame::launch` is frame-aware but scalar and
//! always throws away from the feet, which removes the angled plate that
//! `PlaceSpringParams::launch` authors. On a stage with rotated gravity these
//! would point the wrong way; no smash stage rotates gravity, and this is the
//! one function to fix if one does.

use ambition_platformer2d::engine_core as ae;

/// The smash ruleset states this body's velocity for this instant.
///
/// `why` is for the causal log only. It turns "somebody moved" into "the plate
/// fired" when a replay disagrees.
pub fn command_body_velocity(kin: &mut ae::BodyKinematics, velocity: ae::Vec2, why: &str) {
    kin.vel = velocity;
    bevy::log::debug!(target: "ambition::moves", "commanded velocity: {why} -> {velocity:?}");
}
