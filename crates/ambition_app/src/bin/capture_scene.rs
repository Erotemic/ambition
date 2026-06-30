//! Faithful Bevy scene capture for a camera-follow snapshot.
//!
//! This is the render-stack counterpart to
//! `ambition_gameplay_core/examples/render_room_geometry.rs capture`: it runs
//! the real presentation plugins, forces the main camera to the same
//! `CameraSnapshot2d` policy for an arbitrary focus point, renders into an
//! offscreen image target, and asks Bevy's screenshot pipeline to write that
//! render target to disk. It intentionally knows nothing about portals; portals
//! can later reuse the same "snapshot -> render target" seam.
//!
//! Usage:
//!   cargo run -p ambition_app --bin capture_scene -- <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT] [--warmup N] [--include-ui] [--show-window]
//!   cargo run -p ambition_app --bin capture_scene -- c136 1200,480 /tmp/c136_game.png 1280x720

use std::path::{Path, PathBuf};

use ambition_app::app::{
    PresentationSetupSet, SandboxLdtkPlugin, SandboxPresentationPlugin, SandboxSimulationPlugin,
    StartRoomOverride,
};
use ambition_engine_core as ae;
use ambition_gameplay_core::assets::game_assets::GameAssetConfig;
use ambition_gameplay_core::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraFocus2d, CameraSnapshotResolveInput,
    CameraSnapshotResolveMode,
};
use ambition_gameplay_core::game_mode::GameMode;
use ambition_gameplay_core::session::camera_layers::{FrontHudCamera, MainCamera};
use ambition_render::rendering::{camera_follow, sync_parallax_layers, CameraViewState};
use bevy::app::AppExit;
use bevy::app::{PluginGroup, ScheduleRunnerPlugin};
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::window::{ExitCondition, Window, WindowPlugin, WindowResolution};
use std::time::Duration;

#[derive(Resource, Clone, Debug)]
struct SceneCaptureConfig {
    room_id: String,
    focus: ae::Vec2,
    output: PathBuf,
    size: UVec2,
    warmup_frames: u32,
    include_ui: bool,
    show_window: bool,
}

#[derive(Resource, Clone, Debug)]
struct SceneCaptureTarget {
    image: Handle<Image>,
}

#[derive(Resource, Debug, Default)]
struct SceneCaptureRuntime {
    frames: u32,
    wait_frames: u32,
    requested: bool,
    completed: bool,
    failed: bool,
}

fn main() {
    let config = match SceneCaptureConfig::from_args(std::env::args().skip(1).collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "Usage: capture_scene <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT] [--warmup N] [--include-ui] [--show-window]"
            );
            std::process::exit(2);
        }
    };

    let asset_config = GameAssetConfig::from_args();
    let active_profile = asset_config.asset_profile;
    let asset_root = desktop_asset_root();
    eprintln!(
        "capture_scene: room={} focus=({:.1},{:.1}) size={}x{} out={} asset_root={}",
        config.room_id,
        config.focus.x,
        config.focus.y,
        config.size.x,
        config.size.y,
        config.output.display(),
        asset_root,
    );

    let show_window = config.show_window;
    let mut app = App::new();
    let plugins = DefaultPlugins.set(bevy::asset::AssetPlugin {
        file_path: asset_root,
        ..default()
    });
    if show_window {
        app.add_plugins(plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ambition capture_scene".into(),
                resolution: WindowResolution::new(config.size.x, config.size.y),
                ..default()
            }),
            exit_condition: ExitCondition::DontExit,
            ..default()
        }));
    } else {
        // Default capture is a faithful offscreen render to an Image target.
        // Camera policy produces snapshots; the render backend consumes the
        // snapshot without a primary window or Winit event loop.
        app.add_plugins(
            plugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>(),
        );
        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(0)));
    }
    app.init_state::<GameMode>();
    app.insert_resource(asset_config);
    app.insert_resource(StartRoomOverride(config.room_id.clone()));
    app.insert_resource(config);
    app.insert_resource(SceneCaptureRuntime::default());
    app.add_plugins((
        SandboxSimulationPlugin,
        SandboxLdtkPlugin,
        SandboxPresentationPlugin,
    ));
    app.add_plugins(
        ambition_gameplay_core::assets::sandbox_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
        ),
    );
    app.add_systems(Startup, setup_capture_target.after(PresentationSetupSet));
    app.add_systems(
        Update,
        (
            apply_capture_snapshot
                .after(camera_follow)
                .before(sync_parallax_layers),
            request_capture.after(sync_parallax_layers),
            finish_after_capture,
            fail_after_timeout,
        ),
    );
    app.run();
}

