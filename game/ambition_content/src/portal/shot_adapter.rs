//! Ambition world-seam adapter for the in-flight portal shot.
//!
//! Portal core's [`step_portal_shot`] is a pure helper over the reusable
//! [`SolidWorldQuery`](ambition_actors::platformer_runtime::collision::SolidWorldQuery)
//! seam (+ world bounds): it decides whether a shot travels, places a portal, or
//! fizzles, without ever reading the concrete `Res<RoomGeometry>`. This adapter owns
//! the concrete world — it reads `Res<RoomGeometry>`, calls the helper per shot, and
//! applies the [`PortalShotStep`] outcome (entity spawn/despawn + sfx). Moving
//! the `RoomGeometry` read here keeps portal core's projectile step content-free.

use bevy::prelude::*;

use ambition_actors::platformer_runtime::prelude::SpawnScopedExt;
use ambition_engine_core::RoomGeometry;
use ambition_portal::{
    portal_half_extent, step_portal_shot, PlacedPortal, PortalShot, PortalShotStep, PortalShotWorld,
};

/// Advance portal shots against the concrete collision world. For each shot,
/// call the pure [`step_portal_shot`] over the `RoomGeometry`'s solids + bounds and
/// apply the outcome: open (or replace) the portal of the shot's color on a
/// placeable surface (the warping whoosh + close/attach sfx), or fizzle past
/// range / out of bounds / on a non-placeable surface (the rejection buzz).
pub fn portal_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: Res<RoomGeometry>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut PortalShot)>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let seam = PortalShotWorld {
        solids: &world.0,
        size: world.0.size,
    };
    for (proj_entity, mut proj) in &mut projectiles {
        match step_portal_shot(&proj, &seam, dt) {
            PortalShotStep::Travel {
                pos,
                traveled_delta,
            } => {
                proj.pos = pos;
                proj.traveled += traveled_delta;
            }
            PortalShotStep::Place {
                channel,
                pos,
                normal,
                hit,
            } => {
                // Hit a wall — open (or replace) the portal of this color.
                for (entity, portal) in &portals {
                    if portal.channel == channel {
                        commands.entity(entity).despawn();
                        sfx.write(ambition_sfx::SfxMessage::Play {
                            id: ambition_sfx::ids::PORTAL_CLOSE,
                            pos: hit,
                        });
                    }
                }
                commands.spawn_room_scoped((
                    PlacedPortal::fixed(channel, pos, normal, portal_half_extent(normal)),
                    Name::new(format!("Portal: {}", channel.name())),
                    // Portals are per-room: a room transition despawns them, so
                    // they don't linger and reappear when you leave and come back
                    // (#41).
                ));
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_ATTACH,
                    pos: hit,
                });
                commands.entity(proj_entity).despawn();
            }
            PortalShotStep::Fizzle { pos } => {
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_INVALID,
                    pos,
                });
                commands.entity(proj_entity).despawn();
            }
        }
    }
}
