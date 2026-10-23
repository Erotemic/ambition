//! Ambition bridge: portal-owned carves → the host collision overlay.
//!
//! Portal core's [`publish_portal_carves`](ambition_portal2d::publish_portal_carves) writes the
//! aperture geometry into the portal-owned [`PortalCarves`](ambition_portal2d::PortalCarves)
//! resource. Portal core never names `FeatureEcsWorldOverlay` — it owns the carve *geometry*,
//! while Ambition owns how a carve alters its collision representation.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::SolidWorldQuery;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRooms};
use ambition_portal2d::{host_material_depth, PlacedPortal, PortalCarves, PortalHost, PortalHostDepthsByRoom};

/// Run condition of [`bridge_portal_carves`]: a carve stands this frame, or
/// a room still holds a hole or a glue to clear. With neither, the bridge
/// would leave every overlay as it is.
pub fn carves_or_holes_stand(
    carves: Res<PortalCarves>,
    overlays: Query<&ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
) -> bool {
    !carves.holes.is_empty()
        || !carves.glues.is_empty()
        || overlays
            .iter()
            .any(|overlay| !overlay.portal_holes.is_empty() || !overlay.portal_glues.is_empty())
}

/// Run condition of [`sync_portal_host_depths`]: a portal stands, or depths
/// of one that went remain to clear.
pub fn portals_or_depths_stand(
    portals: Query<(), With<PlacedPortal>>,
    depths: Res<PortalHostDepthsByRoom>,
) -> bool {
    !portals.is_empty() || !depths.0.is_empty()
}

/// Copy this frame's portal-owned carves into the host collision overlay.
///
/// The copy clears and refills `portal_holes` so a frame with no transiting body re-seals the
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
        overlay.portal_holes.clear();
        overlay.portal_glues.clear();
    }
    // Each pair a body is in, for the bodies in it.
    for (room, pass) in &carves.glues {
        let stamp = room.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        if let Some(mut overlay) = overlays.for_room(stamp.as_ref()) {
            overlay.portal_glues.push(pass.clone());
        }
    }
    // Each hole is open for the bodies that opened it, and for no other.
    for hole in &carves.holes {
        let stamp = hole.room.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
        let Some(mut overlay) = overlays.for_room(stamp.as_ref()) else {
            continue;
        };
        overlay.portal_holes.push(ambition_platformer2d_shared_tangle::feature_overlay::PortalHolePass {
            face: hole.face,
            bodies: hole.open_for.clone(),
        });
    }
}

/// Measure the solid host material behind each placed portal's face and
/// publish it into the portal-owned [`PortalHostDepthsByRoom`] seam. Portal core
/// bounds the transit rescue and the carve engagement by these depths — the
/// geometric guard that stops a THIN wall's aperture volume from reaching the
/// open room behind it (walk-through / wrong-side entry). The surfaces a
/// portal may ride are the honest source: authored geometry and moving
/// platforms, uncarved (a carve must not open entry through its own hole).
/// A portal on a moving block is measured through that block where it is.
///
/// A portal on a line (a plank, a slope) with no solid behind it has nothing
/// behind it: depth zero, so the rescue never takes a body that is under a
/// plank and moving away from the portal on top of it. A portal placed in
/// the open keeps the probe depth.
///
/// ⭐ EACH PORTAL IS MEASURED IN ITS OWN LIVE ROOM (OW1 cut 7l). This read the
/// sole live room, so while two rooms were live it did not run, and the
/// depths of the last one-room frame stayed. The depth is filed under the
/// portal's room, so two rooms that each hold a portal of one channel keep
/// two depths. A portal in no live room gets no depth.
pub fn sync_portal_host_depths(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: LiveRooms,
    portals: Query<(Entity, &PlacedPortal)>,
    mut depths: ResMut<PortalHostDepthsByRoom>,
) {
    depths.0.clear();
    // The solids of each room, gathered once for all its portals.
    let mut solids_by_room: Vec<(ambition_portal2d::PortalRoom, Vec<ambition_platformer2d_core::Aabb>)> =
        Vec::new();
    for (entity, portal) in &portals {
        let room = live.of(entity);
        let index = match solids_by_room.iter().position(|(seen, _)| *seen == room) {
            Some(index) => index,
            None => {
                let Some(surfaces) = collision
                    .room(room.map(InRoomInstance).as_ref())
                    .and_then(|room| room.hostable_surfaces())
                else {
                    continue;
                };
                let mut solids = Vec::new();
                surfaces.for_each_solid_aabb(true, &mut |aabb| solids.push(aabb));
                solids_by_room.push((room, solids));
                solids_by_room.len() - 1
            }
        };
        let probe = ambition_portal2d::pieces::CARVE_DEPTH;
        let found = host_material_depth(&solids_by_room[index].1, &portal.frame(), probe);
        let depth = match portal.host {
            PortalHost::Line { .. } => found.unwrap_or(0.0),
            _ => found.unwrap_or(probe),
        };
        depths.push(room, portal.channel, depth);
    }
}
