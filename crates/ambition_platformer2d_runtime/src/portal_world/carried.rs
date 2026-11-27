//! Crushing C5 (crushing-and-obstruction-response.md): a moving host is the
//! driver of what it pushes through its portal.
//!
//! A body straddling a portal has a part behind the portal's plane, and that
//! part is at the partner. When the portal's host moves by `delta`, the body
//! moves by `-delta` relative to the mouth, so the part through the pair
//! moves by the pair's map of `-delta` at the exit, against the solids there.
//! This publishes each such part before platforms advance, so a host's
//! proposal (C3) is admitted against them, and the host, its aperture and the
//! part through it commit together: the aperture follows the host's accepted
//! move (`refresh_hosted_portal_frames` reads the host's recorded delta).
//!
//! Pairwise, like C3: only the host of the portal the body straddles drives
//! it, and the exit's own host moving is not read here.

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, Aabb, AabbExt, BodyKinematics, Vec2};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRooms};
use ambition_platformer2d_world::platforms::obstruction::{full_solids, one_way_supports, CarriedObstruction, CarriedObstructions};
use ambition_portal2d::{find_portal, PlacedPortal, PortalHost, PortalTransit, PortalTuning};

/// The part a numerically exact clip leaves on the plane is not in contact
/// with the exit's own wall: the mapped part is inset by this much.
const CLIP_INSET: f32 = 0.05;

/// Publish, for each body in a portal on a moving face, the part of it the
/// face would push through the pair.
pub(super) fn publish_carried_obstructions(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: LiveRooms,
    tuning: Option<Res<PortalTuning>>,
    bodies: Query<(Entity, &BodyKinematics, &PortalTransit), Without<ae::BodyInStorage>>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut carried: ResMut<CarriedObstructions>,
) {
    carried.0.clear();
    let convention = tuning.as_deref().map(|t| t.convention.map_convention()).unwrap_or_default();
    let mut bodies: Vec<_> = bodies.iter().collect();
    // Identity order, so the published list is the same on every peer.
    bodies.sort_by_key(|(entity, ..)| *entity);
    for (entity, body, transit) in bodies {
        let room = live.of(entity);
        let in_room: Vec<&PlacedPortal> = portals.iter()
            .filter(|(portal, _)| live.of(*portal) == room)
            .map(|(_, portal)| portal)
            .collect();
        let Some(mouth) = find_portal(in_room.iter().copied(), transit.straddling) else { continue };
        let PortalHost::Face { face, .. } = &mouth.host else { continue };
        let Some(exit) = find_portal(in_room.iter().copied(), transit.straddling.partner()) else { continue };
        let Ok(map) = ae::frame::Sim2::through(&mouth.aperture(), &exit.aperture(), convention) else { continue };
        let aabb = body.aabb();
        if !(aabb.min.is_finite() && aabb.max.is_finite()) { continue; }
        let Some(behind) = clip_behind(aabb, mouth.pos, mouth.normal) else { continue };
        let (Ok(x), Ok(y)) = (map.vector(Vec2::X), map.vector(Vec2::Y)) else { continue };
        let mapped: Option<Vec<Vec2>> = behind.iter().map(|p| map.point(*p).ok()).collect();
        let Some(mapped) = mapped.and_then(|points| ae::geometry::point_bounds(points).ok().flatten()) else { continue };
        let mapped = Aabb::new(mapped.center(), (mapped.half_size() - Vec2::splat(CLIP_INSET)).max(Vec2::ZERO));
        let Some(solids) = collision.room(room.map(InRoomInstance).as_ref()).and_then(|room| room.solids()) else { continue };
        // Which of these stop the part depends on the push, known only when
        // the host proposes its move.
        carried.0.push(CarriedObstruction {
            platform: face.geo.clone(), body: entity, map: [x, y], mapped,
            solids: full_solids(&solids, Some(&face.geo)), one_ways: one_way_supports(&solids, Some(&face.geo)),
            gravity_dir: Vec2::Y,
        });
    }
}

/// The polygon of `aabb` behind the plane through `origin` with `normal`
/// (the side the normal points away from), `None` when nothing of it is.
fn clip_behind(aabb: Aabb, origin: Vec2, normal: Vec2) -> Option<Vec<Vec2>> {
    let corners = [aabb.min, Vec2::new(aabb.max.x, aabb.min.y), aabb.max, Vec2::new(aabb.min.x, aabb.max.y)];
    let mut out = Vec::with_capacity(6);
    ae::polygon::clip_half_plane(&corners, |p| -(p - origin).dot(normal), &mut out);
    (out.len() >= 3).then_some(out)
}
