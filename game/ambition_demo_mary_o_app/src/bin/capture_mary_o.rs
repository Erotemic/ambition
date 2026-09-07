//! Photograph Mary-O.
//!
//! this binary is deliberately small, and that is the evidence. Everything
//! hard — building a render target, pointing the cameras at it, reading the
//! texture back, writing the PNG, exiting — is `ambition_render::capture`, shared
//! and game-agnostic. What is left here is the part that genuinely differs
//! between games: which app to build, and when its world is worth photographing.
//! If a second demo needs more than this file's length, the split is in the
//! wrong place.
//!
//! The union of both tools could not open World 1-2, which is the level three open observations
//! were about. `--room` is the whole difference.
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

/// Frames to advance before shooting.
#[derive(Resource)]
struct Warmup {
    remaining: u32,
    /// Frames of held RIGHT before the shot.
    ///
    /// Holding a direction is the cheapest way to reach the rest of a side-scroller.
    walk_right: u32,
    /// Frames to let the LAST walking input actually be simulated.
    ///
    /// One frame is enough: the input written this frame is collected in the
    /// same frame's `InputSet::Collect` (which the writer is now ordered before)
    /// and moves the body on the next simulation step.
    settle: u32,
    /// Where to PUT her before the shutter, in world coordinates.
    ///
    /// because `--walk` cannot reliably reach a place. The level's bricks
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
                // validated here, because the seam it feeds does NOT refuse.
                // `RoomSet::from_parts` activates room 0 for an id it does not
                // hold, so an unknown `--room` would photograph 1-1 and report
                // success — a capture tool that silently shoots the wrong subject
                // is worse than one that cannot shoot it at all.
                let known = ambition_demo_mary_o::provider::mary_o_room_ids();
                if !known.iter().any(|id| id == &asked) {
                    fail(&format!("unknown room '{asked}'; Mary-O has {known:?}"));
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

    // `OffscreenGpu`, never `Headless`: headless sets `backends: None`, so
    // there is no RenderApp and no texture to read. See `RenderMode`.
    // the GAMEPLAY route, not the default launcher home. Booting the launcher
    // and counting frames writes a blank PNG — no amount of warmup walks a menu
    // into a level, and that is the readiness contract `capture` hands back to
    // the caller. It is also literally what the first attempt produced.
    let mut app = ambition_demo_mary_o_app::build_windowed_demo_app_entering(
        ambition_demo_mary_o_app::RenderMode::OffscreenGpu,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        &room,
    );
    // ⛔⛔ THE RUNNER, AND THIS BINARY IS WHY IT EXISTS. `Display::Offscreen`
    // disables `winit`, which is also Bevy's app RUNNER — so without this
    // `run()` below performs exactly ONE update and returns, the process exits
    // 0, and NO FILE IS WRITTEN. A capture that reports success having drawn
    // nothing is the worst failure this tool has, and it was found by trying it.
    //
    // It lives here rather than in the builder because the ENGINE's offscreen
    // face is deliberately caller-stepped: every other offscreen consumer drives
    // `update()` itself and must not inherit a run loop. ⇒ the one consumer that
    // calls `run()` asks for the runner, and says why.
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
        at,
    });
    app.add_systems(Startup, setup_capture_target);
    // ⛔⛔ IN `PostUpdate`, AFTER TRANSFORM PROPAGATION, and that placement is the
    // whole point. As an unordered `Update` system this read `GlobalTransform`
    // while the presentation writers (`sync_visuals`, `animate_player`) were still
    // running in the same stage — so a row could pair THIS frame's sprite with LAST
    // frame's transform and call the mixture "the drawn placement". A GPT review
    // caught it, and it is the same class of error the reporter exists to prevent.
    app.add_systems(
        PostUpdate,
        report_body_against_sprite.after(bevy::transform::TransformSystems::Propagate),
    );
    // the placement runs in the SIM schedule, not beside the other capture
    // systems. It calls `transit_body`, which writes `MotionModel` — rollback
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
    request_capture(&mut commands, &target, &mut progress);
}

