//! **Photograph Mary-O.**
//!
//! ⛔ **before this, four of the five games could not be looked at at all.** The
//! only capture tool here is `capture_scene`, which composes `ambition_app` —
//! Ambition's rooms, Ambition's content. So every visual claim about Mary-O,
//! Sanic, Smash or Pocket was argued rather than seen, and on 2026-08-04 two
//! changes to Mary-O's own art landed unlooked-at for exactly that reason
//! (queue D20).
//!
//! ⭐ **this binary is deliberately small, and that is the evidence.** Everything
//! hard — building a render target, pointing the cameras at it, reading the
//! texture back, writing the PNG, exiting — is `ambition_render::capture`, shared
//! and game-agnostic. What is left here is the part that genuinely differs
//! between games: which app to build, and when its world is worth photographing.
//! If a second demo needs more than this file's length, the split is in the
//! wrong place.
//!
//! ⛔ **and until 2026-08-09 it could only photograph 1-1.** It boots the gameplay
//! route and the gameplay route entered one room, so `--at` could only teleport
//! WITHIN that room and `capture_scene` — which composes `ambition_app`, holding
//! 72 Ambition rooms and zero Mary-O ones — could not reach the demo at all. The
//! union of both tools could not open World 1-2, which is the level three open
//! observations were about (queue D65). `--room` is the whole difference.
//!
//! ```text
//! cargo run -p ambition_demo_mary_o_app --features capture --bin capture_mary_o \
//!     -- OUT.png [WIDTHxHEIGHT] [--room ID] [--warmup N] [--walk N] [--at X,Y] [--no-ui]
//! ```

use std::path::PathBuf;

use ambition_platformer2d::engine_core as ae;
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
    /// ⛔ **without this the camera only ever saw the level's first screen.** A
    /// pipe recolour landed unverified on 2026-08-04 for exactly that reason:
    /// no pipe is in frame at the spawn point, and a capture that cannot travel
    /// can only photograph the opening (queue D22). Holding a direction is the
    /// cheapest way to reach the rest of a side-scroller.
    walk_right: u32,
    /// Frames to let the LAST walking input actually be simulated.
    ///
    /// ⛔ **the walk counter was short by its final frame.** `hold_right_while_walking`
    /// spends the last step and presses RIGHT, and `shoot_when_warm` — chained
    /// immediately after it — requested the picture in that same `Update`. The
    /// press is collected and simulated on a LATER frame than the one that asked
    /// for the shot, so `--walk 1` photographed zero frames of walking and every
    /// other count was one short (GPT 5.6, review through `f0f97f5`).
    ///
    /// One frame is enough: the input written this frame is collected in the
    /// same frame's `InputSet::Collect` (which the writer is now ordered before)
    /// and moves the body on the next simulation step.
    settle: u32,
    /// Where to PUT her before the shutter, in world coordinates.
    ///
    /// ⭐ **because `--walk` cannot reliably reach a place.** The level's bricks
    /// are at x≥1536 and every capture ever taken of Mary-O stopped short of
    /// them, so nobody had actually looked at one — two separate claims about
    /// what a brick looks like were made from screenshots that did not contain a
    /// brick. Walking there means surviving the route; `capture_scene` takes an
    /// X,Y for exactly this reason and Mary-O's own capture binary did not.
    at: Option<ae::Vec2>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut output = PathBuf::from("mary_o.png");
    let mut size = UVec2::new(960, 540);
    let mut warmup = 90u32;
    let mut include_ui = true;
    let mut walk = 0u32;
    let mut at: Option<ae::Vec2> = None;
    let mut room = ambition_demo_mary_o::LEVEL_1_1_ROOM_ID.to_string();

    let mut positional_seen = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--room" => {
                let asked = args
                    .next()
                    .unwrap_or_else(|| fail("--room needs a room id"));
                // ⛔ **validated here, because the seam it feeds does NOT refuse.**
                // `RoomSet::from_parts` activates room 0 for an id it does not
                // hold, so an unknown `--room` would photograph 1-1 and report
                // success — a capture tool that silently shoots the wrong subject
                // is worse than one that cannot shoot it at all.
                if !ambition_demo_mary_o::provider::MARY_O_ROOM_IDS.contains(&asked.as_str()) {
                    fail(&format!(
                        "unknown room '{asked}'; Mary-O has {:?}",
                        ambition_demo_mary_o::provider::MARY_O_ROOM_IDS
                    ));
                }
                room = asked;
            }
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
            "--at" => {
                let raw = args
                    .next()
                    .unwrap_or_else(|| fail("--at needs X,Y in world coordinates"));
                let Some((x, y)) = raw.split_once(',') else {
                    fail(&format!("expected --at X,Y, got '{raw}'"));
                };
                let (Ok(x), Ok(y)) = (x.trim().parse(), y.trim().parse()) else {
                    fail(&format!("expected --at X,Y, got '{raw}'"));
                };
                at = Some(ae::Vec2::new(x, y));
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
    let mut app = ambition_demo_mary_o_app::build_windowed_demo_app_entering(
        ambition_demo_mary_o_app::RenderMode::OffscreenGpu,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        &room,
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
        settle: 1,
        at,
    });
    app.add_systems(Startup, setup_capture_target);
    // ⚠ **the placement runs in the SIM schedule, not beside the other capture
    // systems.** It calls `transit_body`, which writes `MotionModel` — rollback
    // state — and `scripts/check_rollback_mutators_run_in_sim.py` catches
    // rollback state mutated outside the rewinding schedule. It caught this one.
    //
    // A waiver would have been the easy answer ("a capture binary has no peer to
    // desync from"), and it would have been the kind of waiver that is true
    // today and quietly wrong later. Relocating a body IS a simulation event, so
    // it goes where simulation events go, and the contacts it invalidates are
    // re-acquired by the very next sim tick.
    {
        use ambition_platformer2d::platformer::schedule::SimScheduleExt;
        let sim = app.sim_schedule();
        app.add_systems(sim, place_before_the_shutter);
    }
    // ⛔ **the synthetic input must be written BEFORE the frame collects it.**
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
    // ⛔ **AND the walk has to finish.** This shot as soon as the warmup ran
    // out, whatever `--walk` said — so `--warmup 10 --walk 300` took the picture
    // on frame 10 with 290 frames of travel still owed, and the capture showed
    // the spawn point. The flag asked for a journey and the tool photographed
    // the departure lounge. `hold_right_while_walking` is chained BEFORE this,
    // so the frame that spends the last walking step is the frame that shoots.
    if warmup.walk_right > 0 {
        return;
    }
    // The last walking frame's press was written THIS frame at the earliest; let
    // it be collected and simulated before the shutter. See `Warmup::settle`.
    if warmup.settle > 0 {
        warmup.settle -= 1;
        return;
    }
    request_capture(&mut commands, &target, &mut progress);
}

