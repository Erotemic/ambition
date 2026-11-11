use super::*;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::body::geometry::{BodyGeometry, MaterialBudget};
use ambition_portal2d::{PortalChannel, PortalChannelColor, PortalGunColor};
use bevy::camera::Viewport;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

fn test_app() -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.init_resource::<Assets<Image>>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<PortalClipMaterial>>()
        .init_resource::<ambition_portal2d::PortalTuning>()
        .init_resource::<BodyGeometry>()
        .init_resource::<PortalCarryAtlas>()
        .init_resource::<CarryQuadPool>()
        .add_systems(First, give_back_carried_layers)
        .add_systems(Update, (ambition_portal2d::publish_portal_apertures, ambition_platformer2d_shared_tangle::body::geometry::publish_body_geometry_after_motion, ambition_portal2d::publish_portal_passages).chain())
        .add_systems(PostUpdate, carry_through_pairs);
    app.world_mut().spawn((
        Camera2d,
        MainCamera,
        Camera {
            viewport: Some(Viewport {
                physical_size: UVec2::splat(512),
                ..default()
            }),
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            area: Rect::from_center_size(Vec2::ZERO, WORLD),
            ..OrthographicProjection::default_2d()
        }),
    ));
    app
}

fn add_pair(app: &mut App, channel: PortalChannel, entry: f32, exit: f32) {
    for portal in [
        PlacedPortal::fixed(channel, Vec2::new(entry, 300.0), Vec2::NEG_X, 100.0).unwrap(),
        PlacedPortal::fixed(channel.partner(), Vec2::new(exit, 300.0), Vec2::X, 100.0).unwrap(),
    ] {
        app.world_mut().spawn(portal);
    }
}

fn authored_entry() -> PortalChannel {
    PortalChannel::Authored(PortalChannelColor::Purple)
}

fn spawn_body(app: &mut App, budget: PieceBudget, y: f32, entry: PortalChannel) -> Entity {
    let pos = Vec2::new(498.0, y);
    let size = Vec2::splat(48.0);
    let body = app.world_mut().spawn((MaterialBudget(budget), ae::BodyKinematics {
        pos, canonical_size: size, ..Default::default()
    })).id();
    app.world_mut()
        .spawn((
            PortalSceneBody,
            crate::PortalBodyView {
                body,
                pos,
                canonical_size: size,
                material: Default::default(),
                size,
                facing: 1.0,
            },
            crate::PortalTransitView { straddling: entry },
            Sprite::from_color(Color::WHITE, size),
            Transform::from_translation(PortalWorldFrame { size: WORLD }.to_render(pos, 20.0)),
        ))
        .id()
}

#[derive(Debug, PartialEq)]
struct Piece {
    at: Vec3,
    scale: Vec3,
    rotation: Quat,
    positions: Vec<[f32; 3]>,
}

#[derive(Debug, PartialEq)]
struct Outcome {
    on_world: bool,
    active_capture: bool,
    pieces: Vec<Piece>,
}

