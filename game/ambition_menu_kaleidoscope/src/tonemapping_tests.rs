use super::{setup_cube, KaleidoscopeMenuConfig, KaleidoscopePauseCamera};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;

/// Trimming Bevy's defaults left Camera3d on TonyMcMapface without its LUT, producing Bevy's
/// magenta diagnostic output. The cube therefore opts out explicitly: no hidden LUT dependency
/// or LUT-backed transform.
#[test]
fn kaleidoscope_camera_uses_the_cheap_unlit_render_contract() {
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.world_mut().spawn(ambition_menu::ActiveMenuPages::<(), ()>::default());
    app.add_systems(Startup, setup_cube::<(), ()>);
    app.update();

    let world = app.world_mut();
    let mut cameras =
        world.query_filtered::<(&Tonemapping, &Msaa), With<KaleidoscopePauseCamera>>();
    let (tonemapping, msaa) = cameras
        .single(world)
        .expect("setup_cube should spawn exactly one kaleidoscope pause camera");

    assert_eq!(
        *tonemapping,
        Tonemapping::None,
        "the unlit kaleidoscope UI must not inherit Camera3d's LUT-backed tonemapper",
    );
    assert_eq!(
        *msaa,
        Msaa::Sample4,
        "the failed runtime MSAA experiment must not silently rewrite the established camera contract",
    );
}

#[test]
fn setup_creates_a_camera_for_each_published_menu() {
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    let menus = [0, 1].map(|_| {
        app.world_mut()
            .spawn(ambition_menu::ActiveMenuPages::<(), ()>::default())
            .id()
    });
    app.add_systems(Startup, setup_cube::<(), ()>);
    app.update();
    let mut cameras = app
        .world_mut()
        .query_filtered::<&ambition_menu::MenuSurface, With<KaleidoscopePauseCamera>>();
    let owners: std::collections::HashSet<_> =
        cameras.iter(app.world()).map(|surface| surface.0).collect();
    assert_eq!(owners, menus.into_iter().collect());
    for menu in menus {
        assert!(app
            .world()
            .get::<super::KaleidoscopeOpenState>(menu)
            .is_some());
    }
}

#[test]
fn late_menu_scenes_inherit_private_layers_and_release_their_cameras() {
    use super::{assign_scene_layers, bind_scene_layers, MenuRing};
    use bevy::camera::visibility::RenderLayers;
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.add_systems(Update, setup_cube::<(), ()>);
    app.add_systems(PostUpdate, (assign_scene_layers, bind_scene_layers).chain());
    let first = app
        .world_mut()
        .spawn(ambition_menu::ActiveMenuPages::<(), ()>::default())
        .id();
    app.update();
    let nested = app.world_mut().spawn(ChildOf(first)).id();
    let plane = app
        .world_mut()
        .spawn((ChildOf(nested), Mesh3d::default()))
        .id();
    let second = app
        .world_mut()
        .spawn(ambition_menu::ActiveMenuPages::<(), ()>::default())
        .id();
    app.update();
    assert!(app.world().get::<MenuRing>(second).is_some());
    let first_layers = app.world().get::<RenderLayers>(first).unwrap().clone();
    let second_layers = app.world().get::<RenderLayers>(second).unwrap().clone();
    assert!(!first_layers.intersects(&second_layers));
    assert!(!first_layers.intersects(&RenderLayers::layer(0)));
    assert_eq!(
        app.world().get::<RenderLayers>(plane).unwrap(),
        &first_layers
    );
    let camera = app.world_mut().query_filtered::<(Entity, &ambition_menu::MenuSurface, &RenderLayers), With<KaleidoscopePauseCamera>>()
        .iter(app.world()).find(|(_, surface, _)| surface.0 == second).map(|(entity, _, layers)| { assert_eq!(layers, &second_layers); entity }).unwrap();
    app.world_mut().entity_mut(nested).insert(ChildOf(second));
    app.update();
    assert_eq!(
        app.world().get::<RenderLayers>(plane).unwrap(),
        app.world().get::<RenderLayers>(second).unwrap()
    );
    app.world_mut().despawn(second);
    assert!(app.world().get_entity(camera).is_err());
}

