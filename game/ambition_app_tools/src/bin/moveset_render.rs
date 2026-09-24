//! Render a fighter performing a move, one PNG per exact sim tick.
//!
//! This is not `capture_scene`, because it owns its own loop. `capture_scene`
//! calls `App::run()`, so the runner sets the frame cost: while a readback is
//! pending the app keeps updating, and shots are spaced by stride plus GPU
//! time. For a move, that skips startup or active frames.
//!
//! Here simulation time and GPU time are separate. The sim advances only at
//! the canonical manual period. A readback is serviced with
//! `ManualDuration(ZERO)`, which runs the schedules without moving the clock
//! (a readback completes in about three zero-time pumps with `SimTick`
//! frozen). Each PNG names its tick, and GPU latency cannot change which ticks
//! are captured.
//!
//! A press is a request; the engine decides. The manifest carries the
//! intended move and the observed ones, and reports a mismatch instead of
//! caching under the requested name.

use ambition_platformer2d::dev_tools::CombatOverlayLayers;
use ambition_sim_harness::combat_observation::{CombatObservation, ScenarioRoles};
use ambition_sim_harness::move_exercise;

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;
use ambition_sim_harness::{AdapterPreference, DeterministicCaptureSession};
use move_exercise::{verb_named, VERBS};

/// How much world the pictures show by default, in world px across.
///
/// Derived from body sizes: a fighter is about 50 px tall and the default
/// seats are within ~150 px, so 320 holds the pair and their strike volumes.
/// The gameplay camera's presets run from 568 to 1600.
const DEFAULT_VIEW_WIDTH: f32 = 320.0;

/// World px of air kept beyond each fighter when the frame widens to hold both.
const FRAME_MARGIN: f32 = 60.0;

/// The most the frame will widen past `--view-width` to keep the target in it.
/// Past this the picture is a stage again, which is the thing being fixed.
const MAX_FRAME_WIDENING: f32 = 2.0;

const USAGE: &str = "\
moveset_render — render a fighter performing one move, one PNG per simulation tick.

USAGE:
    moveset_render --character ID --verb VERB [--target ID] [--target-behavior WHICH]
                   [--spacing PX] [--view-width PX] [--out DIR] [--frames N]
                   [--stride K] [--combat-overlay on|off]
                   [--adapter auto|hardware|software]