fn outcome(app: &mut App, visual: Entity) -> Outcome {
    let world = app.world_mut();
    let layers = world
        .get::<RenderLayers>(visual)
        .cloned()
        .unwrap_or_default();
    let on_world = layers.intersects(&RenderLayers::layer(0));
    let capture = world
        .query_filtered::<(&Camera, &RenderTarget, &RenderLayers), With<PortalCarryCamera>>()
        .iter(world)
        .find_map(|(camera, target, capture_layers)| {
            if !layers.intersects(capture_layers) {
                return None;
            }
            let RenderTarget::Image(image) = target else {
                return None;
            };
            Some((camera.is_active, image.handle.clone()))
        });
    let Some((active_capture, image)) = capture else {
        return Outcome {
            on_world,
            active_capture: false,
            pieces: Vec::new(),
        };
    };
    let quads: Vec<_> = world
        .query_filtered::<(&Transform, &MeshMaterial2d<PortalClipMaterial>, &Mesh2d, &Visibility), With<PortalBodyPiece>>()
        .iter(world)
        .filter(|(_, _, _, shown)| **shown != Visibility::Hidden)
        .map(|(transform, material, mesh, _)| (*transform, material.0.clone(), mesh.0.clone()))
        .collect();
    let materials = world.resource::<Assets<PortalClipMaterial>>();
    let meshes = world.resource::<Assets<Mesh>>();
    let mut pieces: Vec<_> = quads
        .into_iter()
        .filter_map(|(transform, handle, mesh)| {
            let material = materials
                .get(&handle)
                .expect("a visible quad has its material");
            (material.color_texture == image).then_some(Piece {
                at: transform.translation,
                scale: transform.scale,
                rotation: transform.rotation,
                positions: match meshes.get(&mesh).unwrap().attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
                    bevy::mesh::VertexAttributeValues::Float32x3(positions) => positions.clone(),
                    _ => panic!("piece positions must have three components"),
                },
            })
        })
        .collect();
    pieces.sort_by(|a, b| {
        a.at.x
            .total_cmp(&b.at.x)
            .then(a.at.y.total_cmp(&b.at.y))
            .then(a.at.z.total_cmp(&b.at.z))
    });
    Outcome {
        on_world,
        active_capture,
        pieces,
    }
}

fn warm_app() -> App {
    let mut app = test_app();
    add_pair(&mut app, authored_entry(), 500.0, 532.0);
    add_pair(
        &mut app,
        PortalChannel::Authored(PortalChannelColor::Teal),
        550.0,
        750.0,
    );
    app.update();
    app
}

fn alone(budget: PieceBudget, y: f32) -> Outcome {
    let mut app = warm_app();
    let visual = spawn_body(&mut app, budget, y, authored_entry());
    app.update();
    outcome(&mut app, visual)
}

#[test]
fn independent_bodies_keep_their_geometry_and_depth_on_the_same_link() {
    let scene = |indices: &[usize]| {
        let mut app = warm_app();
        let mut visuals = Vec::new();
        for &index in indices {
            let visual = spawn_body(&mut app, PieceBudget::COMPOSED, 280.0 + index as f32 * 40.0, authored_entry());
            app.world_mut().get_mut::<Transform>(visual).unwrap().translation.z = 20.0 + index as f32 * 4.0;
            visuals.push(visual);
        }
        app.update();
        app.update();
        visuals.into_iter().map(|visual| outcome(&mut app, visual)).collect::<Vec<_>>()
    };
    for order in [[0,1], [1,0]] {
        let actual = scene(&order);
        let exit_depth = |outcome: &Outcome| outcome.pieces.iter()
            .find(|piece| piece.at.z < crate::PORTAL_WINDOW_Z).expect("a carried part").at.z;
        let first = order.iter().position(|index| *index == 0).unwrap();
        let second = order.iter().position(|index| *index == 1).unwrap();
        assert!(exit_depth(&actual[first]) < exit_depth(&actual[second]), "source depth order must survive transit");
        let geometry = |mut outcome: Outcome| {
            for piece in &mut outcome.pieces {
                if piece.at.z < crate::PORTAL_WINDOW_Z { piece.at.z = crate::PORTAL_EXIT_COPY_Z; }
            }
            outcome
        };
        for (index, actual) in order.into_iter().zip(actual) {
            let expected = scene(&[index]).remove(0);
            assert!(actual.active_capture && !actual.on_world);
            assert!(actual.pieces.iter().any(|piece| piece.at.z == 20.0 + index as f32 * 4.0));
            assert_eq!(geometry(actual), geometry(expected), "actor {index}: another root must not change its material");
        }
    }
}

