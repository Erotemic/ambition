//! The collision world's geometry, for observation (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! Three views of one room, each read from [`CollisionWorld`] as a body's
//! sweep reads it, never composed a second way. Each lists EVERY kind of
//! geometry the room's [`ae::World`] holds ([`world_pieces`]): its blocks,
//! its lines (slopes, hills, a mount's back and the slope up it), its water
//! and its climbable regions.
//! - `collision.authored`: the room's authored geometry ([`RoomCollision::base`]).
//!   A piece the composed walls do not hold is listed as inactive.
//! - `collision.composed`: the walls of the room ([`RoomCollision::solids`]):
//!   the authored geometry, the moving platforms and the feature overlay
//!   (breakables, gates, pogo orbs, a ridden mount's lines).
//! - `collision.for_subject`: the walls as one body meets them
//!   ([`RoomCollision::solids_for`]): the gates open for it passed, the portal
//!   holes it opened cut. A composed piece the body does not meet is listed as
//!   inactive.
//! - `body.sweep` (GOBS-2): each body as the movement kernel last moved it,
//!   from its own record ([`ae::SweepSample`]): the box the sweep tested at
//!   the end of the step, the path from where the step began, and an edge of
//!   that box for each contact the kernel reported (ground, head, wall). The
//!   subject alone when the query names one, else every body in the room.
//! - `world.regions` (GOBS-3): the room's authored regions that are not walls:
//!   its loading zones (with how each is entered, and whether it is secret),
//!   camera zones, gravity zones, encounter triggers and lock walls, read from
//!   the live room's spec.

use std::borrow::Cow;

use ambition_geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedShape, ObservedState, ObserverInfo,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy_app::App;
use bevy_ecs::prelude::*;

use crate::collision::{CollisionWorld, RoomCollision};

/// Install the three providers.
pub fn install(app: &mut App) {
    app.add_geometry_observer(
        ObserverInfo {
            name: "collision.authored",
            description: "the room's authored blocks, lines, water and climbable regions; one the composed walls do not hold is inactive",
            needs_subject: false,
        },
        authored,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "collision.composed",
            description: "the walls of the room: authored geometry, moving platforms, breakables, gates, pogo orbs and mounts' lines",
            needs_subject: false,
        },
        composed,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "collision.for_subject",
            description: "the walls as one body meets them: its open gates passed, its portal holes cut",
            needs_subject: true,
        },
        for_subject,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "world.regions",
            description: "the room's loading zones, camera zones, gravity zones, encounter triggers and lock walls",
            needs_subject: false,
        },
        regions,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "body.sweep",
            description: "each body as the kernel last moved it: the box it swept, its path, and its ground, head and wall contacts",
            needs_subject: false,
        },
        body_sweeps,
    );
}

/// What gameplay does with a block of `kind`.
pub fn role(kind: ae::BlockKind) -> &'static str {
    match kind {
        ae::BlockKind::Solid => "solid",
        ae::BlockKind::Barrier => "barrier",
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft } => "blink_wall_soft",
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Hard } => "blink_wall_hard",
        ae::BlockKind::OneWay => "one_way",
        ae::BlockKind::BonkOnly => "bonk_only",
        ae::BlockKind::Hazard => "hazard",
        ae::BlockKind::PogoOrb => "pogo_orb",
        ae::BlockKind::Rebound { .. } => "rebound",
    }
}

fn observed(block: &ae::Block, state: ObservedState) -> GeometryObservation {
    GeometryObservation {
        key: block.name.clone(),
        roles: vec![Cow::Borrowed(role(block.kind))],
        shape: ObservedShape::aabb(block.aabb.min.to_array(), block.aabb.max.to_array()),
        state,
    }
}

