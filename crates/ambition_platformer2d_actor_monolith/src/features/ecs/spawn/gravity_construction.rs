//! Room-side lowering into the gravity capability's construction domain.
//!
//! The gravity capability owns the construction vocabulary and the executable
//! constructor. This adapter owns the room-authoring join: it translates the
//! backend-neutral room spec and applies the same occurrence-continuity
//! decision the zone received when it still lived inside the actor construction
//! union.

use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::gravity::construction::{
    GravityZoneConstructionParams, GravityZoneConstructionRequest,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    OccurrenceDisposition, RoomOccurrenceOutlook,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Translate authored room gravity zones into the capability-owned construction
/// vocabulary. The actor domain never sees these parameters.
///
/// A suppressed authored occurrence stays suppressed. `Reinstated` preserves the
/// historical behaviour: a gravity zone publishes no placed whereabouts — it is
/// scenery, not an object a body can pick up and put down elsewhere — so the
/// authored position remains authoritative and the mismatch is loud rather than
/// silently fabricating relocation support.
pub(super) fn authored_requests(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    outlook: &RoomOccurrenceOutlook,
) -> Vec<GravityZoneConstructionRequest> {
    room.gravity_zones
        .iter()
        .filter_map(|zone| {
            let sim_id = SimId::placement(&zone.id);
            match outlook.disposition(&sim_id) {
                OccurrenceDisposition::Suppressed => return None,
                OccurrenceDisposition::Reinstated { at } => {
                    bevy::log::warn!(
                        target: "ambition_platformer2d::construction",
                        "room `{}` remembers gravity zone `{:?}` at a relocated position \
                         {at:?}, but gravity zones do not publish placed whereabouts; \
                         building it at its authored position",
                        room.id,
                        sim_id,
                    );
                }
                OccurrenceDisposition::Authored => {}
            }
            Some(GravityZoneConstructionRequest {
                sim_id,
                origin: SpawnOrigin::Authored {
                    source: room.id.clone(),
                    instance: zone.id.clone(),
                },
                parameters: GravityZoneConstructionParams {
                    name: zone.name.clone(),
                    center: zone.center,
                    half_extent: zone.half_extent,
                    dir: zone.dir,
                    oscillate_amplitude: zone.oscillate_amplitude,
                    oscillate_freq: zone.oscillate_freq,
                },
                relations: Vec::new(),
            })
        })
        .collect()
}
