//! Render a room or route into an offscreen screenshot using the production app
//! composition and camera policy.
//!
//! Usage:
//! `cargo run -p ambition_app_tools --bin capture_scene -- <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [--warmup N] [--character ID] [--include-ui]`
//!
//! For Smash captures, `--press touch:XxY,...` can drive the select screen before
//! capture; keep coordinate recipes covered by host tests because the UI layout can
//! move.

use std::path::PathBuf;

use ambition_app::app::{PresentationSetupSet, StartRoomOverride};
use ambition_platformer2d::actors::character_runtime::{CharacterLoadDemand, CharacterLoadStates};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::camera_layers::{FrontHudCamera, MainCamera};
use ambition_platformer2d::render::rendering::{
    camera_follow, sync_parallax_layers, CameraViewState,
};
use ambition_platformer2d::sim_view::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraFocus2d, CameraSnapshotResolveInput,
    CameraSnapshotResolveMode,
};
use bevy::app::AppExit;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};

#[derive(Resource, Clone, Debug)]
struct SceneCaptureConfig {
    room_id: String,
    focus: ae::Vec2,
    output: PathBuf,
    size: UVec2,
    warmup_frames: u32,
    /// ⭐⭐ HOW MANY SHOTS, and the whole difference between a photograph and an
    /// animation. `1` (the default) is the single-shot tool this has always
    /// been and is byte-identical to it. Higher re-arms the capture after each
    /// readback and numbers the files `<stem>.0000.png`, so a move can be
    /// photographed while it PLAYS — which is the only way to see what a move
    /// looks like without a person watching it.
    frames: usize,
    /// Sim frames to advance between shots of a sequence. `1` photographs every
    /// frame; higher samples a long move without hundreds of files.
    stride: u32,
    include_ui: bool,
    /// Frame the whole ROOM instead of a point (`--fit-room`).
    ///
    /// the focus positional answers *"what is happening here"*; this answers *"what does this room
    /// LOOK like"*, which is a different question and the one a scale problem is visible in.
    ///
    /// it BYPASSES the camera snapshot rather than feeding it a wider focus:
    /// that resolver clamps to the room and to camera zones, which is exactly
    /// right for gameplay and exactly wrong for a portrait of the room.
    fit_room: bool,
    /// Optional `character_catalog.ron` id to spawn the player AS (its sprite +
    /// moveset). `None` = the default protagonist. Behind `--character <id>`.
    ///
    /// ⭐ ON `--route smash_gameplay` IT SEATS A MATCH instead, two of this
    /// character against each other — because a smash move photographed in an
    /// exploration room is photographed under exploration rules.
    character: Option<String>,
    /// When the focus positional is the literal `player`, center the camera on
    /// the live player entity's position after warmup (no coordinate hunting).
    follow_player: bool,
    /// Keep the DEVELOPER overlays in the shot (`--dev-overlays`).
    ///
    /// A verification screenshot should show the PRODUCT. The debugging use that
    /// genuinely wants nameplates asks for them.
    dev_overlays: bool,
    /// Put the COMBAT debug view in the shot (`--combat-overlay`).
    ///
    /// `--dev-overlays` only stops the tool SILENCING what a build already
    /// shows; the combat volumes are a preset a player reaches through the
    /// settings menu, and every field it sets is off by default. So a swing
    /// could be photographed and its hit polygon could not, which is the one
    /// question a melee capture is usually asked.
    ///
    /// This turns on exactly the `DebugViewMode::Combat` preset — the same
    /// state the menu produces, not a private capture-only rendering path — so
    /// what comes back is a configuration a player can reach. Implies
    /// `--dev-overlays`.
    combat_overlay: bool,
    /// Screen post-process effects to force on (`--screen-effect crt,vignette`).
    ///
    /// ⛔⛔ WITHOUT THIS FLAG A CAPTURE CANNOT SEE THE POST-PROCESS AT ALL, and
    /// it looks like it can. The effects are `UserSettings.video.shaders` state,
    /// every strength is zero by default, and a windowless host inserts
    /// `PersistenceRoot::isolated()` — a fresh temp directory — so a hand-written
    /// `settings.ron` in the player's data dir is not read either. Two captures
    /// taken that way come back BYTE-IDENTICAL and read as "the post-process is
    /// broken"; measured 2026-08-31, that is what they mean by "nothing asked
    /// for an effect".
    ///
    /// ⭐ The same fields the settings menu writes — not a capture-only
    /// rendering path — so what comes back is a look a player can reach.
    screen_effects: Vec<ScreenEffect>,
    /// Input to deliver after reaching the route and before the shutter —
    /// key taps and glass taps (`--press touch:167x523,touch:167x523`).
    ///
    /// Deliberately a generic input vocabulary rather than a `--smash-cpu`
    /// flag, so any route with a lobby gets it for free.
    ///
    /// They are not. `smash_in_the_host.rs` seats a fighter with `click(app, rect)`, which is
    /// `SelectCursor::move_to(rect.center())` and THEN `tap(Enter)` — the POSITION is the
    /// load-bearing half. A bare `Enter` from here commits wherever the cursor happens to sit, so
    /// `--press Down,Enter,Enter` left all four slots reading `NOT PLAYING` and `--route
    /// smash_gameplay` photographed an empty stage.
    ///
    /// `touch:X x Y` is the step that carries a position, and it carries
    /// it down the road a phone uses: two real `TouchInput` messages folded by
    /// Bevy's own `touch_screen_input_system`. `select_screen::touch_tests`
    /// already pins that road, so the tool and the suite drive the same seam
    /// rather than two that can disagree. See [`PressStep::Touch`].
    ///
    /// arrow keys are still the right tool for a LIST — the launcher rows,
    /// the menus — and for gameplay. They just cannot name a rectangle.
    press: Vec<PressStep>,
    /// Photograph a SHELL ROUTE rather than a room (`--route <id>`).
    ///
    /// The surfaces a stranger sees first — the launcher, the startup cards,
    /// the versus stage and its HUD — are routes, and this tool could only ever
    /// reach rooms. Asking for one by room id silently captured the sandbox
    /// instead, which is the worst thing a verification tool can do.
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
    /// How far through `--press` the driver has got, and whether the key it
    /// tapped is still held. A press and its release are two frames because the
    /// surfaces being driven read EDGES — the select screen's own headless
    /// drivers do exactly this, and a held key is not a second press.
    press_cursor: usize,
    press_held: Option<KeyCode>,
    /// ⭐⭐ WHICH SHOT OF A SEQUENCE this is. `0` and a `frames` of 1 is the
    /// single-photograph tool this has always been; anything higher re-arms
    /// after each readback and photographs a MOVING scene, which is what makes
    /// an animation.
    shot: usize,
    /// Frames still to wait before re-arming the next shot of a sequence.
    stride_left: u32,
    /// The finger a `touch:X,Y` step put down and has not lifted yet, and the
    /// id it went down with. Same two-frame shape as a key tap and for the same
    /// reason — a press edge and a release edge are two different frames — but
    /// kept apart from `press_held` because the two travel different seams and
    /// a step that mixed them would release a key that was never pressed.
    touch_held: Option<Vec2>,
    /// The id the NEXT finger goes down with. Fresh per tap rather than reused:
    /// `Touches` keys everything by id, and a second `Started` under an id the
    /// fold has not finished retiring is a state a phone never produces.
    next_touch_id: u64,
    /// Frames left on a `wait` step.
    press_wait: u32,
    /// The frame the last key was released on. The sequence usually STARTS a
    /// route change ("Starting…"), so the shutter has to wait for the state the
    /// presses asked for rather than photograph the moment they were accepted —
    /// the first run of this caught exactly that and photographed the select
    /// screen mid-confirmation.
    press_done_frame: Option<u32>,
    /// How many cameras the route capture has adopted, so the count is
    /// announced when it CHANGES rather than once per frame.
    cameras_adopted: usize,
    /// Has the world this capture photographs finished being BUILT?
    ///
    /// So `--warmup 60` meant "sixty frames after BOOT", of which an unpredictable number happened
    /// before there was anything to simulate. The body ended up on a slightly different tick of its
    /// own idle each run, and because nameplate opacity is ranked by DISTANCE from the focus, a few
    /// pixels of player drift re-ordered the labels and rewrote their text.
    world_ready: bool,
}

