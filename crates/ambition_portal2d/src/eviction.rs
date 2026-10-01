//! Straddle eviction: the only pushout the portal mechanic performs.
//!
//! Pushout breaks position reversibility and hides real bugs, so it is used
//! only here. When a portal moves, closes, or teleports while a body
//! straddles its plane (a re-fired portal, a room reset, a vanished partner),
//! the body is pushed fully to its centroid's side, so it lands in open space
//! and not inside the wall.
//!
//! ## Disabling this
//! A game that wants a lethal portal close does not register
//! [`evict_straddlers_on_portal_change`], and reacts to the same event (a body
//! straddling a portal that just changed) itself. Ambition evicts.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use ambition_platformer2d_core as ae;

use ambition_platformer2d_shared_tangle::body::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;

use crate::color::PortalChannel;
use crate::pieces::{self as pp, PortalAperture};
use crate::types::PlacedPortal;
use crate::PortalRoom;

/// Last frame's placed-portal frame per live room and channel, so
/// [`evict_straddlers_on_portal_change`] can detect a portal that moved or
/// vanished under a straddling body. [`PortalPlugin`](crate::PortalPlugin)
/// initialises it.
#[derive(Resource, Default, Clone)]
pub struct PortalFrameHistory(HashMap<(PortalRoom, PortalChannel), PortalAperture>);

/// Small clearance past the closing plane so the evicted body is unambiguously
/// on one side (not resting exactly on it).
const EVICT_MARGIN: f32 = 1.0;

/// Detect portals that moved or vanished since last frame, and push any body
/// straddling the old plane fully to its centroid's side (see module docs).
pub fn evict_straddlers_on_portal_change(
    mut history: ResMut<PortalFrameHistory>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(Entity, &mut BodyKinematics, Option<&mut ae::SweepSample>)>,
    live: LiveRooms,
) {
    // A hosted aperture moving with its face is the same portal, not a close:
    // compare against the host-carried position. Unhosted portals have zero
    // `frame_delta`. A refire or teleport does not match the host delta, so it
    // still evicts.
    //
    // Keyed by live room and channel (OW1): two rooms can each hold a portal
    // of one channel. Keyed by channel alone, the two overwrote each other and
    // read as a portal that moved.
    let current: HashMap<(PortalRoom, PortalChannel), (PortalAperture, Vec2)> = portals
        .iter()
        .map(|(entity, p)| ((live.of(entity), p.channel), (p.aperture(), p.frame_delta())))
        .collect();

    for (key, old) in history.0.iter() {
        // The plane is unchanged only if a portal of the same channel still
        // sits at the same pos + normal (host-carried motion included);
        // otherwise its old plane is closing.
        let unchanged = current.get(key).is_some_and(|(now, delta)| {
            now.frame.origin.distance(old.frame.origin + *delta) < 1.0
                && now.frame.normal == old.frame.normal
        });
        if unchanged {
            continue;
        }
        // Only a body of the portal's own room can straddle it.
        evict_for_plane(*old, key.0, &mut bodies, &live);
    }

    history.0 = current.into_iter().map(|(c, (ap, _))| (c, ap)).collect();
}

/// Shove every body of `room` straddling `plane` to the side its centroid is
/// on, just past the plane.
fn evict_for_plane(
    plane: PortalAperture,
    room: PortalRoom,
    bodies: &mut Query<(Entity, &mut BodyKinematics, Option<&mut ae::SweepSample>)>,
    live: &LiveRooms,
) {
    let n = plane.frame.normal;
    for (entity, mut kin, mut sweep) in bodies.iter_mut() {
        if live.of(entity) != room {
            continue;
        }
        let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
        if !pp::straddles(body, &plane) {
            continue;
        }
        // Push so the trailing edge clears the plane on the centroid's side.
        let d = pp::front_distance(kin.pos, &plane.frame);
        let half_n = (kin.size * 0.5).dot(n.abs());
        // Position authority (ADR 0024): the closing portal moves the body
        // clear of the plane.
        if d >= 0.0 {
            let push = half_n - d + EVICT_MARGIN;
            if push > 0.0 {
                ambition_platformer2d_core::movement::carry_body(&mut kin, sweep.as_deref_mut(), n * push);
            }
        } else {
            let push = half_n + d + EVICT_MARGIN;
            if push > 0.0 {
                ambition_platformer2d_core::movement::carry_body(&mut kin, sweep.as_deref_mut(), -n * push);
            }
        }
    }
}

#[cfg(test)]
mod tests;
