//! A burst of screenshots from a CPU-versus-CPU match on the smash stage.
//!
//! ```text
//! cargo run -p ambition_demo_smash_app --features visible --bin smash_tool -- match-shots -- \
//!     --out target/shots --frames 8 --every 12 --after 240
//! ```
//!
//! Not to be confused with `capture_probe`, which is about grab/hold capture
//! (`SmashHoldState`), not pictures.
//!
//! A burst, not one shot: every in-match cue (charge pulse, i-frame blink,
//! impact flash, launch trail, parry snap, dizzy ring) lasts a few frames.
//!
//! Spacing is in sim ticks and exact: the windowed builder pins
//! `TimeUpdateStrategy::ManualDuration(timestep)`, so one `update()` is one
//! tick. `--every 12` is a fifth of a second at 60Hz.
//!
//! `--on-ko` starts the burst on the tick a `KnockoutBeatRequested` lands.
//! A knockout is rare and its sparks last under half a second, so a fixed
//! cadence catches one only by luck. Each shot is annotated on stdout with
//! its sim tick, the knockout's world position, and the camera rect, so a
//! picture can be checked against its geometry.
//!
//! It runs on [`Display::Offscreen`], a real backend with no window.
//! Disabling `winit` also removes the app runner, so the burst is exactly as
//! many frames as it asks for.

use bevy::prelude::*;

use ambition_platformer2d::render::capture::{
    adopt_cameras_into_capture_target, request_capture, setup_capture_target, CaptureProgress,
    CaptureSettings, CaptureTarget,
};

/// What the caller asked for.
struct Shots {
    out: std::path::PathBuf,
    frames: u32,
    every: u32,
    after: u32,
    size: UVec2,
    character: String,
    /// Start the burst on the tick a knockout is published rather than at
    /// `--after`.
    on_ko: bool,
}

