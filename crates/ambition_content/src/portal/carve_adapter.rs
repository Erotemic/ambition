//! Ambition bridge: portal-owned carves → the host collision overlay.
//!
//! Portal core's [`publish_portal_carves`](ambition_gameplay_core::portal::publish_portal_carves)
//! writes the aperture geometry into the portal-owned
//! [`PortalCarves`](ambition_gameplay_core::portal::PortalCarves) resource. Portal core never
//! names `FeatureEcsWorldOverlay` — it owns the carve *geometry*, while Ambition
//! owns how a carve alters its collision representation. This bridge copies the
//! published carves into `FeatureEcsWorldOverlay.portal_carves` each frame,
//! ordered identically (publish order preserved), so the collision world sees the
//! same carves the same frame it did before the seam.

use bevy::prelude::*;

use ambition_gameplay_core::features::FeatureEcsWorldOverlay;
use ambition_gameplay_core::portal::{
    measure_host_depth, PlacedPortal, PortalCarves, PortalHostDepths,
};
use ambition_gameplay_core::RoomGeometry;
use ambition_platformer_primitives::world_query::SolidWorldQuery;

/// Copy this frame's portal-owned carves into the host collision overlay.
///
/// Runs in `PortalSet::Carves`, after `publish_portal_carves` (which fills
/// [`PortalCarves`]) and before `SandboxSet::CoreSimulation` consumes the overlay
/// via `world_with_sandbox_solids`. The copy clears and refills
/// `portal_carves` so a frame with no transiting body re-seals the host wall,
/// exactly as the old in-core write did.
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
    world: Res<RoomGeometry>,
    portals: Query<&PlacedPortal>,
    mut depths: ResMut<PortalHostDepths>,
) {
    depths.0.clear();
    if portals.is_empty() {
        return;
    }
    let mut solids: Vec<ambition_engine_core::Aabb> = Vec::new();
    world
        .0
        .for_each_solid_aabb(false, &mut |aabb| solids.push(aabb));
    for portal in &portals {
        let depth = measure_host_depth(
            &solids,
            &portal.frame(),
            ambition_gameplay_core::portal::pieces::CARVE_DEPTH,
        );
        depths.0.push((portal.channel, depth));
    }
}
