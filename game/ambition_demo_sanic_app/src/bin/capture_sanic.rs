//! Photograph Sanic.
//!
//! the SECOND caller of `ambition_render::capture`, and that is the point.
//! A seam with one consumer is a guess. Everything hard is shared — target,
//! camera adoption, readback, PNG, exit — and what is left here is genuinely
//! Sanic's: which app to build and when its world is worth photographing.
//!
//! this binary is deliberately small, and that is the evidence. Everything
//! hard — building a render target, pointing the cameras at it, reading the
//! texture back, writing the PNG, exiting — is `ambition_render::capture`, shared
//! and game-agnostic. What is left here is the part that genuinely differs
//! between games: which app to build, and when its world is worth photographing.
//! If a second demo needs more than this file's length, the split is in the
//! wrong place.
//!
//! ```text
//! cargo run -p ambition_demo_sanic_app --features visible --bin capture_sanic \
//!     -- OUT.png [WIDTHxHEIGHT] [--warmup N] [--walk N] [--no-ui]
//! ```

use std::path::PathBuf;

use ambition_platformer2d::render::capture::{
    adopt_cameras_into_capture_target, finish_after_capture, request_capture, setup_capture_target,
    CaptureProgress, CaptureSettings, CaptureTarget,
};
use bevy::prelude::*;

/// Frames to advance before shooting.
#[derive(Resource)]
struct Warmup {
    remaining: u32,
    /// Frames of held RIGHT before the shot.
    ///
    /// Without it the camera only ever sees the opening screen — and Sanic's
    /// whole point is what happens once he is moving.
    walk_right: u32,
    /// Frames to let the LAST walking input actually be simulated.
    ///
    /// One frame is enough: the input written this frame is collected in the
    /// same frame's `InputSet::Collect` (which the writer is now ordered before)
    /// and moves the body on the next simulation step.
    settle: u32,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut output = PathBuf::from("sanic.png");
    let mut size = UVec2::new(960, 540);
    let mut warmup = 90u32;
    let mut include_ui = true;
    let mut walk = 0u32;
    let mut center_subject = false;

    let mut positional_seen = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmup" => {
                warmup = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--warmup needs a frame count"));
            }
            "--walk" => {
                walk = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--walk needs a frame count"));
            }
            "--no-ui" => include_ui = false,
            "--center-subject" => center_subject = true,
            other if other.starts_with("--") => fail(&format!("unknown flag '{other}'")),
            other if !positional_seen => {
                output = PathBuf::from(other);
                positional_seen = true;
            }
            other => {
                // WIDTHxHEIGHT, the same shape `capture_scene` takes, so a
                // command line moves between the two tools unchanged.
                let Some((w, h)) = other.split_once('x') else {
                    fail(&format!("expected WIDTHxHEIGHT, got '{other}'"));
                };
                let (Ok(w), Ok(h)) = (w.parse(), h.parse()) else {
                    fail(&format!("expected WIDTHxHEIGHT, got '{other}'"));
                };
                size = UVec2::new(w, h);
            }
        }
    }

    // `OffscreenGpu`, never `Headless`: headless sets `backends: None`, so
    // there is no RenderApp and no texture to read. See `RenderMode`.
    // the GAMEPLAY route, not the default launcher home. Booting the launcher
    // and counting frames writes a blank PNG — no amount of warmup walks a menu
    // into a level, and that is the readiness contract `capture` hands back to
    // the caller. It is also literally what the first attempt produced.
    let mut app = ambition_demo_sanic_app::build_windowed_demo_app_with_home(
        ambition_demo_sanic_app::RenderMode::OffscreenGpu,
        ambition_demo_sanic::SANIC_GAMEPLAY_ROUTE,
    );
    // ⛔⛔ THE RUNNER, AND THIS BINARY IS WHY IT EXISTS. `Display::Offscreen`
    // disables `winit`, which is also Bevy's app RUNNER — so without this the
    // `run()` below performs exactly ONE update and returns, the process exits 0
    // and NO FILE IS WRITTEN. It lives here rather than in the builder because
    // the engine's offscreen face is deliberately CALLER-STEPPED: the tests drive
    // `update()` themselves and must not inherit a run loop.
    app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
        std::time::Duration::from_millis(0),
    ));
    app.insert_resource(CaptureSettings {
        output,
        size,
        include_ui,
    });
    app.init_resource::<CaptureProgress>();
    app.insert_resource(Warmup {
        remaining: warmup,
        walk_right: walk,
        settle: 1,
    });
    app.insert_resource(CenterSubject(center_subject));
    app.add_systems(Startup, setup_capture_target);
    // ⭐ AFTER the gameplay camera resolver, not before it. The capture chain
    // below runs `.before(InputSet::Collect)` in `Update`, which is upstream of
    // the follow camera — an override written there is simply overwritten. This
    // sits in `PostUpdate` ahead of propagation, so it wins and still reaches
    // `GlobalTransform` before the frame renders.
    app.add_systems(
        bevy::app::PostUpdate,
        center_the_camera_on_the_subject
            .before(bevy::transform::TransformSystems::Propagate)
            .run_if(|centered: Res<CenterSubject>| centered.0),
    );
    // the synthetic input must be written BEFORE the frame collects it.
    // These sat in `Update` with no edge to `ambition_platformer2d::input::InputSet::Collect`,
    // which is also in `Update` — so whether a press written here was seen by
    // the same frame or the next one was left to whatever order Bevy happened to
    // pick. Ambiguity in a measuring instrument is worse than a known offset:
    // the capture is supposed to be the thing that settles arguments.
    app.add_systems(
        Update,
        (
            adopt_cameras_into_capture_target,
            hold_right_while_walking,
            shoot_when_warm,
            finish_after_capture,
        )
            .chain()
            .before(ambition_platformer2d::input::InputSet::Collect),
    );
    app.run();
}