OPTIONS:
    --character ID   catalog id of the fighter
    --verb VERB      repertoire verb to perform (see below)
    --spacing PX     walk the subject to within PX of the target before the press
                     [default: the match's own seat placement]
    --target ID      who the move is performed against
                     [default: sandbag_infinite, the immortal training dummy]
    --target-behavior WHICH
                     passive | cpu                      [default: passive]
    --view-width PX  how much WORLD is in the picture, centred on the subject
                     [default: 320, about six body widths]
                     ⭐ THE GAMEPLAY CAMERA FRAMES A STAGE (568-1600 px across by
                     the zoom preset) and a fighter is ~50 px tall, so the body
                     an inspector opened the view for was a thumbnail in the
                     corner. `0` gives the frame back to the game's own camera.
    --overlay LAYERS on | off | a comma list of art,hurtboxes,strikes
                     [default: on = all three]
                     ⭐ INDEPENDENT, because the questions are. Whether a volume
                     sits inside the sprite needs the art; where exactly it
                     reaches is easier without it; why it missed wants the
                     hurtboxes without the strikes drawn on top of them.
    --combat-overlay WHICH
                     an alias for --overlay                [default: on]
                     `on` draws the engine's own combat geometry — hurtboxes,
                     live strike volumes, the move readout — over the real
                     rendered art, from the SAME execution. That is the whole
                     point of this binary: one picture, one simulation, no
                     second coordinate system.
    --out DIR        directory for the PNGs and manifest.json  [default: /tmp/moveset_render]
    --frames N       how many pictures                          [default: 24]
    --stride K       simulation ticks between pictures          [default: 1]
    --adapter WHICH  auto | hardware | software                  [default: auto]
                     ⭐ `software` pins Lavapipe, so a CI or agent job does not
                     change behaviour when a driver appears on the machine.
    -h, --help       print this and exit

NOTES:
    Needs a GPU: it boots the real OffscreenGpu composition and reads pixels back.

    Every PNG names the exact `SimTick` it was captured on, and the manifest
    records the intended move against what the engine actually played. A press
    is a request; if the move that came out is not the one asked for, that is
    reported rather than cached under the requested name.

    The manifest also carries the SEMANTIC geometry of every shot — hurtboxes,
    strike volumes, roles, the move clock — sampled before the shutter, so a
    reader knows exactly what the picture shows without measuring pixels.
";

/// The overlay layers this run asked for, so the forcing system does not
/// re-parse them. The gates the gizmo pass reads belong to
/// `ambition_dev_tools` (see `force_combat_overlay` there).
#[derive(bevy::prelude::Resource, Clone, Copy)]
struct RequestedOverlayLayers(CombatOverlayLayers);

/// The world rectangle the pictures are framed ON, written every frame after
/// the game's own camera policy has run.
///
/// The gameplay camera frames a whole stage (568 to 1600 world units), so a
/// 50-unit fighter is a few dozen pixels. An inspector needs a close-up.
///
/// Written every frame after `camera_follow`. That policy runs in `Update`
/// and re-derives the camera, so a one-shot write would be overwritten by the
/// next zero-duration pump, which is the pump the picture is taken on.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug)]
struct InspectorFraming {
    /// World point at the centre of the frame.
    center: Vec2,
    /// World size of the frame, already in the capture's aspect.
    view: Vec2,
}

impl InspectorFraming {
    /// The rectangle as `[x0, y0, x1, y1]`, the same form as a recorded take's
    /// `view`.
    fn rect(&self) -> [f32; 4] {
        [
            self.center.x - self.view.x / 2.0,
            self.center.y - self.view.y / 2.0,
            self.center.x + self.view.x / 2.0,
            self.center.y + self.view.y / 2.0,
        ]
    }
}

/// Where to point the camera for one shot, from the tick's own observation.
///
/// Frame the pair, not only the subject. The default seats are 192 px apart,
/// so a frame centred on the subject puts the target on its edge. This
/// centres between them and widens to hold both, up to a cap (beyond that
/// the shot is a whole stage).
fn frame_for(
    observation: &serde_json::Value,
    view_width: f32,
    aspect: f32,
) -> Option<InspectorFraming> {
    // The position is under `collision`. `row["pos"]` would be `Null` for a
    // missing key and silently frame the origin. The recorder's per-frame
    // `pos`/`half` is a reshape of this nested value.
    let body = |role: &str| -> Option<Vec2> {
        let row = observation["bodies"]
            .as_array()?
            .iter()
            .find(|row| row["role"].as_str() == Some(role))?;
        Some(Vec2::new(
            row["collision"]["pos"][0].as_f64()? as f32,
            row["collision"]["pos"][1].as_f64()? as f32,
        ))
    };
    // No `unwrap_or(ZERO)`. Without a subject there is no inspection framing,
    // so the gameplay camera keeps the frame and the manifest says
    // `view: null`.
    let subject = body("subject")?;
    let target = body("target");
    let (center, span) = match target {
        // Margin on both sides: a body is ~50 px wide and a strike reaches
        // past it.
        Some(target) => (
            (subject + target) / 2.0,
            (subject - target).abs() + Vec2::splat(2.0 * FRAME_MARGIN),
        ),
        // The placeholder has the default view's shape, not a square. For a
        // lone subject `span.y * aspect` is exactly `view_width`, so the
        // default stands and there is one code path below.
        None => (subject, Vec2::new(view_width, view_width / aspect)),
    };
    // Cover the separation in both axes. The view is `width / aspect` high,
    // so height >= span.y exactly when width >= span.y * aspect. This keeps
    // one knob (`width`) and one cap. Aerial moves often separate the pair
    // vertically. The widening cap still applies afterwards: past it the
    // pair is clipped symmetrically.
    let required = span.x.max(span.y * aspect);
    let width = required
        .max(view_width)
        .min(view_width * MAX_FRAME_WIDENING);
    Some(InspectorFraming {
        center,
        view: Vec2::new(width, width / aspect),
    })
}

/// Point the photographed camera at [`InspectorFraming`].
///
/// Set the scaling mode, not `scale`. `scale` multiplies an extent that
/// depends on the mode and the viewport aspect. `AutoMin` states the world
/// rectangle directly.
fn frame_the_inspection(
    framing: Option<Res<InspectorFraming>>,
    // The room is part of the transform. `camera_follow` places the camera at
    // `world.x - room.x/2`, `room.y/2 - world.y` (an offset and a Y flip), so
    // sim coordinates cannot go straight into the transform.
    room: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::engine_core::RoomGeometry,
    >,
    mut cameras: Query<
        (&mut Transform, &mut Projection),
        With<ambition_platformer2d::platformer::camera_layers::MainCamera>,
    >,
) {
    let Some(framing) = framing else {
        return;
    };
    let size = room.0.size;
    for (mut transform, mut projection) in &mut cameras {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = 1.0;
            orthographic.scaling_mode = bevy::camera::ScalingMode::AutoMin {
                min_width: framing.view.x,
                min_height: framing.view.y,
            };
        }
        transform.translation.x = framing.center.x - size.x * 0.5;
        transform.translation.y = size.y * 0.5 - framing.center.y;
        transform.rotation = Quat::IDENTITY;
    }
}

fn force_combat_overlay(
    requested: Res<RequestedOverlayLayers>,
    mut dev_state: Option<ResMut<ambition_platformer2d::dev_tools::DeveloperRuntimeState>>,
    mut developer: Option<ResMut<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
) {
    if let (Some(dev_state), Some(developer)) = (dev_state.as_mut(), developer.as_mut()) {
        ambition_platformer2d::dev_tools::force_combat_overlay(dev_state, developer, requested.0);
    }
}

