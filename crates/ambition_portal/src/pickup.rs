//! The world-resting [`PortalGunPickup`] body and its arm-timer tick.
//!
//! The grant/drop *policy* (action-set stashing so `Attack` fires portals, and
//! reflecting ownership into the Ambition item roster) is Ambition inventory
//! glue and lives in the host portal adapter. Core
//! owns only the pickup body itself and the arming countdown — the simulation
//! of a portal gun resting in the world.

use bevy::prelude::*;

/// A portal gun resting in the world. Walking onto it and pressing `Attack`
/// activates the player's (inactive) portal gun — "pick up the portal gun in
/// a room". Kept distinct from `item_pickup::GroundItem` because the portal
/// gun's ability is the `PortalGun` component, not a `HeldItemSpec` verb.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGunPickup {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Seconds before this pickup can be grabbed. A *just-dropped* gun arms
    /// after a short delay so the same `Attack` press that dropped it (and the
    /// next overlapping frame) can't immediately re-grab it. World-placed
    /// pickups spawn already armed (`0.0`).
    pub arm_timer: f32,
}

// The portal gun is now an LDtk-authored `PortalGunSpawn` entity (spawned at
// room load via `spawn_room_feature_entities`); the old debug near-player
// spawner is retired.

/// Tick down each pickup's [`PortalGunPickup::arm_timer`] so a just-dropped gun
/// becomes grabbable after the short delay. Always runs (cheap; at most a
/// couple of pickups).
pub fn arm_portal_pickups(
    time: Res<ambition_platformer_primitives::time::SimDt>,
    mut pickups: Query<&mut PortalGunPickup>,
) {
    let dt = time.get();
    if dt <= 0.0 {
        return;
    }
    for mut pickup in &mut pickups {
        if pickup.arm_timer > 0.0 {
            pickup.arm_timer = (pickup.arm_timer - dt).max(0.0);
        }
    }
}
