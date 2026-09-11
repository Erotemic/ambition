//! The collision world a projectile flies through.
//!
//! A projectile does NOT see the same solids an actor does: it passes through
//! moving platforms and through breakable/ECS overlay solids, but it must stop
//! on gate solids (lock walls) exactly as it did when those lived in the
//! authored base, and it must fly THROUGH a portal aperture rather than detonate
//! on the wall the portal punched.
//!
//! Lives here (rather than woven into the actor-side stepper) since R3 made every
//! input plain: the authored room, the content-free `FeatureEcsWorldOverlay`, the
//! placed portals, and `ambition_platformer2d_world`'s composite builder. Fable's F2 named
//! this type as the one waiting on that follow-up.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use bevy::ecs::system::SystemParam;
#[cfg(feature = "portal")]
use bevy::prelude::Query;
use bevy::prelude::Res;

/// The portal-carved collision world a projectile collides against. Bundled as a
/// [`SystemParam`] so the stepper can build the carved world without adding two
/// more top-level params (it is already at Bevy's 16-param ceiling).
///
/// A portal punched through a wall leaves the opening non-solid, so a shot fired
/// into a wall portal flies THROUGH the opening instead of detonating on the wall
/// — and `portal_transit` (which already moves the projectile body) carries it
/// out the far portal. Without this the projectile collided against the raw world
/// and could never transit a wall portal.
#[derive(SystemParam)]
pub struct ProjectileCollisionWorld<'w, 's> {
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<'w, 's, RoomGeometry>,
    overlay: Res<'w, FeatureEcsWorldOverlay>,
    // Folded in here (rather than as its own top-level param) because the stepper
    // is already at Bevy's 16-param ceiling.
    #[cfg(feature = "portal")]
    portals: Query<'w, 's, &'static ambition_portal2d::PlacedPortal>,
    // ⭐ AND THE MAP CONVENTION, for the same reason the portals are here: the
    // stepper is at Bevy's parameter ceiling, and this is the session's portal
    // policy rather than a process global. `Option` because a composition
    // without the portal plugin has no tuning.
    #[cfg(feature = "portal")]
    tuning: Option<Res<'w, ambition_portal2d::PortalTuning>>,
}

impl ProjectileCollisionWorld<'_, '_> {
    /// The room world with gate solids (lock walls) AND live objects' contributed
    /// surfaces added, and ONLY the portal apertures carved out — preserving the
    /// projectile's historical raw-world collision (it passes through moving
    /// platforms) while letting a shot sink into a portal opening and transit.
    /// Borrowed (no clone) in the common no-gate, no-object, no-carve case.
    ///
    /// ⛔⛤ **CONTRIBUTED OBJECT SURFACES WERE EXCLUDED, AND Q96 RULED THAT WRONG.**
    /// This doc used to say a projectile *"passes through breakable/ECS overlay
    /// solids"*, and the exclusion was defended on the grounds that admitting them
    /// would make a solid crate invulnerable behind its own wall. The ruling
    /// (2026-09-10) rejects that answer by name: a published surface PARTICIPATES
    /// in projectile collision, and a contributor supplying both a surface and a
    /// damageable volume at one moment of impact yields ONE contact that damages
    /// once AND applies the physical response.
    ///
    /// ⇒ The surfaces are admitted here; the COALESCING that keeps the crate
    /// damageable lives at the contact ordering, where the target is known
    /// (`wall_is_the_targets_own_surface`). Admitting them without that rule is
    /// precisely the immunity the ruling forbids, so the two land together.
    pub fn solids(&self) -> std::borrow::Cow<'_, ae::World> {
        ambition_platformer2d_world::collision::world_with_contributed_solids_and_carves(
            &self.world.0,
            &self.overlay.gate_solids,
            &self.overlay.blocks,
            &self.overlay.portal_carves,
            &self.overlay.removed_block_names,
        )
    }

    /// The session's portal map convention.
    #[cfg(feature = "portal")]
    pub fn portal_convention(&self) -> ambition_portal2d::pieces::MapConvention {
        self.tuning
            .as_deref()
            .map(|tuning| tuning.convention.map_convention())
            .unwrap_or_default()
    }

    /// Snapshot the placed portals for the per-projectile transit test.
    #[cfg(feature = "portal")]
    pub fn portal_list(&self) -> Vec<ambition_portal2d::PlacedPortal> {
        self.portals.iter().cloned().collect()
    }
}