const USAGE: &str = "\
capture_scene — photograph a room or a shell route through the real render stack.

USAGE:
    capture_scene <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
    capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]

OPTIONS:
    --warmup N          frames to settle before the first shot [default: 60]
    --frames N          take N shots, numbered <stem>.NNNN.png [default: 1]
    --stride K          sim frames between shots of a sequence [default: 1]
    --character ID      spawn the player AS this catalog character
                        (with --route smash_gameplay: seats a match of two)
    --route ID          photograph a shell route instead of a room
    --press SEQ         drive input first, e.g. `Down,Enter` or `touch:167x523`
                        (`hold:up` / `release:up` / `wait:30` also work)
    --include-ui        keep the game's UI in the shot
    --dev-overlays      stop silencing the developer chrome
    --combat-overlay    force the COMBAT gizmos on (hitboxes, collision boxes)
    --screen-effect E   force screen post-process effects on, comma separated:
                        crt, grain, vignette, robot, underwater, deep_dream.
                        ⛔ PAIR IT WITH `AMBITION_QUALITY_PROFILE=ultra`: the
                        Potato tier scales screen shaders to zero, and Potato is
                        what a software rasteriser gets seeded to. The tool says
                        so rather than photographing nothing quietly.

⛔⛔ THE PARALLAX BACKDROP GOES THE SAME WAY, AND IT IS 96% OF THE DRAWN AREA.
    `spawn_parallax_layers` early-returns when the parallax budget is disabled,
    and Potato disables it — so a capture taken WITHOUT
    `AMBITION_QUALITY_PROFILE=ultra` on a machine with no GPU is a photograph of
    the room with its sky missing. Measured 2026-09-01 in `water_world`: total
    sprite coverage 631,267 px at the default tier against 14,564,876 px at
    ultra, a 23x difference decided by an environment variable this command line
    does not mention. (World units, not pixels — the ratio is the reading.)
    --fit-room          frame the whole room instead of a point
    -h, --help          print this and exit

NOTES:
    `--dev-overlays` does NOT turn the gizmo pass on — it only stops this tool
    silencing the HUD. For boxes in the shot you want `--combat-overlay`.

    --frames is what makes an animation: a single shot keeps the exact output
    path you named, a sequence numbers its files.
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let config = match SceneCaptureConfig::from_args(std::env::args().skip(1).collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}\n");
            eprint!("{USAGE}");
            std::process::exit(2);
        }
    };

    match config.route.as_deref() {
        Some(route) => eprintln!(
            "capture_scene: route={route} size={}x{} out={}",
            config.size.x,
            config.size.y,
            config.output.display(),
        ),
        None => eprintln!(
            "capture_scene: room={} focus=({:.1},{:.1}) size={}x{} out={}",
            config.room_id,
            config.focus.x,
            config.focus.y,
            config.size.x,
            config.size.y,
            config.output.display(),
        ),
    }

    let mut app = build_capture_app(&config);

    match config.route.clone() {
        Some(route_id) => install_route_capture(&mut app, route_id),
        None => install_room_capture(&mut app),
    }
    app.run();
}

