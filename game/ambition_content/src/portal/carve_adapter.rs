//! Ambition bridge: portal-owned carves → the host collision overlay.
//!
//! Portal core's [`publish_portal_carves`](ambition_portal2d::publish_portal_carves) writes the
//! aperture geometry into the portal-owned [`PortalCarves`](ambition_portal2d::PortalCarves)
//! resource. Portal core never names `FeatureEcsWorldOverlay` — it owns the carve *geometry*,
//! while Ambition owns how a carve alters its collision representation.

use bevy::prelude::*;

use ambition_platformer2d_core::cast::SolidWorldQuery;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use ambition_portal2d::{measure_host_depth, PlacedPortal, PortalCarves, PortalHostDepths};

/// Copy this frame's portal-owned carves into the host collision overlay.
///
/// The copy clears and refills `portal_carves` so a frame with no transiting body re-seals the
/// host wall, exactly as the old in-core write did.
pub fn bridge_portal_carves(
    carves: Res<PortalCarves>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    overlay.portal_carves.clear();
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
pub fn sync_portal_host_depths(
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    portals: Query<&PlacedPortal>,
    mut depths: ResMut<PortalHostDepths>,
) {
    depths.0.clear();
    if portals.is_empty() {
        return;
    }
    let mut solids: Vec<ambition_platformer2d_core::Aabb> = Vec::new();
    world
        .0
        .for_each_solid_aabb(false, &mut |aabb| solids.push(aabb));
    for portal in &portals {
        let depth = measure_host_depth(
            &solids,
            &portal.frame(),
            ambition_portal2d::pieces::CARVE_DEPTH,
        );
        depths.0.push((portal.channel, depth));
    }
}