/// Count the world in, then ask for the picture.
///
/// it waits for an ADOPTED camera, not just for frames. A demo builds its
/// cameras when its shell resolves a route, which is well after `Startup`;
/// shooting before then reads back 960x540 pixels of `(0,0,0,0)` and reports
/// success. That is not hypothetical — it is what the first two attempts wrote,
/// and it took printing the pixel values to tell "nothing drew" apart from "the
/// scene is white". Warmup only starts counting once something is drawing.
fn shoot_when_warm(
    mut commands: Commands,
    target: Option<Res<CaptureTarget>>,
    mut progress: ResMut<CaptureProgress>,
    mut warmup: ResMut<Warmup>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    players: Query<
        &GlobalTransform,
        With<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >,
) {
    let Some(target) = target else {
        return;
    };
    if target.adopted == 0 {
        return;
    }
    if warmup.remaining > 0 {
        warmup.remaining -= 1;
        return;
    }
    // AND the walk has to finish. This shot as soon as the warmup ran out, whatever
    // `--walk` said — so `--warmup 10 --walk 300` took the picture on frame 10 with 290 frames
    // of travel still owed, and the capture showed the spawn point. The flag asked for a
    // journey and the tool photographed the departure lounge.
    if warmup.walk_right > 0 {
        return;
    }
    // See `Warmup::settle`.
    if warmup.settle > 0 {
        warmup.settle -= 1;
        return;
    }
    warn_if_the_subject_left_the_frame(&cameras, &players);
    request_capture(&mut commands, &target, &mut progress);
}

/// Say when the picture does not contain the thing it is a picture OF.
///
/// ⛔⛔ D257, AND IT REPORTED SUCCESS THREE TIMES BEFORE ANYONE LOOKED AT THE
/// FILE. Sanic declares `high_speed_full_bleed()` — velocity-aware framing where
/// the camera LEADS the runner rather than trailing him, which is the whole feel
/// of a speedway and must not be "fixed". But a lead is a fixed world-space
/// offset: shrink the visible region and the runner walks out of frame. Measured
/// 2026-08-29 — correct at 1280x720, subject at the extreme edge at the default
/// 960x540, and NOTHING BUT SKY at 320x240, with `capture: wrote …` and exit 0
/// every time.
///
/// ⭐ This does not change the framing, because that is a design call
/// (`capture_scene` answers the same problem with `--fit-room`). It makes the
/// tool ADMIT the failure instead of reporting success — the same rule the rest
/// of this repo's instruments follow: a probe that can mislead says so in its own
/// output.
fn warn_if_the_subject_left_the_frame(
    cameras: &Query<(&Camera, &GlobalTransform)>,
    players: &Query<
        &GlobalTransform,
        With<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >,
) {
    let Ok(player) = players.single() else {
        return; // no subject to lose; the shot is of whatever is there
    };
    let subject = player.translation();
    // ANY active camera seeing it is enough — a demo may carry a HUD camera too.
    let framed = cameras.iter().any(|(camera, camera_at)| {
        camera.is_active
            && camera
                .world_to_viewport(camera_at, subject)
                .is_ok_and(|point| {
                    camera.logical_viewport_size().is_some_and(|size| {
                        point.x >= 0.0 && point.y >= 0.0 && point.x <= size.x && point.y <= size.y
                    })
                })
    });
    if !framed {
        eprintln!(
            "[capture_sanic] ⛔ THE SUBJECT IS NOT IN FRAME: the player is at \
             ({:.0},{:.0}) and no active camera's viewport contains it. This shot will \
             report success and show the background. The camera LEADS the runner \
             (`high_speed_full_bleed`), so a small --size walks him out of the picture — \
             try the native 1280x720. ⚠ The coordinates are ENGINE WORLD SPACE, where the \
             level is CENTRED on the origin: a 6400-wide level spans -3200..3200, so a \
             large negative x near -3040 is the LDtk spawn at px=146, not an escaped \
             player. See queue.md D257.",
            subject.x, subject.y
        );
    }
}

