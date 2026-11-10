//! The combat read model's geometry, for observation (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! `combat.volumes` reads [`CombatGeometryView`], the view F1's combat layer
//! already draws: each body's hurt volumes and each live strike's hit volume,
//! in the asked room. A body that is deliberately intangible is listed by its
//! collision box, inactive, so it is not mistaken for a body nobody resolved.
//! A body a portal pair cuts is listed by its whole volumes (role
//! `portal_cut`); the pieces are a later packet's (GOBS-3).

use std::borrow::Cow;

use ambition_geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedShape, ObservedState, ObserverInfo,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy::prelude::*;

use crate::{CombatGeometryView, HurtboxSource};

pub fn install(app: &mut App) {
    app.add_geometry_observer(
        ObserverInfo {
            name: "combat.volumes",
            description: "each body's hurt volumes and each live strike's hit volume (the combat read model)",
            needs_subject: false,
        },
        volumes,
    );
}

fn shape(volume: &ae::CombatVolume) -> ObservedShape {
    match volume {
        ae::CombatVolume::Aabb(aabb) => ObservedShape::aabb(aabb.min.to_array(), aabb.max.to_array()),
        ae::CombatVolume::Circle { center, radius } => ObservedShape::Circle {
            center: center.to_array(),
            radius: *radius,
        },
        other => ObservedShape::Polygon {
            points: other.polygon().unwrap_or_default().into_iter().map(|p| p.to_array()).collect(),
        },
    }
}

fn volumes(
    query: In<ObservationQuery>,
    view: Res<CombatGeometryView>,
    roots: Query<&LiveRoomInstance>,
    in_room: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let room = match (query.room, query.subject) {
        (Some(root), _) => roots.get(root).ok().copied(),
        (None, Some(subject)) => in_room.get(subject).ok().map(|room| room.0),
        (None, None) => None,
    };
    let here = |entity: Entity| room.is_none() || in_room.get(entity).ok().map(|r| r.0) == room;
    let mut out = Vec::new();
    for body in view.bodies.iter().filter(|body| here(body.body)) {
        let cut = body.hurt_pieces.is_some();
        if body.hurtbox_source == HurtboxSource::Intangible {
            out.push(GeometryObservation {
                key: format!("hurt {:?}", body.body),
                roles: vec![Cow::Borrowed("hurt")],
                shape: ObservedShape::aabb(body.collision.min.to_array(), body.collision.max.to_array()),
                state: ObservedState::Inactive { reason: Cow::Borrowed("intangible") },
            });
            continue;
        }
        for (n, volume) in body.hurtboxes.iter().enumerate() {
            let mut roles = vec![Cow::Borrowed("hurt")];
            if body.hurtbox_source == HurtboxSource::BodyFallback {
                roles.push(Cow::Borrowed("body_fallback"));
            }
            if cut {
                roles.push(Cow::Borrowed("portal_cut"));
            }
            out.push(GeometryObservation {
                key: format!("hurt {:?} {n}", body.body),
                roles,
                shape: shape(volume),
                state: ObservedState::Active,
            });
        }
    }
    for strike in view.strikes.iter().filter(|strike| here(strike.strike)) {
        let mut roles = vec![Cow::Borrowed("hit")];
        if strike.pieces.is_some() {
            roles.push(Cow::Borrowed("portal_cut"));
        }
        out.push(GeometryObservation {
            key: format!("hit {:?} of {:?}", strike.strike, strike.owner),
            roles,
            shape: shape(&strike.volume),
            state: ObservedState::Active,
        });
    }
    out
}
