//! What a portal transit means for the body it moved.
//!
//! The portal core maps a body's position and velocity through a pair and
//! states each transit on `PortalBodyTransited`. A body can carry more than
//! its velocity: a flight cluster's carried run, a shot's carried world
//! acceleration. These systems map what the body carries in the same frame.
//! They are part of `PortalSchedulePlugin`, so every game with portals has
//! them. What the transit means for the SEAT is `portal_seat`'s.

use bevy::prelude::*;

use ambition_platformer2d_core::body_clusters::BodyKinematics;
use ambition_portal2d::{PortalBodyTransited, PortalTuning};
use ambition_projectiles::ProjectileGameplay;

/// Give every transferred body its carried run momentum: the world-imparted
/// part of the mapped exit velocity's run-axis component becomes
/// `BodyFlightState::carried_run`, the floor the hands-off air-stop assist
/// decays toward. So a portal fling is conserved (Portal physics) while jump
/// drift keeps the tight stop-on-release feel (Hollow Knight control). Runs
/// after `portal_transit` in the same frame, when `BodyKinematics::vel` is the
/// mapped exit velocity. Any transferred body with the flight cluster gets
/// it.
///
/// The transfer rotates momentum; it must not reclassify it. A fall's
/// gravity-earned speed is world-imparted, so a real fling (fall in, wall out)
/// still floors at full strength.
///
/// The run axis is the body's own resolved frame, not the primary body's
/// `GravityField`: a body in another gravity zone runs along another axis, and
/// splitting its velocity on the wrong one would turn its fall into run.
pub fn apply_portal_carried_momentum(
    mut transited: MessageReader<PortalBodyTransited>,
    mut bodies: Query<(
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &mut ambition_platformer2d_core::BodyFlightState,
    )>,
    // The session's portal map convention, from the resource that owns it.
    tuning: Res<PortalTuning>,
) {
    use ambition_portal2d::pieces::portal_map_vec;
    let convention = tuning.convention.map_convention();

    for ev in transited.read() {
        let Ok((kin, frame, mut flight)) = bodies.get_mut(ev.body) else {
            continue;
        };
        let side = frame.basis().side;
        // `kin.vel` is the mapped exit velocity; the map is an isometry, so
        // the swapped-normal map is its inverse (pinned at 45° in pieces).
        let pre_vel = portal_map_vec(kin.vel, ev.exit_normal, ev.enter_normal, convention);
        let controller_run = pre_vel.dot(side) - flight.carried_run;
        let mapped_controller =
            portal_map_vec(controller_run * side, ev.enter_normal, ev.exit_normal, convention);
        flight.carried_run = kin.vel.dot(side) - mapped_controller.dot(side);
    }
}

/// Rotate a projectile's carried world acceleration through the portal, like
/// its velocity.
///
/// `ProjectileGameplay::accel` is a constant world acceleration the shot
/// carries. If only the velocity were mapped, the ponytail boomerang (the only
/// shot with a non-zero one) would leave a rotated portal with its "come home"
/// pull still on the pre-portal axis, and trace a different arc.
///
/// This lives here, not in the portal core, which knows nothing about
/// projectiles. It is the same adapter shape as the carried-momentum and
/// kernel-body reconciliations: it reads the core's `PortalBodyTransited` and
/// its two normals.
///
/// A shot with no authored acceleration is unaffected: mapping `ZERO` gives
/// `ZERO`.
pub fn rotate_projectile_acceleration_after_portal_transit(
    mut transited: MessageReader<PortalBodyTransited>,
    mut shots: Query<&mut ProjectileGameplay>,
    // The session's portal map convention, from the resource that owns it.
    tuning: Res<PortalTuning>,
) {
    use ambition_portal2d::pieces::portal_map_vec;
    let convention = tuning.convention.map_convention();

    for ev in transited.read() {
        let Ok(mut shot) = shots.get_mut(ev.body) else {
            continue;
        };
        if shot.accel == ambition_platformer2d_core::Vec2::ZERO {
            continue;
        }
        shot.accel = portal_map_vec(shot.accel, ev.enter_normal, ev.exit_normal, convention);
    }
}

#[cfg(test)]
mod carried_momentum_tests;
#[cfg(test)]
mod projectile_transit_tests;