#[test]
fn each_body_keeps_its_piece_policy_with_another_body_on_the_same_link() {
    let blocked = alone(PieceBudget { max_hops: 0, ..PieceBudget::SINGLE_HOP }, 280.0);
    let single = alone(PieceBudget::SINGLE_HOP, 280.0);
    let composed = alone(PieceBudget::COMPOSED, 280.0);
    assert!(!blocked.active_capture && blocked.pieces.is_empty());
    assert!(!single.active_capture && single.pieces.is_empty(), "a one-hop budget must refuse the second aperture");
    assert!(
        composed.pieces.len() > single.pieces.len(),
        "the second pair must require composed geometry"
    );

    for budgets in [
        [PieceBudget { max_hops: 0, ..PieceBudget::SINGLE_HOP }, PieceBudget::SINGLE_HOP],
        [PieceBudget::SINGLE_HOP, PieceBudget::COMPOSED],
    ] {
        let expected = [alone(budgets[0], 280.0), alone(budgets[1], 320.0)];
        for order in [[0, 1], [1, 0]] {
            let mut app = warm_app();
            let mut visuals = [Entity::PLACEHOLDER; 2];
            for index in order {
                visuals[index] = spawn_body(
                    &mut app,
                    budgets[index],
                    280.0 + 40.0 * index as f32,
                    authored_entry(),
                );
            }
            app.update();
            for index in 0..2 {
                assert_eq!(
                    outcome(&mut app, visuals[index]),
                    expected[index],
                    "budgets {budgets:?}, spawn order {order:?}, body {index}"
                );
            }
        }
    }
}

#[test]
fn a_free_drawable_keeps_the_room_policy_beside_a_body_with_its_own_policy() {
    let scene = |with_body: bool| {
        let mut app = warm_app();
        app.insert_resource(ambition_portal2d::PortalTuning {
            geometry: ambition_portal2d::PortalGeometry::Composed { max_scale_exponent: 12,
                max_hops: PieceBudget::COMPOSED.max_hops,
                max_pieces: PieceBudget::COMPOSED.max_pieces,
            },
            ..default()
        });
        let body = with_body.then(|| {
            spawn_body(
                &mut app,
                PieceBudget::SINGLE_HOP,
                280.0,
                authored_entry(),
            )
        });
        let visual = app
            .world_mut()
            .spawn((
                Sprite::from_color(Color::WHITE, Vec2::splat(48.0)),
                bevy::sprite::Anchor::default(),
                Transform::from_translation(
                    PortalWorldFrame { size: WORLD }.to_render(Vec2::new(498.0, 320.0), 20.0),
                ),
            ))
            .id();
        app.update();
        let drawn = outcome(&mut app, visual);
        let body = body.map(|body| outcome(&mut app, body));
        (drawn, body)
    };
    let (expected, _) = scene(false);
    let (actual, body) = scene(true);
    let body = body.expect("the second scene includes the body");
    assert!(expected.active_capture && expected.pieces.len() > body.pieces.len());
    assert_eq!(actual, expected);
    assert_eq!(body, alone(PieceBudget::SINGLE_HOP, 280.0));
}

#[test]
fn a_body_stays_visible_on_the_first_frame_of_an_authored_or_new_gun_pair() {
    for placed_late in [false, true] {
        let mut app = test_app();
        let entry = if placed_late {
            PortalChannel::Gun(PortalGunColor::BLUE)
        } else {
            authored_entry()
        };
        let visual = spawn_body(&mut app, PieceBudget::SINGLE_HOP, 300.0, entry);
        if placed_late {
            app.update();
        }
        add_pair(&mut app, entry, 500.0, 532.0);
        app.update();
        let first = outcome(&mut app, visual);
        assert!(first.on_world || first.active_capture, "no camera can draw the source on its first portal frame: {first:?}, placed late {placed_late}");

        app.update();
        let next = outcome(&mut app, visual);
        assert!(
            !next.on_world && next.active_capture && next.pieces.len() >= 2,
            "the capture must take over after preparation: {next:?}, placed late {placed_late}"
        );
    }
}

