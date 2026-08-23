//! A BURST OF SCREENSHOTS from a CPU-versus-CPU match on the smash stage.
//!
//! ```text
//! cargo run -p ambition_demo_smash_app --features visible --bin match_shots -- \
//!     --out target/shots --frames 8 --every 12 --after 240
//! ```
//!
//! ⛔ not to be confused with `capture_probe`, which is about grab/hold
//! CAPTURE — `SmashHoldState` — and has nothing to do with pictures.
//!
//! Why a burst and not a screenshot: every in-match cue this demo has is a
//! BEAT. A charge pulse, an i-frame blink, an impact flash, a launch trail, a
//! parry snap and the dizzy ring are all things that are only true for a few
//! frames, and a single shot taken at an arbitrary moment shows a fighter
//! standing still. Frames spaced across a real exchange is the smallest thing
//! that can show any of them.
//!
//! The spacing is in SIM TICKS and it is exact, not approximate: the windowed
//! builder pins `TimeUpdateStrategy::ManualDuration(timestep)`, so one
//! `update()` is one tick. `--every 12` is a fifth of a second at 60Hz.
//!
//! It runs on [`Display::Offscreen`] — a real backend with no window. Disabling
//! `winit` takes the app RUNNER with it, which is exactly what this wants: the
//! burst is as many frames as it asks for and not one more.

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
        }
    }
}

fn parse_args() -> Shots {
    let mut shots = Shots::default();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().unwrap_or_default();
        match flag.as_str() {
            "--out" => shots.out = std::path::PathBuf::from(value()),
            "--frames" => shots.frames = value().parse().unwrap_or(shots.frames),
            "--every" => shots.every = value().parse().unwrap_or(shots.every).max(1),
            "--after" => shots.after = value().parse().unwrap_or(shots.after),
            "--character" => shots.character = value(),
            "--size" => {
                let raw = value();
                if let Some((w, h)) = raw.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                        shots.size = UVec2::new(w, h);
                    }
                }
            }
            other => eprintln!("match_shots: ignoring unknown argument '{other}'"),
        }
    }
    shots
}

/// Set by the driver on the frame it wants a picture; cleared once asked.
#[derive(Resource, Default)]
struct ShootNow(bool);

/// Ask for the readback, but only once there is something drawing into the
/// target.
///
/// `CaptureTarget::adopted == 0` means no camera is pointed at the texture, and
/// shooting anyway writes a transparent PNG and reports success — the failure
/// this whole module's doc warns about.
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

fn main() {
    let shots = parse_args();
    if let Err(error) = std::fs::create_dir_all(&shots.out) {
        eprintln!(
            "match_shots: cannot create '{}': {error}",
            shots.out.display()
        );
        std::process::exit(2);
    }

    let mut app = ambition_demo_smash_app::build_windowed_demo_app(
        ambition_platformer2d::app::Display::Offscreen,
    );
    app.insert_resource(CaptureSettings {
        output: shots.out.join("pending.png"),
        size: shots.size,
        include_ui: true,
    });
    // THE SURFACE THIS RUN DRAWS TO — and the whole reason the HUD used to be
    // missing from these shots.
    //
    // Every other link existed: the demo declares a full HUD, publishes its
    // readouts, installs `DeclaredHudPlugin`, spawns the `FrontHudCamera` at
    // `order: 9` over a non-clearing target, and the capture adopts it. What
    // was missing is that nothing ever told the layout resolver how big this
    // composition is. `resolve_host_gameplay_presentation` reads the primary
    // window, finds none in an offscreen app, and — without this resource —
    // returns early, leaving `ResolvedGameplayPresentation` at its default. So
    // every HUD slot laid itself out against a rectangle that describes nothing.
    //
    // ⭐ THE RESOURCE ALREADY EXISTED FOR EXACTLY THIS, and `capture_scene` has
    // always declared it. This tool simply never joined. A capture that cannot
    // show a layout is worse than no capture, because it shows a DIFFERENT
    // layout convincingly — which is precisely what these shots did for a week.
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
    // Adoption runs every frame because WHEN a camera appears is composition
    // business, then the shot is asked for after it — same frame, in order, so
    // a camera created this frame can still be shot this frame.
    app.add_systems(
        Update,
        (adopt_cameras_into_capture_target, shoot_when_asked).chain(),
    );

    // ── finish the plugins before stepping ───────────────────────────────
    //
    // `App::run()` does this; `App::update()` does NOT. `RenderPlugin` creates
    // the wgpu device in `finish()`, so a manually stepped offscreen app that
    // goes straight to `update()` panics inside the render app's startup with
    // `Res<RenderDevice>` missing — which reads like a broken composition and
    // is really a missing call. Mary-o's capture never hit it because it calls
    // `run()` with a schedule runner.
    app.finish();
    app.cleanup();

    // ── into a match ─────────────────────────────────────────────────────
    //
    // A burst has to be aimed at a FIGHT, and neither of the two obvious ways
    // gets one. The demo's home is character select, so the default boot
    // photographs a menu — byte-identically, every frame, which is what the
    // first run of this tool produced. Booting straight onto the gameplay route
    // instead gives an empty stage: select is what SEATS the fighters, so
    // skipping it skips them.
    //
    // This is `match_report`'s recipe, which is the one that produces a real
    // CPU-versus-CPU match: boot normally, declare a CPU roster, then ask the
    // shell to go. `SmashSelect::roster` would make every locked seat a HUMAN,
    // which is right for a couch game and wrong for a burst nobody is playing.
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
    // Past the ceremony: every fighter carries scripted control for the whole
    // 3-2-1-GO, so a shot inside the hold photographs bodies that are forbidden
    // to act. Read the count from the ruleset rather than restating it.
    let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
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

    // ── the burst ────────────────────────────────────────────────────────
    let mut written = 0u32;
    for index in 0..shots.frames {
        for _ in 0..shots.every {
            app.update();
        }
        let path = shots.out.join(format!("shot_{index:02}.png"));
        app.world_mut().resource_mut::<CaptureSettings>().output = path.clone();
        *app.world_mut().resource_mut::<CaptureProgress>() = CaptureProgress::default();
        app.world_mut().resource_mut::<ShootNow>().0 = true;
        // A readback is asynchronous, so the picture is not on disk when the
        // frame that asked for it ends. Bounded rather than `loop`: a capture
        // that never completes must fail rather than hang a CI run.
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

fn cameras_are_drawing(app: &mut App) -> bool {
    app.world()
        .get_resource::<CaptureTarget>()
        .is_some_and(|target| target.adopted > 0)
}