/// A line of the world (a slope, a hill, a mount's back) as gameplay meets
/// it. Its first role is what it is; the rest say how it behaves now.
fn line(chain: &ae::SurfaceChain) -> GeometryObservation {
    let ae::SurfaceChain { name, points, closed, kind, velocity, hidden, boarding, turn, carrier, .. } = chain;
    let mut roles = vec![Cow::Borrowed(match kind {
        // A walking body stands on its segments within its slope limit, as
        // a one-way floor; a rider rides all of it.
        ae::SurfaceKind::Ground => "line",
        // Solid inside: a block a portal pair carried turned.
        ae::SurfaceKind::Solid => "solid_line",
    })];
    if carrier.is_some() {
        roles.push(Cow::Borrowed("carried"));
    }
    if *velocity != ae::Vec2::ZERO {
        roles.push(Cow::Borrowed("moving"));
    }
    if *turn != 0.0 {
        roles.push(Cow::Borrowed("turning"));
    }
    if *hidden {
        roles.push(Cow::Borrowed("drawn_by_art"));
    }
    match boarding {
        ae::Boarding::Auto => {}
        ae::Boarding::Up => roles.push(Cow::Borrowed("board_holding_up")),
    }
    let points = points.iter().map(|point| point.to_array()).collect();
    GeometryObservation {
        key: name.clone(),
        roles,
        shape: if *closed { ObservedShape::Polygon { points } } else { ObservedShape::Polyline { points } },
        state: ObservedState::Active,
    }
}

fn volume(key: &str, roles: [&'static str; 2], aabb: &ae::Aabb) -> GeometryObservation {
    GeometryObservation {
        key: key.to_string(),
        roles: roles.into_iter().map(Cow::Borrowed).collect(),
        shape: ObservedShape::aabb(aabb.min.to_array(), aabb.max.to_array()),
        state: ObservedState::Active,
    }
}

/// Every piece of geometry `world` holds, each active, as a body meets it.
///
/// THE DESTRUCTURE HAS NO `..`. A new kind of geometry on [`ae::World`] does
/// not compile until it is listed here, or named here as not geometry a body
/// meets, so it is in F1 and in headless inspection (`inspect_geometry`) the
/// day it lands. A kind that is not on the world (combat volumes, portal
/// openings) has its own observer: `add_geometry_observer` in its owner.
pub fn world_pieces(world: &ae::World) -> Vec<GeometryObservation> {
    let ae::World {
        blocks,
        chains,
        water_regions,
        climbable_regions,
        // Not geometry a body meets: the room's name, its extent (the edges
        // are its walls and zones) and its spawn point.
        name: _,
        size: _,
        spawn: _,
        // How far past the room a body may go, in each body's own gravity
        // frame: not a shape the room has.
        edges: _,
    } = world;
    let mut out: Vec<GeometryObservation> = blocks.iter().map(|block| observed(block, ObservedState::Active)).collect();
    out.extend(chains.iter().map(line));
    out.extend(water_regions.iter().enumerate().map(|(n, water)| {
        let kind = match water.kind {
            ae::WaterKind::Clear => "clear",
            ae::WaterKind::Murky => "murky",
        };
        volume(&format!("water {n}"), ["water", kind], &water.aabb)
    }));
    out.extend(climbable_regions.iter().enumerate().map(|(n, climbable)| {
        let kind = match climbable.kind {
            ae::ClimbableKind::Ladder => "ladder",
            ae::ClimbableKind::Wall => "climbable_wall",
            ae::ClimbableKind::Vine => "vine",
        };
        volume(&format!("climbable {n}"), ["climbable", kind], &climbable.aabb)
    }));
    out
}

/// The same piece, whatever its state.
fn same(a: &GeometryObservation, b: &GeometryObservation) -> bool {
    a.key == b.key && a.roles == b.roles && a.shape == b.shape
}

/// The room the query names: its root's identity, else the subject's room,
/// else the sole live room.
fn room_of<'a>(
    collision: &'a CollisionWorld,
    query: &ObservationQuery,
    roots: &Query<&LiveRoomInstance>,
    bodies: &Query<&InRoomInstance>,
) -> Option<RoomCollision<'a>> {
    let named = match (query.room, query.subject) {
        (Some(root), _) => Some(InRoomInstance(*roots.get(root).ok()?)),
        (None, Some(subject)) => Some(*bodies.get(subject).ok()?),
        (None, None) => None,
    };
    collision.room(named.as_ref())
}

