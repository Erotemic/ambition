//! Ambition world-seam adapter for the in-flight portal shot.
//!
//! Portal core's [`step_portal_shot`] is a pure helper over the reusable
//! [`SolidWorldQuery`](ambition_platformer2d_core::cast::SolidWorldQuery)
//! seam (+ world bounds): it decides whether a shot travels, places a portal, or
//! fizzles, without ever reading the concrete `ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>`. This adapter owns
//! the concrete world — it reads `ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>`, calls the helper per shot, and
//! applies the [`PortalShotStep`] outcome (entity spawn/despawn + sfx). Moving
//! the `RoomGeometry` read here keeps portal core's projectile step content-free.

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
/// [`SimId`]: ambition_platformer2d_shared_tangle::sim_id::SimId
pub fn portal_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut PortalShot)>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let seam = PortalShotWorld {
        solids: &world.0,
        size: world.0.size,
    };
    // Every placement this tick, decided before any of them is applied.
    let mut placements: Vec<Placement> = Vec::new();
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
                placements.push(Placement {
                    channel,
                    pos,
                    normal,
                    hit,
                    traveled: proj.traveled,
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
    let mut opened: Vec<PortalChannel> = Vec::new();
    for winner in &placements {
        if opened.contains(&winner.channel) {
            // A superseded same-tick placement makes no sound of its own: exactly
            // one portal opened on this channel, so exactly one attach cue plays.
            continue;
        }
        opened.push(winner.channel);
        // Hit a wall — open (or replace) the portal of this color.
        for (entity, portal) in &portals {
            if portal.channel == winner.channel {
                commands.entity(entity).despawn();
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_CLOSE,
                    pos: winner.hit,
                });
            }
        }
        commands.spawn_room_scoped((
            PlacedPortal::fixed(
                winner.channel,
                winner.pos,
                winner.normal,
                portal_half_extent(winner.normal),
            ),
            Name::new(format!("Portal: {}", winner.channel.name())),
            // Portals are per-room: a room transition despawns them, so
            // they don't linger and reappear when you leave and come back
            // (#41).
        ));
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