/// THE ONE APP THIS TOOL BUILDS.
///
/// There was no moment to insert them in, so the tool built its own app instead, and that copy
/// silently lost the `--route` positional, the headless display surface, `--dev-overlays`,
/// `--combat-overlay`, and — for two days — the entire room, because nothing added
/// `install_ambition_shell_visuals` to it.
///
/// `build_visible_app_with` is the moment that did not exist. With it, a ROOM and a ROUTE
/// differ by one boolean (which route the shell boots into) and by which capture systems get
/// installed.
fn build_capture_app(config: &SceneCaptureConfig) -> App {
    // The offscreen-GPU mode, always. The older no-window one sets
    // `backends: None` and therefore has no render app at all — a readback under
    // it can never complete, which is what three eliminated hypotheses were
    // circling.
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        // this boolean chooses the INITIAL ROUTE, not whether a shell exists —
        // since K2b both arms are shell-hosted. `true` boots the launcher, which
        // is where a `--route` capture navigates from; `false` boots straight to
        // the gameplay route, which is what `--direct` and every `--start-room`
        // alias mean and what a ROOM capture is.
        config.route.is_some(),
        |app| {
            if config.route.is_none() {
                app.insert_resource(StartRoomOverride(config.room_id.clone()));
                // A capture that photographs a DIFFERENT room than the one asked
                // for is the worst failure this tool has, and it had it: two real
                // room ids and one invented one all produced the hub, each
                // writing a valid PNG and exiting 0.
                app.insert_resource(ambition_app::app::StartRoomMustResolve);
            }
            // Optional "play as this character" override.
            if let Some(character_id) = config.character.clone() {
                eprintln!("capture_scene: player wears character '{character_id}'");
                app.insert_resource(ambition_app::app::StartingCharacterOverride(
                    ambition_platformer2d::actors::avatar::StartingCharacter::new(character_id),
                ));
            }
        },
    );
    // THE SURFACE THIS RUN DRAWS TO. (queue Z′8)
    //
    // A capture that cannot show a layout is worse than no capture, because it
    // shows a DIFFERENT layout convincingly.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(
                config.size.x as f32,
                config.size.y as f32,
            ),
        ),
    );
    // THE ENGINE'S OWN LOG, which every windowless host disables.
    //
    // `build_visible_app` drops `LogPlugin` from `NoWindow` and `OffscreenGpu`
    // for a reason that is true of tests and false of this binary: *"tests build
    // several Apps per process; the tracing subscriber is process-global."* A
    // capture builds exactly one App and then exits.
    //
    // Added after the group rather than by un-disabling it, so it applies to both
    // capture modes at once and cannot be half-wired the way five flags were.
    app.add_plugins(bevy::log::LogPlugin::default());
    app.insert_resource(config.clone());
    app.insert_resource(SceneCaptureRuntime::default());
    app
}

/// The systems a ROOM capture adds on top of [`build_capture_app`].
fn install_room_capture(app: &mut App) {
    app.add_systems(Startup, setup_capture_target.after(PresentationSetupSet));
    app.add_systems(
        Update,
        (
            silence_dev_overlays,
            force_combat_overlay,
            force_screen_effects,
            apply_capture_snapshot
                .after(camera_follow)
                .before(sync_parallax_layers),
            request_capture.after(sync_parallax_layers),
            finish_after_capture,
            fail_after_timeout,
        ),
    );
}

/// One step of a `--press` sequence.
///
/// `Wait` exists because presses fire two frames apart — one to press, one to release, because
/// the surfaces read EDGES — and a ROUTE CHANGE takes far longer than that.
#[derive(Clone, Copy, Debug)]
enum PressStep {
    Tap(KeyCode),
    /// Run this many frames without touching anything.
    Wait(u32),
    /// Press and KEEP HOLDING (`hold:up`), until a matching `release:`.
    ///
    /// A tap cannot express a directional attack. `up,x` taps Up, releases it,
    /// and only then presses attack — by which time the aim axis is back to
    /// neutral and the swing resolves forward. Every tilt and every aerial in
    /// the game is "a direction held while attack is pressed", so a tool that
    /// can only tap can photograph exactly one of the seven.
    Hold(KeyCode),
    /// Let go of a key an earlier `hold:` is still holding (`release:up`).
    Release(KeyCode),
    /// Tap the glass at a point (`touch:167x523`), in LOGICAL window pixels
    /// with a top-left origin — the space `HitRect` and `Node { left, top }`
    /// are already in, and the space a capture's own pixels are in at scale 1.
    ///
    /// this is the step that can work a POINTER screen, and a key tap
    /// cannot. A key is an EDGE with no position, so `Enter` commits wherever
    /// the cursor already sits; the select screen's headless drivers commit at
    /// a rectangle's centre, and the position is the load-bearing half. A
    /// finger carries both, which is why one step type reaches every widget
    /// while no number of arrow taps reliably does.
    ///
    /// a real `TouchInput` message, not a poke at `Touches` — the same
    /// pair of messages winit emits, folded by Bevy's own
    /// `touch_screen_input_system`. So this drives the phone road the product
    /// ships, and any route that answers a finger gets it for free.
    Touch(Vec2),
}

/// Parse `--press Down,Enter,wait,Down,Enter` into taps and pauses.
///
/// The names are the ones a person says out loud, not `ArrowDown`: this is typed
/// by hand at a terminal while looking at a screenshot.
fn parse_press_sequence(text: &str) -> Result<Vec<PressStep>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            if lower == "wait" {
                return Ok(PressStep::Wait(DEFAULT_WAIT_FRAMES));
            }
            if let Some(rest) = lower.strip_prefix("hold:") {
                return parse_key(rest).map(PressStep::Hold);
            }
            if let Some(rest) = lower.strip_prefix("release:") {
                return parse_key(rest).map(PressStep::Release);
            }
            if let Some(rest) = lower.strip_prefix("touch:") {
                return parse_point(rest).map(PressStep::Touch);
            }
            if let Some(rest) = lower.strip_prefix("tap:") {
                return parse_point(rest).map(PressStep::Touch);
            }
            if let Some(rest) = lower.strip_prefix("wait:") {
                return rest
                    .parse::<u32>()
                    .map(PressStep::Wait)
                    .map_err(|_| format!("--press wait:N needs a frame count, got '{rest}'"));
            }
            parse_key(&lower).map(PressStep::Tap)
        })
        .collect()
}

