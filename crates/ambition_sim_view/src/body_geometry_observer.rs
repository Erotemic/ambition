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
    )
    .add_geometry_observer(
        ObserverInfo {
            name: "body.regions",
            description: "the water and climbable regions one body's admitted material touches: each part, the region its movement reads, and each touch it drops with why",
            needs_subject: true,
        },
        regions,
    );
}

/// The material query movement's environment sample asks, for one body:
/// its admitted parts ([`ae::BodyKinematics::occupied`]) or, uncut, its step
/// box, and each water and climbable region they touch
/// ([`ae::World::regions_touched`]). The region the response reads is
/// active; a shadowed region and a climbable met through a map that turns
/// up are inactive with the reason. A refusal is one inactive coarse box.
fn regions(
    query: In<ObservationQuery>,
    bodies: Query<(&ae::BodyKinematics, &InRoomInstance, Option<&SimId>)>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    geometry: Option<Res<BodyGeometry>>,
) -> Vec<GeometryObservation> {
    use ae::frame::thread::OccupiedPart;
    let Some(subject) = query.subject else { return Vec::new() };
    let Ok((body, at, id)) = bodies.get(subject) else { return Vec::new() };
    // The walls movement steps against: a body passing through solids meets
    // no regions either.
    let prepared = collision.prepare_physical();
    let Some(world) = prepared.physical_for(&collision, Some(at), subject) else { return Vec::new() };
    let name = id.map_or_else(|| format!("{subject}"), |id| id.as_str().to_owned());
    let step_box = body.aabb();
    let whole = || vec![OccupiedPart {
        region: vec![step_box.min, ae::Vec2::new(step_box.max.x, step_box.min.y), step_box.max, ae::Vec2::new(step_box.min.x, step_box.max.y)],
        path: Vec::new(),
        to_root: ae::frame::Sim2::IDENTITY,
    }];
    let cut = geometry.as_deref().and_then(|geometry| geometry.motion_for(subject, Some(at.0)))
        .filter(|admission| !admission.links.is_empty())
        .map(|admission| body.occupied(admission));
    let (parts, uncut) = match cut {
        None | Some(Ok(None)) => (whole(), true),
        Some(Ok(Some(parts))) => (parts, false),
        Some(Err(refusal)) => {
            return vec![GeometryObservation {
                key: name,
                roles: vec!["occupied_part".into(), "query_refused".into(), "coarse_bounds".into()],
                shape: ObservedShape::aabb(step_box.min.to_array(), step_box.max.to_array()),
                state: ObservedState::Inactive { reason: format!("{refusal:?}").into() },
            }];
        }
    };
    let mut out: Vec<GeometryObservation> = parts.iter().enumerate().map(|(n, part)| {
        let mut roles = vec![std::borrow::Cow::Borrowed("occupied_part")];
        roles.push(if uncut { "uncut".into() } else if part.path.is_empty() { "root_chart".into() } else { "portal_carried".into() });
        GeometryObservation {
            key: format!("{name} part {n}"),
            roles,
            shape: ObservedShape::Polygon { points: part.region.iter().map(|p| p.to_array()).collect() },
            state: ObservedState::Active,
        }
    }).collect();
    for touch in world.regions_touched(&parts) {
        let (key, role, aabb) = match touch.region {
            ae::TouchedRegion::Water(index) => (format!("{name} water {index}"), "water_touched", world.water_regions[index].aabb),
            ae::TouchedRegion::Climbable(index) => (format!("{name} climbable {index}"), "climbable_touched", world.climbable_regions[index].aabb),
        };
        out.push(GeometryObservation {
            key,
            roles: vec![role.into()],
            shape: ObservedShape::aabb(aabb.min.to_array(), aabb.max.to_array()),
            state: touch.response.refusal().map_or(ObservedState::Active, |reason| ObservedState::Inactive { reason: reason.into() }),
        });
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_geometry_observation::{observe_all, ObserverAnswer};
    use ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot;

    /// `body.regions` reports the query movement's environment sample asks:
    /// the admitted parts of a straddler, the far pool its carried part is
    /// in, no pool behind the entry its root box overlaps, and the far
    /// ladder under both map conventions: climbed where the map keeps up,
    /// listed and refused where it turns up, never dropped.
    #[test]
    fn a_straddlers_regions_are_observed_through_its_admitted_parts() {
        use ae::frame::{MapConvention, PortalAperture, PortalFrame};
        // The pair of the core witness (`movement/tests/water_through_openings.rs`):
        // its mouths face -x and +x, so one convention keeps up and one flips it.
        let far = ae::Aabb { min: ae::Vec2::new(902.0, 150.0), max: ae::Vec2::new(960.0, 260.0) };
        let behind = ae::Aabb { min: ae::Vec2::new(501.0, 480.0), max: ae::Vec2::new(540.0, 520.0) };
        let observe = |convention| {
            let mut app = App::new();
            install(&mut app);
            let room = LiveRoomInstance::ACTIVATION;
            let mut world = ae::World::new("regions", ae::Vec2::splat(2_000.0), ae::Vec2::ZERO, Vec::new());
            world.water_regions.push(ae::WaterRegion::new(behind, ae::WaterKind::Clear, Default::default()));
            world.water_regions.push(ae::WaterRegion::new(far, ae::WaterKind::Clear, Default::default()));
            world.climbable_regions.push(ae::ClimbableRegion::ladder(far));
            app.world_mut().spawn((RoomInstanceRoot, room, ae::RoomGeometry(world)));
            let (pos, half) = (ae::Vec2::new(495.0, 500.0), ae::Vec2::new(12.0, 24.0));
            let body = app.world_mut().spawn((
                ae::BodyKinematics { pos, canonical_size: half * 2.0, ..Default::default() },
                InRoomInstance(room),
            )).id();
            let mut portals = BodyGeometry::new(convention);
            portals.add_pair(Some(room), (
                PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(500.0, 500.0), ae::Vec2::NEG_X), half_length: 80.0 },
                PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(900.0, 200.0), ae::Vec2::X), half_length: 80.0 },
            ));
            app.insert_resource(portals);
            let layers = observe_all(app.world_mut(), ObservationQuery { room: None, subject: Some(body) }, |name| name == "body.regions");
            let ObserverAnswer::Observed { geometry } = &layers[0].answer else { panic!("{layers:?}") };
            geometry.clone()
        };
        let mut ladder_states = Vec::new();
        for convention in [MapConvention::Reflection, MapConvention::Rotation] {
            let seen = observe(convention);
            let with = |role: &str| seen.iter().filter(|seen| seen.roles.iter().any(|r| r == role)).collect::<Vec<_>>();
            assert_eq!(with("occupied_part").len(), 2, "{convention:?}: both admitted parts: {seen:?}");
            assert_eq!(with("portal_carried").len(), 1, "{convention:?}: one part is carried: {seen:?}");
            let water = with("water_touched");
            assert_eq!(water.len(), 1, "{convention:?}: only the far pool is touched, not the one behind the entry: {seen:?}");
            assert!(water[0].key.ends_with("water 1") && water[0].state == ObservedState::Active, "{water:?}");
            let ladder = with("climbable_touched");
            assert_eq!(ladder.len(), 1, "{convention:?}: the far ladder is listed: {seen:?}");
            ladder_states.push(ladder[0].state.clone());
        }
        assert!(ladder_states.contains(&ObservedState::Active), "one convention keeps up and climbs: {ladder_states:?}");
        assert!(ladder_states.iter().any(|state| matches!(state, ObservedState::Inactive { reason } if reason.contains("turns up"))),
            "the other turns up and is refused, not dropped: {ladder_states:?}");
    }
}
