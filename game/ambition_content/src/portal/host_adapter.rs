//! Bridge placed portals to identified world faces.
//!
//! Attribution runs once against uncarved hostable surfaces. After mover
//! integration, hosted portals refresh position, sweep state, and velocity from
//! their face anchor. A portal closes when its host face disappears; unhosted
//! fixtures remain static.

use bevy::prelude::*;

use ambition_portal2d::PlacedPortal;

/// Attribution probe reach behind the placement point, in px. The gun lifts a
/// portal 2px proud of the hit face; authored specs sit on the face. The probe
/// must comfortably cross that lift plus float slack without reaching THROUGH
/// a thin wall to its far face (thinnest authored walls are ≥ 8px).
const HOST_ATTRIBUTE_REACH: f32 = 6.0;

/// Marker: host attribution ran for this portal (whatever the outcome).
/// Attribution is one-shot — a portal that failed to attach stays a static
/// aperture for its lifetime rather than re-scanning every frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalHostScanned;

/// Lazily attach just-placed portals to the identified face they sit on.
pub fn attach_portal_hosts(
    mut commands: Commands,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    mut portals: Query<(Entity, &mut PlacedPortal), Without<PortalHostScanned>>,
) {
    if portals.is_empty() {
        return;
    }
    let Some(view) = collision.hostable_surfaces() else {
        return;
    };
    for (entity, mut portal) in &mut portals {
        // Probe into the face the portal was placed against.
        let probe = portal.pos - portal.normal * HOST_ATTRIBUTE_REACH * 0.5;
        if let Some(face_ref) = view.attribute_face(probe, portal.normal, HOST_ATTRIBUTE_REACH) {
            if let Some(anchor) = view.resolve_face(&face_ref) {
                // Record the authored lift so the per-frame re-derivation
                // reproduces the placement pose exactly (parity for static
                // hosts: refresh writes back the identical `pos`).
                portal.host_lift = (portal.pos - anchor.origin).dot(portal.normal);
                portal.host = Some(face_ref);
            }
        }
        commands.entity(entity).insert(PortalHostScanned);
    }
}

/// Re-derive each hosted aperture frame from its current host face.
pub fn refresh_hosted_portal_frames(
    mut commands: Commands,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    time: Option<Res<ambition_time::WorldTime>>,
    mut portals: Query<(Entity, &mut PlacedPortal)>,
) {
    if !portals.iter().any(|(_, p)| p.host.is_some()) {
        return;
    }
    let Some(view) = collision.hostable_surfaces() else {
        return;
    };
    let dt = time.as_deref().map(|t| t.scaled_dt).unwrap_or(0.0);
    for (entity, mut portal) in &mut portals {
        let Some(host) = portal.host.clone() else {
            continue;
        };
        let Some(anchor) = view.resolve_face(&host) else {
            // The host face left the world: the portal closes with its wall.
            // Eviction sees the vanished plane and clears any straddler.
            commands.entity(entity).despawn();
            continue;
        };
        let new_pos = anchor.origin + portal.normal * portal.host_lift;
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