/// `167x523` — one point in the `--press` vocabulary.
///
/// `x` and not a comma, because the comma is already the STEP separator:
/// `touch:167,523` would arrive here as two steps and the second one would be
/// parsed as a key name. `WIDTHxHEIGHT` is the spelling this tool's own size
/// argument already uses, so there is one separator convention rather than two.
fn parse_point(text: &str) -> Result<Vec2, String> {
    let (x, y) = text
        .split_once('x')
        .ok_or_else(|| format!("--press touch:X x Y needs two numbers, got '{text}'"))?;
    let x = x
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("--press touch: '{x}' is not a number"))?;
    let y = y
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("--press touch: '{y}' is not a number"))?;
    Ok(Vec2::new(x, y))
}

/// One key name from the `--press` vocabulary.
fn parse_key(name: &str) -> Result<KeyCode, String> {
    match name {
        "up" | "arrowup" => Ok(KeyCode::ArrowUp),
        "down" | "arrowdown" => Ok(KeyCode::ArrowDown),
        "left" | "arrowleft" => Ok(KeyCode::ArrowLeft),
        "right" | "arrowright" => Ok(KeyCode::ArrowRight),
        "enter" | "return" => Ok(KeyCode::Enter),
        "space" => Ok(KeyCode::Space),
        "escape" | "esc" => Ok(KeyCode::Escape),
        "z" => Ok(KeyCode::KeyZ),
        "x" => Ok(KeyCode::KeyX),
        "c" => Ok(KeyCode::KeyC),
        // the rest of the default preset's action row, and its absence had a cost. `z`/`x`/`c`
        // are jump/attack/dash; the preset also binds secondary=A, quick_action=E, special=G and
        // interact=F.
        //
        // So the repo's only way to LOOK at a visual change could not reach the one screen most in
        // need of looking at.
        "a" => Ok(KeyCode::KeyA),
        "e" => Ok(KeyCode::KeyE),
        "f" => Ok(KeyCode::KeyF),
        "g" => Ok(KeyCode::KeyG),
        other => Err(format!(
            "--press does not know the key '{other}'. Known: up, down, left, \
                 right, enter, space, escape, z, x, c, a, e, f, g, wait, wait:N, \
                 hold:KEY, release:KEY, touch:XxY"
        )),
    }
}

/// Frames a bare `wait` runs for — comfortably more than a route change.
const DEFAULT_WAIT_FRAMES: u32 = 120;

impl SceneCaptureConfig {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut warmup_frames = 12u32;
        let mut include_ui = false;
        let mut dev_overlays = false;
        let mut combat_overlay = false;
        let mut screen_effects: Vec<ScreenEffect> = Vec::new();
        // One shot every frame, which is the tool this has always been.
        let mut frames: usize = 1;
        let mut stride: u32 = 1;
        let mut fit_room = false;
        let mut character: Option<String> = None;
        let mut route: Option<String> = None;
        let mut press: Vec<PressStep> = Vec::new();
        let mut i = 0usize;
        while i < args.len() {
            // A cursor the arms cannot write cannot be forgotten — an arm that ate one argument
            // and says nothing is a compile error, not a hang.
            let consumed = match args[i].as_str() {
                // it does NOT imply `--dev-overlays`. It did, and that made
                // `silence_dev_overlays` return early, so a capture asking for
                // combat VOLUMES also kept the FPS counter, the debug HUD and
                // the nameplates that read like raw identifiers leaking into
                // player UI. Two independent concerns: clear the developer
                // chrome, and switch the combat gizmos on. The gizmos need
                // `DeveloperRuntimeState.debug` and the gizmo toggles, none of
                // which the chrome settings touch — and `force_combat_overlay`
                // is chained AFTER the silencer either way.
                "--combat-overlay" => {
                    combat_overlay = true;
                    1
                }
                "--dev-overlays" => {
                    dev_overlays = true;
                    1
                }
                "--screen-effect" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--screen-effect requires a value".to_string());
                    };
                    for name in value.split(',').filter(|name| !name.trim().is_empty()) {
                        let effect = ScreenEffect::parse(name)?;
                        if !screen_effects.contains(&effect) {
                            screen_effects.push(effect);
                        }
                    }
                    2
                }
                "--include-ui" => {
                    include_ui = true;
                    1
                }
                "--fit-room" => {
                    fit_room = true;
                    1
                }
                // Keeping it would have forced this tool to keep TWO render modes, which is the
                // fork that ate five features; a flag that shows an empty window is not worth
                // the composition that has to branch for it.
                "--character" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--character requires a catalog id".to_string());
                    };
                    character = Some(value.clone());
                    2
                }
                arg if arg.starts_with("--character=") => {
                    character = Some(arg.trim_start_matches("--character=").to_string());
                    1
                }
                "--press" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--press requires a comma-separated key list".to_string());
                    };
                    press = parse_press_sequence(value)?;
                    2
                }
                arg if arg.starts_with("--press=") => {
                    press = parse_press_sequence(arg.trim_start_matches("--press="))?;
                    1
                }
                "--route" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--route requires a shell route id".to_string());
                    };
                    route = Some(value.clone());
                    2
                }
                arg if arg.starts_with("--route=") => {
                    route = Some(arg.trim_start_matches("--route=").to_string());
                    1
                }
                "--warmup" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--warmup requires a frame count".to_string());
                    };
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    2
                }
                "--frames" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--frames requires a count".to_string());
                    };
                    frames = value
                        .parse::<usize>()
                        .map_err(|_| format!("--frames wants a count, got '{value}'"))?
                        .max(1);
                    2
                }
                arg if arg.starts_with("--frames=") => {
                    let value = arg.trim_start_matches("--frames=");
                    frames = value
                        .parse::<usize>()
                        .map_err(|_| format!("--frames wants a count, got '{value}'"))?
                        .max(1);
                    1
                }
                "--stride" => {
                    let Some(value) = args.get(i + 1) else {
                        return Err("--stride requires a frame count".to_string());
                    };
                    stride = value
                        .parse::<u32>()
                        .map_err(|_| format!("--stride wants a count, got '{value}'"))?
                        .max(1);
                    2
                }
                arg if arg.starts_with("--stride=") => {
                    let value = arg.trim_start_matches("--stride=");
                    stride = value
                        .parse::<u32>()
                        .map_err(|_| format!("--stride wants a count, got '{value}'"))?
                        .max(1);
                    1
                }
                arg if arg.starts_with("--warmup=") => {
                    let value = arg.trim_start_matches("--warmup=");
                    warmup_frames = value
                        .parse::<u32>()
                        .map_err(|_| format!("--warmup must be an integer, got '{value}'"))?;
                    1
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option '{other}'"));
                }
                other => {
                    positional.push(other.to_string());
                    1
                }
            };
            i += consumed;
        }

        // A ROUTE has no room id and no focus point: the shell composes its own
        // surface and its own cameras. Requiring the room positionals anyway
        // would mean inventing values that are then ignored, which is how a
        // flag ends up documented as "pass anything here".
        if let Some(route) = route.clone() {
            // A ROUTE'S POSITIONALS ARE CLASSIFIED, NOT COUNTED.
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
                frames,
                stride,
                // A route IS its UI. Capturing one without it would photograph
                // an empty clear colour and call it the launcher.
                include_ui: true,
                fit_room,
                character,
                follow_player: false,
                dev_overlays,
                combat_overlay,
                screen_effects,
                press,
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
            frames,
            stride,
            size,
            warmup_frames,
            include_ui,
            fit_room,
            character,
            dev_overlays,
            combat_overlay,
            screen_effects,
            follow_player,
            route: None,
            press,
        })
    }
}

