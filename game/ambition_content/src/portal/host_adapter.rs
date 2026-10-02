//! Bridge placed portals to identified world faces.
//!
//! Attribution runs once per portal against uncarved hostable surfaces. After mover
//! integration, hosted portals refresh position, sweep state, and velocity from
//! their face anchor. A portal closes when its host face disappears; unhosted
//! fixtures remain static.

use bevy::prelude::*;

use std::borrow::Cow;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRooms};
use ambition_portal2d::{PlacedPortal, PortalHost, PortalRoom};

/// Attribution probe reach behind the placement point, in px. The gun lifts a
/// portal 2px proud of the hit face; authored specs sit on the face. The probe
/// must comfortably cross that lift plus float slack without reaching THROUGH
/// a thin wall to its far face (thinnest authored walls are ≥ 8px).
const HOST_ATTRIBUTE_REACH: f32 = 6.0;

/// The hostable surfaces of each live room a portal asks for, built once per
/// room (OW1). A portal finds its host in its OWN live room: the sole live
/// room was read here, so while two rooms were live no portal attached and no
/// hosted portal followed its face.
struct SurfacesByRoom<'a> {
    collision: &'a ambition_platformer2d_world::collision::CollisionWorld<'a, 'a>,
    built: Vec<(PortalRoom, Option<Cow<'a, ae::World>>)>,
}

impl<'a> SurfacesByRoom<'a> {
    fn new(collision: &'a ambition_platformer2d_world::collision::CollisionWorld<'a, 'a>) -> Self {
        Self { collision, built: Vec::new() }
    }

    /// The hostable surfaces of `room`, `None` when it is no live room. A
    /// portal with no room reads the sole live room, as `CollisionWorld::room`
    /// does: with two live rooms that is none.
    fn of(&mut self, room: PortalRoom) -> Option<&ae::World> {
        let index = match self.built.iter().position(|(seen, _)| *seen == room) {
            Some(index) => index,
            None => {
                let surfaces = self
                    .collision
                    .room(room.map(InRoomInstance).as_ref())
                    .and_then(|collision| collision.hostable_surfaces());
                self.built.push((room, surfaces));
                self.built.len() - 1
            }
        };
        self.built[index].1.as_deref()
    }
}

/// Attach just-placed portals to the identified face they sit on.
///
/// Each portal is looked at once: the answer is written to
/// [`PlacedPortal::host`], so a portal that found no face is `Static` and is
/// not scanned again. A later scan would see moving platforms in other places
/// and could attach a portal that the confirmed timeline left static.
///
/// The face is looked for in the portal's own live room (OW1). A portal in no
/// live room is not looked at while two rooms are live.
pub fn attach_portal_hosts(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: LiveRooms,
    mut portals: Query<(Entity, &mut PlacedPortal)>,
) {
    if !portals.iter().any(|(_, p)| p.host == PortalHost::Unattributed) {
        return;
    }
    let mut surfaces = SurfacesByRoom::new(&collision);
    for (entity, mut portal) in &mut portals {
        if portal.host != PortalHost::Unattributed {
            continue;
        }
        let Some(view) = surfaces.of(live.of(entity)) else {
            continue;
        };
        // Probe into the face the portal was placed against.
        let probe = portal.pos - portal.normal * HOST_ATTRIBUTE_REACH * 0.5;
        let host = view
            .attribute_face(probe, portal.normal, HOST_ATTRIBUTE_REACH)
            .and_then(|face| {
                let anchor = view.resolve_face(&face)?;
                // The lift makes the per-frame re-derivation give back the
                // placement pose exactly, so a static host writes the same `pos`.
                let lift = (portal.pos - anchor.origin).dot(portal.normal);
                Some(PortalHost::Face { face, lift })
            })
            .unwrap_or(PortalHost::Static);
        portal.host = host;
    }
}

/// Re-derive each hosted aperture frame from its current host face, in the
/// portal's own live room (OW1). A portal in no live room keeps its frame
/// while two rooms are live.
pub fn refresh_hosted_portal_frames(
    mut commands: Commands,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    live: LiveRooms,
    time: Option<Res<ambition_time::WorldTime>>,
    mut portals: Query<(Entity, &mut PlacedPortal)>,
) {
    if !portals.iter().any(|(_, p)| p.host.face().is_some()) {
        return;
    }
    let mut surfaces = SurfacesByRoom::new(&collision);
    let dt = time.as_deref().map(|t| t.sim_dt()).unwrap_or(0.0);
    for (entity, mut portal) in &mut portals {
        let Some((host, lift)) = portal.host.face().map(|(face, lift)| (face.clone(), lift)) else {
            continue;
        };
        let Some(view) = surfaces.of(live.of(entity)) else {
            continue;
        };
        let Some(anchor) = view.resolve_face(&host) else {
            // The host face left the world: the portal closes with its wall.
            // Eviction sees the vanished plane and clears any straddler.
            commands.entity(entity).despawn();
            continue;
        };
        let new_pos = anchor.origin + portal.normal * lift;
        portal.prev_pos = portal.pos;
        portal.pos = new_pos;
        // The host block's `velocity` is the kernels' surface_velocity
        // convention: the authoritative PER-TICK displacement the mover
        // published (never finite-differenced from our own positions).
        // The frame map wants px/s.
        portal.vel = if dt > 0.0 {
            anchor.velocity / dt
        } else {
            Vec2::ZERO
        };
    }
}