fn sim_tick(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let arg = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    if let Some(bad) = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with('-'))
        .find(|a| {
            !matches!(
                a.as_str(),
                "--character"
                    | "--characters"
                    | "--verb"
                    | "--verbs"
                    | "--out"
                    | "--frames"
                    | "--stride"
                    | "--adapter"
                    | "--target"
                    | "--target-behavior"
                    | "--combat-overlay"
                    | "--overlay"
                    | "--spacing"
                    | "--view-width"
            )
        })
    {
        eprintln!("moveset_render: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    // Which verbs and how many fighters. One pair keeps the old spelling.
    // `--characters`/`--verbs` render many in one app: a frame is ~4 ms and
    // an app boot is ~2 s, so one process per move pays the boot every time.
    let verbs: Vec<&'static move_exercise::Verb> = match arg("--verbs")
        .or_else(|| arg("--verb"))
        .as_deref()
    {
        None => {
            eprintln!("moveset_render: --verb or --verbs is required\n");
            print!("{USAGE}");
            std::process::exit(2);
        }
        Some("all") => VERBS.iter().collect(),
        Some(list) => list
            .split(',')
            .map(str::trim)
            .map(|name| match verb_named(name) {
                Some(verb) => verb,
                None => {
                    // Name what is supported. A capture-state move (a pummel,
                    // a throw) needs a grabbed opponent, which this exercise
                    // cannot set up, so it is excluded.
                    eprintln!(
                        "moveset_render: '{name}' is not a verb this exercise can perform.\n\
                         known: {}\n",
                        VERBS.iter().map(|v| v.verb).collect::<Vec<_>>().join(", ")
                    );
                    std::process::exit(2);
                }
            })
            .collect(),
    };
    let asked_characters = arg("--characters").or_else(|| arg("--character"));
    if asked_characters.is_none() {
        eprintln!("moveset_render: --character or --characters is required\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    // A batch is a directory of directories. One pair writes straight into
    // `--out`, the layout the inspector server caches by.
    let batching = arg("--characters").is_some() || arg("--verbs").is_some() || verbs.len() > 1;
    let out_dir =
        std::path::PathBuf::from(arg("--out").unwrap_or_else(|| "/tmp/moveset_render".to_string()));
    let frames: usize = arg("--frames")
        .and_then(|v| v.parse().ok())
        .unwrap_or(24)
        .max(1);
    let stride: u64 = arg("--stride")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    // The training dummy is the default, as in the recorder: both tools must
    // stage the same scenario.
    let asked_target = arg("--target");
    let passive_target = match arg("--target-behavior").as_deref() {
        None | Some("passive") => true,
        Some("cpu") => false,
        Some(word) => {
            eprintln!(
                "moveset_render: unknown --target-behavior '{word}'; expected passive or cpu"
            );
            std::process::exit(2);
        }
    };
    let spacing: Option<f32> = match arg("--spacing") {
        None => None,
        Some(word) => match word.parse::<f32>() {
            Ok(px) if px >= 0.0 => Some(px),
            _ => {
                eprintln!("moveset_render: --spacing wants a non-negative number of pixels");
                std::process::exit(2);
            }
        },
    };
    // How much world is in the picture. `0` gives the frame back to the
    // gameplay camera, which is what a player sees.
    let view_width: f32 = match arg("--view-width") {
        None => DEFAULT_VIEW_WIDTH,
        Some(word) => match word.parse::<f32>() {
            Ok(px) if px >= 0.0 => px,
            _ => {
                eprintln!("moveset_render: --view-width wants a non-negative number of world px");
                std::process::exit(2);
            }
        },
    };

    // One flag, two spellings: `--combat-overlay on|off` and `--overlay` with
    // layer names.
    let asked = arg("--overlay").or_else(|| arg("--combat-overlay"));
    let layers = match asked.as_deref() {
        None | Some("on") => Some(CombatOverlayLayers::default()),
        Some("off") | Some("none") => None,
        Some(list) => {
            let mut chosen = CombatOverlayLayers {
                art: false,
                hurtboxes: false,
                strikes: false,
            };
            for name in list.split(',').map(str::trim) {
                match name {
                    "art" => chosen.art = true,
                    "hurtboxes" | "hurt" => chosen.hurtboxes = true,
                    "strikes" | "hitboxes" => chosen.strikes = true,
                    other => {
                        eprintln!(
                            "moveset_render: unknown overlay layer '{other}'; expected \
                             on, off, or a comma list of art,hurtboxes,strikes"
                        );
                        std::process::exit(2);
                    }
                }
            }
            Some(chosen)
        }
    };
    let combat_overlay = layers.is_some();
    let size = UVec2::new(480, 360);

    // Before the app is built. Bevy reads the adapter environment when it
    // creates the device in plugin `finish()`.
    let adapter = match arg("--adapter").as_deref() {
        None => AdapterPreference::Auto,
        Some(word) => match AdapterPreference::parse(word) {
            Some(pref) => pref,
            None => {
                eprintln!(
                    "moveset_render: unknown --adapter '{word}'; expected auto, hardware or software"
                );
                std::process::exit(2);
            }
        },
    };
    adapter.apply();


    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        true,
        |_app| {},
    );
    if let Some(layers) = layers {
        app.insert_resource(RequestedOverlayLayers(layers));
        // Art and geometry in one image from one execution. The engine's
        // developer overlay draws `CombatGeometryView` over its own
        // presentation; this tool draws no boxes.
        //
        // Forced every frame: the settings load and the developer-tools
        // default both write this state, so a startup-only write races them.
        app.add_systems(Update, force_combat_overlay);
    }
    // After the camera policy, before the parallax. `camera_follow`
    // re-derives the camera every `Update`, and the parallax layers read the
    // camera it leaves. `capture_scene` orders its snapshot here for the same
    // reasons.
    if view_width > 0.0 {
        app.add_systems(
            Update,
            frame_the_inspection
                .after(ambition_platformer2d::render::rendering::camera_follow)
                .before(ambition_platformer2d::render::rendering::sync_parallax_layers),
        );
    }
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(size.x as f32, size.y as f32),
        ),
    );
    // The session does the capture work; this binary owns composition and
    // ordering, because `PresentationSetupSet` belongs to the product shell.
    DeterministicCaptureSession::install(&mut app, size, out_dir.join("frame.png"));
    app.add_systems(
        Startup,
        ambition_platformer2d::capture::setup_capture_target
            .after(ambition_app::app::PresentationSetupSet),
    );

    // Finalize before stepping. Bevy builds the render device in plugin
    // `finish()`, which `App::run()` performs and a hand-driven loop does not;
    // without it `bevy_pbr` panics ("Res<RenderDevice> failed validation").
    ambition_platformer2d::runtime::finalize(&mut app);
    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    let camera = DeterministicCaptureSession::adopt(canonical, size);

    for _ in 0..30 {
        app.update();
    }

    // Resolve the roster after the app exists: `grid` means the set this
    // composition can seat, and only the prepared registry knows it.
    let characters: Vec<String> = match asked_characters.as_deref() {
        Some("grid") | Some("all") => {
            let registry = app
                .world()
                .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
                .expect("the composed host has a prepared-character registry");
            ambition_demo_smash::select::SmashRoster::assemble(registry)
                .ids()
                .map(|id| id.to_string())
                .collect()
        }
        Some(list) => list.split(',').map(|x| x.trim().to_string()).collect(),
        None => unreachable!("checked above"),
    };

    // Say how long this takes: a grid is hundreds of renders.
    let pairs = characters.len() * verbs.len();
    if pairs > 1 {
        eprintln!(
            "[moveset-render] {pairs} render(s): {} character(s) x {} verb(s)",
            characters.len(),
            verbs.len()
        );
    }

    let started = std::time::Instant::now();
    let mut index: Vec<serde_json::Value> = Vec::new();
    let mut failures = 0usize;
    for character in &characters {
        for verb in &verbs {
            // The training dummy is the default, as in the recorder.
            let target = asked_target
                .clone()
                .unwrap_or_else(|| ambition_demo_smash::INSPECTION_TARGET.to_string());
            // One pair writes straight into `--out`; a batch uses the layout
            // the inspector server caches by, so a corpus is the browser cache.
            let pair_dir = if batching {
                out_dir.join(format!("{character}__{}", verb.verb))
            } else {
                out_dir.clone()
            };
            std::fs::create_dir_all(&pair_dir).expect("the output directory is creatable");
            // Remove stale PNGs first. A shorter run would leave the previous
            // move's tail, and the server would serve it with this move.
            for stale in std::fs::read_dir(&pair_dir).into_iter().flatten().flatten() {
                if stale.path().extension().is_some_and(|e| e == "png") {
                    let _ = std::fs::remove_file(stale.path());
                }
            }
            let request = PairRequest {
                character,
                verb,
                target: &target,
                passive_target,
                spacing,
                view_width,
                aspect: size.x as f32 / size.y as f32,
                frames,
                stride,
                combat_overlay,
                layers,
                adapter,
                out_dir: pair_dir.clone(),
                batched: batching,
            };
            match render_pair(&mut app, &camera, &request) {
                Ok(manifest) => {
                    println!(
                        "[render] {character:<24} {:<16} {} frame(s){}",
                        verb.verb,
                        manifest["frames"],
                        if manifest["reached_intended_move"] == serde_json::json!(true) {
                            String::new()
                        } else {
                            format!(
                                " {}: intended {}, engine played {}",
                                manifest["outcome"].as_str().unwrap_or("?").to_uppercase(),
                                manifest["intended_move"],
                                manifest["observed_moves"]
                            )
                        }
                    );
                    index.push(serde_json::json!({
                        "character": character,
                        "verb": verb.verb,
                        "dir": pair_dir.file_name().and_then(|n| n.to_str()),
                        "frames": manifest["frames"],
                        "outcome": manifest["outcome"],
                        "reached_intended_move": manifest["reached_intended_move"],
                    }));
                }
                Err(why) => {
                    // One bad pair is not a bad run. Record the failure (it is
                    // a finding) and continue.
                    failures += 1;
                    println!("[render] {character:<24} {:<16} FAILED - {why}", verb.verb);
                    index.push(serde_json::json!({
                        "character": character,
                        "verb": verb.verb,
                        "dir": pair_dir.file_name().and_then(|n| n.to_str()),
                        "failed": why,
                    }));
                }
            }
        }
    }

    if batching {
        let elapsed = started.elapsed().as_secs_f32();
        let doc = serde_json::json!({
            "schema": "ambition.moveset_render_index.v1",
            "renders": index,
            "failures": failures,
            "seconds": elapsed,
            // The cost per pair, for planning a corpus.
            "seconds_per_render": if pairs > 0 { elapsed / pairs as f32 } else { 0.0 },
            "renderer_built": renderer_built(),
        });
        std::fs::write(
            out_dir.join("index.json"),
            serde_json::to_string_pretty(&doc).expect("the index serializes"),
        )
        .expect("the index is writable");
        println!(
            "[moveset-render] {} render(s), {failures} failed, {elapsed:.1}s ({:.2}s each) -> {}",
            index.len(),
            elapsed / pairs.max(1) as f32,
            out_dir.display()
        );
    }
    if failures > 0 && !batching {
        std::process::exit(1);
    }
}

