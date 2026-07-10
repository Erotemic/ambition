//! Ambition bridge: CC6 host attachment — portals ride identified geometry.
//!
//! Portal core owns the aperture MODEL (`PlacedPortal.host` — a
//! [`GeoFaceRef`](ambition_engine_core::GeoFaceRef), plus the derived
//! `pos`/`vel`/`prev_pos` caches). It never names the concrete composed world.
//! This adapter owns the two world-seam steps of the §5-P2 frame order:
//!
//! 1. **Attach** (lazy, once per placed portal): attribute the portal's
//!    placement point to the identified block face it sits on, in the
//!    UNCARVED authored + movers view. Attribution runs against
//!    `RoomGeometry` + moving platforms — never the carved composition: the
//!    carve replaces the host block with anonymous derived pieces, and a
//!    portal must anchor to the durable authored face (§3.6 rule 2).
//! 2. **Refresh** (each frame, §5-P2 step 2, after movers integrate and
//!    before eviction/carves/transit): re-derive the hosted aperture's
//!    `pos` from the face anchor, record `prev_pos` (the aperture's own
//!    sweep sample for the relative transit trigger), and derive `vel`
//!    (px/s, for the Galilean transfer map) from the host block's
//!    authoritative per-tick displacement. A hosted portal whose face is
//!    GONE from the composed view closes — a portal cannot exist without
//!    its host face (the eviction system then handles any straddler on the
//!    vanished plane).
//!
//! Unhosted portals (attribution found no identified face — fixtures,
//! anonymous fixture geometry) are left exactly as placed: zero velocity,
//! zero frame delta, byte-identical to the pre-CC6 portal.

use bevy::prelude::*;

use ambition_actors::world::platforms::world_with_moving_platforms;
use ambition_engine_core::RoomGeometry;
use ambition_portal::PlacedPortal;
use ambition_world::collision::MovingPlatformSet;

/// Attribution probe reach behind the placement point, in px. The gun lifts a
/// portal 2px proud of the hit face; authored specs sit on the face. The probe
/// must comfortably cross that lift plus float slack without reaching THROUGH
/// a thin wall to its far face (thinnest authored walls are ≥ 8px).
const HOST_ATTRIBUTE_REACH: f32 = 6.0;

/// Marker: host attribution ran for this portal (whatever the outcome).
/// Attribution is one-shot — a portal that failed to attach stays a static
/// aperture for its lifetime rather than re-scanning every frame.
#[derive(Component)]
pub struct PortalHostScanned;

/// Lazily attach just-placed portals to the identified face they sit on.
pub fn attach_portal_hosts(
    mut commands: Commands,
    room: Option<Res<RoomGeometry>>,
    platforms: Option<Res<MovingPlatformSet>>,
    mut portals: Query<(Entity, &mut PlacedPortal), Without<PortalHostScanned>>,
) {
    let Some(room) = room else { return };
    if portals.is_empty() {
        return;
    }
    let view = hostable_view(&room, platforms.as_deref());
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

/// §5-P2 step 2: re-derive each hosted aperture's frame from its host face.
pub fn refresh_hosted_portal_frames(
    mut commands: Commands,
    room: Option<Res<RoomGeometry>>,
    platforms: Option<Res<MovingPlatformSet>>,
    time: Option<Res<ambition_time::WorldTime>>,
    mut portals: Query<(Entity, &mut PlacedPortal)>,
) {
    let Some(room) = room else { return };
    if !portals.iter().any(|(_, p)| p.host.is_some()) {
        return;
    }
    let view = hostable_view(&room, platforms.as_deref());
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

/// The uncarved authored + movers view portals may anchor to.
fn hostable_view(
    room: &RoomGeometry,
    platforms: Option<&MovingPlatformSet>,
) -> ambition_engine_core::World {
    match platforms {
        Some(set) if !set.0.is_empty() => world_with_moving_platforms(&room.0, &set.0),
        _ => room.0.clone(),
    }
}
