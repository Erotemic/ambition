use ambition_persistence::settings::{ScreenShaderSettings, UserSettings};
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_render::{
    screen_effects::{ScreenEffectCamera, ScreenEffectsPlugin},
    spirit_field::{SpiritAura, SpiritFieldPlugin},
    void_field::{VoidAura, VoidFieldPlugin},
};
use bevy::{
    camera::{RenderTarget, Viewport},
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
        view::Msaa,
        RenderPlugin,
    },
};

#[derive(Resource, Default)]
struct Pixels(Vec<u8>);

fn capture(app: &mut App, target: &Handle<Image>) -> Vec<u8> {
    for _ in 0..12 {
        app.update();
    }
    app.world_mut().resource_mut::<Pixels>().0.clear();
    let request = app
        .world_mut()
        .spawn(Readback::texture(target.clone()))
        .observe(|read: On<ReadbackComplete>, mut pixels: ResMut<Pixels>| {
            pixels.0.clone_from(&read.data);
        })
        .id();
    for _ in 0..120 {
        app.update();
        let pixels = &mut app.world_mut().resource_mut::<Pixels>().0;
        if !pixels.is_empty() {
            let pixels = std::mem::take(pixels);
            app.world_mut().despawn(request);
            assert_eq!(pixels.len(), 64 * 64 * 4);
            return pixels;
        }
    }
    panic!("GPU readback did not complete");
}

fn set_effect(app: &mut App, enable: impl FnOnce(&mut ScreenShaderSettings)) {
    let mut settings = app.world_mut().resource_mut::<UserSettings>();
    settings.video.shaders = ScreenShaderSettings {
        strength: 1.0,
        ..default()
    };
    enable(&mut settings.video.shaders);
}

// Run with `cargo test -p ambition_render --test screen_pass -- --ignored`.
#[test]
#[ignore = "requires a GPU or a software graphics adapter"]
fn camera_passes_render_independently_and_together() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::window::WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        RenderPlugin {
            synchronous_pipeline_compilation: true,
            ..default()
        },
        ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::camera::CameraPlugin,
        bevy::core_pipeline::CorePipelinePlugin,
        bevy::sprite::SpritePlugin,
        bevy::text::TextPlugin,
        bevy::sprite_render::SpriteRenderPlugin,
        ScreenEffectsPlugin,
        SpiritFieldPlugin,
        VoidFieldPlugin,
    ));
    app.init_resource::<UserSettings>();
    app.init_resource::<Pixels>();
    app.insert_resource(ClearColor(Color::srgb(0.68, 0.40, 0.45)));
    app.finish();
    app.cleanup();

    let mut image = Image::new_target_texture(64, 64, TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let output = RenderTarget::Image(target.clone().into());
    let camera = app
        .world_mut()
        .spawn((
            Camera2d,
            output.clone(),
            ScreenEffectCamera,
            MainCamera,
            Msaa::Off,
        ))
        .id();
    app.world_mut().spawn((
        Sprite::from_color(Color::srgb(0.1, 0.72, 0.9), Vec2::splat(32.0)),
        Transform::default(),
    ));
    let off = capture(&mut app, &target);
    assert!(
        off.chunks_exact(4).any(|pixel| pixel != &off[..4]),
        "the reference frame must show a sprite"
    );
    let effects: [(&str, fn(&mut ScreenShaderSettings)); 6] = [
        ("CRT", |s| s.crt_strength = 1.0),
        ("Grain", |s| s.film_grain_strength = 1.0),
        ("Robot", |s| s.robot_death_strength = 1.0),
        ("Underwater", |s| s.underwater_strength = 1.0),
        ("Deep Dream", |s| s.deep_dream_strength = 1.0),
        ("Vignette", |s| s.vignette_strength = 1.0),
    ];
    for (name, effect) in effects {
        set_effect(&mut app, effect);
        assert!(
            capture(&mut app, &target) != off,
            "{name} must change the picture"
        );
    }
    let vignette = capture(&mut app, &target);
    let void = app
        .world_mut()
        .spawn((
            VoidAura {
                radius: 50.0,
                strength: 1.0,
            },
            Transform::default(),
        ))
        .id();
    let with_void = capture(&mut app, &target);
    assert!(
        with_void != vignette,
        "the void field must render with the screen filter"
    );
    let spirit = app
        .world_mut()
        .spawn((
            SpiritAura {
                radius: 50.0,
                strength: 1.0,
                tint: Vec3::new(0.1, 0.9, 0.3),
            },
            Transform::default(),
        ))
        .id();
    let combined = capture(&mut app, &target);
    assert!(
        combined != with_void,
        "the spirit field must render with the void field and screen filter"
    );
    set_effect(&mut app, |_| {});
    let fields = capture(&mut app, &target);
    assert!(combined != fields);
    app.world_mut().despawn(void);
    app.world_mut().despawn(spirit);
    assert_eq!(capture(&mut app, &target), off);

    use bevy::camera::{visibility::RenderLayers, ClearColorConfig};
    // The last camera presents the whole shared target without clearing it.
    app.world_mut().spawn((
        Camera2d,
        output.clone(),
        Camera {
            order: -1,
            ..default()
        },
        Msaa::Off,
    ));
    app.world_mut().spawn((
        Camera2d,
        output,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        RenderLayers::layer(1),
    ));
    let mut view = app.world_mut().get_mut::<Camera>(camera).unwrap();
    view.clear_color = ClearColorConfig::None;
    view.viewport = Some(Viewport {
        physical_size: UVec2::new(32, 64),
        ..default()
    });
    let off = capture(&mut app, &target);
    set_effect(&mut app, |s| s.vignette_strength = 1.0);
    let on = capture(&mut app, &target);
    assert!(on != off);
    for (after, before) in on.chunks_exact(256).zip(off.chunks_exact(256)) {
        assert_eq!(
            &after[128..],
            &before[128..],
            "the other half of the target must stay intact"
        );
    }

    #[cfg(feature = "card_view")]
    {
        let world = app.world_mut();
        let mut planes = world.query_filtered::<&mut Camera, With<Camera2d>>();
        for mut plane in planes.iter_mut(world) {
            plane.is_active = false;
        }
        world.spawn((
            Camera3d::default(),
            RenderTarget::Image(target.clone().into()),
            Transform::from_xyz(0.0, 0.0, 100.0),
            MainCamera,
            ScreenEffectCamera,
            Msaa::Off,
        ));
        set_effect(&mut app, |_| {});
        let no_effect = capture(&mut app, &target);
        set_effect(&mut app, |s| s.vignette_strength = 1.0);
        let filtered = capture(&mut app, &target);
        assert_ne!(filtered, no_effect, "the 3D graph runs the screen filter");

        app.world_mut().spawn((VoidAura { radius: 30.0, strength: 1.0 }, Transform::default()));
        let darkened = capture(&mut app, &target);
        assert_ne!(darkened, filtered, "the 3D graph adds the void field");

        app.world_mut().spawn((
            SpiritAura { radius: 25.0, strength: 1.0, tint: Vec3::new(0.1, 0.9, 0.3) },
            Transform::from_xyz(3.0, 0.0, 0.0),
        ));
        assert_ne!(capture(&mut app, &target), darkened, "the 3D graph adds the spirit field");
    }
}
