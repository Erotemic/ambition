//! Native body and projectile material, from the shared geometry query.

use ambition_geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedShape,
    ObservedState, ObserverInfo,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::{
    body::geometry::BodyGeometry,
    lifecycle::{InRoomInstance, LiveRoomInstance},
    projectile::ProjectileGameplay,
    sim_id::SimId,
};
use bevy::prelude::*;

pub fn install(app: &mut App) {
    app.add_geometry_observer(
        ObserverInfo {
            name: "body.material",
            description: "native body parts, source cuts and maps; refused geometry is inactive",
            needs_subject: false,
        },
        bodies,
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "combat.projectiles",
            description: "live projectile parts and inactive impact-burst previews",
            needs_subject: false,
        },
        projectiles,
    );
}

type Bodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ae::BodyKinematics,
        &'static InRoomInstance,
        Option<&'static SimId>,
        Has<ambition_projectiles::LiveProjectile>,
        Option<&'static ProjectileGameplay>,
    ),
>;

fn bodies(
    query: In<ObservationQuery>,
    roots: Query<&LiveRoomInstance>,
    bodies: Bodies,
    geometry: Res<BodyGeometry>,
) -> Vec<GeometryObservation> {
    observe(&query, &roots, &bodies, &geometry, false)
}

fn projectiles(
    query: In<ObservationQuery>,
    roots: Query<&LiveRoomInstance>,
    bodies: Bodies,
    geometry: Res<BodyGeometry>,
) -> Vec<GeometryObservation> {
    observe(&query, &roots, &bodies, &geometry, true)
}

fn observe(
    query: &ObservationQuery,
    roots: &Query<&LiveRoomInstance>,
    bodies: &Bodies,
    geometry: &BodyGeometry,
    shots: bool,
) -> Vec<GeometryObservation> {
    let room = query.room.and_then(|root| roots.get(root).ok()).copied();
    let mut out = Vec::new();
    for (entity, body, at, id, shot, game) in bodies {
        if shot != shots
            || room.is_some_and(|room| room != at.0)
            || (!shots && query.subject.is_some_and(|subject| subject != entity))
        {
            continue;
        }
        let name = id.map_or_else(|| format!("{entity}"), |id| id.as_str().to_owned());
        let role = if shots { "projectile" } else { "body_material" };
        let parts = body
            .canonical_box()
            .and_then(|domain| Ok((domain, body.material_map()?)))
            .map_err(|fault| format!("{fault:?}"))
            .and_then(|(domain, map)| {
                geometry
                    .body_pieces(
                        ae::frame::thread::MaterialPrimitive {
                            domain: &domain,
                            to_world: map,
                        },
                        entity,
                        Some(at.0),
                    )
                    .map_err(|reason| format!("{reason:?}"))
            });
        match parts {
            Ok(parts) => out.extend(crate::combat_geometry_observer::observed_pieces(
                &parts, &name, role,
            )),
            Err(reason) => {
                let coarse = body.aabb();
                out.push(GeometryObservation {
                    key: name.clone(),
                    roles: vec![role.into(), "query_refused".into(), "coarse_bounds".into()],
                    shape: ObservedShape::aabb(coarse.min.to_array(), coarse.max.to_array()),
                    state: ObservedState::Inactive {
                        reason: reason.into(),
                    },
                });
            }
        }
        if let Some(game) = game.filter(|game| game.splash_half_extent > 0.0) {
            let preview = ae::Aabb::new(body.pos, ae::Vec2::splat(game.splash_half_extent));
            out.push(GeometryObservation {
                key: format!("{name}/impact_preview"),
                roles: vec!["projectile".into(), "splash_preview".into()],
                shape: ObservedShape::aabb(preview.min.to_array(), preview.max.to_array()),
                state: ObservedState::Inactive {
                    reason: "on impact; preview at the current position".into(),
                },
            });
        }
    }
    out
}