/// Photograph a shell ROUTE through the player's own composition.
///
/// A confirmation starts a ROUTE CHANGE, and the route it starts has its own load, its own
/// cameras and its own readiness. So the presses end one capture and begin another — zeroing
/// the frame count and un-setting readiness re-runs the machinery that already knows how to
/// wait for a route ("warmup is a duration, readiness is a FACT"), and the deadline is
/// recomputed with it rather than eaten by it.
///
/// this lived inside the deferred-release branch of a TAP, so it ran only
/// when the last step was a tap. `Hold`, `Release` and `Wait` advance the cursor
/// through a different path and none of them completed the sequence — and BOTH
/// shaped-volume examples this tool ships end in `release:`, so neither ever got
/// the post-input warmup its documentation promises. The shutter could fire
/// almost immediately after the final release instead of N ticks into the action
/// it triggered, which for a tool whose purpose is photographing a specific
/// moment is the whole ballgame.
///
/// Spent means all FOUR are exhausted: no steps left, no key tap awaiting its
/// release, no finger still on the glass, no wait counting down. Asking one
/// question in one place is what stops the next step type from being forgotten
/// the way these three were.
fn complete_press_sequence_if_spent(
    config: &SceneCaptureConfig,
    runtime: &mut SceneCaptureRuntime,
) {
    if runtime.press_cursor < config.press.len()
        || runtime.press_held.is_some()
        || runtime.touch_held.is_some()
        || runtime.press_wait > 0
    {
        return;
    }
    if runtime.press_done_frame.is_some() {
        return;
    }
    runtime.press_done_frame = Some(0);
    runtime.frames = 0;
    runtime.world_ready = false;
    runtime.cameras_adopted = 0;
    eprintln!("capture_scene: press sequence complete; waiting for the state it asked for");
}

/// Silence developer overlays unless requested.
///
/// Enforce this as a system because startup reloads settings after construction.
/// Use normal player settings rather than disabling plugins so captures exercise
/// the same presentation configuration as the game.
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

/// One screen post-process effect, as `--screen-effect` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScreenEffect {
    Crt,
    Grain,
    Vignette,
    Robot,
    Underwater,
    DeepDream,
}

impl ScreenEffect {
    fn parse(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "crt" => Ok(Self::Crt),
            "grain" | "film_grain" => Ok(Self::Grain),
            "vignette" => Ok(Self::Vignette),
            "robot" | "robot_death" => Ok(Self::Robot),
            "underwater" => Ok(Self::Underwater),
            "deep_dream" | "deepdream" => Ok(Self::DeepDream),
            other => Err(format!(
                "unknown screen effect '{other}'; expected one of \
                 crt, grain, vignette, robot, underwater, deep_dream"
            )),
        }
    }

    /// Write this effect at full strength into the settings the shader reads.
    fn force(
        self,
        shaders: &mut ambition_platformer2d::persistence::settings::ScreenShaderSettings,
    ) {
        match self {
            Self::Crt => shaders.crt_strength = 1.0,
            Self::Grain => shaders.film_grain_strength = 1.0,
            Self::Vignette => shaders.vignette_strength = 1.0,
            Self::Robot => shaders.robot_death_strength = 1.0,
            Self::Underwater => shaders.underwater_strength = 1.0,
            Self::DeepDream => shaders.deep_dream_strength = 1.0,
        }
    }
}