/// One (character, verb) render: what to stage and where to put it.
struct PairRequest<'a> {
    character: &'a str,
    verb: &'static move_exercise::Verb,
    target: &'a str,
    passive_target: bool,
    spacing: Option<f32>,
    /// World px across the picture, or `0` to leave the gameplay camera alone.
    view_width: f32,
    /// The capture's width/height, so a framing states a rectangle the picture
    /// actually has rather than one the aspect quietly widens.
    aspect: f32,
    frames: usize,
    stride: u64,
    combat_overlay: bool,
    layers: Option<CombatOverlayLayers>,
    adapter: AdapterPreference,
    out_dir: std::path::PathBuf,
    /// Whether this pair shared its app with others — see the manifest field.
    batched: bool,
}

/// How many fighters are staged, and how many the ceremony still holds.
fn staging_census(app: &mut App) -> (usize, usize) {
    let world = app.world_mut();
    let mut all = world.query::<&ambition_platformer2d::actor::MatchSeat>();
    let staged = all.iter(world).count();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::actor::MatchSeat,
        With<ambition_platformer2d::characters::control::ControlHolds>,
    >();
    (staged, q.iter(world).count())
}

/// The mtime of the binary drawing these pictures, as a unix timestamp.
///
/// Stamped by the binary that drew the picture, not by whichever binary is on
/// disk later, so an offline corpus can be refused once the renderer changes.
fn renderer_mtime() -> Option<f64> {
    let built = std::env::current_exe().ok()?.metadata().ok()?.modified().ok()?;
    Some(built.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64())
}

