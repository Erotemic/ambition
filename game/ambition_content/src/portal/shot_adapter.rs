//! Ambition world-seam adapter for the in-flight portal shot.
//!
//! Portal core's [`step_portal_shot`] is a pure helper over the reusable
//! [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery)
//! seam (+ world bounds): it decides whether a shot travels, places a portal, or
//! fizzles, without ever reading the concrete `RoomGeometry`. This adapter owns
//! the concrete world: it reads the `RoomGeometry` of each shot's own live
//! room (`LiveRoomOf`), calls the helper per shot, and applies the
//! [`PortalShotStep`] outcome (entity spawn/despawn + sfx). Moving the
//! `RoomGeometry` read here keeps portal core's projectile step content-free.

use bevy::prelude::*;

use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d::platformer::lifecycle::{SessionCommands, SpawnSessionScopedExt};
use ambition_portal2d::{
    step_portal_shot, PlacedPortal, PortalChannel, PortalShot, PortalShotStep,
    PortalShotWorld,
};

/// Step portal shots against their live room geometry.
///
/// At most one portal of each channel may remain per room. Resolve
/// same-tick placements deterministically: newer shots win (less
/// distance traveled), then geometry breaks ties. Apply placement only
/// inside the shot's room; a shot without a live room does not move.
/// Fizzles and successful placements emit different sound requests.
///
/// ⛔ Two portals never share a place ([`PlacedPortal::opening_overlaps`]):
/// a placement whose opening would overlap a portal of another channel in
/// its room (one already open, or one this tick opened first) fizzles, and
/// the portal its channel had stays.
pub fn portal_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    mut commands: SessionCommands,
    mut projectiles: Query<(Entity, &mut PortalShot)>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // The session that owns each portal a shot opens. A shot is an entity of
    // a session, so with no session there is no shot to step.
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    // Every placement this tick, decided before any of them is applied.
    let mut placements: Vec<Placement> = Vec::new();
    for (proj_entity, mut proj) in &mut projectiles {
        let Some(geometry) = world.of(proj_entity) else {
            continue;
        };
        let seam = PortalShotWorld {
            solids: &geometry.0,
            size: geometry.0.size,
        };
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
                placements.push(Placement {
                    channel,
                    pos,
                    normal,
                    hit,
                    traveled: proj.traveled,
                    room: world.room_of(proj_entity),
                });
                // The shot is spent whether or not its placement wins the channel.
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

    // Newest first across every channel, then geometry — a total order, so the
    // walk below picks the same winner for each channel on every machine and on
    // every resimulation of this tick.
    placements.sort_by(Placement::newest_first);
    let mut opened: Vec<(ambition_portal2d::PortalRoom, PortalChannel)> = Vec::new();
    let mut opened_here: Vec<(ambition_portal2d::PortalRoom, PlacedPortal)> = Vec::new();
    for winner in &placements {
        if opened.contains(&(winner.room, winner.channel)) {
            // A superseded same-tick placement makes no sound of its own: exactly
            // one portal opened on this channel in this room, so exactly one
            // attach cue plays.
            continue;
        }
        opened.push((winner.room, winner.channel));
        let opening = PlacedPortal::fixed(winner.channel, winner.pos, winner.normal, ambition_portal2d::PORTAL_OPENING_HALF);
        let overlapped = portals
            .iter()
            .filter(|(entity, portal)| portal.channel != winner.channel && world.room_of(*entity) == winner.room)
            .map(|(_, portal)| portal)
            .chain(opened_here.iter().filter(|(room, _)| *room == winner.room).map(|(_, portal)| portal))
            .any(|portal| opening.opening_overlaps(portal));
        if overlapped {
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_INVALID,
                pos: winner.hit,
            });
            continue;
        }
        opened_here.push((winner.room, opening.clone()));
        // Hit a wall — open (or replace) the portal of this color in this room.
        for (entity, portal) in &portals {
            if portal.channel == winner.channel && world.room_of(entity) == winner.room {
                commands.entity(entity).despawn();
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_CLOSE,
                    pos: winner.hit,
                });
            }
        }
        let mut portal = commands.spawn_room_in_session(
            scope,
            (
                opening,
                Name::new(format!("Portal: {}", winner.channel.name())),
                // One portal per channel per room: a derived identity, so the same
                // wall re-placed is the same logical object and a rewind can name
                // it (S4).
                ambition_platformer2d::platformer::sim_id::SimId::singleton(
                    "portal",
                    &winner.channel.name(),
                ),
                // Portals are per-room: a room transition despawns them, so
                // they don't linger and reappear when you leave and come back
                // (#41).
            ),
        );
        if let Some(room) = winner.room {
            portal.insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(room));
        }
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PORTAL_ATTACH,
            pos: winner.hit,
        });
    }
}

/// One shot's decision to open a portal, held until every shot has decided.
struct Placement {
    channel: PortalChannel,
    pos: Vec2,
    normal: Vec2,
    hit: Vec2,
    /// The shot's distance covered before this tick. Speed is constant, so a
    /// SMALLER value is a more recently fired shot.
    traveled: f32,
    /// The live room of the shot, which the portal it opens is in.
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

impl Placement {
    /// The total order that picks a channel's winner: newest first, then geometry.
    ///
    /// Total across CHANNELS too, deliberately. Two placements on different
    /// channels never contest each other, but ordering the whole list once is what
    /// makes the walk that applies them repeatable rather than query-ordered.
    fn newest_first(a: &Placement, b: &Placement) -> std::cmp::Ordering {
        a.traveled
            .total_cmp(&b.traveled)
            .then_with(|| a.pos.x.total_cmp(&b.pos.x))
            .then_with(|| a.pos.y.total_cmp(&b.pos.y))
            .then_with(|| a.normal.x.total_cmp(&b.normal.x))
            .then_with(|| a.normal.y.total_cmp(&b.normal.y))
    }
}
