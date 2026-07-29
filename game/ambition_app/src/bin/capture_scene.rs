//! Faithful Bevy scene capture for a camera-follow snapshot.
//!
//! This is the render-stack counterpart to
//! `ambition::actors/examples/render_room_geometry.rs capture`: it runs
//! the real presentation plugins, forces the main camera to the same
//! `CameraSnapshot2d` policy for an arbitrary focus point, renders into an
//! offscreen image target, and asks Bevy's screenshot pipeline to write that
//! render target to disk. It intentionally knows nothing about portals; portals
//! can later reuse the same "snapshot -> render target" seam.
//!
//! Usage:
//!   cargo run -p ambition_app --bin capture_scene -- <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [--warmup N] [--character ID] [--include-ui] [--show-window]
//!   cargo run -p ambition_app --bin capture_scene -- c136 1200,480 /tmp/c136_game.png 1280x720
//!   # center on the player, spawned AS the pirate admiral:
//!   cargo run -p ambition_app --bin capture_scene -- central_hub_main player /tmp/p.png --character npc_pirate_admiral --warmup 40

use std::path::{Path, PathBuf};

use ambition::engine_core as ae;
use ambition::platformer::camera_layers::{FrontHudCamera, MainCamera};
use ambition::platformer::schedule::GameMode;
use ambition::render::rendering::{camera_follow, sync_parallax_layers, CameraViewState};
use ambition::sim_view::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraFocus2d, CameraSnapshotResolveInput,
    CameraSnapshotResolveMode,
};
use ambition::sprite_sheet::game_assets::GameAssetConfig;
use ambition_app::app::{
    PresentationSetupSet, SandboxLdtkPlugin, SandboxPresentationPlugin, SandboxSimulationPlugin,
    StartRoomOverride,
};
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
    /// Optional `character_catalog.ron` id to spawn the player AS (its sprite +
    /// moveset). `None` = the default protagonist. Behind `--character <id>`.
    character: Option<String>,
    /// When the focus positional is the literal `player`, center the camera on
    /// the live player entity's position after warmup (no coordinate hunting).
    follow_player: bool,
    /// Photograph a SHELL ROUTE rather than a room (`--route <id>`).
    ///
    /// The surfaces a stranger sees first — the launcher, the startup cards,
    /// the versus stage and its HUD — are routes, and this tool could only ever
    /// reach rooms. Asking for one by room id silently captured the sandbox
    /// instead, which is the worst thing a verification tool can do (queue Z1).
    route: Option<String>,
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
    /// Route mode only: the shell has been told where to go. One request, not
    /// one per frame — a `GoTo` every update would restart the route forever.
    route_requested: bool,
    /// How many cameras the route capture has adopted, so the count is
    /// announced when it CHANGES rather than once per frame.
    cameras_adopted: usize,
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

    // A ROUTE is photographed through the composition a PLAYER runs: the shell
    // host, built by the same `build_visible_app` the desktop binary uses, on
    // the offscreen-GPU render mode. That mode exists because the older
    // no-window one sets `backends: None` and therefore has no render app at
    // all — a readback under it can never complete, which is what three
    // eliminated hypotheses were circling (queue Z1).
    if let Some(route_id) = config.route.clone() {
        run_route_capture(config, route_id);
        return;
    }

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
    // Optional "play as this character" override, inserted BEFORE the sandbox
    // preparation consumes it before publishing the exact session world.
    if let Some(character_id) = config.character.clone() {
        eprintln!("capture_scene: player wears character '{character_id}'");
        app.insert_resource(ambition_app::app::StartingCharacterOverride(
            ambition::actors::avatar::StartingCharacter::new(character_id),
        ));
    }
    app.insert_resource(config);
    app.insert_resource(SceneCaptureRuntime::default());
    app.add_plugins((
        SandboxSimulationPlugin,
        SandboxLdtkPlugin,
        SandboxPresentationPlugin,
    ));
    app.add_plugins(
        ambition::actors::assets::sandbox_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
            &ambition_content::worlds::world_manifest(),
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
        let mut character: Option<String> = None;
        let mut route: Option<String> = None;
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
                "--character" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--character requires a catalog id".to_string());
                    };
                    character = Some(value.clone());
                    i += 2;
                }
                arg if arg.starts_with("--character=") => {
                    character = Some(arg.trim_start_matches("--character=").to_string());
                    i += 1;
                }
                "--route" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--route requires a shell route id".to_string());
                    };
                    route = Some(value.clone());
                    i += 2;
                }
                arg if arg.starts_with("--route=") => {
                    route = Some(arg.trim_start_matches("--route=").to_string());
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

        // A ROUTE has no room id and no focus point: the shell composes its own
        // surface and its own cameras. Requiring the room positionals anyway
        // would mean inventing values that are then ignored, which is how a
        // flag ends up documented as "pass anything here".
        if let Some(route) = route.clone() {
            // **A ROUTE'S POSITIONALS ARE CLASSIFIED, NOT COUNTED.**
            //
            // Route mode takes no room id and no focus point, so its first
            // positional is the OUTPUT — and a caller who reasonably typed the
            // room-mode form (`<ROOM> <FOCUS> <OUT.png> <WxH> --route ...`) had
            // the room id silently adopted as the output path. The tool then
            // rendered the whole scene and failed at the very end with "the image
            // format could not be determined", naming nothing the caller had
            // typed. Found by running it (2026-07-29).
            //
            // Classifying instead of counting means an unexpected argument is
            // NAMED, and named before any work happens rather than after.
            let mut output: Option<PathBuf> = None;
            let mut size: Option<UVec2> = None;
            for value in &positional {
                if let Some(parsed) = parse_image_size(value) {
                    size = Some(parsed);
                } else if looks_like_image_path(value) {
                    output = Some(PathBuf::from(value));
                } else {
                    return Err(format!(
                        "--route takes no ROOM_ID or focus point: the shell composes its own \
                         surface and its own cameras. `{value}` is neither an output path \
                         (*.png / *.jpg) nor a size (WIDTHxHEIGHT).\n  \
                         usage: capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT]"
                    ));
                }
            }
            let output = output.unwrap_or_else(|| PathBuf::from(format!("/tmp/route_{route}.png")));
            let size = size.unwrap_or(UVec2::new(1280, 720));
            return Ok(Self {
                room_id: String::new(),
                focus: ae::Vec2::ZERO,
                output,
                size,
                warmup_frames: warmup_frames.max(90),
                // A route IS its UI. Capturing one without it would photograph
                // an empty clear colour and call it the launcher.
                include_ui: true,
                show_window,
                character,
                follow_player: false,
                route: Some(route),
            });
        }

        let Some(room_id) = positional.first().cloned() else {
            return Err("missing ROOM_ID".to_string());
        };
        let Some(focus_text) = positional.get(1) else {
            return Err("missing X,Y focus (or the literal `player`)".to_string());
        };
        // `player` centers the camera on the live player entity (handy for
        // starting-character shots — no per-room spawn coordinate to look up).
        let follow_player = focus_text.eq_ignore_ascii_case("player");
        let focus = if follow_player {
            ae::Vec2::ZERO
        } else {
            parse_vec2(focus_text).ok_or_else(|| {
                format!("focus must be X,Y world coordinates or `player`, got '{focus_text}'")
            })?
        };
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
            character,
            follow_player,
            route: None,
        })
    }
}