impl Default for Shots {
    fn default() -> Self {
        Self {
            out: std::path::PathBuf::from("target/shots"),
            // Enough to catch a beat that lasts a few frames somewhere in an
            // exchange, few enough to look at all of them.
            frames: 8,
            // A fifth of a second at 60Hz.
            every: 12,
            // Four seconds in: past the 3-2-1-GO countdown and into a fight.
            after: 240,
            size: UVec2::new(960, 540),
            character: ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string(),
            on_ko: false,
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct MatchShotsArgs {
    /// Directory to write the burst into.
    #[arg(long, default_value = "target/shots")]
    pub out: std::path::PathBuf,
    /// How many frames to capture. Enough to catch a beat that lasts a few
    /// frames somewhere in an exchange, few enough to look at all of them.
    #[arg(long, default_value_t = 8)]
    pub frames: u32,
    /// Ticks between frames. A fifth of a second at 60 Hz.
    #[arg(long, default_value_t = 12, value_parser = at_least_one)]
    pub every: u32,
    /// Tick to start on. Four seconds in: past the 3-2-1-GO countdown and into
    /// a fight.
    #[arg(long, default_value_t = 240)]
    pub after: u32,
    /// Frame size, as `WIDTHxHEIGHT`.
    #[arg(long, default_value = "960x540", value_parser = parse_size)]
    pub size: UVec2,
    /// Which fighter to photograph.
    #[arg(long, default_value_t = ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string())]
    pub character: String,
    /// Start the burst on the tick a knockout is published rather than at
    /// `--after`.
    #[arg(long)]
    pub on_ko: bool,
}

/// Refuse zero instead of rounding it up, so the value the caller typed is
/// the value that runs.
fn at_least_one(raw: &str) -> Result<u32, String> {
    match raw.parse::<u32>() {
        Ok(0) => Err("must be at least 1 tick between frames".to_string()),
        Ok(n) => Ok(n),
        Err(e) => Err(e.to_string()),
    }
}

/// `WIDTHxHEIGHT`. A malformed value is reported, not replaced by the
/// default.
fn parse_size(raw: &str) -> Result<UVec2, String> {
    let (w, h) = raw
        .split_once('x')
        .ok_or_else(|| format!("expected WIDTHxHEIGHT, got '{raw}'"))?;
    let w: u32 = w.parse().map_err(|_| format!("bad width in '{raw}'"))?;
    let h: u32 = h.parse().map_err(|_| format!("bad height in '{raw}'"))?;
    Ok(UVec2::new(w, h))
}

impl From<MatchShotsArgs> for Shots {
    fn from(a: MatchShotsArgs) -> Self {
        Self {
            out: a.out,
            frames: a.frames,
            every: a.every,
            after: a.after,
            size: a.size,
            character: a.character,
            on_ko: a.on_ko,
        }
    }
}

/// Set by the driver on the frame it wants a picture; cleared once asked.
#[derive(Resource, Default)]
struct ShootNow(bool);

/// Ask for the readback, but only once there is something drawing into the
/// target.
///
/// `CaptureTarget::adopted == 0` means no camera draws into the texture.
/// Shooting then writes a transparent PNG and reports success.
fn shoot_when_asked(
    mut commands: Commands,
    mut now: ResMut<ShootNow>,
    target: Option<Res<CaptureTarget>>,
    mut progress: ResMut<CaptureProgress>,
) {
    if !now.0 {
        return;
    }
    let Some(target) = target else { return };
    if target.adopted == 0 {
        return;
    }
    request_capture(&mut commands, &target, &mut progress);
    now.0 = false;
}

pub fn run(args: MatchShotsArgs) {
    let shots: Shots = args.into();
    if let Err(error) = std::fs::create_dir_all(&shots.out) {
        eprintln!(
            "match_shots: cannot create '{}': {error}",
            shots.out.display()
        );
        std::process::exit(2);
    }

    let mut app = crate::build_windowed_demo_app(
        ambition_platformer2d::app::Display::Offscreen,
    );
    app.insert_resource(CaptureSettings {
        output: shots.out.join("pending.png"),
        size: shots.size,
        include_ui: true,
    });
    // Declare the surface this run draws to. In an offscreen app,
    // `resolve_host_gameplay_presentation` finds no primary window and,
    // without `HeadlessDisplaySurface`, leaves `ResolvedGameplayPresentation`
    // at its default (`WINDOW_W x WINDOW_H`, 1600x900). Then HUD slots land off
    // the image and `publish_camera_viewport` crops the picture to its centre.
    // `capture_scene` declares the same resource.
    //
    // The camera's world framing depends only on the viewport's aspect, which
    // is 16:9 either way, so shots taken without this are wrong in pixels, not
    // in camera policy.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(shots.size.x as f32, shots.size.y as f32),
        ),
    );
    app.init_resource::<CaptureProgress>();
    app.init_resource::<ShootNow>();
    app.add_systems(
        Startup,
        setup_capture_target
            .after(ambition_platformer2d::presentation::PlatformerPresentationSetupSet),
    );
    // Adopt cameras every frame (when a camera appears is composition
    // business), then shoot in the same frame, so a new camera can be shot at
    // once.
    app.add_systems(
        Update,
        (adopt_cameras_into_capture_target, shoot_when_asked).chain(),
    );

    // Finish the plugins before stepping. `App::run()` does this;
    // `App::update()` does not. `RenderPlugin` creates the wgpu device in
    // `finish()`, so without it the render app panics on a missing
    // `Res<RenderDevice>`.
    app.finish();
    app.cleanup();

    // Into a match. The demo boots to character select, and select seats the
    // fighters, so booting straight to the gameplay route gives an empty
    // stage. As in `match_report`: boot normally, declare a CPU roster, then
    // ask the shell to go. `SmashSelect::roster` would make every locked seat
    // human.
    for _ in 0..30 {
        app.update();
    }
    let characters = [shots.character.as_str(), shots.character.as_str()];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Wait past the 3-2-1-GO: fighters are held for the whole ceremony. Read
    // the count from the ruleset. This binary needs
    // `required-features = ["visible", "capture"]`, so ordinary checks and
    // test runs do not build it.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as u32 + shots.after) {
        app.update();
    }
    if !cameras_are_drawing(&mut app) {
        eprintln!(
            "match_shots: no camera adopted the capture target after {} ticks — every shot \
             would be a transparent PNG reported as a success",
            shots.after
        );
        std::process::exit(1);
    }

