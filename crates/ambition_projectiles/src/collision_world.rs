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
//! placed portals, and `ambition_world`'s composite builder. Fable's F2 named
//! this type as the one waiting on that follow-up.

use ambition_engine_core as ae;
use ambition_engine_core::RoomGeometry;
use ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Query, Res};

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
    world: Res<'w, RoomGeometry>,
    overlay: Res<'w, FeatureEcsWorldOverlay>,
    // Folded in here (rather than as its own top-level param) because the stepper
    // is already at Bevy's 16-param ceiling.
    portals: Query<'w, 's, &'static ambition_portal::PlacedPortal>,
}

impl ProjectileCollisionWorld<'_, '_> {
    /// The room world with gate solids (lock walls) added and ONLY the portal
    /// apertures carved out — preserves the projectile's historical raw-world
    /// collision (it passes through moving platforms) while still colliding with
    /// gate solids and letting a shot sink into a portal opening and transit.
    /// Borrowed (no clone) in the common no-gate, no-carve case.
    pub fn solids(&self) -> std::borrow::Cow<'_, ae::World> {
        ambition_world::collision::world_with_gate_solids_and_carves(
            &self.world.0,
            &self.overlay.gate_solids,
            &self.overlay.portal_carves,
            &self.overlay.removed_block_names,
        )
    }

    /// Snapshot the placed portals for the per-projectile transit test.
    pub fn portal_list(&self) -> Vec<ambition_portal::PlacedPortal> {
        self.portals.iter().cloned().collect()
    }
}
