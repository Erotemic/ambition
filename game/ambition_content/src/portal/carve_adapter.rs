//! Ambition bridge: portal-owned carves → the host collision overlay.
//!
//! Portal core's [`publish_portal_carves`](ambition_portal2d::publish_portal_carves) writes the
//! aperture geometry into the portal-owned [`PortalCarves`](ambition_portal2d::PortalCarves)
//! resource. Portal core never names `FeatureEcsWorldOverlay` — it owns the carve *geometry*,
//! while Ambition owns how a carve alters its collision representation.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::SolidWorldQuery;
use ambition_platformer2d_core::RoomGeometry;
use ambition_portal2d::{measure_host_depth, PlacedPortal, PortalCarves, PortalHostDepthsByRoom};

/// Copy this frame's portal-owned carves into the host collision overlay.
///
/// The copy clears and refills `portal_carves` so a frame with no transiting body re-seals the
/// host wall, exactly as the old in-core write did.
///
/// ⭐ EACH CARVE GOES TO THE LIVE ROOM IT WAS CUT IN (OW1 cut 7l), and every
/// room with no carve this frame is re-sealed. Portal core cuts a hole per
/// room (`PortalCarves::holes` names the room of each), so two live rooms each
/// carve the walls of their own pair. A hole in no live room goes to the sole
/// live room, so while two rooms are live it is not written.
pub fn bridge_portal_carves(
    carves: Res<PortalCarves>,
    mut overlays: ambition_platformer2d::world::RoomOverlays,
) {
    for mut overlay in overlays.each() {
        overlay.portal_carves.clear();
    }
    for (room, hole) in &carves.holes {
        let stamp = room.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(mut overlay) = overlays.for_room(stamp.as_ref()) else {
            continue;
        };
        overlay.portal_carves.push(*hole);
    }
}

/// Measure the solid host material behind each placed portal's face and
/// publish it into the portal-owned [`PortalHostDepthsByRoom`] seam. Portal core
/// bounds the transit rescue and the carve engagement by these depths — the
/// geometric guard that stops a THIN wall's aperture volume from reaching the
/// open room behind it (walk-through / wrong-side entry). The base
/// [`RoomGeometry`] is the honest source: portal carves must not open
/// sight/entry through their own hole, and moving-platform overlays are not
/// portal hosts.
///
/// ⭐ EACH PORTAL IS MEASURED IN ITS OWN LIVE ROOM (OW1 cut 7l). This read the
/// sole live room, so while two rooms were live it did not run, and the
/// depths of the last one-room frame stayed. The depth is filed under the
/// portal's room, so two rooms that each hold a portal of one channel keep
/// two depths. A portal in no live room gets no depth.
pub fn sync_portal_host_depths(
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut depths: ResMut<PortalHostDepthsByRoom>,
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
        depths.push(world.room_of(entity), portal.channel, depth);
    }
}