/// Force the asked-for screen effects on, every frame, and SAY SO when the
/// visual-quality budget is about to scale them back to nothing.
///
/// ⛔⛔ THE WARNING IS THE POINT, and it is here because its absence cost two
/// captures and an hour. `sync_screen_effect_settings_from_video_settings`
/// clamps the global strength to `quality.budget.shaders.screen_shader_scale`,
/// which is **0.0 on the Potato tier** — and Potato is what this machine's
/// software rasteriser (`llvmpipe`) gets seeded to on a first run. So the honest
/// arms of a post-process comparison came back byte-identical while every
/// setting said the effect was on. An instrument that cannot deliver what it was
/// asked for must say so rather than return a plausible frame.
///
/// Every frame, like `force_combat_overlay`, because the settings load and the
/// quality seed both write this state and a Startup-only write races them.
fn force_screen_effects(
    config: Res<SceneCaptureConfig>,
    quality: Option<Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>>,
    mut settings: ResMut<ambition_platformer2d::persistence::settings::UserSettings>,
    mut warned: Local<bool>,
) {
    if config.screen_effects.is_empty() {
        return;
    }
    let mut wanted = settings.video.shaders.clone();
    wanted.strength = 1.0;
    for effect in &config.screen_effects {
        effect.force(&mut wanted);
    }
    if settings.video.shaders != wanted {
        settings.video.shaders = wanted;
    }
    if !*warned {
        if let Some(quality) = quality.as_ref() {
            let scale = quality.budget.shaders.screen_shader_scale;
            if scale <= 0.001 {
                *warned = true;
                eprintln!(
                    "capture_scene: ⛔ the {:?} visual-quality tier scales screen \
                     shaders to {scale:.2}, so --screen-effect will photograph \
                     NOTHING. Re-run with AMBITION_QUALITY_PROFILE=ultra.",
                    quality.profile
                );
            }
        }
    }
}

/// Force the COMBAT debug preset on, every frame, when `--combat-overlay` asked.
///
/// Every frame rather than once at startup because the settings load and the
/// developer-tools default both write this state, and a Startup-only write is a
/// race against whichever of them runs later. Idempotent, so the cost of being
/// certain is a comparison per frame.
fn force_combat_overlay(
    config: Res<SceneCaptureConfig>,
    mut dev_state: Option<ResMut<ambition_platformer2d::dev_tools::DeveloperRuntimeState>>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if !config.combat_overlay {
        return;
    }
    // The three gates the gizmo pass reads live in `force_combat_overlay`, so a
    // tool asking for combat geometry cannot satisfy two of them and photograph
    // a swing with no volume on it.
    if let (Some(dev_state), Some(developer)) = (dev_state.as_mut(), developer.as_mut()) {
        ambition_platformer2d::dev_tools::force_combat_overlay(
            dev_state,
            developer,
            Default::default(),
        );
    }
}

/// The systems a ROUTE capture adds on top of [`build_capture_app`], plus the
/// two things only a route needs to be told.
fn install_route_capture(app: &mut App, route_id: String) {
    // Found while trying to look at the programmatic vanity card, which is exactly the kind of
    // change that must be looked at rather than compiled.
    //
    // Composed only when it is the route being asked for: `compose_..._sequence`
    // also makes startup the INITIAL route, which is correct here and would put
    // a card in front of every other capture.
    if route_id == ambition_app::app::shell_host::AMBITION_STARTUP_ROUTE {
        ambition_app::app::shell_host::compose_ambition_startup_sequence(app);
    }

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

    app.add_systems(Startup, setup_route_capture_target);
    app.add_systems(
        Update,
        (
            silence_dev_overlays,
            force_combat_overlay,
            force_screen_effects,
            go_to_route,
            adopt_route_cameras,
            request_capture,
            finish_after_capture,
            fail_after_timeout,
        )
            .chain(),
    );
}

/// Drive the shell to the requested route — unless it is already there.
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
    // ⭐⭐ A SMASH MOVE IS PHOTOGRAPHED UNDER SMASH RULES, and that needs a
    // CAST. Jon, 2026-08-28: *"when we are doing smash moves we probably should
    // be using the smash stage and not any ambition stages, to make sure that
    // we're actually getting smash rules and not ambition which might be
    // different."* He is right, and the tool made the wrong road the easy one:
    // `--route smash_gameplay` with no roster activates a stage with NOBODY ON
    // IT — the camera sits at its default over empty sky, which D130 already
    // recorded once as a mystery — so the only way to see a fighter was
    // `--character <id>` on an exploration ROOM, under exploration rules.
    //
    // ⛔ THE SAME TWO LINES `moveset_takes::reseat` USES, deliberately: a
    // roster resource and the route change. The alternative is the select
    // screen's documented tap coordinates, which have drifted three times in
    // two weeks and pick fighters by GRID CELL rather than by name.
    if route == ambition_demo_smash::SMASH_GAMEPLAY_ROUTE {
        if let Some(character) = config.character.clone() {
            eprintln!(
                "capture_scene: seating '{character}' twice on the smash stage,                  so the route comes up with a match rather than an empty stage"
            );
            commands.insert_resource(ambition_demo_smash::smash_roster([
                character.as_str(),
                character.as_str(),
            ]));
        } else {
            eprintln!(
                "capture_scene: '{route}' with no --character seats no cast; the                  stage will come up empty"
            );
        }
    }
    commands.write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(route),
    ));
}

