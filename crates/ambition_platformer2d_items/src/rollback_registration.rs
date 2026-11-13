//! Register domain state for rollback.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone::<crate::pickups::PickupFeature>(OWNER, "feature.pickup");
    registrar.rollback_component_clone::<crate::pickups::Collected>(OWNER, "feature.collected");
    registrar.rollback_resource_clone_checksum::<crate::pickups::ConsumedSinceCheckpoint>(
        OWNER,
        "feature.consumed_since_checkpoint",
        "the one-time pickups consumed and the authored chests opened since the last checkpoint, with their room and whose horizons own each (Q151)",
        crate::pickups::ConsumedSinceCheckpoint::checksum,
    );
    registrar.rollback_resource_clone_checksum::<crate::narrative::BagUsesInRooms>(
        OWNER,
        "item.bag_uses_in_rooms",
        "the cells bodies used from the shared bag, and what a room's conversation added to it or sold from it, within the rewind horizon, with the room and tick of each, which a room rewind takes back (ISL-2d)",
        crate::narrative::BagUsesInRooms::checksum,
    );
    registrar.rollback_component_clone::<crate::pickups::PickupCollectLock>(
        OWNER,
        "feature.pickup_collect_lock",
    );
    registrar.rollback_component_clone::<crate::pickups::PickupArt>(OWNER, "feature.pickup_art");
    registrar.rollback_component_clone::<crate::pickups::ReturnedCoins>(OWNER, "feature.returned_coins");
    registrar.rollback_component_clone_probed::<crate::pickups::PickupMagnet>(
        OWNER,
        "item.pickup_magnet",
        |magnet| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            magnet.range.to_bits().hash(&mut hasher);
            magnet.speed.to_bits().hash(&mut hasher);
            hasher.finish()
        },
    );
}