#[test]
fn a_moving_rotating_pair_keeps_two_pieces_as_the_body_changes_ends() {
    let mut app = test_app();
    let entry = authored_entry();
    let a = app.world_mut().spawn(PlacedPortal::fixed(entry, Vec2::new(2200.0, 876.0), Vec2::X, 100.0).unwrap()).id();
    let b = app.world_mut().spawn(PlacedPortal::fixed(entry.partner(), Vec2::new(2550.0, 680.0), Vec2::X, 100.0).unwrap()).id();
    let visual = spawn_body(&mut app, PieceBudget::COMPOSED, 300.0, entry);
    app.world_mut().entity_mut(visual).insert(Aabb {
        center: Vec3::ZERO.into(),
        half_extents: Vec3::new(24.0, 24.0, 0.0).into(),
    });
    app.update();
    app.update();
    let mut images = None;
    let materials = app.world().resource::<Assets<PortalClipMaterial>>().len();
    let meshes = app.world().resource::<Assets<Mesh>>().len();

    for degree in 0..360 {
        let angle = (degree as f32).to_radians();
        let center = Vec2::new(2200.0 + 40.0 * angle.sin(), 876.0 + 20.0 * angle.cos());
        for (entity, pos, normal) in [
            (a, center, Vec2::from_angle(angle)),
            (b, center + Vec2::new(350.0, -196.0), Vec2::from_angle(angle * 0.7)),
        ] {
            let mut portal = app.world_mut().get_mut::<PlacedPortal>(entity).unwrap();
            portal.pos = pos;
            portal.normal = normal;
        }
        let side = app.world().get::<PlacedPortal>(if degree < 180 { a } else { b }).unwrap().clone();
        let body = app.world().get::<crate::PortalBodyView>(visual).unwrap().body;
        let pos = side.pos + side.normal * 3.0;
        app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().pos = pos;
        app.world_mut().get_mut::<crate::PortalBodyView>(visual).unwrap().pos = pos;
        app.world_mut().get_mut::<crate::PortalTransitView>(visual).unwrap().straddling = side.channel;
        *app.world_mut().get_mut::<Transform>(visual).unwrap() =
            Transform::from_translation(PortalWorldFrame { size: WORLD }.to_render(side.pos, 20.0));
        app.update();
        let drawn = outcome(&mut app, visual);
        assert!(drawn.active_capture && !drawn.on_world, "degree={degree}: {drawn:?}");
        assert_eq!(drawn.pieces.len(), 2, "degree={degree}: {drawn:?}");
        let count = app.world().resource::<Assets<Image>>().len();
        assert_eq!(count, *images.get_or_insert(count));
        assert_eq!(app.world().resource::<Assets<PortalClipMaterial>>().len(), materials);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), meshes);
    }
}

#[test]
fn a_body_part_is_drawn_through_an_opening_before_centre_transit_starts() {
    let mut app = test_app();
    add_pair(&mut app, authored_entry(), 500.0, 800.0);
    let visual = spawn_body(&mut app, PieceBudget::COMPOSED, 300.0, authored_entry());
    app.world_mut().entity_mut(visual).remove::<crate::PortalTransitView>();
    for _ in 0..6 { app.update(); }
    let without_transit = outcome(&mut app, visual);
    assert!(!without_transit.on_world);
    assert!(without_transit.active_capture);
    assert!(without_transit.pieces.len() >= 2);
    app.world_mut().entity_mut(visual).insert(crate::PortalTransitView { straddling: authored_entry() });
    for _ in 0..3 { app.update(); }
    assert_eq!(without_transit, outcome(&mut app, visual));
}

