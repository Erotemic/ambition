//! The collision world's geometry, for observation (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! Three views of one room, each read from [`CollisionWorld`] as a body's
//! sweep reads it, never composed a second way:
//! - `collision.authored`: the room's authored blocks ([`RoomCollision::base`]).
//!   A block the composed walls do not hold is listed as inactive.
//! - `collision.composed`: the walls of the room ([`RoomCollision::solids`]):
//!   the authored blocks, the moving platforms and the feature overlay
//!   (breakables, gates, pogo orbs).
//! - `collision.for_subject`: the walls as one body meets them
//!   ([`RoomCollision::solids_for`]): the gates open for it passed, the portal
//!   holes it opened cut. A composed wall the body does not meet is listed as
//!   inactive.

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
            description: "the room's authored blocks; one the composed walls do not hold is inactive",
            needs_subject: false,
        },
        authored,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "collision.composed",
            description: "the walls of the room: authored blocks, moving platforms, breakables, gates and pogo orbs",
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

fn same(a: &ae::Block, b: &ae::Block) -> bool {
    a.name == b.name && a.kind == b.kind && a.aabb.min == b.aabb.min && a.aabb.max == b.aabb.max
}

fn observed(block: &ae::Block, state: ObservedState) -> GeometryObservation {
    GeometryObservation {
        key: block.name.clone(),
        roles: vec![Cow::Borrowed(role(block.kind))],
        shape: ObservedShape::aabb(block.aabb.min.to_array(), block.aabb.max.to_array()),
        state,
    }
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

/// Each block of `listed`, active if `holder` holds it, else inactive for `reason`.
fn listed_against(listed: &ae::World, holder: &ae::World, reason: &'static str) -> Vec<GeometryObservation> {
    listed
        .blocks
        .iter()
        .map(|block| {
            let state = if holder.blocks.iter().any(|kept| same(kept, block)) {
                ObservedState::Active
            } else {
                ObservedState::Inactive { reason: Cow::Borrowed(reason) }
            };
            observed(block, state)
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
    room.solids().map_or_else(Vec::new, |walls| walls.blocks.iter().map(|block| observed(block, ObservedState::Active)).collect())
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
    out.extend(met.blocks.iter().filter(|block| !walls.blocks.iter().any(|known| same(known, block))).map(|block| observed(block, ObservedState::Active)));
    out
}
