//! Room-side lowering into the portal-gun capability construction domain.
//!
//! The portal capability owns the construction vocabulary and executable
//! constructor. This adapter owns the room-authoring join: it translates the
//! backend-neutral room spec and applies the same occurrence-continuity decision
//! the pickup received when it still lived inside the actor construction union.

use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::{
    OccurrenceDisposition, RoomOccurrenceOutlook,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Translate authored room portal-gun placements into the capability-owned
/// construction vocabulary. The actor domain never sees these parameters.
///
/// A suppressed authored occurrence stays suppressed. `Reinstated` currently
/// preserves the historical behavior: portal-gun pickups have no placement
/// producer/reinstatement road, so the authored position remains authoritative
/// and the mismatch is loud rather than silently fabricating relocation support.
pub(super) fn authored_requests(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    outlook: &RoomOccurrenceOutlook,
) -> Vec<ambition_portal2d::PortalGunConstructionRequest> {
    room.portal_gun_spawns
        .iter()
        .filter_map(|gun| {
            let sim_id = SimId::placement(&gun.id);
            match outlook.disposition(&sim_id) {
                OccurrenceDisposition::Suppressed => return None,
                OccurrenceDisposition::Reinstated { at } => {
                    bevy::log::warn!(
                        target: "ambition_platformer2d::construction",
                        "room `{}` remembers portal-gun occurrence `{:?}` at a relocated \
                         position {at:?}, but portal-gun pickups do not yet publish placed \
                         whereabouts; building it at its authored position",
                        room.id,
                        sim_id,
                    );
                }
                OccurrenceDisposition::Authored => {}
            }
            Some(ambition_portal2d::PortalGunConstructionRequest {
                sim_id,
                origin: SpawnOrigin::Authored {
                    source: room.id.clone(),
                    instance: gun.id.clone(),
                },
                parameters: ambition_portal2d::PortalGunConstructionParams {
                    name: gun.name.clone(),
                    pos: gun.pos,
                    half_extent: gun.half_extent,
                },
                relations: Vec::new(),
            })
        })
        .collect()
}