/// Create the image a route capture draws into, and nothing else.
///
/// The retargeting is [`adopt_route_cameras`]' job — it has to run EVERY frame, because a shell
/// route has no cameras at `Startup` — and this function makes the target they get pointed at.
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
/// A shell route has no cameras at `Startup` — it spawns them when the route composes, several
/// frames in. The count is printed for that reason.
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
    // A shell route composes a stack (world, then UI, then overlays). Ordering by
    // `Camera::order` and clearing only on the lowest is what makes the stack composite instead
    // of compete.
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
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::engine_core::RoomGeometry,
    >,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::world::rooms::RoomSet,
    >,
    user_settings: Res<ambition_platformer2d::persistence::settings::UserSettings>,
    ease_tuning: Res<ambition_platformer2d::platformer::camera_ease::CameraEaseTuning>,
    // `CameraViewState` is a COMPONENT on the local view now, not a process
    // global — a capture app stages exactly one view, so this writes the one it
    // staged rather than a resource every consumer shared.
    mut view_states: Query<&mut CameraViewState, With<ambition_platformer2d::sim_view::LocalView>>,
    // THE SIM BODY, not the render visual.
    //
    // This queried `BodyKinematics` `With<PlayerVisual>`, and `PlayerVisual` is a
    // RENDER-side marker on a by-id render entity that carries no kinematics. So
    // the query never matched, `player` focus fell back to `config.focus` — which
    // is `Vec2::ZERO` in that mode — and every room capture photographed the origin
    // while the player stood elsewhere.
    player_q: Query<
        &ambition_platformer2d::platformer::body::BodyKinematics,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut cameras: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let active_spec = room_set.active_spec();
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);
    let focus_center = if config.follow_player {
        player_q
            .iter()
            .next()
            .map(|k| k.pos)
            .unwrap_or(config.focus)
    } else {
        config.focus
    };
    // `--fit-room`: the whole room, centred, scaled to fit. Nothing else here
    // can produce this — the resolver below clamps to the room and to camera
    // zones, so asking it for a wide shot gives back a gameplay shot.
    if config.fit_room {
        // the projection's own SCALING MODE, not a multiplier on the base view. The first
        // version computed `scale = max(room / base_view)` and framed the hall at about a fifth
        // of the image: `scale` multiplies an extent that depends on the mode and on the
        // viewport's aspect, so the arithmetic only holds when those agree.
        for (mut transform, mut projection) in &mut cameras {
            if let Projection::Orthographic(orthographic) = &mut *projection {
                orthographic.scale = 1.0;
                orthographic.scaling_mode = bevy::camera::ScalingMode::AutoMin {
                    min_width: world.0.size.x,
                    min_height: world.0.size.y,
                };
            }
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
            transform.rotation = Quat::IDENTITY;
        }
        return;
    }

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
            chart_transit: None,
            // this is the `--fit-room` / focus-point path, which deliberately
            // BYPASSES the live resolve; a captured MATCH goes through
            // `resolve_camera_observation` and gets the cast's box from there.
            must_frame_world: None,
            ease_tuning: *ease_tuning,
            screen_framing: None,
            reference_frame: Default::default(),
            subject_down: None,
        },
        None,
    );

    let x = snapshot.center_world.x - world.0.size.x * 0.5;
    let y = world.0.size.y * 0.5 - snapshot.center_world.y;
    for mut view_state in &mut view_states {
        *view_state = CameraViewState::from(&snapshot);
    }

    for (mut transform, mut projection) in &mut cameras {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = snapshot.orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
        transform.rotation = Quat::from_rotation_z(snapshot.rotation_radians);
    }
}

/// Generous on purpose: a route that loads assets can legitimately take a while,
/// and a false failure in a verification tool is as bad as a false success.
const ROUTE_CAMERA_GRACE_FRAMES: u32 = 600;

