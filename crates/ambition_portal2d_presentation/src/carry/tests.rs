use super::*;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::portal_passages::{PortalPassages, PortalPieceBudget};
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
        .init_resource::<PortalPassages>()
        .init_resource::<PortalCarryAtlas>()
        .init_resource::<CarryQuadPool>()
        .add_systems(First, give_back_carried_layers)
        .add_systems(Update, ambition_portal2d::publish_portal_passages)
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
        PlacedPortal::fixed(channel, Vec2::new(entry, 300.0), Vec2::NEG_X, 100.0),
        PlacedPortal::fixed(channel.partner(), Vec2::new(exit, 300.0), Vec2::X, 100.0),
    ] {
        app.world_mut().spawn(portal);
    }
}

fn authored_entry() -> PortalChannel {
    PortalChannel::Authored(PortalChannelColor::Purple)
}

fn spawn_body(app: &mut App, budget: Option<PieceBudget>, y: f32, entry: PortalChannel) -> Entity {
    let body = app.world_mut().spawn(PortalPieceBudget(budget)).id();
    let pos = Vec2::new(498.0, y);
    let size = Vec2::splat(48.0);
    app.world_mut()
        .spawn((
            PortalSceneBody,
            crate::PortalBodyView {
                body,
                pos,
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
    planes: [Vec4; CLIP_PLANES],
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
        .query_filtered::<(&Transform, &MeshMaterial2d<PortalClipMaterial>, &Visibility), With<PortalBodyPiece>>()
        .iter(world)
        .filter(|(_, _, shown)| **shown != Visibility::Hidden)
        .map(|(transform, material, _)| (*transform, material.0.clone()))
        .collect();
    let materials = world.resource::<Assets<PortalClipMaterial>>();
    let mut pieces: Vec<_> = quads
        .into_iter()
        .filter_map(|(transform, handle)| {
            let material = materials
                .get(&handle)
                .expect("a visible quad has its material");
            (material.color_texture == image).then_some(Piece {
                at: transform.translation,
                scale: transform.scale,
                rotation: transform.rotation,
                planes: [
                    material.clip0,
                    material.clip1,
                    material.clip2,
                    material.clip3,
                    material.clip4,
                    material.clip5,
                ],
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

fn alone(budget: Option<PieceBudget>, y: f32) -> Outcome {
    let mut app = warm_app();
    let visual = spawn_body(&mut app, budget, y, authored_entry());
    app.update();
    outcome(&mut app, visual)
}

#[test]
fn each_body_keeps_its_piece_policy_with_another_body_on_the_same_link() {
    let transfer = alone(None, 280.0);
    let single = alone(Some(PieceBudget::SINGLE_HOP), 280.0);
    let composed = alone(Some(PieceBudget::COMPOSED), 280.0);
    assert!(transfer.on_world && !transfer.active_capture && transfer.pieces.is_empty());
    assert!(!single.on_world && single.active_capture && single.pieces.len() >= 2);
    assert!(
        composed.pieces.len() > single.pieces.len(),
        "the second pair must require composed geometry"
    );

    for budgets in [
        [None, Some(PieceBudget::SINGLE_HOP)],
        [Some(PieceBudget::SINGLE_HOP), Some(PieceBudget::COMPOSED)],
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
            geometry: ambition_portal2d::PortalGeometry::Composed {
                max_hops: PieceBudget::COMPOSED.max_hops,
                max_pieces: PieceBudget::COMPOSED.max_pieces,
            },
            ..default()
        });
        let body = with_body.then(|| {
            spawn_body(
                &mut app,
                Some(PieceBudget::SINGLE_HOP),
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
    assert_eq!(body, alone(Some(PieceBudget::SINGLE_HOP), 280.0));
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
        let visual = spawn_body(&mut app, Some(PieceBudget::SINGLE_HOP), 300.0, entry);
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
