//! Ambition's reactions to a generic portal transit.
//!
//! The generic portal core drives every body through a placed pair without
//! naming player, boss, enemy, or projectile. This module mirrors the primary
//! player's input/trace side effects after a transit event so the controller
//! sees `PortalEmission` / `PortalInputWarp` on the same frame, and completes
//! the transit for kernel bodies and projectiles.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak;
use ambition_platformer2d_core::body_clusters::BodyKinematics;
use ambition_portal2d::{
    BodyTeleported, PortalBodyTransited, PortalEmission, PortalInputWarp, PortalTuning,
};
use ambition_projectiles::ProjectileGameplay;

/// Carry the `portal_reverses_facing` gameplay setting into the editable
/// portal tuning, and propose it.
///
/// It writes `EditablePortalTuning`, not `PortalTuning`. Writing the
/// authority from inside `GgrsSchedule` would do this:
///
/// ```text
/// developer edits reorient_facing in the portal inspector
///   -> proposed, admitted, published into PortalTuning
///   -> GgrsSchedule runs
///   -> THIS overwrites it from the persisted gameplay setting
/// ```
///
/// Then the last writer wins. It would also read `UserSettings` inside the
/// rollback window, so a replay of frame N would see the current setting.
///
/// `EditablePortalTuning` is where the field is authored, and
/// `publish_editable_portal_tuning` is the only writer of the authority.
///
/// For `reorient_facing`, the gameplay setting is the author and the panel is
/// not. This system is change-guarded on `editable.reorient_facing != want`,
/// so it does nothing while they agree and republishes the persisted value
/// when they differ. An inspector edit of this field is therefore reverted on
/// the next pass, with no race. The panel's row states this.
///
/// The other `EditablePortalTuning` fields are panel-authored: this system
/// touches one field only.
///
/// The gameplay setting defaults off, so by default the player keeps the same
/// facing through a same-wall portal turn-around; the portal crate's own
/// default stays on for standalone use. Change-guarded, so an untouched
/// setting proposes nothing and never stops a rollback baseline.
pub fn sync_portal_reorient_from_settings(
    // Optional: headless / unit-test apps may run portal transit without the
    // settings resource. Absent → leave the portal crate's default (ON).
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut editable: ResMut<ambition_platformer2d::portal::EditablePortalTuning>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    let Some(settings) = settings else {
        return;
    };
    let want = settings.gameplay.portal_reverses_facing;
    if editable.reorient_facing != want {
        editable.reorient_facing = want;
        pending.propose(ambition_platformer2d::portal::portal_tuning_domain());
    }
}

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

/// Apply driven-body input/trace side effects after generic portal transit.
/// Reads [`PortalBodyTransited`] events and, for the CONTROLLED subject only
/// (a possessed actor while possessing, else the home avatar):
///
/// - emits [`BodyTeleported`] (so the gameplay trace treats the position snap as
///   intentional and doesn't auto-dump on it),
/// - inserts the [`PortalEmission`] emergence guard (held input can't push back
///   into the exit wall for a short window), and
/// - inserts the [`PortalInputWarp`] held-input warp iff this convention's
///   map flips horizontal movement and a movement input is held.
///
/// [`PortalEmission`] and [`PortalInputWarp`] are input and must never be
/// referenced by the portal core. This runs `.after(portal_transit)` and
/// `.before` the player controller, so these components exist in the frame the
/// controller runs.
pub fn portal_player_input_adapter(
    mut commands: Commands,
    tuning: Res<PortalTuning>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut teleported: MessageWriter<BodyTeleported>,
    mut trail_breaks: MessageWriter<TrailContinuityBreak>,
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    slots: Res<ambition_characters::control::SlotControls>,
    raw: Res<ambition_characters::control::SeatRawFrames>,
    drivers: Query<&ambition_characters::control::DrivingParticipant>,
) {
    for ev in transited.read() {
        // Only a driven body has input and trace side effects. The seat is read off
        // the body that transited, so any seat's hold is warped.
        let Ok(driver) = drivers.get(ev.body) else {
            continue;
        };
        let frame = ambition_platformer2d_actor_monolith::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            driver.0,
        );
        let held = Vec2::new(frame.axis_x, frame.axis_y);
        // Trace: the position snap is intentional.
        teleported.write(BodyTeleported { body: ev.body });
        // Trail: the body remained continuous in the quotient space, but its
        // ordinary world coordinates snapped. Emit the neutral trail seam so
        // the trail chunks instead of drawing a fake line across the room.
        trail_breaks.write(TrailContinuityBreak {
            body: ev.body,
            resume_at: ev.exit_pos,
        });
        // Protect the emergence so the floored exit velocity carries the body
        // out before held input can fight it.
        commands.entity(ev.body).insert(PortalEmission {
            exit_normal: ev.exit_normal,
            timer: tuning.emission_time_s,
        });
        // Warp held input only when the active portal map keeps ordinary
        // horizontal movement expressible and flips it. A floor↔wall 90° turn
        // would rotate a horizontal hold into "up", which the controller can't
        // use as ordinary movement.
        if ev.input_warp && held.length() > tuning.input_held_epsilon {
            commands.entity(ev.body).insert(PortalInputWarp {
                n_in: ev.enter_normal,
                n_out: ev.exit_normal,
                anchor: held,
            });
        }
    }
}

#[cfg(test)]
mod projectile_transit_tests;
#[cfg(test)]
mod carried_momentum_tests;
