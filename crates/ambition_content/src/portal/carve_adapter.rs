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
use ambition_gameplay_core::portal::PortalCarves;

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