    // The burst.
    if shots.on_ko {
        match step_until_a_knockout(&mut app) {
            Some(note) => println!("match_shots: knockout — {note}"),
            None => {
                eprintln!(
                    "match_shots: no knockout in this match, so an --on-ko burst would \
                     photograph an ordinary exchange and be filed as a knockout"
                );
                std::process::exit(1);
            }
        }
    }
    let mut written = 0u32;
    for index in 0..shots.frames {
        // The first shot of an `--on-ko` burst is the knockout's own frame, so
        // do not step first.
        if !(shots.on_ko && index == 0) {
            for _ in 0..shots.every {
                app.update();
            }
        }
        println!("  shot_{index:02} {}", frame_note(&mut app));
        let path = shots.out.join(format!("shot_{index:02}.png"));
        app.world_mut().resource_mut::<CaptureSettings>().output = path.clone();
        *app.world_mut().resource_mut::<CaptureProgress>() = CaptureProgress::default();
        app.world_mut().resource_mut::<ShootNow>().0 = true;
        // A readback is asynchronous. Bound the wait so a capture that never
        // completes fails instead of hanging CI.
        let mut settled = false;
        for _ in 0..240 {
            app.update();
            if app.world().resource::<CaptureProgress>().completed {
                settled = true;
                break;
            }
        }
        let progress = app.world().resource::<CaptureProgress>();
        if !settled || progress.failed {
            eprintln!(
                "match_shots: shot {index} did not land at {}",
                path.display()
            );
            std::process::exit(1);
        }
        written += 1;
    }

    println!(
        "match_shots: wrote {written} frames to {} ({}x{} px, one every {} ticks from tick {})",
        shots.out.display(),
        shots.size.x,
        shots.size.y,
        shots.every,
        shots.after,
    );
}

/// Step the match until a knockout is published, or the match is over.
///
/// Reads the same `KnockoutBeatRequested` intents the beat draws from, so
/// the wait and the picture share one fact. The knockout is a message, so
/// read it through this tool's own cursor, as presentation does.
fn step_until_a_knockout(app: &mut App) -> Option<String> {
    for _ in 0..12_000 {
        app.update();
        if !knockouts_this_frame(app).is_empty() {
            return Some(frame_note(app));
        }
    }
    None
}

/// The knockout intents standing this frame, drained through this tool's own
/// cursor so reading them here does not consume presentation's copy.
fn knockouts_this_frame(app: &mut App) -> Vec<(ambition_platformer2d::engine_core::Vec2, bool, f32)> {
    let Some(messages) = app
        .world()
        .get_resource::<bevy::prelude::Messages<ambition_platformer2d::vfx::vfx::KnockoutBeatRequested>>()
    else {
        return Vec::new();
    };
    let mut cursor = messages.get_cursor();
    cursor
        .read(messages)
        .map(|ko| (ko.pos, ko.eliminated, ko.speed))
        .collect()
}

/// What this frame actually holds: the sim tick, any knockout published on it,
/// and the camera rect it is framed by.
///
/// A cue drawn just outside the frame looks the same as one never drawn.
/// The clearance printed here is the distance from the knockout to the
/// nearest frame edge, which is what a clamp would change.
fn frame_note(app: &mut App) -> String {
    let tick = app
        .world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
        .unwrap_or_default();
    let kos = knockouts_this_frame(app);
    let observer = ambition_platformer2d::sim_view::the_only_view(app.world_mut());
    let camera = app
        .world()
        .entity(observer)
        .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
        // A view that is not framed yet reports no frame; do not measure a
        // default window.
        .and_then(|resolved| {
            resolved
                .frame()
                .map(|frame| (frame.snapshot.center_world, frame.snapshot.visible_view))
        });
    let Some((centre, visible)) = camera else {
        return format!("t{tick} no camera");
    };
    let half = visible / 2.0;
    let mut note = format!(
        "t{tick} frame {:.0}x{:.0}@({:.0},{:.0})",
        visible.x, visible.y, centre.x, centre.y
    );
    for (pos, eliminated, speed) in kos {
        let clearance = (half.x - (pos.x - centre.x).abs()).min(half.y - (pos.y - centre.y).abs());
        note.push_str(&format!(
            "  KO ({:.0},{:.0}) elim={eliminated} v={speed:.0} clearance {clearance:.0}",
            pos.x, pos.y
        ));
    }
    note
}

fn cameras_are_drawing(app: &mut App) -> bool {
    app.world()
        .get_resource::<CaptureTarget>()
        .is_some_and(|target| target.adopted > 0)
}
