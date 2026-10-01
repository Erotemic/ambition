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

use ambition_platformer2d::actor::SpawnScopedExt;
use ambition_platformer2d_core::RoomGeometry;
use ambition_portal2d::{
    portal_half_extent, step_portal_shot, PlacedPortal, PortalChannel, PortalShot, PortalShotStep,
    PortalShotWorld,
};

/// Advance portal shots against the concrete collision world. For each shot,
/// call the pure [`step_portal_shot`] over the `RoomGeometry`'s solids + bounds and
/// apply the outcome: open (or replace) the portal of the shot's color on a
/// placeable surface (the warping whoosh + close/attach sfx), or fizzle past
/// range / out of bounds / on a non-placeable surface (the rejection buzz).
///
/// ⛔⛔ TWO SHOTS OF ONE COLOUR CAN LAND ON ONE TICK, and this used to end with
/// two portals of that colour in the world. Each shot was applied against the
/// SAME pre-system `portals` query while its despawn/spawn sat in a deferred
/// command buffer, so neither could see the portal the other had just queued:
/// both despawned the old blue and both spawned a new one. `PlacedPortal`'s whole
/// API assumes at most one per channel — [`ambition_portal2d::find_portal`] is a
/// `.find()` — so the second portal was unreachable geometry that still carved
/// the world.
///
/// ⚠ IT WAS UNREACHABLE UNTIL THE UPSTREAM FIX. `portal_fire_system` kept only
/// `read().last()`, so one tick could never produce two shots to begin with;
/// repairing that loss is what made this downstream assumption live. A fix that
/// widens a producer owes a look at every consumer that was narrow because the
/// producer was.
///
/// ⭐ THE WINNER IS THE NEWEST SHOT, which is what "opens **or replaces**" already
/// meant — a later placement replaces an earlier one. Among shots resolving on
/// the same tick there is no later, so the rule reads the shot's own age:
/// [`PortalShot::traveled`] is distance covered at a fixed speed, so the SMALLEST
/// traveled is the most recently fired. Geometry breaks the remaining tie, and a
/// tie there means two shots that would place the identical portal, where the
/// choice cannot be observed.
///
/// ⛔ NOT `sim_selection::winner_by`: its tie-break vocabulary is [`SimId`], and a
/// portal shot carries none. Here the placement is fully determined by its own
/// geometry, which is a stronger tie-break than an id would be — two shots with
/// equal keys produce byte-identical portals.
///
/// ⭐ EACH SHOT STEPS AGAINST ITS OWN LIVE ROOM (OW1 cut 7l). This read the
/// sole live room, a `Single`, so while two rooms were live the system did not
/// run and every shot hung in the air. A shot in no live room does not move.
/// The portal it opens carries the shot's room.
///
/// ⭐ A PLACEMENT REPLACES THE PORTAL OF ITS CHANNEL IN ITS OWN ROOM ONLY.
/// Portal core pairs, carves and transits by live room (`PortalsByRoom`), so
/// each live room holds its own pair. This replaced the channel's portal in
/// EVERY room: a blue shot in one room closed the blue portal of the other.
///
/// [`SimId`]: ambition_platformer2d_shared_tangle::sim_id::SimId
pub fn portal_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut PortalShot)>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
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
    for winner in &placements {
        if opened.contains(&(winner.room, winner.channel)) {
            // A superseded same-tick placement makes no sound of its own: exactly
            // one portal opened on this channel in this room, so exactly one
            // attach cue plays.
            continue;
        }
        opened.push((winner.room, winner.channel));
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
        let mut portal = commands.spawn_room_scoped((
            PlacedPortal::fixed(
                winner.channel,
                winner.pos,
                winner.normal,
                portal_half_extent(winner.normal),
            ),
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
        ));
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