/// The same instant, readable.
fn renderer_built() -> Option<String> {
    let secs = renderer_mtime()? as i64;
    let days = secs / 86_400;
    // A civil date without pulling in a date crate for one provenance string.
    let (mut y, mut d) = (1970i64, days);
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let len = if leap { 366 } else { 365 };
        if d < len {
            break;
        }
        d -= len;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while m < 12 && d >= months[m] {
        d -= months[m];
        m += 1;
    }
    Some(format!(
        "{y:04}-{:02}-{:02} {:02}:{:02}",
        m + 1,
        d + 1,
        (secs % 86_400) / 3600,
        (secs % 3600) / 60
    ))
}

/// Stage one pair, drive it, and photograph it.
///
/// It re-seats every time. `afford_recovery` refuses a recovery whose airtime
/// already spent one, so a move rendered after another in the same match can
/// produce nothing. A batch saves the app boot, not the isolation.
fn render_pair(
    app: &mut App,
    camera: &DeterministicCaptureSession,
    req: &PairRequest,
) -> Result<serde_json::Value, String> {
    let character = req.character;
    let verb = req.verb;
    let target = req.target;
    let passive_target = req.passive_target;
    let spacing = req.spacing;
    let view_width = req.view_width;
    let aspect = req.aspect;
    let frames = req.frames;
    let stride = req.stride;
    let combat_overlay = req.combat_overlay;
    let layers = req.layers;
    let adapter = req.adapter;
    let out_dir = &req.out_dir;
    let previous_scope = app.world()
        .resource::<ambition_platformer2d::actor::ActiveSessionScope>().current();
    // The target is a seat with a stand-still brain, not a frozen body. A
    // live opponent walks into the strike.
    let roster = if passive_target {
        ambition_demo_smash::smash_roster_with_passive_targets([
            character,
            target,
        ])
    } else {
        ambition_demo_smash::smash_roster([character, target])
    };
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut live = false;
    for _ in 0..1200 {
        app.update();
        // Three conditions. Seated is not running, and running is not acting:
        // the opening ceremony holds the cast with `ControlHolds`, and a press
        // during it is discarded.
        let (staged, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&ambition_platformer2d::actor::MatchSeat>();
            let staged = all.iter(world).count();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actor::MatchSeat,
                With<ambition_platformer2d::characters::control::ControlHolds>,
            >();
            (staged, q.iter(world).count())
        };
        // The session is the identity, not the name. The outgoing cast
        // matches a re-seat of the same fighter, so wait for a scope that is
        // present and different, and for both seats. See
        // `moveset_takes::reseat`.
        let scope = app.world()
            .resource::<ambition_platformer2d::actor::ActiveSessionScope>().current();
        if scope.is_some() && scope != previous_scope
            && staged > 0
            && held == 0
            && move_exercise::seat_character(app, 0).as_deref() == Some(character)
            && move_exercise::seat_character(app, 1).as_deref() == Some(target)
            && ambition_platformer2d::rollback::session_is_active(app.world())
        {
            live = true;
            break;
        }
    }
    if !live {
        // Name the condition that failed. The common failure is the first: a
        // character id not on the smash grid seats nobody, so `staged` stays 0.
        let (staged, held) = staging_census(app);
        let rollback_session = ambition_platformer2d::rollback::session_is_active(app.world());
        let seated = move_exercise::seat_character(app, 0);
        let why = if staged > 0 && seated.as_deref() != Some(character) {
            format!(
                "the stage never became '{character}' — after 1200 updates seat zero \
                 still wears {seated:?}. The roster was published and the route asked \
                 for, so this is the match transition not completing rather than a \
                 character the grid does not carry."
            )
        } else if staged == 0 {
            format!(
                "'{character}' seated nobody — the match staged 0 fighters, so this is \
                 almost certainly a character id the smash grid does not carry rather \
                 than anything about rendering. Seatable ids: run `moveset_export` and \
                 read `characters[].id` from its bundle."
            )
        } else if held > 0 {
            format!(
                "'{character}' is seated ({staged}) but the opening ceremony still holds \
                 {held} of the cast under ControlHolds after 1200 updates — a press \
                 driven now would be discarded."
            )
        } else if !rollback_session {
            format!(
                "'{character}' is seated and free, but no rollback session became active \
                 in 1200 updates."
            )
        } else {
            format!("'{character}' never became drivable")
        };
        return Err(why);
    }

    // ── Settle, then prepare ──
    //
    // The settle is required. `session_is_active` is true while the cast is
    // still dropping in, so `take_off` could see a falling body, call it
    // airborne without jumping, and the fighter would land before the press.
    // The recorder settles the same way.
    if move_exercise::subject(app).is_none() {
        return Err(format!("nobody reached seat zero for '{character}'"));
    }
    let quiet = move_exercise::settle(app);
    // Spacing before posture, through the same `stage_for_press` the recorder
    // uses. A directional aerial needs the target behind, below, or dropping
    // onto her; the two tools must stage one scenario.
    let (closed, _asked) = move_exercise::stage_for_press(app, verb, spacing);
    let prepared = quiet && move_exercise::prepare(app, verb)
        && move_exercise::stage_airborne(app, verb);

    // What this press is supposed to produce, from the host's verb binding.
    // Without it, a back air that resolves as a forward air counts as a
    // success. Spacing is measured at the press: a connect launches the
    // target, so the gap after the run is larger.
    let spacing_at_press = move_exercise::gap_to_seat(app, 1).map(f32::abs);
    let intended = move_exercise::intended_move(app, character, verb.verb);
    // After staging. Roles are entity identities, and the route spawns the
    // cast.
    let scenario = ScenarioRoles::from_seats(app.world_mut(), 0, 1);

    // ── Perform, and photograph on exact ticks ──
    //
    // The exercise is a tick schedule. `--frames` and `--stride` choose only
    // what is observed, not how long a button is held.
    let facing = move_exercise::facing_of(app);
    let mut observed: std::collections::BTreeSet<String> = Default::default();
    let mut shots: Vec<serde_json::Value> = Vec::new();
    let mut pumps_total = 0usize;
    let mut action_tick = 0usize;

    for shot in 0..frames {
        move_exercise::step(
            app,
            move_exercise::action_frame(verb, action_tick, facing),
        );
        let tick = sim_tick(app);
        let at_action = action_tick;
        action_tick += 1;

        // Read the tick's semantics before the shutter, and write them into
        // the manifest afterwards from these values. `SimTick` is frozen
        // across a zero-duration pump, but the world is not: each pump runs
        // `Update`. A later read would describe a different moment.
        let at_shutter = move_exercise::subject(app);
        let shot_move = at_shutter.as_ref().and_then(|s| s.playing.clone());
        let shot_grounded = at_shutter.as_ref().and_then(|s| s.grounded);
        let shot_pose = at_shutter.as_ref().and_then(|s| s.pose);
        let shot_clip = at_shutter.and_then(|s| s.clip.clone());
        // The geometry the picture shows, as numbers: this tick's volumes,
        // roles, and move clock, so a reader that cannot see the PNG knows
        // what is in the frame. Sampled before the shutter, like `move`.
        let roles = scenario.resolve(app.world_mut());
        let observation = CombatObservation::capture(app.world_mut(), &roles).to_json();
        if let Some(id) = shot_move.clone() {
            observed.insert(id);
        }

        // Written before the shutter, read during it. `capture` pumps
        // `Update`, so `camera_follow` re-derives the stage framing and
        // `frame_the_inspection` puts this back after it.
        let framing = (view_width > 0.0)
            .then(|| frame_for(&observation, view_width, aspect))
            .flatten();
        if let Some(framing) = framing {
            app.world_mut().insert_resource(framing);
        }

        // Arm the shot, service the GPU with zero-duration pumps, and refuse
        // the frame if the fixed clock moved.
        let captured = match camera.capture(app, out_dir.join(format!("frame.{shot:04}.png")))
        {
            Ok(frame) => frame,
            Err(error) => return Err(format!("shot {shot}: {error}")),
        };
        debug_assert_eq!(captured.sim_tick, tick, "the session photographs the tick it was given");
        pumps_total += captured.pumps;
        shots.push(serde_json::json!({
            "file": format!("frame.{shot:04}.png"),
            // The world rectangle this picture shows, in the `[x0, y0, x1, y1]`
            // form of a recorded take's `view`. `null` means the gameplay
            // camera framed it.
            "view": framing.map(|f| f.rect()),
            // The absolute tick this picture belongs to...
            "sim_tick": tick,
            // ...and the tick of the exercise, which is what a recorded take's
            // frame index means. Separate runs share this origin.
            "action_tick": at_action,
            // Saved at the shutter, not re-read after the pump — see above.
            "move": shot_move,
            // What the engine meant to draw on this tick, beside the picture.
            // A mismatch between the PNG and the engine's animation decision
            // is visible.
            "pose": shot_pose.map(|p| format!("{p:?}")),
            "clip": shot_clip,
            // The body's posture on this tick. `prepared` says a posture was
            // set up before the press; this says what the body was doing
            // when the shutter opened.
            "grounded": shot_grounded,
            // The same tick's combat truth, in the one schema every recorder
            // writes: bodies with roles and hurtboxes, live strikes, move clock.
            "observation": observation,
        }));

        // Advance the rest of the stride on the same schedule.
        for _ in 1..stride {
            move_exercise::step(
                app,
                move_exercise::action_frame(verb, action_tick, facing),
            );
            action_tick += 1;
            if let Some(id) = move_exercise::playing_move(app) {
                observed.insert(id);
            }
        }
    }

    // A short request must not shorten the move. `--frames 4 --stride 2`
    // stops before the release at tick 37, so run the exercise past the
    // release without pictures, and report what was seen after the last shot
    // separately.
    let observed_in_shots = observed.clone();
    let mut after_capture: std::collections::BTreeSet<String> = Default::default();
    while !move_exercise::released_by(action_tick) {
        move_exercise::step(
            app,
            move_exercise::action_frame(verb, action_tick, facing),
        );
        action_tick += 1;
        if let Some(id) = move_exercise::playing_move(app) {
            after_capture.insert(id.clone());
            observed.insert(id);
        }
    }
    let last_action_tick = action_tick.saturating_sub(1);

    // Success is the intended move appearing in the posture asked for. A
    // grounded and an airborne up-B can share a move id. `prepare` reports
    // only what was true before the press, and a body airborne by a hair can
    // land on the next tick, so check the first shot (the press tick).
    let airborne_at_press = shots
        .first()
        .and_then(|shot| shot["grounded"].as_bool())
        .map(|grounded| !grounded);
    let posture_held = !verb.airborne || airborne_at_press == Some(true);
    let verdict = move_exercise::outcome(prepared && posture_held, intended.as_deref(), &observed);
    // The adapter used, not the one requested. A preference steers wgpu but
    // does not command it.
    let adapter_used = app
        .world()
        .get_resource::<bevy::render::renderer::RenderAdapterInfo>()
        .map(|info| format!("{} ({:?})", info.0.name, info.0.device_type))
        .unwrap_or_else(|| "unknown".to_string());
    let manifest = serde_json::json!({
        "character": character,
        "verb": verb.verb,
        "verb_label": verb.label,
        // Who was in the scenario, in the recorder's vocabulary.
        "subject": character,
        "target": target,
        "target_behavior": if passive_target { "passive" } else { "cpu" },
        // Whether the pictures carry geometry, so "no hitbox" differs from
        // "overlay off".
        "combat_overlay": combat_overlay,
        // Which layers are drawn, so "no hurtbox" differs from "hurtboxes not
        // drawn".
        "overlay_layers": layers.map(|l| serde_json::json!({
            "art": l.art, "hurtboxes": l.hurtboxes, "strikes": l.strikes,
        })),
        "requested_spacing": spacing,
        // The lens: close-ups or stage shots. `0` means the gameplay camera
        // framed them.
        "view_width": view_width,
        // Requested and reached spacing are separate numbers.
        "spacing_closed": closed,
        "spacing_at_press": spacing_at_press,
        "observation_schema": ambition_sim_harness::OBSERVATION_SCHEMA,
        "prepared": prepared,
        "settled": quiet,
        // What the body was doing on the press tick, which is the posture the
        // pictures actually show.
        "airborne_at_press": airborne_at_press,
        "posture_held": posture_held,
        "intended_move": intended,
        "observed_moves": observed.iter().cloned().collect::<Vec<_>>(),
        "observed_in_shots": observed_in_shots.iter().cloned().collect::<Vec<_>>(),
        "observed_after_capture": after_capture.iter().cloned().collect::<Vec<_>>(),
        "reached_intended_move": verdict.reached(),
        "outcome": verdict.as_str(),
        "hold_policy": "move_exercise_default",
        "hold_ticks": move_exercise::HOLD_TICKS,
        // What this run did, not what the schedule says. A capture horizon
        // shorter than the release is allowed; claiming the charge paid out
        // is not.
        "last_action_tick": last_action_tick,
        "release_reached": move_exercise::released_by(last_action_tick),
        "frames": shots.len(),
        "stride": stride,
        "shots": shots,
        "renderer": "moveset_render",
        // Which mode drew it: pixels are not byte-comparable across modes.
        // Two single-shot runs are identical; the same pair in a batch
        // differs on ~0.26% of pixels by at most 6/255 (presentation state
        // carried across the re-seat). Compare batched with batched.
        "batched": req.batched,
        // When the drawing binary was built, so a cache can tell a current
        // picture from an old one without asking the binary on disk now.
        "renderer_built": renderer_built(),
        "renderer_mtime": renderer_mtime(),
        // The adapter that drew these pixels, beside the one requested. PNGs
        // compare only when both runs report the same here; `auto` means the
        // machine chose.
        "adapter_requested": format!("{adapter:?}").to_lowercase(),
        "adapter_used": adapter_used,
        "zero_time_pumps": pumps_total,
    });
    std::fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("the manifest serializes"),
    )
    .map_err(|error| format!("the manifest is not writable: {error}"))?;
    Ok(manifest)

}

