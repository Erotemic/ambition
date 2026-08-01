//! Faithful Bevy scene capture for a camera-follow snapshot.
//!
//! This is the render-stack counterpart to
//! `ambition_platformer2d::actors/examples/render_room_geometry.rs capture`: it runs
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

use std::path::PathBuf;

use ambition_platformer2d::actors::character_runtime::{CharacterLoadDemand, CharacterLoadStates};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::camera_layers::{FrontHudCamera, MainCamera};
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::render::rendering::{CameraViewState, camera_follow, sync_parallax_layers};
use ambition_platformer2d::sim_view::camera_snapshot::{
    CameraFocus2d, CameraSnapshotResolveInput, CameraSnapshotResolveMode,
    resolve_follow_camera_snapshot,
};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig;
use ambition_app::app::{
    PresentationSetupSet, AmbitionGameLdtkRuntimePlugin, AmbitionGamePresentationPlugin, AmbitionGameSimulationPlugin,
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
    /// Keep the DEVELOPER overlays in the shot (`--dev-overlays`).
    ///
    /// Off by default, and that default is the point: this tool exists to show
    /// what is actually on screen, and a `desktop_dev` build puts a debug banner,
    /// an FPS counter and per-entity nameplates on top of the product. Reading
    /// those as product is a mistake I made for a whole session — the Ambition
    /// route's `military_tower_door` / `hall_of_bosses_door` labels look exactly
    /// like raw identifiers leaking into player UI, and are debug nameplates
    /// (2026-07-29).
    ///
    /// A verification screenshot should show the PRODUCT. The debugging use that
    /// genuinely wants nameplates asks for them.
    dev_overlays: bool,
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
    /// **Has the world this capture photographs finished being BUILT?**
    ///
    /// Warmup used to count from frame zero, and the world is constructed
    /// asynchronously — assets stream, the room stages, the player spawns. So
    /// `--warmup 60` meant "sixty frames after BOOT", of which an unpredictable
    /// number happened before there was anything to simulate. The body ended up
    /// on a slightly different tick of its own idle each run, and because
    /// nameplate opacity is ranked by DISTANCE from the focus, a few pixels of
    /// player drift re-ordered the labels and rewrote their text.
    ///
    /// That is the whole of the ~130px noise floor two identical runs used to
    /// show (AC6). Counting from readiness makes N frames mean N ticks of a
    /// world that exists.
    world_ready: bool,
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
    pin_the_clock(&mut app);
    app.init_state::<GameMode>();
    app.insert_resource(asset_config);
    app.insert_resource(StartRoomOverride(config.room_id.clone()));
    // A capture that photographs a DIFFERENT room than the one asked for is the
    // worst failure this tool has, and it had it: two real room ids and one
    // invented one all produced the hub, each writing a valid PNG and exiting 0.
    app.insert_resource(ambition_app::app::StartRoomMustResolve);
    // Optional "play as this character" override, inserted BEFORE the sandbox
    // preparation consumes it before publishing the exact session world.
    if let Some(character_id) = config.character.clone() {
        eprintln!("capture_scene: player wears character '{character_id}'");
        app.insert_resource(ambition_app::app::StartingCharacterOverride(
            ambition_platformer2d::actors::avatar::StartingCharacter::new(character_id),
        ));
    }
    // **THE SURFACE THIS RUN DRAWS TO.**
    //
    // There is no `Window` here, so the host's layout resolver found none and
    // returned — leaving `ResolvedGameplayPresentation` at its DEFAULT, whose
    // display rect is 1600x900. Every HUD position was therefore laid out for a
    // 1600x900 screen and then rendered into whatever the capture size is: a card
    // centred at x=800 lands right of centre in a 960-wide image, on top of a
    // fighter (queue Z′8, measured 2026-07-29).
    //
    // A capture that cannot show a layout is worse than no capture, because it
    // shows a DIFFERENT layout convincingly.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(config.size.x as f32, config.size.y as f32),
        ),
    );
    app.insert_resource(config);
    app.insert_resource(SceneCaptureRuntime::default());
    app.add_plugins((
        AmbitionGameSimulationPlugin,
        AmbitionGameLdtkRuntimePlugin,
        AmbitionGamePresentationPlugin,
        // **THE LAYOUT RESOLVER**, which the sandbox plugins do not install.
        //
        // Without it `ResolvedGameplayPresentation` stays at its DEFAULT, whose
        // display rect is `WINDOW_W x WINDOW_H` (1600x900) — so every HUD slot
        // laid out for a 1600x900 screen and was then rendered into whatever the
        // capture size is. A card centred at x=800 lands right of centre in a
        // 960-wide image, on top of a fighter, and reads as a game bug that a
        // window would not have (measured 2026-07-29).
        //
        // A capture that cannot show a layout is worse than no capture, because
        // it shows a DIFFERENT layout convincingly.
    ));
    // The layout resolver, if this composition does not already have it. Guarded
    // because the two capture modes build their apps differently and only one of
    // them goes through `build_visible_app`.
    if !app
        .is_plugin_added::<ambition_platformer2d::host::gameplay_presentation::HostGameplayPresentationPlugin>()
    {
        app.add_plugins(ambition_platformer2d::host::gameplay_presentation::HostGameplayPresentationPlugin);
    }
    app.add_plugins(
        ambition_platformer2d::actors::assets::platformer_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
            &ambition_content::worlds::world_manifest(),
        ),
    );
    app.add_systems(Startup, setup_capture_target.after(PresentationSetupSet));
    app.add_systems(
        Update,
        (
            // ⚠ ROOM mode needs this as much as route mode does, and the first
            // version wired it only into route mode — so `--dev-overlays` was
            // silently ignored for half the tool's invocations. Same
            // two-app-builders shape as the `--route` positional bug and the
            // headless-surface insert; third time in one session, which is why
            // this comment names it rather than just fixing it (2026-07-29).
            silence_dev_overlays,
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
        let mut dev_overlays = false;
        let mut show_window = false;
        let mut character: Option<String> = None;
        let mut route: Option<String> = None;
        let mut i = 0usize;
        while i < args.len() {
            match args[i].as_str() {
                "--dev-overlays" => {
                    dev_overlays = true;
                    i += 1;
                }
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
                dev_overlays,
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
            dev_overlays,
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
/// Silence the developer overlays unless the caller asked for them.
///
/// A `desktop_dev` build draws a debug banner, an FPS counter and per-entity
/// nameplates on top of the product, and this tool exists to show what is on
/// screen. Reading that scaffolding as product is a mistake worth engineering
/// against: on the Ambition route the nameplates read `military_tower_door` and
/// `hall_of_bosses_door`, which look exactly like raw identifiers leaking into
/// player UI (2026-07-29).
///
/// ⚠ a SYSTEM, not a one-shot insert at build time. Settings are loaded and
/// re-applied during startup, so a value written before the first update is
/// overwritten by the load — which is exactly what the first version of this did,
/// silently, leaving every overlay on screen. Forcing it each frame is
/// order-independent, and for a capture tool that costs nothing.
///
/// Written as SETTINGS rather than by disabling plugins, because the settings are
/// what a player toggles: a clean capture is a configuration a player can reach,
/// not a special build that might diverge from one.
fn silence_dev_overlays(
    config: Res<SceneCaptureConfig>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if config.dev_overlays {
        return;
    }
    if settings.gameplay.debug_hud_visible {
        settings.gameplay.debug_hud_visible = false;
    }
    if settings.video.show_fps {
        settings.video.show_fps = false;
    }
    if let Some(developer) = developer.as_mut() {
        if developer.show_hud {
            developer.show_hud = false;
        }
    }
}

/// **A frame of warmup must mean the same amount of TIME every run.**
///
/// The capture advances the app with `ScheduleRunnerPlugin::run_loop(ZERO)` —
/// as fast as the machine goes — and Bevy's default clock advances by the real
/// duration each frame actually took. So `--warmup 60` bought sixty frames of
/// *whatever the CPU managed*, and two runs of the same binary with identical
/// arguments landed on different animation poses.
///
/// That was measured, not assumed: two identical runs differed by ~132 pixels,
/// and an unrelated change measured 800–1450 — a number I nearly filed as a
/// rendering regression before tiling the two images and seeing the same
/// silhouette with its arms in a different part of the idle bob (2026-07-29).
///
/// It matters beyond tidiness. This tool is the repository's eyes: every "the
/// room renders", "the sprite is distinct", "nothing changed" conclusion is a
/// pixel comparison, and a comparison against a moving baseline is only as good
/// as the gap between signal and noise. Pinning the clock makes a zero-diff
/// evidence instead of a coincidence.
///
/// 60 Hz, matching `ambition_platformer2d_runtime::SIM_TICK_HZ`, so a warmup frame is a sim
/// tick and `--warmup N` reads as "N ticks in".
fn pin_the_clock(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(1.0 / ambition_platformer2d::runtime::SIM_TICK_HZ),
    ));
}

fn run_route_capture(config: SceneCaptureConfig, route_id: String) {
    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        true,
    );
    // **THE SURFACE THIS RUN DRAWS TO.** (queue Z′8)
    //
    // `OffscreenGpu` sets `primary_window: None`, so the host's layout resolver
    // finds no window and `ResolvedGameplayPresentation` keeps its DEFAULT —
    // whose display rect is `WINDOW_W x WINDOW_H`, 1600x900. Every HUD slot then
    // lays out for a 1600x900 screen and is rendered into whatever the capture
    // size is: a card declared `.centered()` centres at x=800 and lands right of
    // centre in a 960-wide image, on top of a fighter. It reads as a game bug
    // that a real window does not have (measured 2026-07-29).
    //
    // ⚠ the ROOM-mode builder below needs the same insert, and each mode builds
    // its own app — which is exactly how the first attempt at this missed.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(config.size.x as f32, config.size.y as f32),
        ),
    );

    let known: Vec<String> = {
        let catalog = app
            .world()
            .get_resource::<ambition_platformer2d::game_shell::ShellRouteCatalog>();
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

    // ⚠ **AND THE CLOCK, in BOTH builders.** Room mode pins it too; this tool
    // has now shipped the same half-wired flag three times (the `--route`
    // positional, the headless surface above, `--dev-overlays`) because each
    // mode assembles its own app and a change to one reads as done.
    pin_the_clock(&mut app);

    app.insert_resource(config.clone());
    app.insert_resource(SceneCaptureRuntime::default());
    app.add_systems(Startup, setup_route_capture_target);
    app.add_systems(
        Update,
        (
            silence_dev_overlays,
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

/// Drive the shell to the requested route — **unless it is already there.**
///
/// ⚠ this sent the `GoTo` unconditionally on the first update, and for the HOME
/// route that photographed a surface a player never sees: the shell's own
/// `initialize_shell` activates home, then this navigated to it AGAIN, so the
/// launcher tree was torn down and rebuilt a second time
/// (`ShellActivationId` 1 → 2) before the shutter. A capture tool that stages a
/// state the game does not reach is the same defect class as one that
/// photographs the wrong room silently — this row's whole subject.
///
/// Found while instrumenting rebuild counts for an unrelated question.
///
/// It WAITS for the router to settle rather than firing on frame one: before
/// initialization there is no active route to compare against, and navigating
/// then would race the shell's own boot. `fail_after_timeout` covers a shell
/// that never activates anything.
fn go_to_route(
    config: Res<SceneCaptureConfig>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    router: Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut commands: Commands,
) {
    if runtime.route_requested {
        return;
    }
    let Some(route) = config.route.clone() else {
        runtime.route_requested = true;
        return;
    };
    let Some(active) = router.active.as_ref() else {
        // The shell has not settled yet; do not race its boot.
        return;
    };
    runtime.route_requested = true;
    if active.route_id.as_str() == route {
        eprintln!(
            "capture_scene: the shell is already on '{route}'; photographing it \
             as booted rather than re-activating it"
        );
        return;
    }
    commands.write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(route),
    ));
}

/// Create the image a route capture draws into, and nothing else.
///
/// ⚠ this used to be documented as *"Retarget EVERY camera, not the gameplay
/// markers"* and carried a `Query<(Entity, &mut Camera)>` it never touched. The
/// retargeting is [`adopt_route_cameras`]' job — it has to run EVERY frame,
/// because a shell route has no cameras at `Startup` — and this function makes
/// the target they get pointed at.
///
/// The unused query was the compiler saying so: a doc comment describing a
/// neighbour's behaviour reads as a second implementation of it, and the next
/// person to debug a blank route capture would have looked here first
/// (2026-07-29).
fn setup_route_capture_target(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    mut images: ResMut<Assets<Image>>,
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
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ambition_platformer2d::engine_core::RoomGeometry>,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ambition_platformer2d::actors::rooms::RoomSet>,
    user_settings: Res<ambition_platformer2d::persistence::settings::UserSettings>,
    ease_tuning: Res<ambition_platformer2d::platformer::camera_ease::CameraEaseTuning>,
    mut view_state: ResMut<CameraViewState>,
    // **THE SIM BODY, not the render visual.**
    //
    // This queried `BodyKinematics` `With<PlayerVisual>`, and `PlayerVisual` is a
    // RENDER-side marker on a by-id render entity that carries no kinematics. So
    // the query never matched, `player` focus fell back to `config.focus` — which
    // is `Vec2::ZERO` in that mode — and every room capture photographed the origin
    // while the player stood elsewhere.
    //
    // That is the whole of the "room mode renders an empty image" defect: the
    // room stages fine (138 room visuals, a player body at (950, 904)) and the
    // camera sat at (0, 120). Measured, after three wrong guesses (2026-07-29).
    player_q: Query<
        &ambition_platformer2d::platformer::body::BodyKinematics,
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >,
    mut cameras: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
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

/// Whether there is a constructed world to start counting warmup against.
///
/// Two conditions, and the second one is the interesting half.
///
/// **The body exists.** A player-focused capture waits for the body it is going
/// to centre on; a coordinate-focused one has nothing specific to wait for.
///
/// **Its ART has a terminal answer.** A decoded sheet RESIZES the body it
/// belongs to — `SpritePosedBody` derives the collision box from the art — so a
/// sheet that lands on frame 7 in one run and frame 11 in another gives the body
/// a different shape for a different number of ticks while it is still falling
/// toward the floor, and it settles a pixel or two apart. `character_reveal_ready`
/// is the existing answer to "has every staged character finished loading, one
/// way or the other" (§4.9 forbids the silent third state), so waiting on it
/// removes the asynchrony rather than hoping it has passed.
fn world_is_ready(
    player_q: &Query<
        &ambition_platformer2d::platformer::body::BodyKinematics,
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >,
    follow_player: bool,
    art: Option<(&CharacterLoadDemand, &CharacterLoadStates)>,
) -> bool {
    if follow_player && player_q.iter().next().is_none() {
        return false;
    }
    match art {
        Some((demand, states)) => {
            ambition_platformer2d::actors::character_runtime::character_reveal_ready(demand, states)
        }
        // A composition with no character-load seam has no art to wait for.
        None => true,
    }
}

fn request_capture(
    mut commands: Commands,
    config: Res<SceneCaptureConfig>,
    target: Option<Res<SceneCaptureTarget>>,
    mut runtime: ResMut<SceneCaptureRuntime>,
    player_q: Query<
        &ambition_platformer2d::platformer::body::BodyKinematics,
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >,
    art_demand: Option<Res<CharacterLoadDemand>>,
    art_states: Option<Res<CharacterLoadStates>>,
) {
    if runtime.requested || runtime.completed {
        if runtime.requested {
            runtime.wait_frames = runtime.wait_frames.saturating_add(1);
        }
        return;
    }
    // **WARMUP COUNTS FROM A READY WORLD, not from boot.**
    //
    // Same distinction the route-camera check below draws — warmup is a
    // duration, readiness is a fact — applied to the other end of the capture.
    // Until the body being photographed exists there is nothing for a tick to
    // advance, so frames spent waiting for it are not warmup, they are latency.
    if !runtime.world_ready {
        let art = art_demand.as_deref().zip(art_states.as_deref());
        if !world_is_ready(&player_q, config.follow_player, art) {
            return;
        }
        runtime.world_ready = true;
    }
    runtime.frames += 1;
    if runtime.frames < config.warmup_frames.max(1) {
        return;
    }
    // **WHERE THE SUBJECT ACTUALLY IS**, printed once, at the tick the image is
    // taken. A capture tool that reports the room and not the pose can tell you
    // an image was written and nothing about whether two images should match —
    // and comparing two captures is what this tool is FOR. It is also the
    // measurement that separates a simulation difference from a rendering one
    // (AC6, 2026-07-29).
    if let Some(kin) = player_q.iter().next() {
        println!(
            "capture_scene: subject at ({:.4}, {:.4}) after {} warmup tick(s)",
            kin.pos.x, kin.pos.y, runtime.frames
        );
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

/// The backstop for a capture that never finishes, DERIVED from what the run was
/// asked to wait for rather than fixed at 600.
///
/// ⚠ this was a flat `runtime.frames > 600`, which quietly preempted every
/// policy above it. The route-readiness check allows `warmup + 600` frames for a
/// camera to appear, so for ANY warmup above zero the generic timeout fired
/// first: the route-specific diagnostic — the one that says *which* route never
/// produced a camera — was unreachable, and a `--warmup` above 600 could not
/// complete at all (GPT 5.6, 2026-07-29).
///
/// It never produced a false SUCCESS, which is why it survived a session of use.
/// A backstop that fires before the thing it is backstopping is still just a
/// shorter timeout wearing a policy's name.
fn fail_after_timeout(
    mut commands: Commands,
    runtime: Res<SceneCaptureRuntime>,
    config: Res<SceneCaptureConfig>,
) {
    if runtime.completed {
        return;
    }
    // Whatever the readiness policies may legitimately still be waiting for,
    // plus the same slack the readback itself gets.
    let budget = config.warmup_frames.max(1) + ROUTE_CAMERA_GRACE_FRAMES;
    if runtime.frames > budget || runtime.wait_frames > 600 {
        eprintln!(
            "capture_scene: timed out waiting for texture readback after {} frames \
             (warmup {} + grace {})",
            runtime.frames, config.warmup_frames, ROUTE_CAMERA_GRACE_FRAMES
        );
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

/// **THE Z′14 BUG, and it was one character.**
///
/// This carried its own copy of the asset-root rule, and the copy said
/// `crates/ambition_platformer2d::actors/assets` — a `::` where the crate name has a `_`. No
/// such directory can exist, `canonicalize` failed every time, and the fallback
/// pointed the room composition at the workspace-root `assets/` tree, which
/// holds IPFS metadata and none of the actor sprites, shaders or sounds.
///
/// So room-mode capture wrote a valid PNG of a room whose art never resolved,
/// and exited 0. Six measurements narrowed it to "the entities exist and the
/// sprite half does not"; this is why (GPT 5.6, 2026-07-29). Route mode goes
/// through the visible app and its own correct root, which is exactly why
/// `--route` looked fine while rooms did not.
///
/// ⚠ the lesson is the duplication, not the typo. `ambition_asset_manager`
/// exists *because* a demo that rendered nothing standalone was this same
/// divergence, and the fix then was to make one helper the single source of
/// truth. A second copy in a verification tool is worse than a second copy
/// anywhere else: the tool's whole job is to tell you what is on screen.
fn desktop_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    ambition_platformer2d::asset_manager::actors_desktop_asset_root()
}