/// Photograph a shell ROUTE through the player's own composition.
///
/// Fails LOUDLY on an unknown route, naming the ones that exist: a capture that
/// silently photographs somewhere else is how a blind agent reports the wrong
/// thing with confidence, and that is the defect this whole mode exists to
/// remove.
fn run_route_capture(config: SceneCaptureConfig, route_id: String) {
    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        true,
    );

    let known: Vec<String> = {
        let catalog = app
            .world()
            .get_resource::<ambition::game_shell::ShellRouteCatalog>();
        catalog
            .map(|catalog| catalog.ids().map(|id| id.to_string()).collect())
            .unwrap_or_default()
    };
    if !known.is_empty() && !known.iter().any(|id| id == &route_id) {
        eprintln!(
            "capture_scene: unknown route '{route_id}'. Known routes: {}",
            known.join(", ")
        );
        std::process::exit(2);
    }

    app.insert_resource(config.clone());
    app.insert_resource(SceneCaptureRuntime::default());
    app.add_systems(Startup, setup_route_capture_target);
    app.add_systems(
        Update,
        (
            go_to_route,
            adopt_route_cameras,
            request_capture,
            finish_after_capture,
            fail_after_timeout,
        )
            .chain(),
    );
    app.run();
}

/// Drive the shell to the requested route once, on the first update.
fn go_to_route(
    config: Res<SceneCaptureConfig>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    mut commands: Commands,
) {
    if runtime.route_requested {
        return;
    }
    runtime.route_requested = true;
    let Some(route) = config.route.clone() else {
        return;
    };
    commands.write_message(ambition::game_shell::ShellCommand::GoTo(
        ambition::game_shell::ShellRouteId::new(route),
    ));
}

/// Retarget EVERY camera, not the gameplay markers.
///
/// A shell route has neither a `MainCamera` nor a `FrontHudCamera` — the
/// launcher reports three cameras carrying neither marker — so the marker-keyed
/// setup built an image nothing drew into. Which camera a route composes is the
/// route's business; that it must land in the capture target is ours.
fn setup_route_capture_target(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<(Entity, &mut Camera)>,
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
    commands.insert_resource(SceneCaptureTarget { image });
}