/// Each piece of `listed`, active if `holder` holds it, else inactive for `reason`.
fn listed_against(listed: &ae::World, holder: &ae::World, reason: &'static str) -> Vec<GeometryObservation> {
    let held = world_pieces(holder);
    world_pieces(listed)
        .into_iter()
        .map(|mut piece| {
            if !held.iter().any(|kept| same(kept, &piece)) {
                piece.state = ObservedState::Inactive { reason: Cow::Borrowed(reason) };
            }
            piece
        })
        .collect()
}

fn authored(
    query: In<ObservationQuery>,
    collision: CollisionWorld,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let Some(room) = room_of(&collision, &query, &roots, &bodies) else {
        return Vec::new();
    };
    room.solids().map_or_else(Vec::new, |walls| listed_against(room.base(), &walls, "not in the composed walls"))
}

fn composed(
    query: In<ObservationQuery>,
    collision: CollisionWorld,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let Some(room) = room_of(&collision, &query, &roots, &bodies) else {
        return Vec::new();
    };
    room.solids().map_or_else(Vec::new, |walls| world_pieces(&walls))
}

fn for_subject(
    query: In<ObservationQuery>,
    collision: CollisionWorld,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let Some(subject) = query.subject else {
        return Vec::new();
    };
    let Some(room) = room_of(&collision, &query, &roots, &bodies) else {
        return Vec::new();
    };
    let (Some(walls), Some(met)) = (room.solids(), room.solids_for(subject)) else {
        return Vec::new();
    };
    let mut out = listed_against(&walls, &met, "open for this body");
    // What the body meets that the room's walls do not hold (the far side of
    // a portal it is in).
    let known = world_pieces(&walls);
    out.extend(world_pieces(&met).into_iter().filter(|piece| !known.iter().any(|known| same(known, piece))));
    out
}

/// The edge of `aabb` whose outward normal is nearest `toward`.
fn edge_toward(aabb: &ae::Aabb, toward: ae::Vec2) -> ([f32; 2], [f32; 2]) {
    let (min, max) = (aabb.min, aabb.max);
    if toward.x.abs() > toward.y.abs() {
        let x = if toward.x > 0.0 { max.x } else { min.x };
        ([x, min.y], [x, max.y])
    } else {
        let y = if toward.y > 0.0 { max.y } else { min.y };
        ([min.x, y], [max.x, y])
    }
}

fn body_sweeps(
    query: In<ObservationQuery>,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<(
        Entity,
        &ae::SweepSample,
        Option<&InRoomInstance>,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&ae::BodyGroundState>,
        Option<&ae::BodyWallState>,
    )>,
) -> Vec<GeometryObservation> {
    let room = match (query.room, query.subject) {
        (Some(root), _) => roots.get(root).ok().copied(),
        (None, Some(subject)) => bodies.get(subject).ok().and_then(|(_, _, room, ..)| room.map(|room| room.0)),
        (None, None) => None,
    };
    let mut out = Vec::new();
    for (entity, sweep, at, id, ground, wall) in &bodies {
        if query.subject.is_some_and(|subject| subject != entity) || (room.is_some() && at.map(|at| at.0) != room) {
            continue;
        }
        let name = id.map_or_else(|| format!("{entity}"), |id| id.as_str().to_owned());
        let tested = ae::Aabb::new(sweep.curr, sweep.half);
        out.push(GeometryObservation {
            key: format!("{name}/box"),
            roles: vec![Cow::Borrowed("body_box")],
            shape: ObservedShape::aabb(tested.min.to_array(), tested.max.to_array()),
            state: ObservedState::Active,
        });
        if sweep.prev != sweep.curr {
            out.push(GeometryObservation {
                key: format!("{name}/path"),
                roles: vec![Cow::Borrowed("sweep")],
                shape: ObservedShape::Segment { a: sweep.prev.to_array(), b: sweep.curr.to_array() },
                state: ObservedState::Active,
            });
        }
        let down = if sweep.down == ae::Vec2::ZERO { ae::Vec2::Y } else { sweep.down };
        let mut contact = |role: &'static str, toward: ae::Vec2| {
            let (a, b) = edge_toward(&tested, toward);
            out.push(GeometryObservation {
                key: format!("{name}/{role}"),
                roles: vec![Cow::Borrowed(role)],
                shape: ObservedShape::Segment { a, b },
                state: ObservedState::Active,
            });
        };
        if let Some(ground) = ground {
            if ground.on_ground {
                contact("contact_ground", down);
            }
            if ground.head_contact {
                contact("contact_head", -down);
            }
        }
        if let Some(wall) = wall.filter(|wall| wall.on_wall && wall.wall_normal_x != 0.0) {
            // The wall's normal points out of it, toward the body.
            contact("contact_wall", ae::Vec2::new(-wall.wall_normal_x.signum(), 0.0));
        }
    }
    out
}

