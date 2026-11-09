//! The room geometry and aperture snapshot used by projectile contacts.

use ambition_platformer2d_core as ae;
use bevy::ecs::system::SystemParam;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, LiveRooms};
use bevy::prelude::Entity;
#[cfg(feature = "portal")]
use bevy::prelude::{Res, Query};

/// Room geometry stays in its physical chart. The material query scopes each
/// contact through the aperture snapshot.
#[derive(SystemParam)]
pub struct ProjectileCollisionWorld<'w, 's> {
    collision: ambition_platformer2d_world::collision::CollisionWorld<'w, 's>,
    /// Which live room a shot and each thing it may reach are in (OW1 cut
    /// 4). Here and not a top-level parameter, for the reason the portals
    /// are.
    live_rooms: LiveRooms<'w, 's>,
    // Folded in here (rather than as its own top-level param) because the stepper
    // is already at Bevy's 16-param ceiling.
    #[cfg(feature = "portal")]
    portals: Query<'w, 's, (Entity, &'static ambition_portal2d::PlacedPortal)>,
    #[cfg(feature = "portal")]
    passages: Option<Res<'w, ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>>,
    /// The pair each shot is on the back of: it does not take the shot, so
    /// the shot's leg does not end at it (`first_portal_crossing`).
    #[cfg(feature = "portal")]
    backsides: Query<'w, 's, &'static ambition_portal2d::PortalBackside>,
}

impl ProjectileCollisionWorld<'_, '_> {
    /// The room's authored and contributed solids. Material cells define portal
    /// openings during the query. The room geometry is not carved.
    pub fn solids_in(&self, room: Option<LiveRoomInstance>) -> Option<std::borrow::Cow<'_, ae::World>> {
        self.collision.room(room.map(InRoomInstance).as_ref())?.solids()
    }

    /// The aperture snapshot and policy for this shot's motion.
    pub fn motion_for(&self, shot: Entity, room: Option<LiveRoomInstance>) -> Option<ae::frame::thread::MotionAdmission<'_>> {
        #[cfg(feature = "portal")]
        { self.passages.as_deref().and_then(|p| p.motion_for(shot,room)) }
        #[cfg(not(feature = "portal"))]
        { let _ = (shot,room); None }
    }

    /// Admit the complete swept material before the projectile commits its step.
    #[cfg(feature = "portal")]
    pub fn admit_motion(
        &self,
        shot: Entity,
        room: Option<LiveRoomInstance>,
        before: ae::BodyKinematics,
        after: ae::BodyKinematics,
        time: std::ops::Range<f64>,
        travel: ae::LinearTravel,
        acceleration: ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration,
    ) -> Result<(), ae::frame::thread::Refusal> {
        let Some(admission) = self.motion_for(shot, room) else {
            return Ok(());
        };
        if admission.links.is_empty() { return Ok(()); }
        let map = before.material_map(ae::Vec2::ZERO).map_err(|reason| ae::frame::thread::Refusal {
            path: Vec::new(), link: None, reason: ae::frame::thread::RefusalReason::Numeric(reason),
        })?;
        let (delta, until) = travel.line(before.pos, after.pos).map_err(|reason| ae::frame::thread::Refusal {
            path: Vec::new(), link: None, reason: ae::frame::thread::RefusalReason::Numeric(reason),
        })?;
        let domain = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::ZERO, before.canonical_size * 0.5));
        let source = ae::frame::material_cast::MaterialCast::new(
            ae::frame::thread::MaterialPrimitive { domain: &domain, to_world: map },
            before.pos,
            delta,
            time,
        ).prefix(until)?;
        admission.admit(source, after.vel, &[acceleration.0])
    }

    /// The pair `shot` is on the back of, if any.
    #[cfg(feature = "portal")]
    pub fn backside_of(&self, shot: Entity) -> Option<ambition_portal2d::PortalBackside> {
        self.backsides.get(shot).ok().copied()
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

    /// Snapshot the placed portals for the per-projectile transit test,
    /// grouped by live room. A shot threads only the portals of its own room
    /// (`shot_room`): a pair in one room does not carry a shot of another
    /// room that flies through the same coordinates (OW1).
    #[cfg(feature = "portal")]
    pub fn portals_by_room(&self) -> ambition_portal2d::PortalsByRoom {
        ambition_portal2d::PortalsByRoom::collect(self.portals.iter(), &self.live_rooms)
    }
}
