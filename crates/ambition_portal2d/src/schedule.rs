//! The portal-owned [`PortalSet`] schedule labels (carves, input warp, weapon,
//! transit, room-reset ordering). Host systems order against these only on a
//! real portal dependency; [`PortalSimulationPlugin`](crate::PortalSimulationPlugin)
//! wires the simulation systems into them.

use bevy::prelude::*;

/// Portal-owned schedule labels.
///
/// Order external systems against these only for a real dependency on portal
/// behavior; otherwise use the app-level sets.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PortalSet {
    /// Publish portal collision carves (the host orders this against any
    /// early-world snapshot it needs, e.g. a gravity-zone snapshot).
    Carves,
    /// Input rewrites that happen before the host input frame is synced.
    InputWarp,
    /// Host input to portal intent (a host adapter). Runs before the weapon
    /// and projectile consumers, in the same frame.
    InputAdapter,
    /// Fire, toggle, and projectile systems (gameplay-gated by the host).
    WeaponAndProjectiles,
    /// Ownership maintenance that runs even when gameplay is gated: orphan cleanup and
    /// aerial-roll readiness. Chained after [`PortalSet::WeaponAndProjectiles`].
    WeaponMaintenance,
    /// Reset-time portal and gravity cleanup.
    RoomReset,
    /// Temporary ability suppression while crossing a portal aperture.
    TransitGuards,
    /// This tick's portal frames: link resolution, aperture equalisation, and
    /// eviction of bodies straddling a plane that moved or closed. Eviction
    /// moves bodies, so a host places this set with its other carries, before
    /// anything reads the tick's travelled path. [`PortalSet::Transit`] follows
    /// it.
    Frame,
    /// PlacedPortal cooldown, body transit, item transit, and actor roll updates.
    Transit,
}
