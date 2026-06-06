use bevy::prelude::*;

/// Portal-owned schedule labels.
///
/// These labels are intentionally local to the portal subsystem. External
/// systems should order against them only when they have a real semantic
/// dependency on portal behavior; otherwise use the broader app-level sets.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PortalSet {
    /// Snapshot gravity zones and publish portal collision carves.
    GravityAndCarves,
    /// Input rewrites that happen before the player input frame is synced.
    InputWarp,
    /// Fire, toggle, projectile, and ownership maintenance systems.
    WeaponAndProjectiles,
    /// Reset-time portal and gravity cleanup.
    RoomReset,
    /// Temporary ability suppression while crossing a portal aperture.
    TransitGuards,
    /// PlacedPortal cooldown, body transit, item transit, and actor roll updates.
    Transit,
}