/// Adopt a route's cameras AS THEY APPEAR, every frame.
///
/// A shell route has no cameras at `Startup` — it spawns them when the route
/// composes, several frames in. Retargeting once at startup therefore adopted
/// ZERO cameras and produced a blank image that was written successfully and
/// reported as a capture, which is the same silent-wrong-answer this whole mode
/// exists to remove, three levels down. The count is printed for that reason.
fn adopt_route_cameras(
    mut commands: Commands,
    target: Option<Res<SceneCaptureTarget>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    mut cameras: Query<(Entity, &mut Camera)>,
) {
    let Some(target_res) = target else {
        return;
    };
    let target = RenderTarget::Image(ImageRenderTarget::from(target_res.image.clone()));
    // ONE target, several cameras — so only the FIRST may clear it.
    //
    // A shell route composes a stack (world, then UI, then overlays). Point
    // them all at one image with their default clear config and each wipes the
    // one before it, so the file that lands is whatever the LAST camera drew
    // over a fresh clear: a blank rectangle, written successfully, reported as
    // a capture. Ordering by `Camera::order` and clearing only on the lowest is
    // what makes the stack composite instead of compete.
    let mut ordered: Vec<(Entity, isize)> = cameras
        .iter()
        .map(|(entity, camera)| (entity, camera.order))
        .collect();
    ordered.sort_by_key(|(entity, order)| (*order, entity.index()));
    for (rank, (entity, order)) in ordered.iter().enumerate() {
        if let Ok((_, mut camera)) = cameras.get_mut(*entity) {
            camera.is_active = true;
            camera.clear_color = if rank == 0 {
                ClearColorConfig::Default
            } else {
                ClearColorConfig::None
            };
        }
        commands.entity(*entity).insert((target.clone(), Msaa::Off));
        eprintln!(
            "capture_scene: camera {rank} (order {order}) -> capture image{}",
            if rank == 0 { ", clears" } else { ", overlays" }
        );
    }
    if ordered.len() != runtime.cameras_adopted {
        eprintln!(
            "capture_scene: retargeted {} camera(s) to the capture image",
            ordered.len()
        );
        runtime.cameras_adopted = ordered.len();
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
    world: ambition::platformer::lifecycle::SessionWorldRef<ambition::engine_core::RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<ambition::actors::rooms::RoomSet>,
    user_settings: Res<ambition::persistence::settings::UserSettings>,
    ease_tuning: Res<ambition::platformer::camera_ease::CameraEaseTuning>,
    mut view_state: ResMut<CameraViewState>,
    player_q: Query<
        &ambition::platformer::body::BodyKinematics,
        With<ambition::render::rendering::PlayerVisual>,
    >,
    mut cameras: Query<
        (&mut Transform, &mut Projection),
        (
            With<MainCamera>,
            Without<ambition::render::rendering::PlayerVisual>,
        ),
    >,
) {
    let active_spec = room_set.active_spec();
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);
    // `player` focus mode: center on the live player body (falls back to the
    // fixed focus if the player isn't spawned yet).
    let focus_center = if config.follow_player {
        player_q
            .iter()
            .next()
            .map(|k| k.pos)
            .unwrap_or(config.focus)
    } else {
        config.focus
    };
    let snapshot = resolve_follow_camera_snapshot(
        CameraSnapshotResolveInput {
            world: &world.0,
            camera_zones: &active_spec.camera_zones,
            focus: CameraFocus2d {
                center_world: focus_center,
                size: ae::Vec2::new(30.0, 48.0),
                base_size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
                velocity_world: ae::Vec2::ZERO,
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
            screen_framing: None,
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

/// How long a route gets to produce a camera AFTER warmup, before this is called
/// a failure rather than a slow start.
///
/// Generous on purpose: a route that loads assets can legitimately take a while,
/// and a false failure in a verification tool is as bad as a false success.
const ROUTE_CAMERA_GRACE_FRAMES: u32 = 600;

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
    // **A ROUTE CAPTURE WAITS FOR A CAMERA, not for a clock.** (GPT 5.6, 2026-07-29)
    //
    // Warmup is a duration; readiness is a fact. With only the duration, a route
    // that is slow, broken, or never builds a camera at all let this tool read
    // back an untouched capture texture, write a blank PNG, and print success —
    // in a tool whose entire purpose is to stop a verification from silently
    // photographing the wrong thing.
    //
    // Rooms are exempt because their camera comes from the room setup this
    // binary performs itself; a route's comes from the shell, asynchronously.
    if config.route.is_some() && runtime.cameras_adopted == 0 {
        if runtime.frames < config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES {
            return;
        }
        runtime.failed = true;
        runtime.requested = true;
        eprintln!(
            "capture_scene: route '{}' adopted NO camera within {} frames. Nothing would \
             have been drawn, so no image is written — a blank PNG reported as success is \
             the failure this check exists for. Check that the route id is real and that \
             its presentation actually builds a camera.",
            config.route.as_deref().unwrap_or("<none>"),
            config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES,
        );
        commands.write_message(AppExit::from_code(2));
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

/// Does this argument name an image file the encoder can actually write?
///
/// Checked at PARSE time. The encoder infers its format from the extension, so a
/// path without one fails only after the scene has rendered and the readback has
/// come back — the most expensive possible moment to learn that an argument was
/// wrong, and the error names the extension rather than the argument.
fn looks_like_image_path(value: &str) -> bool {
    std::path::Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "tga" | "webp"
            )
        })
}

fn parse_image_size(text: &str) -> Option<UVec2> {
    let (w, h) = text.split_once('x').or_else(|| text.split_once('X'))?;
    Some(UVec2::new(w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn desktop_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let dev_assets =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/ambition::actors/assets");
    match dev_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
}
