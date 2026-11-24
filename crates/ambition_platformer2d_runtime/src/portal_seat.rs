//! What a portal transit means for the body a seat drives.
//!
//! The portal core moves every body the same way and names no seat, input or
//! player. It states each transit on `PortalBodyTransited`. These systems
//! apply the consequences that follow from a seat driving the body: it turns
//! around on a same-wall turn-around, the trace and trail are told the snap was
//! intentional, and the seat's held input is guarded as the body emerges. They
//! are part of `PortalSchedulePlugin`, so every game with portals has them.

use bevy::prelude::*;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;

use ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak;
use ambition_portal2d::{
    BodyTeleported, PortalBodyTransited, PortalEmission, PortalInputWarp, PortalTuning,
};

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
    mut transited: ReplayMessageReader<PortalBodyTransited>,
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
            resume_at: ev.transfer.exit_pos,
        });
        // Protect the emergence so the floored exit velocity carries the body
        // out before held input can fight it.
        commands.entity(ev.body).insert(PortalEmission {
            exit_normal: ev.transfer.exit.frame.normal,
            timer: tuning.emission_time_s,
        });
        // Warp held input only when the active portal map keeps ordinary
        // horizontal movement expressible and flips it. A floor↔wall 90° turn
        // would rotate a horizontal hold into "up", which the controller can't
        // use as ordinary movement.
        if ev.input_warp && held.length() > tuning.input_held_epsilon {
            commands.entity(ev.body).insert(PortalInputWarp {
                map: ev.transfer.map.orthogonal(),
                anchor: held,
            });
        }
    }
}

/// Apply the two portal input guards to the LOCOMOTION of each guarded body a
/// seat drives (IN2): the same-wall held-input warp (soft: it drops when the
/// hold is released or clearly changes direction) and the emergence guard
/// (held movement cannot push back into the exit wall while the guard is
/// fresh). Both are mild, so a portal never feels like a hard input latch.
///
/// ⭐ AFTER THE TRANSLATION, ON THE BODY'S RESOLVED CONTROL. These guards used
/// to rewrite the seat's raw stick against world-space portal normals, which
/// ignored the stick's frame mode: under `BodyRelativeStrict` with down
/// `(1,0)`, raw `(0,-1)` is world `(-1,0)`, so the raw-axis strip kept an
/// inward push and removed a tangential one. And every verb read the shared
/// stick, so a movement guard changed an attack's direction. Here the body's
/// `locomotion` is taken to world space through the body's own resolved frame,
/// guarded there, and brought back.
///
/// The policies, explicitly:
/// - the warp continues a HELD stick, so it applies to locomotion, the flyer's
///   velocity target, the facing request and an attack aimed by the movement
///   stick. A C-stick attack keeps its own aim;
/// - the emergence strip is a physical constraint on travel, so it applies to
///   locomotion and the velocity target only;
/// - a body that is not flying cannot command travel along its local Y (that
///   axis means crouch or descend), so neither guard changes it: only the side
///   component follows the world-space result.
///
/// ⛔ ONLY A FRAME THE TRANSLATOR WROTE THIS TICK. `ActorControl` is latched; a
/// guard applied to a value it already guarded would warp it twice. A system
/// does not see its own writes as changes, so `is_changed` is exactly "the
/// seat's translation wrote it since this guard last ran".
///
/// Per seat: each guarded body names its own [`DrivingParticipant`], whose
/// raw stick decides whether the hold continues.
///
/// [`DrivingParticipant`]: ambition_characters::control::DrivingParticipant
pub fn warp_portal_input(
    time: Option<Res<ambition_time::WorldTime>>,
    mut commands: Commands,
    tuning: Res<PortalTuning>,
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    slots: Res<ambition_characters::control::SlotControls>,
    raw: Res<ambition_characters::control::SeatRawFrames>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::control::DrivingParticipant,
        &mut ambition_characters::control::ActorControl,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        Option<&ambition_platformer2d_core::BodyFlightState>,
        Option<&PortalInputWarp>,
        Option<&mut PortalEmission>,
    )>,
) {
    let sim_dt = time.as_deref().map_or(0.0, |t| t.sim_dt());
    for (entity, driver, mut control, resolved, flight, warp, emission) in &mut bodies {
        if warp.is_none() && emission.is_none() {
            continue;
        }
        let frame = ambition_platformer2d_actor_monolith::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            driver.0,
        );
        // The seat's own stick says whether the hold continues: released or
        // clearly turned, the warp drops.
        let stick = Vec2::new(frame.axis_x, frame.axis_y);
        let mut warp = warp.copied();
        if let Some(held) = warp {
            let released = stick.length() < tuning.input_held_epsilon;
            let turned = held.anchor.length() > 0.01
                && stick.normalize_or_zero().dot(held.anchor.normalize_or_zero()) < tuning.input_warp_keep_cos;
            if released || turned {
                commands.entity(entity).remove::<PortalInputWarp>();
                warp = None;
            }
        }
        let mut inward_normal = None;
        if let Some(mut emission) = emission {
            emission.timer -= sim_dt;
            if emission.timer <= 0.0 {
                commands.entity(entity).remove::<PortalEmission>();
            } else {
                inward_normal = Some(emission.exit_normal);
            }
        }
        if (warp.is_none() && inward_normal.is_none()) || !control.is_changed() {
            continue;
        }
        let basis = resolved.copied().unwrap_or_default().basis();
        let free = flight.is_some_and(|flight| flight.fly_enabled);
        let warped = |world: Vec2| match warp {
            Some(held) => held.map * world,
            None => world,
        };
        let emerging = |world: Vec2| match inward_normal {
            Some(normal) => world - world.dot(normal).min(0.0) * normal,
            None => world,
        };
        // A local command through a world-space guard. Only the TRAVEL the
        // body's motor can express goes through it: a flyer travels on both
        // axes, a walker only along its side axis. A walker's local Y is
        // posture (crouch, descend), not travel, so it stays out of the
        // projection and is kept as it was; projecting the whole `(x, y)` and
        // restoring Y afterwards let posture leak into the new side travel
        // under an oblique frame.
        let guard_travel = |local: Vec2, guard: &dyn Fn(Vec2) -> Vec2| {
            let travel = if free { local } else { Vec2::new(local.x, 0.0) };
            let out = basis.to_local(guard(basis.to_world(travel)));
            if free { out } else { Vec2::new(out.x, local.y) }
        };
        let frame_out = &mut control.0;
        let before = frame_out.locomotion.vec();
        let after = guard_travel(before, &|world| emerging(warped(world)));
        frame_out.locomotion = ambition_platformer2d_core::LocalAxes::from_vec(after);
        // The flyer's command is world-space already.
        frame_out.velocity_target =
            ambition_platformer2d_core::WorldVec2(emerging(warped(frame_out.velocity_target.0)));
        if warp.is_some() {
            // The facing request follows the continued hold.
            if frame_out.facing != 0.0 {
                frame_out.facing = if after.x.abs() > 0.01 { after.x.signum() } else { 0.0 };
            }
            // An attack the movement stick aimed continues with the held
            // stick; a C-stick attack keeps its own aim. The translator says
            // which it chose. An aim is a direction, not travel, so it is
            // carried through the map whole, with no posture rule.
            if !frame_out.attack_axis_from_aim_stick {
                let aim = frame_out.attack_axis.vec();
                frame_out.attack_axis =
                    ambition_platformer2d_core::LocalAxes::from_vec(basis.to_local(warped(basis.to_world(aim))));
            }
        }
    }
}

#[cfg(test)]
mod tests;
