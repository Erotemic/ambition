//! Compatibility pickup for Ambition's portal-gun workflow.
//!
//! Not part of the portal core: portals can be static, scripted, moving, or
//! opened by any emitter. Pickup and equip details stay in this module.

use bevy::prelude::*;

/// A portal gun lying in the world. A host adapter decides who can pick it up
/// and how that maps to inventory and abilities.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGunPickup {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Which portal pair the gun in this pickup owns (see [`PortalGun`]). The
    /// pickup carries it so a dropped gun keeps its pair.
    ///
    /// [`PortalGun`]: crate::PortalGun
    pub pair: u8,
    /// A just-dropped gun arms after a short delay, so the `Attack` press that
    /// dropped it cannot grab it again. World-placed pickups start armed (`0.0`).
    pub arm_timer: f32,
}

// FIXME(portal-gun-seam): this pickup is an Ambition compatibility artifact. A
// standalone portal crate should expose generic portal-openers and leave pickup
// / inventory policy entirely host-side.

/// The set [`arm_portal_pickups`] runs in.
///
/// The arming pass (here) and the Ambition inventory grant (host) are in
/// different crates, but must run in that order inside
/// `ItemPickupSet::CoreHeldItems`. The set has one member: this crate does not
/// know `pickup_portal_gun_system`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalPickupArming;

/// Tick down each pickup's [`PortalGunPickup::arm_timer`]. Always runs; it is
/// cheap.
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
