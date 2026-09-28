//! Bridge placed portals to identified world faces.
//!
//! Attribution runs once per portal against uncarved hostable surfaces. After mover
//! integration, hosted portals refresh position, sweep state, and velocity from
//! their face anchor. A portal closes when its host face disappears; unhosted
//! fixtures remain static.

use bevy::prelude::*;

use ambition_portal2d::{PlacedPortal, PortalHost};

/// Attribution probe reach behind the placement point, in px. The gun lifts a
/// portal 2px proud of the hit face; authored specs sit on the face. The probe
/// must comfortably cross that lift plus float slack without reaching THROUGH
/// a thin wall to its far face (thinnest authored walls are ≥ 8px).
const HOST_ATTRIBUTE_REACH: f32 = 6.0;

/// Attach just-placed portals to the identified face they sit on.
///
/// Each portal is looked at once: the answer is written to
/// [`PlacedPortal::host`], so a portal that found no face is `Static` and is
/// not scanned again. A later scan would see moving platforms in other places
/// and could attach a portal that the confirmed timeline left static.
pub fn attach_portal_hosts(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    mut portals: Query<&mut PlacedPortal>,
) {
    if !portals.iter().any(|p| p.host == PortalHost::Unattributed) {
        return;
    }
    let Some(view) = collision.hostable_surfaces() else {
        return;
    };
    for mut portal in &mut portals {
        if portal.host != PortalHost::Unattributed {
            continue;
        }
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

/// Re-derive each hosted aperture frame from its current host face.
pub fn refresh_hosted_portal_frames(
    mut commands: Commands,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    time: Option<Res<ambition_time::WorldTime>>,
    mut portals: Query<(Entity, &mut PlacedPortal)>,
) {
    if !portals.iter().any(|(_, p)| p.host.face().is_some()) {
        return;
    }
    let Some(view) = collision.hostable_surfaces() else {
        return;
    };
    let dt = time.as_deref().map(|t| t.scaled_dt).unwrap_or(0.0);
    for (entity, mut portal) in &mut portals {
        let Some((host, lift)) = portal.host.face().map(|(face, lift)| (face.clone(), lift)) else {
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