// The framing arithmetic has unit tests because a wrong rectangle fails
// nothing at runtime: the manifest just records it.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 4:3, the capture shape these numbers are quoted at.
    const ASPECT: f32 = 4.0 / 3.0;

    fn observation(subject: [f32; 2], target: Option<[f32; 2]>) -> serde_json::Value {
        let mut bodies = vec![json!({"role": "subject", "collision": {"pos": subject}})];
        if let Some(target) = target {
            bodies.push(json!({"role": "target", "collision": {"pos": target}}));
        }
        json!({ "bodies": bodies })
    }

    /// The view must contain the pair plus the margin on each side.
    fn holds_both(view: Vec2, subject: [f32; 2], target: [f32; 2]) -> bool {
        let need_x = (subject[0] - target[0]).abs() + 2.0 * FRAME_MARGIN;
        let need_y = (subject[1] - target[1]).abs() + 2.0 * FRAME_MARGIN;
        view.x >= need_x - 0.01 && view.y >= need_y - 0.01
    }

    #[test]
    fn vertical_separation_widens_the_frame_that_holds_both_fighters() {
        // 200 px of vertical separation needs 320 px of height with these
        // margins.
        let subject = [0.0, 0.0];
        let target = [0.0, 200.0];
        let framing = frame_for(&observation(subject, Some(target)), DEFAULT_VIEW_WIDTH, ASPECT)
            .expect("a placed subject frames");

        assert!(
            framing.view.y >= 320.0 - 0.01,
            "200 px of vertical separation plus 2x{FRAME_MARGIN} px of margin needs \
             320 px of height; got {} (view {:?}). `span.y` is not reaching the width.",
            framing.view.y,
            framing.view,
        );
        assert!(holds_both(framing.view, subject, target));
    }

    #[test]
    fn horizontal_separation_still_frames_exactly_as_before() {
        // Control: the default seats are 192 px apart in x. Taking the max
        // must not change that case, or "always widen" would pass above.
        let subject = [-96.0, 0.0];
        let target = [96.0, 0.0];
        let framing = frame_for(&observation(subject, Some(target)), DEFAULT_VIEW_WIDTH, ASPECT)
            .expect("a placed subject frames");

        // 192 + 120 = 312, under the 320 default, so the default width stands.
        assert!(
            (framing.view.x - DEFAULT_VIEW_WIDTH).abs() < 0.01,
            "a 192 px horizontal pair fits the default width and must not widen; got {:?}",
            framing.view,
        );
        assert!(holds_both(framing.view, subject, target));
    }

    #[test]
    fn the_widening_cap_still_bounds_a_vertical_pair() {
        // Control: `span.y` must go through the same cap. A pair this far
        // apart is a stage, not a shot.
        let framing = frame_for(
            &observation([0.0, 0.0], Some([0.0, 4000.0])),
            DEFAULT_VIEW_WIDTH,
            ASPECT,
        )
        .expect("a placed subject frames");

        assert!(
            (framing.view.x - DEFAULT_VIEW_WIDTH * MAX_FRAME_WIDENING).abs() < 0.01,
            "the cap stopped bounding the frame once span.y could raise it; got {:?}",
            framing.view,
        );
    }

    #[test]
    fn a_lone_subject_keeps_the_plain_default_frame() {
        // Control: no target means no pair; the default span must not widen
        // by its own y.
        let framing = frame_for(&observation([12.0, -34.0], None), DEFAULT_VIEW_WIDTH, ASPECT)
            .expect("a placed subject frames");

        assert!((framing.view.x - DEFAULT_VIEW_WIDTH).abs() < 0.01, "{:?}", framing.view);
        assert_eq!(framing.center, Vec2::new(12.0, -34.0));
    }

    #[test]
    fn an_unplaceable_subject_still_yields_no_framing() {
        // Control: no subject stays `None`, not a picture of the origin.
        assert!(frame_for(&json!({"bodies": []}), DEFAULT_VIEW_WIDTH, ASPECT).is_none());
    }
}