impl SceneCaptureConfig {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut warmup_frames = 12u32;
        let mut include_ui = false;
        let mut show_window = false;
        let mut i = 0usize;
        while i < args.len() {
            match args[i].as_str() {
                "--include-ui" => {
                    include_ui = true;
                    i += 1;
                }
                "--show-window" => {
                    show_window = true;
                    i += 1;
                }
                "--warmup" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--warmup requires a frame count".to_string());
                    };
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    i += 2;
                }
                arg if arg.starts_with("--warmup=") => {
                    let value = arg.trim_start_matches("--warmup=");
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    i += 1;
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option '{other}'"));
                }
                other => {
                    positional.push(other.to_string());
                    i += 1;
                }
            }
        }

        let Some(room_id) = positional.first().cloned() else {
            return Err("missing ROOM_ID".to_string());
        };
        let Some(focus_text) = positional.get(1) else {
            return Err("missing X,Y focus".to_string());
        };
        let focus = parse_vec2(focus_text)
            .ok_or_else(|| format!("focus must be X,Y world coordinates, got '{focus_text}'"))?;
        let output = positional
            .get(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/room_{room_id}_game.png")));
        let size = positional
            .get(3)
            .and_then(|text| parse_image_size(text))
            .unwrap_or(UVec2::new(1280, 720));
        Ok(Self {
            room_id,
            focus,
            output,
            size,
            warmup_frames,
            include_ui,
            show_window,
        })
    }
}

fn setup_capture_target(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut images: ResMut<Assets<Image>>,
    mut main_cameras: Query<(Entity, &mut Camera), With<MainCamera>>,
    mut hud_cameras: Query<(Entity, &mut Camera), (With<FrontHudCamera>, Without<MainCamera>)>,
) {
    if let Some(parent) = config.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "capture_scene: failed to create output directory '{}': {error}",
                parent.display()
            );
            commands.write_message(AppExit::from_code(2));
            return;
        }
    }

    let mut capture_image = Image::new_target_texture(
        config.size.x.max(1),
        config.size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    capture_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(capture_image);
    let target = RenderTarget::Image(ImageRenderTarget::from(image.clone()));

    for (entity, mut camera) in &mut main_cameras {
        camera.is_active = true;
        commands.entity(entity).insert((target.clone(), Msaa::Off));
    }
    for (entity, mut camera) in &mut hud_cameras {
        camera.is_active = config.include_ui;
        if config.include_ui {
            commands.entity(entity).insert((target.clone(), Msaa::Off));
        }
    }
    commands.insert_resource(SceneCaptureTarget { image });
}

