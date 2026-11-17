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
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedPlane, ObservedShape, ObservedState, ObserverInfo,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRooms, LiveRoomInstance};
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
    live: LiveRooms,
    names: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> Vec<GeometryObservation> {
    let room = match (query.room, query.subject) {
        (Some(root), _) => roots.get(root).ok().copied(),
        (None, Some(subject)) => live.of(subject),
        (None, None) => None,
    };
    let here = |entity: Entity| room.is_none() || live.of(entity) == room;
    let name = |entity: Entity| names.get(entity).map_or_else(|_| format!("{entity}"), |id| id.as_str().to_owned());
    let mut out = Vec::new();
    for body in view.bodies.iter().filter(|body| here(body.body)) {
        let cut = body.hurt_pieces.is_some();
        if body.hurtbox_source == HurtboxSource::Intangible {
            out.push(GeometryObservation {
                key: format!("hurt {}", name(body.body)),
                roles: vec![Cow::Borrowed("hurt")],
                shape: ObservedShape::aabb(body.collision.min.to_array(), body.collision.max.to_array()),
                state: ObservedState::Inactive { reason: Cow::Borrowed("intangible") },
            });
            continue;
        }
        for (n, volume) in body.hurtboxes.iter().enumerate() {
            let mut roles = vec![Cow::Borrowed("hurt")];
            roles.push(Cow::Borrowed(body.hurtbox_source.as_str()));
            if cut {
                roles.push(Cow::Borrowed("portal_cut"));
            }
            out.push(GeometryObservation {
                key: format!("hurt {} {n}", name(body.body)),
                roles,
                shape: shape(volume),
                state: whole_state(cut, body.hurt_refusal.as_deref()),
            });
        }
        if let Some(pieces) = &body.hurt_pieces {
            out.extend(observed_pieces(pieces, &format!("hurt {}", name(body.body)), "hurt"));
        }
    }
    for strike in view.strikes.iter().filter(|strike| {
        room.is_none() || live.stamped(strike.strike).or_else(|| live.of(strike.owner)) == room
    }) {
        let mut roles = vec![Cow::Borrowed("hit")];
        if strike.pieces.is_some() {
            roles.push(Cow::Borrowed("portal_cut"));
        }
        let key = format!("hit {} of {}", name(strike.strike), name(strike.owner));
        if let Some(pieces) = &strike.pieces {
            out.extend(observed_pieces(pieces, &key, "hit"));
        }
        out.push(GeometryObservation {
            key,
            roles,
            shape: shape(&strike.volume),
            state: whole_state(strike.pieces.is_some(), strike.geometry_refusal.as_deref()),
        });
    }
    out
}

/// A volume a portal pair cuts is tested by its pieces, not whole.
fn whole_state(cut: bool, refusal: Option<&str>) -> ObservedState {
    if let Some(reason) = refusal {
        ObservedState::Inactive { reason: format!("geometry refused: {reason}").into() }
    } else if cut {
        ObservedState::Inactive { reason: Cow::Borrowed("cut by a portal pair: combat tests its pieces") }
    } else {
        ObservedState::Active
    }
}

