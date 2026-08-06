//! **Photograph TwinTrack**, including the observatory panel.
//!
//! ⭐ **the THIRD caller of `ambition_render::capture`**, which is what the seam
//! was split for. Target, adoption, readback, PNG and exit are all shared; what
//! is here is TwinTrack's own: how long to run the course before shooting, and
//! the one camera the shared adoption cannot know about.
//!
//! ⛔ **the observatory camera is neither `MainCamera` nor `FrontHudCamera`**, so
//! the shared adoption leaves it pointed at a window that does not exist in
//! offscreen mode. A picture missing it would be a picture of the laboratory
//! chart alone — the half the observatory exists to contrast with — while
//! reporting success. `adopt_the_observatory` is that one extra edge.
//!
//! ```text
//! cargo run -p ambition_demo_twintrack_app --features visible \
//!     --bin capture_twintrack -- OUT.png [WIDTHxHEIGHT] [--warmup N] [--run N] [--no-ui]
//! ```

use std::path::PathBuf;

use ambition_demo_twintrack::ObservatoryCamera;
use ambition_platformer2d::render::capture::{
    adopt_cameras_into_capture_target, finish_after_capture, request_capture, CaptureAdopted,
    CaptureProgress, CaptureSettings, CaptureTarget,
};
use bevy::prelude::*;

/// How the course is driven before the shutter.
#[derive(Resource)]
struct Warmup {
    /// Frames to wait after something is drawing. A demo builds its cameras when
    /// its shell resolves a route, well after `Startup`.
    remaining: u32,
    /// Frames of held RIGHT — TwinTrack's phase machine only leaves `Ready` once
    /// the traveler has actually departed, so a capture that presses nothing
    /// photographs a laboratory at rest with an empty sky.
    run_right: u32,
    /// One frame for the last press to be collected and simulated.
    settle: u32,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut output = PathBuf::from("twintrack.png");
    let mut size = UVec2::new(1280, 720);
    let mut warmup = 60u32;
    let mut run = 240u32;
    let mut include_ui = true;

    let mut positional_seen = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmup" => {
                warmup = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--warmup needs a frame count"));
            }
            "--run" => {
                run = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--run needs a frame count"));
            }
            "--no-ui" => include_ui = false,
            other if other.starts_with("--") => fail(&format!("unknown flag '{other}'")),
            other if !positional_seen => {
                output = PathBuf::from(other);
                positional_seen = true;
            }
            other => {
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

    let mut app = ambition_demo_twintrack_app::build_windowed_demo_app_with(
        ambition_demo_twintrack_app::RenderMode::OffscreenGpu,
    );
    app.insert_resource(CaptureSettings {
        output,
        size,
        include_ui,
    });
    app.init_resource::<CaptureProgress>();
    app.insert_resource(Warmup {
        remaining: warmup,
        run_right: run,
        settle: 1,
    });
    app.add_systems(
        Startup,
        ambition_platformer2d::render::capture::setup_capture_target,
    );
    // Before `InputSet::Collect`, so a press written here is seen by the SAME
    // frame rather than by whichever one Bevy happened to order next.
    app.add_systems(
        Update,
        (
            adopt_cameras_into_capture_target,
            adopt_the_observatory,
            hold_right_while_running,
            shoot_when_warm,
            finish_after_capture,
        )
            .chain()
            .before(ambition_platformer2d::input::InputSet::Collect),
    );
    app.run();
}

/// Point the observatory camera at the capture texture too.
///
/// ⚠ it does NOT count itself in `CaptureTarget::adopted`: that counter is the
/// readiness signal for "the world is drawing", and the observatory rig appears
/// a frame or two later than the main camera. Counting it would be harmless; not
/// counting it keeps the meaning of the number one thing.
fn adopt_the_observatory(
    mut commands: Commands,
    target: Option<Res<CaptureTarget>>,
    mut cameras: Query<(Entity, &mut Camera), (With<ObservatoryCamera>, Without<CaptureAdopted>)>,
) {
    let Some(target) = target else {
        return;
    };
    let render_target = bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget::from(
        target.image.clone(),
    ));
    for (entity, mut camera) in &mut cameras {
        camera.is_active = true;
        commands.entity(entity).insert((
            render_target.clone(),
            bevy::render::view::Msaa::Off,
            CaptureAdopted,
        ));
    }
}

/// Count the course in, then ask for the picture.
///
/// ⛔ **waits for an ADOPTED camera, not just for frames.** Shooting before a
/// camera is drawing reads back a texture of `(0,0,0,0)` and reports success.
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
    if warmup.run_right > 0 {
        return;
    }
    if warmup.settle > 0 {
        warmup.settle -= 1;
        return;
    }
    request_capture(&mut commands, &target, &mut progress);
}

/// Hold RIGHT while there are running frames left, and fire the transmitter.
///
/// ⚠ **written into `ButtonInput` directly** — the demo's binding layer reads the
/// same resource a real keyboard fills, so this exercises the real input path.
/// The interact press is periodic rather than aimed: the transmitter's own
/// proper-time cooldown decides which attempts become packets, and a capture is
/// not the place to reproduce the acceptance test's exact script.
fn hold_right_while_running(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    target: Option<Res<CaptureTarget>>,
    mut warmup: ResMut<Warmup>,
) {
    if target.is_none_or(|target| target.adopted == 0) {
        return;
    }
    if warmup.run_right == 0 {
        keys.release(KeyCode::ArrowRight);
        keys.release(KeyCode::KeyF);
        return;
    }
    warmup.run_right -= 1;
    keys.press(KeyCode::ArrowRight);
    if warmup.run_right % 40 < 3 {
        keys.press(KeyCode::KeyF);
    } else {
        keys.release(KeyCode::KeyF);
    }
}

fn fail(message: &str) -> ! {
    eprintln!("capture_twintrack: {message}");
    eprintln!("usage: capture_twintrack OUT.png [WIDTHxHEIGHT] [--warmup N] [--run N] [--no-ui]");
    std::process::exit(2);
}
