//! Straddle eviction: a pushout for a plane that moved or closed under a body.
//!
//! Pushout breaks position reversibility and hides real bugs, so the portal
//! mechanic has two and no more: this one, and the snag ([`crate::snag`]),
//! which holds a body that cuts a portal's plane past an end of its opening. When a portal moves, closes, or teleports while a body
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

use ambition_platformer2d_core::{self as ae, AabbExt};

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRooms, SessionScopeActivated};

use crate::color::PortalChannel;
use crate::pieces::{self as pp, PortalAperture, PortalFrame};
use crate::types::PlacedPortal;
use crate::PortalRoom;

/// Last frame's placed-portal frame per live room and channel, so
/// [`evict_straddlers_on_portal_change`] can detect a portal that moved or
/// vanished under a straddling body. [`PortalPlugin`](crate::PortalPlugin)
/// initialises it.
#[derive(Resource, Default, Clone)]
pub struct PortalFrameHistory(HashMap<(PortalRoom, PortalChannel), PortalAperture>);

impl PortalFrameHistory {
    /// How many placed portals the last frame had. A diagnostic: the map
    /// iterates in no fixed order, so a count is what two hosts can compare.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Forget last frame's portals when a gameplay session activates.
///
/// The history describes the rooms of one session. The first room of each
/// session has the same live key
/// ([`LiveRoomInstance::ACTIVATION`](ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION)),
/// so a frame that the last session left reads as a portal of the new room
/// that closed. On the first tick of the new session
/// [`evict_straddlers_on_portal_change`] then pushes a body away from a plane
/// that is not in its world.
///
/// MEASURED 2026-10-04 on the shell host: after a session that placed one
/// portal, the next session started with one frame here and a fresh host
/// started with none. The two agreed from tick 1, when the eviction rewrote
/// the history.
///
/// The activation is before the session world is live, so the timeline that a
/// rewind can cross does not exist yet.
pub fn forget_portal_frames_on_activation(
    mut activated: MessageReader<SessionScopeActivated>,
    mut history: ResMut<PortalFrameHistory>,
) {
    if activated.read().count() > 0 {
        history.0.clear();
    }
}

/// Small clearance past the closing plane so the evicted body is unambiguously
/// on one side (not resting exactly on it).
const EVICT_MARGIN: f32 = 1.0;

/// Detect portals that moved or vanished since last frame, and push any body
/// straddling the old plane fully to its centroid's side (see module docs).
pub fn evict_straddlers_on_portal_change(
    mut history: ResMut<PortalFrameHistory>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(Entity, &mut BodyKinematics, Option<&mut ae::SweepSample>, Option<&ae::PoseOwnedExternally>)>,
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
    let current = current_frames(&portals, &live);
    for (room, plane) in closing_planes(&history, &current) {
        evict_for_plane(plane, room, &mut bodies, &live);
    }
    history.0 = current.into_iter().map(|(c, (ap, _))| (c, ap)).collect();
}

/// The same eviction for free bodies `B` (a thrown item): a plane that closes
/// under one pushes it clear as it pushes a kernel body. Runs before
/// [`evict_straddlers_on_portal_change`], which then moves the history on.
pub fn evict_free_straddlers_on_portal_change<B: crate::FreePortalBody>(
    history: Res<PortalFrameHistory>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(Entity, &mut B, Option<&B::Presence>)>,
    live: LiveRooms,
) {
    if history.0.is_empty() {
        return;
    }
    let current = current_frames(&portals, &live);
    for (room, plane) in closing_planes(&history, &current) {
        for (entity, mut body, presence) in &mut bodies {
            if live.of(entity) != room || !B::present(presence) {
                continue;
            }
            let aabb = ae::Aabb::new(body.pos(), body.half_extent());
            if let Some(push) = eviction_push(aabb, body.pos(), &plane) {
                let (pos, vel) = (body.pos() + push, body.vel());
                body.emerge(pos, vel);
            }
        }
    }
}

type CurrentFrames = HashMap<(PortalRoom, PortalChannel), (PortalAperture, PortalFrame)>;

fn current_frames(portals: &Query<(Entity, &PlacedPortal)>, live: &LiveRooms) -> CurrentFrames {
    portals
        .iter()
        .map(|(entity, p)| ((live.of(entity), p.channel), (p.aperture(), p.prev_frame())))
        .collect()
}

/// The planes that closed since last frame, in a fixed order (room, then
/// channel): a body straddling two of them is pushed by each in the same
/// order on every host and every resimulation.
///
/// A plane is unchanged if a portal of the same channel started this tick
/// where it ended the last: whatever its host did since (moved it, turned
/// it) is the motion of one plane. Otherwise it was placed again or closed,
/// and its old plane is closing.
fn closing_planes(history: &PortalFrameHistory, current: &CurrentFrames) -> Vec<(PortalRoom, PortalAperture)> {
    let mut closing: Vec<_> = history
        .0
        .iter()
        .filter(|(key, old)| {
            !current.get(*key).is_some_and(|(_, start)| {
                start.origin.distance(old.frame.origin) < 1.0e-3
                    && start.normal.distance(old.frame.normal) < 1.0e-4
            })
        })
        .map(|(key, old)| (*key, *old))
        .collect();
    closing.sort_by_key(|(key, _)| *key);
    closing.into_iter().map(|((room, _), plane)| (room, plane)).collect()
}

/// How far a body straddling `plane` moves so its trailing edge clears it
/// on its centroid's side; `None` when it does not straddle it.
fn eviction_push(body: ae::Aabb, center: ae::Vec2, plane: &PortalAperture) -> Option<ae::Vec2> {
    if !pp::straddles(body, plane) {
        return None;
    }
    let n = plane.frame.normal;
    let d = pp::front_distance(center, &plane.frame);
    let half_n = body.half_size().dot(n.abs());
    let (push, dir) = if d >= 0.0 { (half_n - d + EVICT_MARGIN, n) } else { (half_n + d + EVICT_MARGIN, -n) };
    (push > 0.0).then_some(dir * push)
}

/// Shove every body of `room` straddling `plane` to the side its centroid is
/// on, just past the plane.
fn evict_for_plane(
    plane: PortalAperture,
    room: PortalRoom,
    bodies: &mut Query<(Entity, &mut BodyKinematics, Option<&mut ae::SweepSample>, Option<&ae::PoseOwnedExternally>)>,
    live: &LiveRooms,
) {
    for (entity, mut kin, mut sweep, held) in bodies.iter_mut() {
        // Its owner places it: a plane that moves does not move it.
        if live.of(entity) != room || crate::transit::pose_is_held(held) {
            continue;
        }
        // The box the body has: turned as its last step turned it.
        let body = kin.collision_box(sweep.as_deref());
        // Position authority (ADR 0024): the closing portal moves the body
        // clear of the plane.
        if let Some(push) = eviction_push(body, kin.pos, &plane) {
            ambition_platformer2d_core::movement::carry_body(&mut kin, sweep.as_deref_mut(), push);
        }
    }
}

#[cfg(test)]
mod tests;
