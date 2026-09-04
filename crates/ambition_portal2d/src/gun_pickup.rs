//! Compatibility pickup for Ambition's portal-gun workflow.
//!
//! This is not part of the mathematical portal seam: portals may be static,
//! scripted, moving, or opened by arbitrary emitters. Keep pickup/equip details
//! in this sequestered module so the public portal API can evolve toward
//! topology, placement, transit, and view math without requiring a gun.

use bevy::prelude::*;

/// A portal gun resting in the world for the current Ambition compatibility
/// workflow. A host adapter decides which controlled actor can pick it up and
/// how that grant maps into inventory / abilities.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGunPickup {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Which portal pair the gun in this pickup owns — see [`PortalGun`].
    ///
    /// ⭐ **THE PAIR IS A PROPERTY OF THE GUN, SO IT HAS TO SURVIVE THE FLOOR.**
    /// A gun is a world pickup between being dropped and being picked up again;
    /// if the pickup did not carry the pair, every drop would silently reset a
    /// red/yellow gun to the default blue/orange, and the pair a level author
    /// placed would last exactly until the player fumbled it.
    ///
    /// [`PortalGun`]: crate::PortalGun
    pub pair: u8,
    /// A *just-dropped* gun arms after a short delay so the same `Attack` press that dropped it
    /// (and the next overlapping frame) can't immediately re-grab it. World-placed pickups
    /// spawn already armed (`0.0`).
    pub arm_timer: f32,
}

// FIXME(portal-gun-seam): this pickup is an Ambition compatibility artifact. A
// standalone portal crate should expose generic portal-openers and leave pickup
// / inventory policy entirely host-side.

/// The set [`arm_portal_pickups`] runs in.
///
/// The arming pass and the Ambition inventory GRANT live in different crates by design — this one
/// is generic portal code, the grant knows about Ambition items — but they must run in that order
/// inside `ItemPickupSet::CoreHeldItems`.
///
/// ONE member, and the split is the point: this crate must not learn what
/// `pickup_portal_gun_system` is, so the set can only ever hold the generic half.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalPickupArming;

/// Tick down each pickup's [`PortalGunPickup::arm_timer`] so a just-dropped gun
/// becomes grabbable after the short delay. Always runs (cheap; at most a
/// couple of pickups).
pub fn arm_portal_pickups(
    time: Res<ambition_platformer2d_shared_tangle::time::SimDt>,
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