fn apply_capture_snapshot(
    config: Res<SceneCaptureConfig>,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    room_set: Res<ambition_gameplay_core::rooms::RoomSet>,
    user_settings: Res<ambition_gameplay_core::persistence::settings::UserSettings>,
    ease_tuning: Res<ambition_gameplay_core::CameraEaseTuning>,
    mut view_state: ResMut<CameraViewState>,
    mut cameras: Query<
        (&mut Transform, &mut Projection),
        (
            With<MainCamera>,
            Without<ambition_render::rendering::PlayerVisual>,
        ),
    >,
) {
    let active_spec = room_set.active_spec();
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);
    let snapshot = resolve_follow_camera_snapshot(
        CameraSnapshotResolveInput {
            world: &world.0,
            camera_zones: &active_spec.camera_zones,
            focus: CameraFocus2d {
                center_world: config.focus,
                size: ae::Vec2::new(30.0, 48.0),
                base_size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
            },
            base_view,
            viewport_px: ae::Vec2::new(config.size.x as f32, config.size.y as f32),
            aspect_policy: user_settings.video.camera_aspect,
            framing: user_settings.video.camera_framing,
            overview_scale: 1.0,
            encounter_scale: 1.0,
            overview_camera: false,
            snap_camera: true,
            blink: None,
            dt: 0.0,
            mode: CameraSnapshotResolveMode::Instant,
            extra_clamp_center_world: None,
            ease_tuning: *ease_tuning,
        },
        None,
    );

    let x = snapshot.center_world.x - world.0.size.x * 0.5;
    let y = world.0.size.y * 0.5 - snapshot.center_world.y;
    *view_state = CameraViewState::from(&snapshot);

    for (mut transform, mut projection) in &mut cameras {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = snapshot.orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
        transform.rotation = Quat::from_rotation_z(snapshot.rotation_radians);
    }
}

fn request_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    target: Option<Res<SceneCaptureTarget>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
) {
    if runtime.requested || runtime.completed {
        if runtime.requested {
            runtime.wait_frames = runtime.wait_frames.saturating_add(1);
        }
        return;
    }
    runtime.frames += 1;
    if runtime.frames < config.warmup_frames.max(1) {
        return;
    }
    let Some(target) = target else {
        return;
    };
    commands
        .spawn(Readback::texture(target.image.clone()))
        .observe(save_readback_to_disk);
    runtime.requested = true;
    eprintln!(
        "capture_scene: texture readback requested -> {}",
        config.output.display()
    );
}

fn finish_after_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    runtime: Res<SceneCaptureRuntime>,
) {
    if !runtime.completed || runtime.failed {
        return;
    }
    println!(
        "capture_scene: wrote {} ({}x{} px)",
        config.output.display(),
        config.size.x,
        config.size.y,
    );
    commands.write_message(AppExit::Success);
}

fn save_readback_to_disk(
    event: On<ReadbackComplete>,
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut runtime: ResMut<SceneCaptureRuntime>,
) {
    commands.entity(event.entity).despawn();
    let width = config.size.x.max(1);
    let height = config.size.y.max(1);
    let row_bytes = width as usize * 4;
    let padded_row_bytes = row_bytes.div_ceil(256) * 256;
    let expected = padded_row_bytes * height as usize;
    if event.data.len() < expected {
        eprintln!(
            "capture_scene: readback returned {} bytes, expected at least {expected}",
            event.data.len()
        );
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }

    let mut pixels = vec![0u8; row_bytes * height as usize];
    for y in 0..height as usize {
        let src = y * padded_row_bytes;
        let dst = y * row_bytes;
        pixels[dst..dst + row_bytes].copy_from_slice(&event.data[src..src + row_bytes]);
    }

    let Some(image) = image::RgbaImage::from_raw(width, height, pixels) else {
        eprintln!("capture_scene: failed to build PNG buffer");
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    };
    if let Err(error) = image.save(&config.output) {
        eprintln!(
            "capture_scene: failed to save '{}': {error}",
            config.output.display()
        );
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }
    runtime.completed = true;
}

fn fail_after_timeout(mut commands: Commands, runtime: Res<SceneCaptureRuntime>) {
    if runtime.completed {
        return;
    }
    if runtime.frames > 600 || runtime.wait_frames > 600 {
        eprintln!("capture_scene: timed out waiting for texture readback");
        commands.write_message(AppExit::from_code(1));
    }
}

fn parse_vec2(text: &str) -> Option<ae::Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(ae::Vec2::new(
        x.trim().parse().ok()?,
        y.trim().parse().ok()?,
    ))
}

fn parse_image_size(text: &str) -> Option<UVec2> {
    let (w, h) = text.split_once('x').or_else(|| text.split_once('X'))?;
    Some(UVec2::new(w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn desktop_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let dev_assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_gameplay_core/assets");
    match dev_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
}