/// How a loading zone is entered.
fn activation_role(activation: crate::rooms::LoadingZoneActivation) -> &'static str {
    match activation {
        crate::rooms::LoadingZoneActivation::EdgeExit => "zone_edge_exit",
        crate::rooms::LoadingZoneActivation::Door => "zone_door",
        crate::rooms::LoadingZoneActivation::Walk => "zone_walk",
    }
}

fn region(key: String, roles: Vec<Cow<'static, str>>, min: ae::Vec2, max: ae::Vec2) -> GeometryObservation {
    GeometryObservation {
        key,
        roles,
        shape: ObservedShape::aabb(min.to_array(), max.to_array()),
        state: ObservedState::Active,
    }
}

/// See the module docs: `world.regions`. The room the query names (its root,
/// else the subject's room), else the sole live room.
fn regions(
    query: In<ObservationQuery>,
    rooms: crate::rooms::LiveRoomSpecs,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let named = match (query.room, query.subject) {
        (Some(root), _) => roots.get(root).ok().copied(),
        (None, Some(subject)) => bodies.get(subject).ok().map(|room| room.0),
        (None, None) => None,
    };
    let live = rooms.live_specs();
    let spec = match named {
        Some(room) => live.iter().find(|(live, _)| *live == room).map(|(_, spec)| *spec),
        None if live.len() == 1 => Some(live[0].1),
        None => None,
    };
    let Some(spec) = spec else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for zone in &spec.loading_zones {
        let mut roles = vec![Cow::Borrowed("loading_zone"), Cow::Borrowed(activation_role(zone.activation))];
        if zone.is_secret() {
            roles.push(Cow::Borrowed("zone_secret"));
        }
        out.push(region(format!("zone {}", zone.id), roles, zone.aabb.min, zone.aabb.max));
    }
    for zone in &spec.camera_zones {
        let mut roles = vec![Cow::Borrowed("camera_zone")];
        if zone.cinematic_lock {
            roles.push(Cow::Borrowed("camera_cinematic_lock"));
        }
        out.push(region(format!("camera {}", zone.id), roles, zone.aabb.min, zone.aabb.max));
    }
    for zone in &spec.gravity_zones {
        out.push(region(
            format!("gravity {}", zone.id),
            vec![Cow::Borrowed("gravity_zone")],
            zone.center - zone.half_extent,
            zone.center + zone.half_extent,
        ));
    }
    for trigger in &spec.encounter_triggers {
        out.push(region(
            format!("encounter {}", trigger.id),
            vec![Cow::Borrowed("encounter_trigger")],
            trigger.min,
            trigger.min + trigger.size,
        ));
    }
    for (n, wall) in spec.lock_walls.iter().enumerate() {
        let key = if wall.id.is_empty() { format!("lock wall {n}") } else { format!("lock wall {}", wall.id) };
        let mut roles = vec![Cow::Borrowed("lock_wall")];
        if wall.gated_by.is_some() {
            roles.push(Cow::Borrowed("lock_wall_gated"));
        }
        out.push(region(key, roles, wall.min, wall.min + wall.size));
    }
    out
}