#[test]
fn a_reaching_swing_does_not_move_a_bodys_near_foot_into_the_aperture_wedge() {
    let mut app = test_app();
    let entry = authored_entry();
    for portal in [
        PlacedPortal::fixed(entry, Vec2::new(534.0, 312.0), Vec2::NEG_Y, 40.0).unwrap(),
        PlacedPortal::fixed(entry.partner(), Vec2::new(200.0, 100.0), Vec2::NEG_Y, 40.0).unwrap(),
    ] { app.world_mut().spawn(portal); }
    let visual = spawn_body(&mut app, PieceBudget::SINGLE_HOP, 300.0, entry);
    for _ in 0..6 { app.update(); }
    let foot = PortalWorldFrame { size: WORLD }.to_render(Vec2::new(492.0, 322.0), 0.0).truncate();
    let paints = |outcome: &Outcome| outcome.pieces.iter().any(|piece| {
        if piece.positions.len() < 3 { return false; }
        let points: Vec<Vec2> = piece.positions.iter().map(|position|
            (piece.at + piece.rotation * (piece.scale * Vec3::from_array(*position))).truncate()).collect();
        let mut positive = false;
        let mut negative = false;
        for index in 0..points.len() {
            let a = points[index]; let b = points[(index+1)%points.len()];
            let side = (b-a).perp_dot(foot-a);
            positive |= side > 0.001;
            negative |= side < -0.001;
        }
        !(positive && negative)
    });
    assert!(paints(&outcome(&mut app, visual)));
    app.world_mut().spawn((Sprite::from_color(Color::WHITE, Vec2::splat(48.0)),
        bevy::sprite::Anchor::default(),
        Transform::from_translation(PortalWorldFrame { size: WORLD }.to_render(Vec2::new(512.0, 320.0), 25.0)),
        ambition_platformer2d_shared_tangle::body::geometry::ReachesFrom(Vec2::new(498.0,300.0))));
    for _ in 0..3 { app.update(); }
    assert!(paints(&outcome(&mut app, visual)), "the foot outside the mouth must remain in the near chart");
}

#[test]
fn capture_resolution_tracks_the_view_without_changing_material_and_releases_idle_targets() {
    let mut app = warm_app();
    let visual = spawn_body(&mut app, PieceBudget::COMPOSED, 300.0, authored_entry());
    for _ in 0..3 { app.update(); }
    let expected = outcome(&mut app, visual);
    let initial_side = app.world().resource::<PortalCarryAtlas>().slots[0].side;
    let camera = app.world_mut().query_filtered::<Entity, With<MainCamera>>().single(app.world()).unwrap();
    app.world_mut().get_mut::<Camera>(camera).unwrap().viewport.as_mut().unwrap().physical_size = UVec2::splat(2048);
    app.update();
    let large_side = app.world().resource::<PortalCarryAtlas>().slots[0].side;
    assert!(large_side > initial_side);
    assert_eq!(outcome(&mut app, visual), expected);
    if let Projection::Orthographic(projection) = &mut *app.world_mut().get_mut::<Projection>(camera).unwrap() {
        projection.area = Rect::from_center_size(Vec2::ZERO, WORLD * 0.5);
    }
    app.update();
    assert!(app.world().resource::<PortalCarryAtlas>().slots[0].side > large_side);
    assert_eq!(outcome(&mut app, visual), expected);
    let retired = app.world().resource::<PortalCarryAtlas>().slots[0].image.clone().unwrap();
    app.world_mut().despawn(visual);
    app.update();
    assert_eq!(app.world().resource::<PortalCarryAtlas>().load.texture_bytes, 0);
    assert!(!app.world().resource::<Assets<Image>>().contains(retired.id()));
    assert!(app.world().resource::<Assets<PortalClipMaterial>>().iter().all(|(_,material)| material.color_texture != retired));
}

