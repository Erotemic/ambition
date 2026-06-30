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
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Captured, Screenshot};
use bevy::window::{ExitCondition, Window, WindowPlugin, WindowResolution};

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
    requested: bool,
    completed: bool,
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

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                file_path: asset_root,
                ..default()
            })
            .set(WindowPlugin {
                // The default WindowPlugin exits as soon as there are no open
                // windows. This tool primarily renders to an offscreen Image, so
                // keep the app alive until `finish_after_capture` or
                // `fail_after_timeout` sends AppExit explicitly. `--show-window`
                // is still available when a visible debugging window is useful.
                primary_window: config.show_window.then(|| Window {
                    title: "Ambition capture_scene".into(),
                    resolution: WindowResolution::new(config.size.x, config.size.y),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
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

    let image = images.add(Image::new_target_texture(
        config.size.x.max(1),
        config.size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
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
        return;
    }
    runtime.frames += 1;
    if runtime.frames < config.warmup_frames.max(1) {
        return;
    }
    let Some(target) = target else {
        return;
    };
    let path = config.output.to_string_lossy().into_owned();
    commands
        .spawn(Screenshot::image(target.image.clone()))
        .observe(save_to_disk(path.clone()));
    runtime.requested = true;
    eprintln!("capture_scene: screenshot requested -> {path}");
}

fn finish_after_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    captured: Query<Entity, With<Captured>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
) {
    if runtime.completed || !runtime.requested {
        return;
    }
    if captured.iter().next().is_some() {
        runtime.completed = true;
        println!(
            "capture_scene: wrote {} ({}x{} px)",
            config.output.display(),
            config.size.x,
            config.size.y,
        );
        commands.write_message(AppExit::Success);
    }
}

fn fail_after_timeout(mut commands: Commands, runtime: Res<SceneCaptureRuntime>) {
    if runtime.completed {
        return;
    }
    if runtime.frames > 600 {
        eprintln!("capture_scene: timed out waiting for screenshot capture");
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
