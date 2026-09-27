//! What a portal transit means for the body a seat drives.
//!
//! The portal core moves every body the same way and names no seat, input or
//! player. It states each transit on `PortalBodyTransited`. These systems
//! apply the consequences that follow from a seat driving the body: it turns
//! around on a same-wall turn-around, the trace and trail are told the snap was
//! intentional, and the seat's held input is guarded as the body emerges. They
//! are part of `PortalSchedulePlugin`, so every game with portals has them.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak;
use ambition_portal2d::pieces::portal_map_vec;
use ambition_portal2d::{
    BodyTeleported, PortalBodyTransited, PortalEmission, PortalInputWarp, PortalTuning,
};

/// Turn around the body a seat drives when its transit states a facing flip.
///
/// The `portal_reverses_facing` setting reverses the CONTROLLED body's facing,
/// and the controlled body is the one carrying [`DrivingParticipant`]:
/// possession moves that component to the possessed body, while the home body
/// keeps its player-population markers. So this asks the driver, not the
/// population. It runs in `PortalSet::Transited`, after the core's transit.
///
/// [`DrivingParticipant`]: ambition_characters::control::DrivingParticipant
pub fn turn_the_driven_body_around_after_portal_transit(
    mut transited: MessageReader<ambition_portal2d::PortalBodyTransited>,
    tuning: Res<ambition_portal2d::PortalTuning>,
    mut driven: Query<
        &mut ambition_platformer2d_core::BodyKinematics,
        With<ambition_characters::control::DrivingParticipant>,
    >,
) {
    for event in transited.read() {
        if !(event.facing_flip && tuning.reorient_facing) {
            continue;
        }
        if let Ok(mut kin) = driven.get_mut(event.body) {
            kin.facing = -kin.facing;
        }
    }
}

/// Mark the body a seat drives after its transit: the trace and trail
/// notices, and the two input guards that [`warp_portal_input`] applies.
///
/// For each [`PortalBodyTransited`] of a body with a [`DrivingParticipant`]:
///
/// - emits [`BodyTeleported`], so the gameplay trace treats the position snap
///   as intentional and does not dump on it;
/// - emits a [`TrailContinuityBreak`], so the trail starts a new chunk at the
///   exit instead of drawing a line across the room;
/// - inserts the [`PortalEmission`] emergence guard (held input cannot push
///   back into the exit wall for a short time); and
/// - inserts the [`PortalInputWarp`] held-input warp when the convention's map
///   flips horizontal movement and a movement input is held.
///
/// These are seat facts: the portal core names no seat and no input. It runs
/// in `PortalSet::Transited`, before the controller of the next tick.
///
/// [`DrivingParticipant`]: ambition_characters::control::DrivingParticipant
pub fn guard_the_driven_body_after_portal_transit(
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

/// Apply the two input guards to the frame of the seat that drives each
/// guarded body: the same-wall held-input warp (soft: it drops when the hold is
/// released or clearly changes direction) and the emergence guard (held input
/// cannot push back into the exit wall while the guard is fresh). Both are mild,
/// so a portal never feels like a hard input latch.
///
/// Per seat: each guarded body names its own [`DrivingParticipant`], and the
/// guards shape that seat's frame through `shape_seat_frame`.
/// [`guard_the_driven_body_after_portal_transit`] inserts the guards on a
/// crossing.
///
/// [`DrivingParticipant`]: ambition_characters::control::DrivingParticipant
pub fn warp_portal_input(
    time: Option<Res<ambition_time::WorldTime>>,
    mut commands: Commands,
    tuning: Res<PortalTuning>,
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    mut slots: ResMut<ambition_characters::control::SlotControls>,
    mut raw: ResMut<ambition_characters::control::SeatRawFrames>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::control::DrivingParticipant,
        Option<&PortalInputWarp>,
        Option<&mut PortalEmission>,
    )>,
) {
    let sim_dt = time.as_deref().map_or(0.0, |t| t.sim_dt());
    for (entity, driver, warp, emission) in &mut bodies {
        if warp.is_none() && emission.is_none() {
            continue;
        }
        let slot = driver.0;
        let frame = ambition_platformer2d_actor_monolith::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            slot,
        );
        let mut dir = bevy::prelude::Vec2::new(frame.axis_x, frame.axis_y);

        // Same-wall held-input warp: a hold that survives the crossing is mapped
        // through the portal, and one that is released or clearly redirected
        // drops the guard.
        if let Some(warp) = warp {
            if dir.length() < tuning.input_held_epsilon {
                commands.entity(entity).remove::<PortalInputWarp>();
            } else if warp.anchor.length() > 0.01
                && dir.normalize_or_zero().dot(warp.anchor.normalize_or_zero())
                    < tuning.input_warp_keep_cos
            {
                commands.entity(entity).remove::<PortalInputWarp>();
            } else {
                dir = portal_map_vec(dir, warp.n_in, warp.n_out, tuning.convention.map_convention());
            }
        }

        // Emergence guard: strip held input that pushes back into the exit wall
        // while it is fresh.
        if let Some(mut emission) = emission {
            emission.timer -= sim_dt;
            if emission.timer <= 0.0 {
                commands.entity(entity).remove::<PortalEmission>();
            } else {
                let into = dir.dot(emission.exit_normal);
                if into < 0.0 {
                    dir -= into * emission.exit_normal;
                }
            }
        }

        ambition_platformer2d_actor_monolith::control::shape_seat_frame(
            latches.as_deref(),
            rollback.as_deref(),
            &mut slots,
            &mut raw,
            slot,
            |frame| {
                frame.axis_x = dir.x;
                frame.axis_y = dir.y;
            },
        );
    }
}

#[cfg(test)]
mod tests;