#[test]
fn storage_pressure_reduces_quality_and_keeps_more_than_sixteen_families_partitioned() {
    let mut app = warm_app();
    let bytes = 20 * 32 * 32 * 4;
    app.world_mut().resource_mut::<PortalCarryAtlas>().budget.max_texture_bytes = bytes;
    let visuals: Vec<_> = (0..20).map(|index|
        spawn_body(&mut app, PieceBudget::COMPOSED, 280.0 + index as f32, authored_entry())).collect();
    for _ in 0..4 { app.update(); }
    let load = app.world().resource::<PortalCarryAtlas>().load;
    assert_eq!(load.families, 20);
    assert_eq!(load.material_proxies, 0);
    assert_eq!(load.reduced_resolution, 20);
    assert!(load.texture_bytes <= bytes);
    for &visual in &visuals {
        let result = outcome(&mut app, visual);
        assert!(result.active_capture && !result.on_world && result.pieces.len() >= 2, "{result:?}");
    }
    app.world_mut().resource_mut::<PortalCarryAtlas>().budget.max_texture_bytes = 0;
    app.update();
    app.update();
    let load = app.world().resource::<PortalCarryAtlas>().load;
    assert_eq!(load.material_proxies, 20);
    assert_eq!(load.texture_bytes, 0);
    for visual in visuals {
        assert!(app.world().get::<RenderLayers>(visual).unwrap().iter().next().is_none());
        assert!(app.world().get::<PortalCarried>(visual).is_some());
    }
    let drawn = app.world_mut().query_filtered::<&Visibility, With<PortalBodyPiece>>()
        .iter(app.world()).filter(|shown| **shown != Visibility::Hidden).count();
    assert!(drawn >= 40, "overflow must retain the admitted near and far geometry");
}

#[test]
fn a_material_proxy_keeps_physical_scale_and_persistent_reflection() {
    let mut app = warm_app();
    app.world_mut().resource_mut::<ambition_portal2d::PortalTuning>().convention = ambition_portal2d::PortalConvention::Reflection;
    app.world_mut().resource_mut::<PortalCarryAtlas>().budget.max_texture_bytes = 0;
    let visual = spawn_body(&mut app, PieceBudget::COMPOSED, 300.0, authored_entry());
    let material = ae::frame::BodyMaterial::new(0.5, Mat2::from_cols(-Vec2::X,Vec2::Y)).unwrap();
    let body = app.world().get::<crate::PortalBodyView>(visual).unwrap().body;
    {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
        kin.canonical_size = Vec2::new(32.0,48.0); kin.material = material;
    }
    {
        let mut view = app.world_mut().get_mut::<crate::PortalBodyView>(visual).unwrap();
        view.canonical_size = Vec2::new(32.0,48.0); view.material = material; view.size = Vec2::new(16.0,24.0);
    }
    for _ in 0..3 { app.update(); }
    let mut area = 0.0;
    let mut flips = Vec::new();
    let world = app.world_mut();
    let pieces: Vec<_> = world.query_filtered::<(&Transform,&Mesh2d,&MeshMaterial2d<PortalClipMaterial>,&Visibility), With<PortalBodyPiece>>()
        .iter(world).filter(|(_,_,_,shown)| **shown != Visibility::Hidden)
        .map(|(pose,mesh,material,_)| (*pose,mesh.0.clone(),material.0.clone())).collect();
    for (pose,mesh,handle) in pieces {
        let mesh = world.resource::<Assets<Mesh>>().get(&mesh).unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(points) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() else { panic!("positions"); };
        for triangle in mesh.indices().unwrap().iter().collect::<Vec<_>>().chunks_exact(3) {
            let p: Vec<_> = triangle.iter().map(|index| pose.transform_point(Vec3::from_array(points[*index])).truncate()).collect();
            area += (p[1]-p[0]).perp_dot(p[2]-p[0]).abs()*0.5;
        }
        flips.push(world.resource::<Assets<PortalClipMaterial>>().get(&handle).unwrap().control.x > 0.5);
    }
    assert!((area - 32.0*48.0*0.25).abs() < 0.01, "{area}");
    assert!(flips.contains(&true) && flips.contains(&false), "root reflection and reflected transit must both apply: {flips:?}");
}
