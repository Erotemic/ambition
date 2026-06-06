use bevy::prelude::*;

/// Portal-owned schedule labels.
///
/// These labels are intentionally local to the portal subsystem. External
/// systems should order against them only when they have a real semantic
/// dependency on portal behavior; otherwise use the broader app-level sets.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PortalSet {
    /// Publish portal collision carves (after the gravity-zone snapshot owned by
    /// `crate::mechanics::gravity::GravityPlugin`).
    Carves,
    /// Input rewrites that happen before the player input frame is synced.
    InputWarp,
    /// Ambition input → portal intent translation (the content adapter), run
    /// before the weapon/projectile consumers so the intents are visible the
    /// same frame.
    InputAdapter,
    /// Fire, toggle, and projectile systems (gameplay-gated by the host).
    WeaponAndProjectiles,
    /// Ownership maintenance that must run even when gameplay is not allowed
    /// (orphan-portal cleanup + aerial-roll readiness). Chained after
    /// [`PortalSet::WeaponAndProjectiles`]; the host does NOT gate this set,
    /// preserving the pre-extraction behavior where `despawn_orphaned_portals`
    /// and `ensure_actor_roll` ran unconditionally inside `PlayerSimulation`.
    WeaponMaintenance,
    /// Reset-time portal and gravity cleanup.
    RoomReset,
    /// Temporary ability suppression while crossing a portal aperture.
    TransitGuards,
    /// PlacedPortal cooldown, body transit, item transit, and actor roll updates.
    Transit,
}
