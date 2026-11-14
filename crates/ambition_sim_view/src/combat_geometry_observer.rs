//! The combat read model's geometry, for observation (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! `combat.volumes` reads [`CombatGeometryView`], the view F1's combat layer
//! already draws: each body's hurt volumes and each live strike's hit volume,
//! in the asked room. A body that is deliberately intangible is listed by its
//! collision box, inactive, so it is not mistaken for a body nobody resolved.
//! A body or strike a portal pair cuts is listed by the pieces combat tests
//! (GOBS-3): each piece's region where it reaches, role `portal_piece` (and
//! `portal_carried` for a piece mapped through a pair), with its whole volume
//! inactive beside them, since combat does not test it whole.

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
                state: whole_state(cut),
            });
        }
        if let Some(pieces) = &body.hurt_pieces {
            out.extend(observed_pieces(pieces, &format!("hurt {:?}", body.body), "hurt"));
        }
    }
    for strike in view.strikes.iter().filter(|strike| here(strike.strike)) {
        let mut roles = vec![Cow::Borrowed("hit")];
        if strike.pieces.is_some() {
            roles.push(Cow::Borrowed("portal_cut"));
        }
        let key = format!("hit {:?} of {:?}", strike.strike, strike.owner);
        if let Some(pieces) = &strike.pieces {
            out.extend(observed_pieces(pieces, &key, "hit"));
        }
        out.push(GeometryObservation {
            key,
            roles,
            shape: shape(&strike.volume),
            state: whole_state(strike.pieces.is_some()),
        });
    }
    out
}

/// A volume a portal pair cuts is tested by its pieces, not whole.
fn whole_state(cut: bool) -> ObservedState {
    if cut {
        ObservedState::Inactive { reason: Cow::Borrowed("cut by a portal pair: combat tests its pieces") }
    } else {
        ObservedState::Active
    }
}

/// Each piece of a cut volume where combat reaches it. A piece the cut left
/// empty has no region and is not listed.
fn observed_pieces(pieces: &[ae::frame::thread::VolumePiece], owner: &str, role: &'static str) -> Vec<GeometryObservation> {
    pieces
        .iter()
        .enumerate()
        .filter_map(|(n, piece)| {
            let region = piece.region()?;
            let mut roles = vec![Cow::Borrowed(role), Cow::Borrowed("portal_piece")];
            if piece.is_carried() {
                roles.push(Cow::Borrowed("portal_carried"));
            }
            Some(GeometryObservation {
                key: format!("{owner} piece {n}"),
                roles,
                shape: ObservedShape::Polygon { points: region.into_iter().map(|p| p.to_array()).collect() },
                state: ObservedState::Active,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_geometry_observation::{observe_all, ObserverAnswer};

    /// A body a portal pair cuts is observed by its pieces: each piece active
    /// where combat reaches it, the carried one marked so, and the whole
    /// volume beside them inactive, since combat does not test it whole.
    #[test]
    fn a_cut_body_is_observed_by_the_pieces_combat_tests() {
        let mut app = App::new();
        install(&mut app);
        let body = app.world_mut().spawn_empty().id();
        let near = ae::Aabb::new(ae::Vec2::new(100.0, 80.0), ae::Vec2::new(6.0, 10.0));
        let far = ae::Aabb::new(ae::Vec2::new(400.0, 80.0), ae::Vec2::new(6.0, 10.0));
        let mut carried = ae::frame::thread::VolumePiece::whole(ae::CombatVolume::Aabb(far));
        carried.path = vec![0];
        let whole = ae::Aabb::new(ae::Vec2::new(103.0, 80.0), ae::Vec2::new(9.0, 10.0));
        app.insert_resource(crate::CombatGeometryView {
            bodies: vec![crate::CombatBodyGeometryView {
                body,
                collision: whole,
                hurtboxes: vec![ae::CombatVolume::Aabb(whole)],
                hurt_pieces: Some(vec![ae::frame::thread::VolumePiece::whole(ae::CombatVolume::Aabb(near)), carried]),
                hurtbox_source: HurtboxSource::Published,
                damage_taken: 0,
                facing: 1.0,
                hitstun_s: 0.0,
                hitlag_s: 0.0,
                landing_lag_s: 0.0,
                jump_squat_s: 0.0,
                velocity: ae::Vec2::ZERO,
                grounded: true,
                on_wall: false,
                wall_normal_x: 0.0,
                move_state: None,
            }],
            strikes: Vec::new(),
        });
        let layers = observe_all(app.world_mut(), ObservationQuery { room: None, subject: None }, |name| name == "combat.volumes");
        let ObserverAnswer::Observed { geometry } = &layers[0].answer else { panic!("{layers:?}") };
        let pieces: Vec<_> = geometry.iter().filter(|seen| seen.roles.iter().any(|role| role == "portal_piece")).collect();
        assert_eq!(pieces.len(), 2, "both pieces are listed: {geometry:?}");
        assert!(pieces.iter().all(|piece| piece.state == ObservedState::Active), "{pieces:?}");
        let carried: Vec<_> = pieces.iter().filter(|seen| seen.roles.iter().any(|role| role == "portal_carried")).collect();
        assert_eq!(carried.len(), 1, "the far piece is the carried one: {pieces:?}");
        let ObservedShape::Polygon { points } = &carried[0].shape else { panic!("a piece is its region: {carried:?}") };
        assert!(points.iter().all(|p| (p[0] - 400.0).abs() <= 6.0), "the carried piece is at the far end: {points:?}");
        let whole = geometry.iter().find(|seen| seen.key == format!("hurt {body:?} 0")).expect("the whole volume is listed");
        assert!(matches!(whole.state, ObservedState::Inactive { .. }), "the whole volume is not what combat tests: {whole:?}");
    }
}
