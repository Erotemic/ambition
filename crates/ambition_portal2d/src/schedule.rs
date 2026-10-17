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
    /// This tick's apertures: each hosted portal takes its host's pose, then
    /// links resolve and each pair's opening is equalised. A host places it
    /// after its moving hosts advance and before anything collides with or
    /// crosses an aperture, so the carve, the collision world and the
    /// crossing all see one pose. [`PortalSet::Carves`] follows it.
    Pose,
    /// Publish portal collision carves, after [`PortalSet::Pose`] (the host
    /// also orders this against any early-world snapshot it needs, e.g. a
    /// gravity-zone snapshot).
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
    /// Eviction of bodies straddling a plane that moved or closed since the
    /// last tick. Eviction moves bodies, so a host places this set with its
    /// other carries, before anything reads the tick's travelled path.
    /// [`PortalSet::Transit`] follows it.
    Frame,
    /// PlacedPortal cooldown, body transit, item transit, and actor roll updates.
    Transit,
    /// The consequences of this tick's body transits, in [`PortalSet::Transit`]
    /// after [`portal_transit`](crate::portal_transit) has moved the bodies. A
    /// host that reads `PortalBodyTransited` puts its system here, so it does
    /// not name the core's system.
    Transited,
    /// This tick's pairs and straddlers published for other domains
    /// ([`PortalPassages`](ambition_platformer2d_shared_tangle::portal_passages::PortalPassages)),
    /// after [`PortalSet::Transit`]. A host places it before anything that
    /// asks where a volume is (combat).
    Passages,
}