/// Hold RIGHT while there are walking frames left.
///
/// written into `ButtonInput` directly, which is what `capture_scene`
/// does: the demo's binding layer reads the same resource a real keyboard fills,
/// so this exercises the actual input path rather than a bypass.
fn hold_right_while_walking(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    target: Option<Res<CaptureTarget>>,
    mut warmup: ResMut<Warmup>,
) {
    // Walking starts only once something is drawing, for the same reason the
    // warmup does: frames spent waiting for a body are latency, not travel.
    if target.is_none_or(|target| target.adopted == 0) {
        return;
    }
    if warmup.walk_right == 0 {
        keys.release(KeyCode::ArrowRight);
        keys.release(KeyCode::KeyZ);
        return;
    }
    warmup.walk_right -= 1;
    keys.press(KeyCode::ArrowRight);
    // A periodic hop, for the same reason Mary-O's capture needs one: a
    // traversal that cannot jump reaches the first hazard and stops. Not a
    // route — a capture wanting a specific one wants a real `--press` sequence
    // like `capture_scene`'s.
    let phase = warmup.walk_right % 45;
    if phase < 12 {
        keys.press(KeyCode::KeyZ);
    } else {
        keys.release(KeyCode::KeyZ);
    }
}

fn fail(message: &str) -> ! {
    eprintln!("capture_sanic: {message}");
    eprintln!(
        "usage: capture_sanic OUT.png [WIDTHxHEIGHT] [--warmup N] [--walk N] [--no-ui] \
         [--center-subject]"
    );
    std::process::exit(2);
}

/// Whether this capture frames the SUBJECT rather than the gameplay camera.
#[derive(Resource)]
struct CenterSubject(bool);

/// `--center-subject`: put the runner in the middle of the picture.
///
/// ⭐ THIS IS THE FIX D257 ASKED FOR, AND IT DELIBERATELY DOES NOT TOUCH THE
/// FRAMING ITSELF. Sanic declares `high_speed_full_bleed`, whose camera LEADS the
/// runner — that lead is the whole feel of a speedway and must not be "fixed".
/// But a lead is a fixed WORLD-SPACE offset, so shrinking the visible region
/// walks the subject out of the picture: correct at 1280x720, at the extreme edge
/// at the default 960x540, and nothing but sky at 320x240.
///
/// ⇒ a capture wanting a PORTRAIT asks for one, exactly as `capture_scene`
/// answers the same problem with `--fit-room`. Gameplay framing is untouched and
/// remains the default here; this flag is a capture-time override.
///
/// ⛔ NOT `--fit-room`, which is the wrong shape for these demos: sanic's speedway
/// is 6400px wide, so fitting the whole room makes the subject a speck. The demo
/// captures are portraits OF A SUBJECT — which is what
/// `warn_if_the_subject_left_the_frame` already checks for.
fn center_the_camera_on_the_subject(
    mut cameras: Query<(&Camera, &mut Transform)>,
    players: Query<
        &GlobalTransform,
        With<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >,
) {
    let Ok(player) = players.single() else {
        return; // no subject to centre on; leave the gameplay framing alone
    };
    let subject = player.translation();
    // Every ACTIVE camera, which is the same set `warn_if_the_subject_left_the_frame`
    // asks — so the guard and the override cannot disagree about what "in frame"
    // means. ⚠ Bevy UI lays out in SCREEN space and is unaffected; a sprite-based
    // HUD on its own camera does move with this, which is what `--no-ui` is for.
    for (camera, mut transform) in &mut cameras {
        if !camera.is_active {
            continue;
        }
        transform.translation.x = subject.x;
        transform.translation.y = subject.y;
    }
}