/// Hold RIGHT while there are walking frames left.
///
/// ⚠ **written into `ButtonInput` directly**, which is what `capture_scene`
/// does: the demo's binding layer reads the same resource a real keyboard fills,
/// so this exercises the actual input path rather than a bypass.
/// Put her where `--at` says, once, as soon as something is drawing.
///
/// ⚠ **through the movement authority**, not a bare pose write: a discrete
/// relocation that leaves the body's contacts and collapsed sweep describing the
/// spawn point is the defect ADR 0024's authorities exist to prevent, and a
/// capture tool that corrupts the thing it photographs is worse than no tool.
fn place_before_the_shutter(
    mut warmup: ResMut<Warmup>,
    target: Option<Res<CaptureTarget>>,
    mut bodies: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actors::features::MotionModel,
        ),
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >,
) {
    if target.is_none_or(|target| target.adopted == 0) {
        return;
    }
    let Some(pos) = warmup.at else {
        return;
    };
    let Ok((mut item, mut model)) = bodies.single_mut() else {
        return;
    };
    let mut clusters = item.as_clusters_mut();
    ae::movement::transit_body(
        &mut model,
        &mut clusters,
        pos,
        ae::movement::TransitVelocity::Zero,
    );
    warmup.at = None;
}

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
    // ⛔ **walking alone cannot cross a side-scroller.** The first version held
    // only Right and she died to the first snake — the capture came back at the
    // spawn point with a life spent, which looks exactly like the flag not
    // working. A traversal that cannot jump reaches the first hazard and stops.
    //
    // A periodic hop is not a solution to the level, it is enough to keep moving
    // past the early enemies; a capture that needs a specific route wants a real
    // `--press` sequence like `capture_scene`'s.
    let phase = warmup.walk_right % 45;
    if phase < 12 {
        keys.press(KeyCode::KeyZ);
    } else {
        keys.release(KeyCode::KeyZ);
    }
}

fn fail(message: &str) -> ! {
    eprintln!("capture_mary_o: {message}");
    eprintln!(
        "usage: capture_mary_o OUT.png [WIDTHxHEIGHT] [--room ID] [--warmup N] \
         [--walk N] [--at X,Y] [--no-ui]"
    );
    std::process::exit(2);
}
