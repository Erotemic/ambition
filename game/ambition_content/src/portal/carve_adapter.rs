//! Ambition bridge: portal-owned carves → the host collision overlay.
//!
//! Portal core's [`publish_portal_carves`](ambition_portal2d::publish_portal_carves) writes the
//! aperture geometry into the portal-owned [`PortalCarves`](ambition_portal2d::PortalCarves)
//! resource. Portal core never names `FeatureEcsWorldOverlay` — it owns the carve *geometry*,
//! while Ambition owns how a carve alters its collision representation.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::SolidWorldQuery;
use ambition_platformer2d_core::RoomGeometry;
use ambition_portal2d::{measure_host_depth, PlacedPortal, PortalCarves, PortalHostDepths};

/// Copy this frame's portal-owned carves into the host collision overlay.
///
/// The copy clears and refills `portal_carves` so a frame with no transiting body re-seals the
/// host wall, exactly as the old in-core write did.
///
/// ⭐ THE CARVES GO TO THE LIVE ROOM THE PORTALS ARE IN (OW1 cut 7l), and every
/// other live room is re-sealed. This wrote the sole live room's overlay, so
/// while two rooms were live no wall was carved. A carve does not name its
/// portal, so the room is the one room every placed portal is in. When the
/// portals are in two rooms (a pair can be split, see
/// `portal_projectile_step`), no room is carved: a transit between rooms is
/// not a crossing this seam can make.
pub fn bridge_portal_carves(
    carves: Res<PortalCarves>,
    portals: Query<Entity, With<PlacedPortal>>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut overlays: ambition_platformer2d::world::RoomOverlays,
) {
    for mut overlay in overlays.each() {
        overlay.portal_carves.clear();
    }
    if carves.holes.is_empty() {
        return;
    }
    let mut rooms = portals.iter().map(|portal| live.of(portal));
    let Some(Some(room)) = rooms.next() else {
        return;
    };
    if rooms.any(|other| other != Some(room)) {
        return;
    }
    let stamp = ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(room);
    let Some(mut overlay) = overlays.for_room(Some(&stamp)) else {
        return;
    };
    overlay.portal_carves.extend_from_slice(&carves.holes);
}

/// Measure the solid host material behind each placed portal's face and
/// publish it into the portal-owned [`PortalHostDepths`] seam. Portal core
/// bounds the transit rescue and the carve engagement by these depths — the
/// geometric guard that stops a THIN wall's aperture volume from reaching the
/// open room behind it (walk-through / wrong-side entry). The base
/// [`RoomGeometry`] is the honest source: portal carves must not open
/// sight/entry through their own hole, and moving-platform overlays are not
/// portal hosts.
///
/// ⭐ EACH PORTAL IS MEASURED IN ITS OWN LIVE ROOM (OW1 cut 7l). This read the
/// sole live room, so while two rooms were live it did not run, and the
/// depths of the last one-room frame stayed. A portal in no live room gets no
/// depth.
pub fn sync_portal_host_depths(
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut depths: ResMut<PortalHostDepths>,
) {
    depths.0.clear();
    for (entity, portal) in &portals {
        let Some(geometry) = world.of(entity) else {
            continue;
        };
        let mut solids: Vec<ambition_platformer2d_core::Aabb> = Vec::new();
        geometry
            .0
            .for_each_solid_aabb(false, &mut |aabb| solids.push(aabb));
        let depth = measure_host_depth(
            &solids,
            &portal.frame(),
            ambition_portal2d::pieces::CARVE_DEPTH,
        );
        depths.0.push((portal.channel, depth));
    }
}