#[test]
#[ignore = "needs a wgpu adapter; writes /tmp/ambition-two-menu-scenes.ppm"]
fn two_menu_cameras_draw_only_their_own_scene_into_their_regions() {
    use super::{KaleidoscopeMenuPlugin, KaleidoscopeOpenState};
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::{ImageRenderTarget, RenderTarget, Viewport};
    use bevy::render::gpu_readback::{Readback, ReadbackComplete};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
    use std::sync::{Arc, Mutex};
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: None,
        exit_condition: bevy::window::ExitCondition::DontExit,
        ..default()
    }));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.insert_resource(KaleidoscopeMenuConfig {
        camera_starts_active: true,
        ring_starts_visible: true,
        draw_edge_frame: false,
        draw_nav_arrows: false,
        draw_selection_corners: false,
        pickable_controls: false,
        ..default()
    });
    app.add_plugins(KaleidoscopeMenuPlugin::<u8, ()>::default());
    let mut image = Image::new_fill(
        Extent3d {
            width: 640,
            height: 360,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let menus = [
        ambition_menu::MenuColor::rgba(1.0, 0.0, 0.0, 1.0),
        ambition_menu::MenuColor::rgba(0.0, 1.0, 0.0, 1.0),
    ]
    .map(|color| {
        app.world_mut()
            .spawn((
                ambition_menu::ActiveMenuPages {
                    pages: (0..4u8)
                        .map(|id| {
                            ambition_menu::MenuPageModel::<u8, ()>::new(
                                id,
                                "",
                                if id == 0 {
                                    color
                                } else {
                                    ambition_menu::MenuColor::rgba(0.0, 0.0, 0.0, 1.0)
                                },
                            )
                        })
                        .collect(),
                    active: Some(0u8),
                    visible: true,
                    version: 1,
                },
                KaleidoscopeOpenState {
                    amount: 1.0,
                    target: 1.0,
                },
            ))
            .id()
    });
    let capture = Arc::new(Mutex::new(None::<Vec<u8>>));
    let receive = capture.clone();
    app.world_mut()
        .spawn(Readback::texture(target.clone()))
        .observe(move |event: On<ReadbackComplete>| {
            *receive.lock().unwrap() = Some(event.data.clone());
        });
    app.finish();
    app.cleanup();
    app.update();
    let cameras: Vec<_> = app
        .world_mut()
        .query_filtered::<(Entity, &ambition_menu::MenuSurface), With<KaleidoscopePauseCamera>>()
        .iter(app.world())
        .map(|(entity, surface)| (entity, surface.0))
        .collect();
    for (entity, owner) in cameras {
        let column = menus.iter().position(|menu| *menu == owner).unwrap() as u32;
        let mut camera = app.world_mut().get_mut::<Camera>(entity).unwrap();
        camera.viewport = Some(Viewport {
            physical_position: UVec2::new(column * 320, 0),
            physical_size: UVec2::new(320, 360),
            ..default()
        });
        drop(camera);
        app.world_mut()
            .entity_mut(entity)
            .insert(RenderTarget::Image(ImageRenderTarget::from(target.clone())));
    }
    let started = std::time::Instant::now();
    let pixels = loop {
        app.update();
        if let Some(bytes) = capture.lock().unwrap().as_ref() {
            let left = &bytes[(180 * 640 + 160) * 4..][..4];
            let right = &bytes[(180 * 640 + 480) * 4..][..4];
            if left[0] > 200 && left[1] < 30 && right[1] > 200 && right[0] < 30 {
                break bytes.clone();
            }
        }
        assert!(
            started.elapsed().as_secs() < 60,
            "both menu regions did not draw their own scene"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    use std::io::Write;
    let mut file = std::fs::File::create("/tmp/ambition-two-menu-scenes.ppm").unwrap();
    file.write_all(b"P6\n640 360\n255\n").unwrap();
    let rgb: Vec<u8> = pixels
        .chunks_exact(4)
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect();
    file.write_all(&rgb).unwrap();
}
