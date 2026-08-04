//! **Photograph Sanic.**
//!
//! ⭐ **the SECOND caller of `ambition_render::capture`, and that is the point.**
//! A seam with one consumer is a guess. Everything hard is shared — target,
//! camera adoption, readback, PNG, exit — and what is left here is genuinely
//! Sanic's: which app to build and when its world is worth photographing.
//!
//! ⚠ **Sanic is the game Jon reported a bug in that was fixed BLIND** — *"in
//! sanic the button text doesn't match what the controls really are"* (queue
//! D17, fixed 2026-08-04 without ever seeing the screen). This is how that
//! fix gets looked at.
//!
//! ⭐ **this binary is deliberately small, and that is the evidence.** Everything
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

/// Frames to advance before shooting. A demo boots its shell, loads a room and
/// spawns a body; a capture taken before that is a picture of an empty world.
#[derive(Resource)]
struct Warmup {
    remaining: u32,
    /// Frames of held RIGHT before the shot.
    ///
    /// Without it the camera only ever sees the opening screen — and Sanic's
    /// whole point is what happens once he is moving.
    walk_right: u32,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut output = PathBuf::from("sanic.png");
    let mut size = UVec2::new(960, 540);
    let mut warmup = 90u32;
    let mut include_ui = true;
    let mut walk = 0u32;

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

    // ⚠ `OffscreenGpu`, never `Headless`: headless sets `backends: None`, so
    // there is no RenderApp and no texture to read. See `RenderMode`.
    // ⚠ the GAMEPLAY route, not the default launcher home. Booting the launcher
    // and counting frames writes a blank PNG — no amount of warmup walks a menu
    // into a level, and that is the readiness contract `capture` hands back to
    // the caller. It is also literally what the first attempt produced.
    let mut app = ambition_demo_sanic_app::build_windowed_demo_app_with_home(
        ambition_demo_sanic_app::RenderMode::OffscreenGpu,
        ambition_demo_sanic::SANIC_GAMEPLAY_ROUTE,
    );
    app.insert_resource(CaptureSettings {
        output,
        size,
        include_ui,
    });
    app.init_resource::<CaptureProgress>();
    app.insert_resource(Warmup {
        remaining: warmup,
        walk_right: walk,
    });
    app.add_systems(Startup, setup_capture_target);
    app.add_systems(
        Update,
        (
            adopt_cameras_into_capture_target,
            hold_right_while_walking,
            shoot_when_warm,
            finish_after_capture,
        )
            .chain(),
    );
    app.run();
}

/// Count the world in, then ask for the picture.
///
/// ⛔ **it waits for an ADOPTED camera, not just for frames.** A demo builds its
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
    request_capture(&mut commands, &target, &mut progress);
}

/// Hold RIGHT while there are walking frames left.
///
/// ⚠ **written into `ButtonInput` directly**, which is what `capture_scene`
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
    eprintln!("usage: capture_sanic OUT.png [WIDTHxHEIGHT] [--warmup N] [--no-ui]");
    std::process::exit(2);
}