/// Whether there is a constructed world to start counting warmup against.
///
/// Two conditions, and the second one is the interesting half.
///
/// The body exists. A player-focused capture waits for the body it is going
/// to centre on; a coordinate-focused one has nothing specific to wait for.
///
/// Its ART has a terminal answer. A decoded sheet RESIZES the body it
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
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
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
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    seated_q: Query<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::platformer::body::BodyKinematics,
    )>,
    art_demand: Option<Res<CharacterLoadDemand>>,
    art_states: Option<Res<CharacterLoadStates>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut fingers: MessageWriter<TouchInput>,
) {
    if runtime.requested || runtime.completed {
        if runtime.requested {
            runtime.wait_frames = runtime.wait_frames.saturating_add(1);
        }
        return;
    }
    // ⭐ THE GAP BETWEEN SHOTS OF A SEQUENCE. Counted here rather than in the
    // readback handler because this is the system that runs once per frame; the
    // handler fires on a GPU event and cannot count sim frames.
    if runtime.stride_left > 0 {
        runtime.stride_left -= 1;
        return;
    }
    // WARMUP COUNTS FROM A READY WORLD, not from boot.
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
    // Drive the route's own lobby before the shutter.
    //
    // `press_wait` belongs in this condition. Without it a trailing `wait:N` left the
    // sequence "inactive" the moment its cursor passed the last step, so the wait was never
    // counted down and never completed — a step type that silently did nothing when it happened
    // to be last.
    if runtime.press_cursor < config.press.len()
        || runtime.press_held.is_some()
        || runtime.touch_held.is_some()
        || runtime.press_wait > 0
    {
        if runtime.press_wait > 0 {
            runtime.press_wait -= 1;
            complete_press_sequence_if_spent(&config, &mut runtime);
            return;
        }
        if let Some(key) = runtime.press_held.take() {
            keys.release(key);
            complete_press_sequence_if_spent(&config, &mut runtime);
        } else if let Some(at) = runtime.touch_held.take() {
            // The lift, at the SAME point the finger went down. A touch that
            // ended somewhere else is a drag, and a drag means "drop it here"
            // to the screen this drives — a tap that travelled by accident
            // would place a token nobody moved.
            fingers.write(TouchInput {
                phase: TouchPhase::Ended,
                position: at,
                window: Entity::PLACEHOLDER,
                force: None,
                id: runtime.next_touch_id - 1,
            });
            complete_press_sequence_if_spent(&config, &mut runtime);
        } else {
            match config.press[runtime.press_cursor] {
                PressStep::Tap(key) => {
                    keys.press(key);
                    runtime.press_held = Some(key);
                    eprintln!(
                        "capture_scene: pressed {key:?} ({} of {})",
                        runtime.press_cursor + 1,
                        config.press.len()
                    );
                }
                PressStep::Hold(key) => {
                    // Deliberately NOT recorded in `press_held`: that field is
                    // "the tap awaiting its release next frame", and a hold is
                    // the opposite — it outlives the step that started it.
                    keys.press(key);
                    eprintln!(
                        "capture_scene: holding {key:?} ({} of {})",
                        runtime.press_cursor + 1,
                        config.press.len()
                    );
                }
                PressStep::Release(key) => {
                    keys.release(key);
                    eprintln!(
                        "capture_scene: released {key:?} ({} of {})",
                        runtime.press_cursor + 1,
                        config.press.len()
                    );
                }
                PressStep::Touch(at) => {
                    let id = runtime.next_touch_id;
                    runtime.next_touch_id += 1;
                    fingers.write(TouchInput {
                        phase: TouchPhase::Started,
                        position: at,
                        window: Entity::PLACEHOLDER,
                        force: None,
                        id,
                    });
                    runtime.touch_held = Some(at);
                    eprintln!(
                        "capture_scene: touched ({:.0}, {:.0}) ({} of {})",
                        at.x,
                        at.y,
                        runtime.press_cursor + 1,
                        config.press.len()
                    );
                }
                PressStep::Wait(frames) => {
                    runtime.press_wait = frames;
                    eprintln!(
                        "capture_scene: waiting {frames} frames ({} of {})",
                        runtime.press_cursor + 1,
                        config.press.len()
                    );
                }
            }
            runtime.press_cursor += 1;
            complete_press_sequence_if_spent(&config, &mut runtime);
        }
        return;
    }
    if let Some(kin) = player_q.iter().next() {
        println!(
            "capture_scene: subject at ({:.4}, {:.4}) after {} warmup tick(s)",
            kin.pos.x, kin.pos.y, runtime.frames
        );
    } else {
        // SEAT ORDER, not query order. Bevy iterates by archetype, so an
        // unsorted list would compare two captures of the same match and find
        // them different because the rows moved.
        let mut seated: Vec<_> = seated_q
            .iter()
            .map(|(seat, kin)| (seat.0, kin.pos))
            .collect();
        seated.sort_by_key(|(seat, _)| *seat);
        if seated.is_empty() {
            // SAY SO. "No pose line" and "no subject" were indistinguishable,
            // and this tool's whole job is to stop a verification photographing
            // the wrong thing quietly.
            println!(
                "capture_scene: NO SUBJECT — no primary player and no seated body \
                 after {} warmup tick(s); this image proves nothing about a pose",
                runtime.frames
            );
        } else {
            for (seat, pos) in seated {
                println!(
                    "capture_scene: seat {seat} at ({:.4}, {:.4}) after {} warmup tick(s)",
                    pos.x, pos.y, runtime.frames
                );
            }
        }
    }
    // A ROUTE CAPTURE WAITS FOR A CAMERA, not for a clock.
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
    if config.frames > 1 {
        println!(
            "capture_scene: wrote {} frame(s) as {}.NNNN.{} ({}x{} px, stride {})",
            config.frames,
            config.output.with_extension("").display(),
            config
                .output
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "png".to_string()),
            config.size.x,
            config.size.y,
            config.stride,
        );
    } else {
        println!(
            "capture_scene: wrote {} ({}x{} px)",
            config.output.display(),
            config.size.x,
            config.size.y,
        );
    }
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
    // ⭐ ONE SHOT KEEPS ITS EXACT NAME. A sequence numbers its files, because a
    // caller that asked for forty pictures wants forty files and a caller that
    // asked for one wants the path it named — silently renaming the single-shot
    // output would break every existing recipe that photographs a room.
    let path = if config.frames <= 1 {
        config.output.clone()
    } else {
        let stem = config
            .output
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "frame".to_string());
        let ext = config
            .output
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "png".to_string());
        config
            .output
            .with_file_name(format!("{stem}.{:04}.{ext}", runtime.shot))
    };
    if let Err(error) = image.save(&path) {
        eprintln!(
            "capture_scene: failed to save '{}': {error}",
            path.display()
        );
        runtime.failed = true;
        runtime.completed = true;
        commands.write_message(AppExit::from_code(1));
        return;
    }
    runtime.shot += 1;
    if runtime.shot < config.frames {
        // ⛔ RE-ARM, DO NOT COMPLETE. `request_capture` is guarded on
        // `requested || completed`, so clearing both is what lets the next shot
        // be taken — and the stride is what makes the scene DIFFERENT by then.
        // Without a stride the sequence photographs one instant many times.
        runtime.requested = false;
        runtime.wait_frames = 0;
        runtime.stride_left = config.stride;
        return;
    }
    runtime.completed = true;
}

/// this was a flat `runtime.frames > 600`, which quietly preempted every
/// policy above it. The route-readiness check allows `warmup + 600` frames for a
/// camera to appear, so for ANY warmup above zero the generic timeout fired
/// first: the route-specific diagnostic — the one that says *which* route never
/// produced a camera — was unreachable, and a `--warmup` above 600 could not
/// complete at all.
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
/// Checked at PARSE time.
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

// This file kept its own copy of the asset-root rule, and the copy said
// `crates/ambition_platformer2d::actors/assets` — a `::` where the crate name
// has a `_`. No such directory can exist, `canonicalize` failed every time, and
// the fallback pointed the room composition at the workspace-root `assets/`
// tree, which holds IPFS metadata and none of the actor sprites, shaders or
// sounds. Room-mode capture wrote a valid PNG of a room whose art never
// resolved, and exited 0. Route mode went through the visible app and its own
// correct root, which is exactly why `--route` looked fine while rooms did not
// .
//
// It has no app of its own now: `build_visible_app_with` resolves the root, and there is
// nothing here left to disagree with it.