/// Each piece of a cut volume where combat reaches it. A piece the cut left
/// empty has no region and is not listed.
pub(crate) fn observed_pieces(pieces: &[ae::frame::thread::VolumePiece], owner: &str, role: &'static str) -> Vec<GeometryObservation> {
    pieces
        .iter()
        .enumerate()
        .filter_map(|(n, piece)| {
            if piece.is_empty() { return None; }
            let mut roles = vec![Cow::Borrowed(role), Cow::Borrowed("material_piece")];
            if pieces.len() > 1 || piece.is_carried() || !piece.source_bounds().is_empty() {
                roles.push(Cow::Borrowed("portal_piece"));
            }
            if piece.is_carried() {
                roles.push(Cow::Borrowed("portal_carried"));
            }
            Some(GeometryObservation {
                key: format!("{owner} piece {n}"),
                roles,
                shape: ObservedShape::Clipped {
                    primitive: Box::new(shape(piece.source())),
                    planes: piece.source_bounds().iter().map(|plane| ObservedPlane {
                        inward: plane.inward.to_array(), offset: plane.offset,
                    }).collect(),
                    transform: piece.transform().affine_rows(),
                    path: piece.path.clone(),
                },
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
                hurt_refusal: None,
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
        let ObservedShape::Clipped { primitive, transform, path, .. } = &carried[0].shape else { panic!("a piece retains its source: {carried:?}") };
        assert_eq!(**primitive, shape(&ae::CombatVolume::Aabb(far)));
        assert_eq!(*transform, ae::frame::Sim2::IDENTITY.affine_rows());
        assert_eq!(path, &[0]);
        let whole = geometry.iter().find(|seen| seen.key == format!("hurt {body} 0")).expect("the whole volume is listed");
        assert!(matches!(whole.state, ObservedState::Inactive { .. }), "the whole volume is not what combat tests: {whole:?}");
    }
    #[test]
    fn a_scaled_portal_cap_keeps_its_circle_and_exact_source_planes() {
        use ae::frame::{MapConvention, PortalAperture, PortalFrame};
        use ae::frame::thread::{MaterialPrimitive, PieceBudget, PortalLink, partition};
        let enter = PortalAperture { frame: PortalFrame::fixed(ae::Vec2::ZERO, ae::Vec2::NEG_Y), half_length: 40.0 };
        let exit = PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(500.0, -200.0), ae::Vec2::Y), half_length: 20.0 };
        let circle = ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 20.0 };
        let parts = partition(MaterialPrimitive::world(&circle), ae::Vec2::new(0.0, -10.0),
            &PortalLink::both_ways(0, (enter, exit)), None, PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
        let reported = observed_pieces(&parts, "circle", "hurt");
        let part = parts.iter().find(|part| part.is_carried()).unwrap();
        let report = reported.iter().find(|report| report.roles.iter().any(|role| role == "portal_carried")).unwrap();
        let ObservedShape::Clipped { primitive, planes, transform, path } = &report.shape else { panic!("{report:?}") };
        assert_eq!(**primitive, shape(&circle));
        assert!(!planes.is_empty());
        assert_eq!(planes.len(), part.source_bounds().len());
        for (plane, source) in planes.iter().zip(part.source_bounds()) {
            assert_eq!(plane.inward, source.inward.to_array());
            assert_eq!(plane.offset, source.offset);
        }
        assert_eq!(*transform, part.transform().affine_rows());
        assert_eq!(path, &part.path);
        assert!(!part.meets(&ae::frame::thread::VolumePiece::whole(ae::CombatVolume::circle(ae::Vec2::new(509.0, -191.0), 1.0))));
    }

    #[test]
    fn an_unstamped_strike_uses_its_owners_room_in_a_two_room_world() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, RoomInstanceRoot};
        let mut app = App::new();
        install(&mut app);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let roots = rooms.map(|room| app.world_mut().spawn((RoomInstanceRoot, room)).id());
        let owners = rooms.map(|room| app.world_mut().spawn(InRoomInstance(room)).id());
        let strikes = owners.map(|owner| {
            let strike = app.world_mut().spawn_empty().id();
            crate::CombatStrikeGeometryView {
                volume: ae::CombatVolume::circle(ae::Vec2::ZERO, 10.0),
                pieces: None, geometry_refusal: None, strike, owner, damage: 1,
                anchored_to_body: true, depicted_by_owner: false, ray: None, hit: vec![],
            }
        });
        app.insert_resource(CombatGeometryView { bodies: vec![], strikes: strikes.to_vec() });
        for (n, root) in roots.into_iter().enumerate() {
            let layers = observe_all(app.world_mut(), ObservationQuery { room: Some(root), subject: Some(owners[n]) }, |_| true);
            let ObserverAnswer::Observed { geometry } = &layers[0].answer else { panic!("{layers:?}") };
            assert_eq!(geometry.len(), 1, "a strike belongs only to its owner's room: {geometry:?}");
            assert!(geometry[0].key.ends_with(&format!("of {}", owners[n])));
        }
    }

}