/// Hold RIGHT while there are walking frames left.
///
/// written into `ButtonInput` directly, which is what `capture_scene`
/// does: the demo's binding layer reads the same resource a real keyboard fills,
/// so this exercises the actual input path rather than a bypass.
/// Put her where `--at` says, once, as soon as something is drawing.
///
/// through the movement authority, not a bare pose write: a discrete
/// ⭐⭐ THE ONE FRAME WHERE BOTH HALVES ARE PRINTED TOGETHER.
///
/// Chasing Mary-O's floating sprite produced five refuted hypotheses in an
/// afternoon, and every one died the same way: a number read without knowing which
/// BRANCH produced it. Every instrument in this area reports ONE side — the picture
/// shows a disagreement it cannot quantify, and the components quantify one half
/// each and never meet.
///
/// ⇒ This prints the BODY (world position, collision size, and the bottom edge its
/// feet stand on) beside the DRAWN sprite (final `transform.translation` and the
/// quad it was given), on the shutter frame. A placement defect is then a
/// subtraction rather than an argument.
///
/// ⚠ It reports rather than asserts, deliberately: what "aligned" means depends on
/// the sheet's own feet anchor, and inventing a tolerance here would be a second
/// authority for a fact the manifest already holds.
fn report_body_against_sprite(
    target: Option<Res<CaptureTarget>>,
    warmup: Res<Warmup>,
    // ⭐ EVERY BODY, not just the player. The review asks for representative-sheet
    // A/Bs before the placement seam is repaired, and the demo course already
    // stages several sheet-authored characters in one frame -- the snake, the AI
    // slop, the player. One capture can price them all if the reporter stops
    // filtering to `PrimaryPlayerOnly`.
    //
    // ⚠ WHAT THIS STILL CANNOT DO: PAIR a body with its drawable. They are separate
    // entities and only some carry `PresentationOf`, so the two lists below are
    // printed side by side and matched by eye or by position. That is enough to
    // price ONE character you can identify (the player, by `player_visual=true`)
    // and not enough for an automatic per-sheet misalignment table. ⇒ The pairing
    // is the next thing this tool wants, and it wants a real link rather than a
    // proximity heuristic.
    bodies: Query<(
        bevy::prelude::Entity,
        &ae::BodyKinematics,
        Option<&ambition_platformer2d::combat::components::FeatureId>,
    )>,
    drawn: Query<(
        Entity,
        &GlobalTransform,
        &Sprite,
        Has<ambition_platformer2d::platformer::lifecycle::PlayerVisual>,
        // ⭐ THE JOIN KEY. The drawable is a DIFFERENT entity from the body -- measured,
        // ids 684..695 against 824..946, disjoint -- and only some drawables carry
        // `PresentationOf` (the effect overlays do; the actor sprite does not). What
        // the actor sprite DOES carry is the feature id the renderer bound it from,
        // and the body carries the same id as `FeatureId`. That is a REAL link rather
        // than a position heuristic, which is the difference between a table that is
        // right and one that is usually right.
        Option<&ambition_platformer2d::render::rendering::FeatureVisual>,
        Option<&ambition_platformer2d::platformer::lifecycle::PresentationOf>,
        // The ANCHOR, because size and anchor arrive together from the trimmed
        // basis and only one of them can be checked against the box by eye.
        Option<&bevy::sprite::Anchor>,
    )>,
    // ⭐ WHICH BRANCH `sync_visuals` TOOK. `authored_render` and `authored_offset`
    // are both gated on one `sheet_authored_body` flag, so they are Some together
    // or None together -- and the branch that applies the offset is the SAME branch
    // that sets the size from `authored_render`. Printing both is the only way to
    // know whether a placement came from the sheet or from the feet-anchor
    // fallback, which is a distinction four of my hypotheses turned on.
    poses: Query<
        &ambition_platformer2d::sim_view::BodyPoseView,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    // Membership test for "is this body the player", so the marker pairing above can
    // ask it of a body it already has rather than running a second positional query.
    player_body: Query<(), ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>,
    mut reported: Local<bool>,
) {
    // ⛔⛔ THE SHUTTER'S OWN CONDITION, NOT AN APPROXIMATION OF IT. This gated on
    // `warmup.remaining` alone, while `shoot_when_warm` ALSO waits for the walk and
    // the settle — so `--warmup 10 --walk 300` printed ~290 rows of travel and
    // labelled each one the drawn placement. A GPT review caught it: the comment
    // said "the frame the shutter fires" and the code said "some frame after the
    // warmup".
    //
    // ⇒ Mirroring the three conditions would be a SECOND copy of the shutter rule,
    // which is how the two come to disagree again. `CaptureProgress::requested` is
    // what `request_capture` sets, so asking IT is asking the shutter.
    if target.is_none_or(|target| target.adopted == 0) {
        return;
    }
    if warmup.remaining > 0 || warmup.walk_right > 0 || warmup.settle > 0 {
        return;
    }
    // ⚠ ONCE. The gate above stays true for the handful of frames the readback
    // takes, so without this it prints five near-identical rows and a reader has to
    // decide which one was the photograph. Measured: 5 rows before this line, and
    // ~120 before the walk/settle conditions were added.
    if std::mem::replace(&mut *reported, true) {
        return;
    }
    let mut rows: Vec<(
        bevy::prelude::Entity,
        ae::Vec2,
        ae::Vec2,
        Option<String>,
        ae::Vec2,
    )> = bodies
        .iter()
        .map(|(e, kin, id)| (e, kin.pos, kin.size, id.map(|i| i.0.clone()), kin.vel))
        .filter(|(_, _, size, _, _)| size.x > 1.0 && size.y > 1.0)
        // ⛔⛔ A MOVING BODY BREAKS THE INVARIANT THIS TABLE IS BUILT ON, and mixing
        // moving bodies in is what made the first version of this table irreproducible.
        // MEASURED: two snakes with the IDENTICAL quad (73.87x48.0) read space_sums
        // 6.8 apart, and re-running the same binary twice gave different actor rows
        // while the player's row stayed put to three decimals. The sprite samples the
        // body a frame away from where this reporter reads it, so anything with
        // velocity contributes its own travel to the sum. ⇒ A body at rest is the only
        // one whose sum is a statement about PLACEMENT rather than about timing.
        .filter(|(_, _, _, _, vel)| vel.x.abs() < 0.01 && vel.y.abs() < 0.01)
        .collect();
    rows.sort_by_key(|(e, _, _, _, _)| e.index());
    for (entity, pos, size, id, _) in &rows {
        eprintln!(
            "[align] body {entity:?} id={} pos=({:.2},{:.2}) size=({:.2},{:.2}) feet_y={:.2}",
            id.as_deref().unwrap_or("-"),
            pos.x,
            pos.y,
            size.x,
            size.y,
            pos.y + size.y * 0.5,
        );
    }
    // ⭐ THE JOINED TABLE, which is the thing the review actually asked for: for each
    // body that has a drawable bound to its feature id, the body's box and the quad
    // drawn for it, side by side, in ONE frame. A per-sheet misalignment row is
    // `sprite_feet - body_feet`; a sheet whose placement is correct reads ~0.
    for (entity, pos, size, id, _) in &rows {
        let Some(id) = id.as_deref() else { continue };
        let Some((de, dt, ds, da)) = drawn
            .iter()
            .find(|(_, _, _, _, v, _, _)| v.is_some_and(|v| v.id == id))
            .map(|(e, t, s, _, _, _, a)| (e, t, s, a))
        else {
            continue;
        };
        let t = dt.translation();
        let body_feet = pos.y + size.y * 0.5;
        // The quad's feet depend on the anchor: `Anchor` is a fraction of the quad
        // measured from its CENTRE, y up, so the bottom edge sits at
        // `translation.y - (0.5 + anchor.y) * height`.
        let (qh, ay) = (
            ds.custom_size.map(|q| q.y).unwrap_or(0.0),
            da.map(|a| a.0.y).unwrap_or(0.0),
        );
        let sprite_feet = t.y - (0.5 + ay) * qh;
        // ⛔⛔ `sprite_feet - body_feet` IS NOT THE MISALIGNMENT, and reading it as one
        // would have made every actor in this scene look ~448px wrong. MEASURED: the
        // two numbers live in DIFFERENT SPACES -- the body's y runs DOWN in sim pixels,
        // the sprite's runs UP in Bevy world units -- so the two are related by a
        // y-flip about some origin, and a flip makes the raw difference track the
        // body's POSITION rather than any error.
        //
        // ⭐ WHAT A FLIP LEAVES INVARIANT IS THE SUM. `body_feet + sprite_feet` is a
        // CONSTANT for every correctly-placed body, whatever its position -- measured
        // 383.14 across five snakes standing at five different heights. So the honest
        // instrument is the sum, and the SPREAD of the sum across bodies is the
        // misalignment: two sheets whose sums differ by N are drawn N pixels apart
        // relative to their own feet. An absolute verdict needs the origin; a
        // per-sheet COMPARISON, which is what the review asked for, does not.
        eprintln!(
            "[align] JOIN id={id} body={entity:?} draw={de:?} space_sum={:.3} body_feet={body_feet:.2} sprite_feet={sprite_feet:.2} quad={:?} anchor_y={ay:.4}",
            body_feet + sprite_feet,
            ds.custom_size,
        );
    }
    for pose in &poses {
        eprintln!(
            "[align]   pose authored_render={:?} authored_offset={:?}",
            pose.authored_render, pose.authored_offset,
        );
    }
    // ⭐ THE PLAYER IS NOT IN THE JOIN ABOVE, and that is the one character this whole
    // investigation is about. Its sprite is bound by the `PlayerVisual` path rather
    // than from a feature id, so it carries no `FeatureVisual` and the id join cannot
    // see it. ⇒ Pair it by its MARKERS instead, which is a real link too: exactly one
    // body is `PrimaryPlayerOnly` and exactly one drawable is `PlayerVisual`.
    if let (Some((pe, ppos, psize, _, _)), Some((de, dt, ds, da))) = (
        rows.iter()
            .find(|(e, _, _, _, _)| player_body.get(*e).is_ok())
            .cloned(),
        drawn
            .iter()
            .find(|(_, _, _, is_player, _, _, _)| *is_player)
            .map(|(e, t, s, _, _, _, a)| (e, t, s, a)),
    ) {
        let body_feet = ppos.y + psize.y * 0.5;
        let (qh, ay) = (
            ds.custom_size.map(|q| q.y).unwrap_or(0.0),
            da.map(|a| a.0.y).unwrap_or(0.0),
        );
        let sprite_feet = dt.translation().y - (0.5 + ay) * qh;
        eprintln!(
            "[align] JOIN id=<PLAYER> body={pe:?} draw={de:?} space_sum={:.3} body_feet={body_feet:.2} sprite_feet={sprite_feet:.2} quad={:?} anchor_y={ay:.4}",
            body_feet + sprite_feet,
            ds.custom_size,
        );
    }
    // EVERY drawable, with its markers — because "the player's sprite" turned out
    // to name more than one entity, and a diagnostic that reports `single()` hides
    // exactly that.
    for (entity, transform, sprite, visual, _feature, presented, anchor) in &drawn {
        let t = transform.translation();
        eprintln!(
            "[align]   {entity:?} xy=({:.2},{:.2}) quad={:?} anchor={:?} player_visual={} presentation_of={:?}",
            t.x,
            t.y,
            sprite.custom_size,
            anchor.map(|a| a.0),
            visual,
            presented.map(|p| p.0),
        );
    }
}

/// relocation that leaves the body's contacts and collapsed sweep describing the
/// spawn point is the defect ADR 0024's authorities exist to prevent, and a
/// capture tool that corrupts the thing it photographs is worse than no tool.
fn place_before_the_shutter(
    mut warmup: ResMut<Warmup>,
    target: Option<Res<CaptureTarget>>,
    mut bodies: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        ),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
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
    // Capture traversal needs jumping as well as rightward movement to clear
    // ordinary side-scroller hazards.
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
