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
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, RoomInstanceRoot};
use bevy::prelude::{Entity, Query, With};
#[cfg(feature = "portal")]
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
    /// Every live room's identity, geometry and collision overlay, each
    /// tuple off ONE root.
    rooms: Query<
        'w,
        's,
        (
            Option<&'static LiveRoomInstance>,
            &'static RoomGeometry,
            &'static FeatureEcsWorldOverlay,
        ),
        With<RoomInstanceRoot>,
    >,
    /// Which live room a shot and each thing it may reach are in (OW1 cut
    /// 4). Here and not a top-level parameter, for the reason the portals
    /// are.
    live_rooms: LiveRooms<'w, 's>,
    // Folded in here (rather than as its own top-level param) because the stepper
    // is already at Bevy's 16-param ceiling.
    #[cfg(feature = "portal")]
    portals: Query<'w, 's, (Entity, &'static ambition_portal2d::PlacedPortal)>,
    // ⭐ AND THE MAP CONVENTION, for the same reason the portals are here: the
    // stepper is at Bevy's parameter ceiling, and this is the session's portal
    // policy rather than a process global. `Option` because a composition
    // without the portal plugin has no tuning.
    #[cfg(feature = "portal")]
    tuning: Option<Res<'w, ambition_portal2d::PortalTuning>>,
}

impl ProjectileCollisionWorld<'_, '_> {
    /// The room world with gate solids (lock walls) and live objects' contributed
    /// surfaces added, and only the portal apertures carved out. Projectiles still
    /// pass through moving platforms, and a shot can sink into a portal opening
    /// and transit. Borrowed (no clone) when there is no gate, object or carve.
    ///
    /// Contributed object surfaces take part in projectile collision (ruling Q96).
    /// When a contributor supplies both a surface and a damageable volume, the
    /// impact is one contact that damages once and applies the physical response.
    /// That merge happens at contact ordering (`wall_is_the_targets_own_surface`);
    /// without it, a solid crate would be immune behind its own wall.
    ///
    /// It is the world of the live room `room` names (`None`: the sole live
    /// room), and `None` when that room is not live. See
    /// `CollisionWorld::room` for the rule.
    pub fn solids_in(&self, room: Option<LiveRoomInstance>) -> Option<std::borrow::Cow<'_, ae::World>> {
        let mut matching = self
            .rooms
            .iter()
            .filter(|(live, ..)| room.is_none() || live.copied() == room);
        let (_, geometry, overlay) = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        Some(
            ambition_platformer2d_world::collision::world_with_contributed_solids_and_carves(
                &geometry.0,
                &overlay.gate_solids,
                &overlay.blocks,
                &overlay.portal_carves,
                &overlay.removed_block_names,
            ),
        )
    }

    /// The live room a shot flies in: its own stamp, else its firer's room.
    pub fn shot_room(&self, shot: Entity, owner: Option<Entity>) -> Option<LiveRoomInstance> {
        self.live_rooms
            .stamped(shot)
            .or_else(|| self.live_rooms.of(owner.unwrap_or(shot)))
    }

    /// The live room a thing a shot may reach is in.
    pub fn room_of(&self, entity: Entity) -> Option<LiveRoomInstance> {
        self.live_rooms.of(entity)
    }

    /// The session's portal map convention.
    #[cfg(feature = "portal")]
    pub fn portal_convention(&self) -> ambition_portal2d::pieces::MapConvention {
        self.tuning
            .as_deref()
            .map(|tuning| tuning.convention.map_convention())
            .unwrap_or_default()
    }

    /// Snapshot the placed portals for the per-projectile transit test,
    /// grouped by live room. A shot threads only the portals of its own room
    /// (`shot_room`): a pair in one room does not carry a shot of another
    /// room that flies through the same coordinates (OW1).
    #[cfg(feature = "portal")]
    pub fn portals_by_room(&self) -> ambition_portal2d::PortalsByRoom {
        ambition_portal2d::PortalsByRoom::collect(self.portals.iter(), &self.live_rooms)
    }
}
