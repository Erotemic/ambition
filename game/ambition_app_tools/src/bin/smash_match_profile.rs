//! Profile an ACTUAL Smash match — windowless on a machine with no GPU, or in a
//! REAL WINDOW on a machine that has one.
//!
//! ⛔⛔ THIS EXISTS BECAUSE NOTHING ELSE PROFILES A MATCH. Measured 2026-08-29:
//! `run_game.sh smash` builds the standalone demo, passes `--window` AND
//! `--headless`, and opens on CHARACTER SELECT — so it profiles a menu. And
//! every headless room measures a two-body world: sweeping `--start-room` over
//! `goblin_encounter`, `central_hub_complex` and the default all report
//! `entities=64, bodies=1-2`. Every frame baseline taken before this therefore
//! describes the engine's FIXED OVERHEAD, not gameplay.
//!
//! ⭐ The road is the one `app_it` already proves works — build the visible app,
//! install a roster, and route to the gameplay screen. That is the SHIPPED
//! composition (rollback host and all), not the demo shell, so what it measures
//! is what Jon plays. The two render modes differ in ONE argument and in who
//! drives the loop; everything about reaching a live round is shared.
//!
//! ```bash
//! # No GPU: step the match by hand, as fast as the machine will go.
//! AMBITION_PROFILE_CENSUS=1 AMBITION_PROFILE_CENSUS_HZ=20 \
//!   cargo run -p ambition_app_tools --bin smash_match_profile -- --ticks 3000
//!
//! # A GPU desktop: a real window, winit's loop, hardware rendering.
//! AMBITION_PROFILE_CENSUS=1 \
//!   cargo run -p ambition_app_tools --bin smash_match_profile -- --window
//! ```
//!
//! ⚠ Census rows are sampled on WALL time. A windowless match runs far faster
//! than real time, so leave `AMBITION_PROFILE_CENSUS_HZ` high enough that the run
//! outlives the first interval — otherwise the only row you get is startup, and
//! a `frames=1` row reporting `Update=127ms` is PLUGIN BUILD, not a frame. A
//! WINDOWED run is paced by the display, so its default 1 Hz is right.
//!
//! ⛔ THE TWO MODES ARE NOT COMPARABLE AND MUST NEVER BE SUBTRACTED. `NoWindow`
//! selects `backends: None`: no adapter, no render app, no drawing at all. The
//! bundle's `gpu.rendering` field carries `headless` vs `hardware` into the
//! history's comparability key for exactly this reason.

use bevy::prelude::*;

use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
use ambition_platformer2d::characters::control::{ScriptedControl, SlotControls};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// Frames to let the shell settle before the roster lands. A roster inserted
/// into an unbuilt shell is dropped; the integration tests wait the same 30.
const SETTLE_FRAMES: u32 = 30;

/// Frames to wait for the opening ceremony to release the cast before giving
/// up. Ten seconds at 60 Hz — the ceremony is ~3s and dev mode runs it 10x
/// fast, so this bounds a HANG, it does not encode the ceremony's length.
const LIVE_DEADLINE_FRAMES: u32 = 600;

/// How often the windowed run re-checks that a match is still happening.
///
/// ⚠ NOT EVERY FRAME, AND THE INSTRUMENT IS THE REASON. `World::query` builds a
/// fresh `QueryState` on each call, which walks the archetype set — and a live
/// match takes this app past two thousand entities. Twice a frame, forever, in
/// the harness's OWN system, is the profiler perturbing the thing it measures.
/// Twice every two seconds is not.
const PREMISE_CHECK_EVERY: u32 = 120;

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn arg_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// The cast, as the round's own state: how many seats exist, and how many are
/// still held by the opening ceremony's scripted control.
///
/// ⛔⛔ WAIT FOR THE ROUND, NOT FOR A NUMBER. A fixed frame count silently
/// encodes the ceremony's LENGTH, and dev mode runs that ceremony 10x fast —
/// the same mistake has broken four fixture families. The condition is
/// observable: a cast exists, and nothing in it is still held.
/// Spawn `count` plain sprites into the live match, spread over the stage.
///
/// ⭐⭐ THE SCALING KNOB THE CAMPAIGN LACKED. Every measurement so far has been a
/// two-body match with about forty sprites, and the question the whole campaign
/// was opened about — *a room with hundreds of sprites can visibly chug* — could
/// not be ASKED at that size. This varies ONE dimension on the REAL stack: same
/// app, same schedules, same render path, more sprites.
///
/// ⛔ THEY ARE DELIBERATELY PLAIN. No gameplay components, no bodies, no
/// collision — a `Sprite` with a `Transform` and a `Visibility`, which is what
/// Bevy's extraction and batching see. Adding gameplay would measure the
/// gameplay instead and confound the dimension being varied.
///
/// ⚠ ONE SHARED COLOUR AND NO TEXTURE, so this measures the per-sprite path and
/// NOT batch breaking. Varying texture/material to find batch breaks is a
/// separate curve and needs its own knob — say so rather than letting a reader
/// assume this one covers it.
fn spawn_scaling_sprites(world: &mut World, count: usize) {
    // ⛔⛔ ANCHOR ON A SPRITE THE CAMERA ALREADY DRAWS. The first version placed
    // a grid around the world origin and every single one was CULLED: the census
    // reported `sprites=1025` beside `sprites_visible=6`, so the curve measured
    // a thousand invisible sprites and came out flat. A scaling curve whose
    // population is culled is worse than no curve, and only having
    // `sprites_visible` in the same census row caught it.
    // ⛔⛔ ANCHOR ON A FIGHTER, NOT ON "ANY VISIBLE SPRITE". The previous version
    // took the first entity with `ViewVisibility` set and still culled all
    // thousand: the camera census reports `Main Camera layers=0+2+5` beside a
    // `Front HUD Camera layers=1`, so "visible" matched a HUD sprite in SCREEN
    // space and the grid landed nowhere the world camera looks. A `MatchSeat`
    // body is world-space by definition and is what the match camera frames.
    // ⛔⛔ THE FIGHTER'S POSITION IS IN `BodyKinematics`, NOT IN A `GlobalTransform`.
    // Querying `(&MatchSeat, &GlobalTransform)` matched NOTHING and tripped the
    // abort below even though the round was live — because this engine splits
    // simulation from presentation: the sim body carries `BodyKinematics`, and
    // the `Transform` lives on a separate presentation entity projected from it.
    // Anchoring on the sim position is anchoring on where the fighter actually
    // IS, which is what the match camera follows.
    let anchor = {
        let mut seated = world.query::<(&MatchSeat, &BodyKinematics)>();
        seated.iter(world).map(|(_, kin)| kin.pos).next()
    };
    let Some(anchor) = anchor else {
        eprintln!(
            "[smash-profile] ABORT: no seated fighter to anchor on, so the scaling \
             population would be culled and the curve would be meaningless"
        );
        std::process::exit(3);
    };

    // A tight deterministic grid around that anchor: same placement every run at
    // the same count, so a comparison is not a lottery over what the camera
    // happens to frame.
    let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
    let pitch = 6.0;
    let half = (columns as f32 * pitch) * 0.5;
    for index in 0..count {
        let column = (index % columns) as f32;
        let row = (index / columns) as f32;
        world.spawn((
            Sprite {
                color: Color::srgb(0.6, 0.6, 0.9),
                custom_size: Some(Vec2::splat(8.0)),
                ..Default::default()
            },
            Transform::from_xyz(
                anchor.x + column * pitch - half,
                anchor.y + row * pitch - half,
                0.0,
            ),
            Visibility::Visible,
        ));
    }
    eprintln!(
        "[smash-profile] scaling_sprites_spawned={count} columns={columns} anchor={anchor:?}"
    );
}

/// Say — loudly — when a scaling population was spawned and then CULLED.
///
/// ⛔⛔ THE GUARD THAT WAS MISSING. `spawn_scaling_sprites` checks it has an
/// anchor; it cannot check that the sprites became VISIBLE, because visibility
/// is computed later in the frame. Four placement attempts each spawned a
/// thousand sprites and each measured a flat curve, and only
/// `[census] draws sprites_visible` — a number in a DIFFERENT row — revealed
/// that none of them was ever drawn.
///
/// ⚠ AND THE ANSWER TURNED OUT TO BE ARCHITECTURAL: a raw
/// `Sprite + Transform + Visibility` is never drawn in this composition at all.
/// Presentation is PROJECTED PER VIEW (`[census] draws per_view_projections`),
/// so a sprite outside that projection is invisible wherever it is put. A
/// synthetic sprite benchmark here has to go THROUGH the projection. This check
/// exists so the next person learns that in one run instead of four.
fn warn_if_scaling_sprites_were_culled(world: &mut World, spawned: usize) {
    if spawned == 0 {
        return;
    }
    let visible = {
        let mut q = world.query::<&ViewVisibility>();
        q.iter(world).filter(|v| v.get()).count()
    };
    if visible < spawned / 2 {
        eprintln!(
            "[smash-profile] ⛔ SCALING POPULATION CULLED: spawned {spawned} sprites and only \
             {visible} entities are visible. This run measures INVISIBLE sprites and its curve \
             is meaningless. A raw Sprite+Transform+Visibility is not drawable in this \
             composition — presentation is projected per view."
        );
    }
}

fn cast_state(world: &mut World) -> (usize, usize) {
    let seated = world.query::<&MatchSeat>().iter(world).count();
    let held = world
        .query_filtered::<&MatchSeat, With<ScriptedControl>>()
        .iter(world)
        .count();
    (seated, held)
}

/// Install the roster and ask the shell for the gameplay route. Both in one
/// tick, in this order: the route activation reads the roster.
fn seat_the_match(world: &mut World, fighters: usize) {
    world.insert_resource(ambition_demo_smash::smash_roster(vec!["actor"; fighters]));
    world.write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
}

/// ⛔ THE PREMISE, CHECKED RATHER THAN ASSUMED. A profile of a match that
/// quietly ended is a profile of a results screen, and it looks exactly like a
/// cheap frame.
fn report_end_of_run(world: &mut World, seats_at_start: usize) {
    let (seats, _) = cast_state(world);
    // ⛔⛔ A KO DURING THE MEASURED WINDOW CHANGES THE POPULATION UNDER THE
    // MEASUREMENT. The frame mean then averages two different matches, and two
    // arms that each lost a different number of fighters are not comparable at
    // all. `seats == 0` is only the extreme case of this.
    if seats != seats_at_start && seats > 0 {
        eprintln!(
            "[smash-profile] ⚠ POPULATION CHANGED DURING MEASUREMENT: {seats_at_start} seats at \
             the start, {seats} at the end. The rows above average a match whose cast changed; \
             do not compare this arm against one that kept its cast."
        );
    }
    if seats == 0 {
        eprintln!(
            "[smash-profile] WARNING: no seats remain — the match ended during the measured \
             window, so the census rows above mix a match with whatever followed it"
        );
    }
    eprintln!("[smash-profile] done seats_at_end={seats}");
}

fn main() {
    // ⭐ THE STARTUP ANCHOR, FIRST LINE. `StartupProfiler` measures from here;
    // without it the report begins mid-plugin-build and says so
    // ("app construction NOT MEASURED"), which is what this binary printed
    // until now. Startup is the number a phone player feels, and plugin build
    // scales with registered systems — so this is what prices a composition
    // change that the FRAME cannot see.
    ambition_platformer2d::dev_tools::profiling::note_process_start();
    let args: Vec<String> = std::env::args().collect();
    let ticks: u32 = arg_value(&args, "--ticks")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    // Four is the cap: `SlotControls::MAX_SLOTS` is 4, so a roster longer than
    // that is not a scaling axis — it is a silently clamped one.
    let fighters: usize = arg_value(&args, "--fighters")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
        .clamp(2, SlotControls::MAX_SLOTS);
    // Wall seconds of LIVE match to measure before quitting, windowed only.
    // ⭐ It starts when the ROUND goes live, not when the process starts: a cold
    // launch spends ten-plus seconds on cargo, assets and the shell, and a
    // budget that counted those would measure a different window on every
    // machine. Zero (the default) means "play until you close the window".
    let seconds: f32 = arg_value(&args, "--seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    // ⭐ The scaling dimension. Zero (the default) leaves every prior measurement
    // in this binary exactly comparable — the knob adds nothing when unused.
    let scaling_sprites: usize = arg_value(&args, "--sprites")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    if arg_flag(&args, "--window") {
        run_windowed(fighters, seconds);
    } else {
        run_windowless(fighters, ticks, scaling_sprites);
    }
}

/// The no-GPU arm: build the app with no window, step it by hand, measure a
/// fixed number of ticks after the round goes live.
fn run_windowless(fighters: usize, ticks: u32, scaling_sprites: usize) {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);

    // ⭐ ONE APP, ONE PROCESS: the exact condition that makes a global tracing
    // subscriber safe here and unsafe in the test binary — which is why
    // `build_visible_app` drops `LogPlugin` from every windowless mode. Tracy's
    // recorder is a LAYER ON THAT SUBSCRIBER, so without this a
    // `--features profile` capture of this binary records ZERO zones, and
    // per-system timing is the one measurement a machine with no GPU still has.
    #[cfg(feature = "profile")]
    app.add_plugins(bevy::log::LogPlugin::default());

    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    seat_the_match(app.world_mut(), fighters);

    let mut live_at = None;
    for tick in 0..LIVE_DEADLINE_FRAMES {
        app.update();
        let (seated, held) = cast_state(app.world_mut());
        if seated > 0 && held == 0 {
            live_at = Some(tick);
            break;
        }
    }
    let Some(live_at) = live_at else {
        eprintln!(
            "[smash-profile] ABORT: the opening ceremony never released the cast, so nothing \
             below would have measured a match"
        );
        std::process::exit(3);
    };
    // ⛔⛔ THE ROSTER YOU ASKED FOR IS NOT NECESSARILY THE ROSTER THAT SEATED, and
    // a run that quietly seats fewer answers a DIFFERENT question than the one
    // on the command line. Measured 2026-08-29: `--fighters 4` reported 3
    // bodies, which invalidated a fighter-count scaling comparison that
    // otherwise looked like three clean arms.
    let (seated_now, _) = cast_state(app.world_mut());
    if seated_now != fighters {
        eprintln!(
            "[smash-profile] ⛔ ROSTER MISMATCH: asked for {fighters} fighters, {seated_now} are \
             seated. This run does NOT measure a {fighters}-fighter match, and comparing it \
             against another arm compares different rosters."
        );
    }
    // ⭐ THE ENTITY COUNT AT THE QUIET MOMENT. Differencing TOTAL live entities
    // between a 2- and a 4-fighter run does not work later on: combat VFX spawn
    // and despawn make `live` swing ~40 WITHIN a single run, which is as large as
    // the whole fighter delta. Right here the round has just gone live and no
    // combat has happened, so two arms differ by their ROSTER and little else —
    // which is what makes "how many entities is a fighter" askable at all.
    let live_entities = app
        .world_mut()
        .query::<bevy::prelude::Entity>()
        .iter(app.world())
        .count();
    // ⭐ SPRITES TOO, at the same quiet moment. 8 entities per fighter cannot
    // explain the ~39us of `PostUpdate` a fighter adds, so the next candidate is
    // what the pipeline actually DRAWS rather than what was spawned.
    let live_sprites = app
        .world_mut()
        .query::<&bevy::prelude::Sprite>()
        .iter(app.world())
        .count();
    eprintln!(
        "[smash-profile] fighters={fighters} seated={seated_now} live_after_ticks={live_at} \
         entities_at_go_live={live_entities} sprites_at_go_live={live_sprites} \
         measuring={ticks}"
    );

    // ⛔ AFTER the round goes live, not before: sprites spawned into the opening
    // ceremony would be swept by the session teardown that runs between the
    // lobby and the stage, and the run would silently measure zero of them.
    if scaling_sprites > 0 {
        spawn_scaling_sprites(app.world_mut(), scaling_sprites);
        // Visibility is computed later in the frame, so ask on the NEXT one.
        app.update();
        warn_if_scaling_sprites_were_culled(app.world_mut(), scaling_sprites);
    }

    for _ in 0..ticks {
        app.update();
    }

    report_end_of_run(app.world_mut(), seated_now);
}

/// The GPU arm: a real window, winit's event loop, hardware rendering.
///
/// ⛔ THE LOOP IS NOT OURS HERE. `app.run()` never returns until the window
/// closes, so the settle / seat / wait-for-live sequence the windowless arm
/// writes as straight-line code has to become a system that reaches the same
/// states one frame at a time. [`MatchDriver`] is that sequence, not a second
/// policy — the conditions it tests are the ones above.
fn run_windowed(fighters: usize, seconds: f32) {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::Windowed,
        true,
        |app| {
            // ⛔ A PROFILING RUN MUST NOT WRITE THE DEVELOPER'S SAVE. The
            // windowless modes get this from `build_visible_app` because a
            // non-session App must not have the side effect; a windowed one
            // normally SHOULD have it, and this process is the exception —
            // it is an instrument wearing the game's composition.
            app.insert_resource(ambition_platformer2d::persistence::PersistenceRoot::isolated());
        },
    );
    // ⭐ NO STARTUP CEREMONY. `run_visible` composes the "Powered by Ambition"
    // run-in for the shipped binary; this run is about the match, and the cards
    // are ten seconds of measured logo.
    app.insert_resource(MatchDriver {
        fighters,
        measure_for: (seconds > 0.0).then_some(seconds),
        live_frames: 0,
        warned_empty: false,
        seats_at_live: 0,
        stage: Stage::Settling(SETTLE_FRAMES),
    });
    app.add_systems(Update, drive_match);
    eprintln!(
        "[smash-profile] windowed: fighters={fighters} measure_for={}",
        if seconds > 0.0 {
            format!("{seconds}s of live match")
        } else {
            "until the window closes".to_string()
        }
    );
    app.run();
}

/// Where the windowed run is in the sequence that reaches a live round.
///
/// `Copy` so [`drive_match`] can read the stage out of the driver and assign a
/// new one in the same arm; every variant holds a `Copy` payload already.
#[derive(Clone, Copy)]
enum Stage {
    /// Frames left to let the shell finish building before the roster lands.
    Settling(u32),
    /// Frames spent waiting for the ceremony to release the cast.
    WaitingForLive(u32),
    /// The round is live; the instant it became so.
    Live(std::time::Instant),
    /// Measured, reported, exit written. Nothing further to do.
    Done,
}

#[derive(Resource)]
struct MatchDriver {
    fighters: usize,
    /// `None` means "until the window closes".
    measure_for: Option<f32>,
    /// Frames since the round went live, for [`PREMISE_CHECK_EVERY`].
    live_frames: u32,
    warned_empty: bool,
    /// Seats at the moment the round went LIVE — the baseline a KO is measured
    /// against, so `report_end_of_run` can say the cast changed underneath.
    seats_at_live: usize,
    stage: Stage,
}

/// An EXCLUSIVE system on purpose: seating a match inserts a resource, writes a
/// shell command, and queries two populations, which is `&mut World` work in any
/// case — and writing it that way lets this arm call the SAME helpers the
/// windowless arm calls instead of restating their conditions in system params.
fn drive_match(world: &mut World) {
    // Taken out and put back so the body can mutate the world freely; the
    // alternative is threading a resource borrow through every helper.
    let Some(mut driver) = world.remove_resource::<MatchDriver>() else {
        return;
    };
    match driver.stage {
        Stage::Settling(0) => {
            seat_the_match(world, driver.fighters);
            driver.stage = Stage::WaitingForLive(0);
        }
        Stage::Settling(left) => driver.stage = Stage::Settling(left - 1),
        Stage::WaitingForLive(waited) => {
            let (seated, held) = cast_state(world);
            if seated > 0 && held == 0 {
                eprintln!(
                    "[smash-profile] live after {waited} frames; fighters={}",
                    driver.fighters
                );
                driver.seats_at_live = cast_state(world).0;
                if driver.seats_at_live != driver.fighters {
                    eprintln!(
                        "[smash-profile] ⛔ ROSTER MISMATCH: asked for {} fighters, {} are seated.",
                        driver.fighters, driver.seats_at_live
                    );
                }
                driver.stage = Stage::Live(std::time::Instant::now());
            } else if waited >= LIVE_DEADLINE_FRAMES {
                eprintln!(
                    "[smash-profile] ABORT: the opening ceremony never released the cast, so \
                     this bundle would have measured a menu"
                );
                world.write_message(bevy::app::AppExit::from_code(3));
                driver.stage = Stage::Done;
            } else {
                driver.stage = Stage::WaitingForLive(waited + 1);
            }
        }
        Stage::Live(since) => {
            // ⛔ THE PREMISE, WHILE IT IS STILL CHECKABLE. An unattended run
            // that outlives its own match records a results screen; say so at
            // the moment it happens rather than only at the end, because a
            // developer who closes the window never reaches the end.
            driver.live_frames += 1;
            if !driver.warned_empty
                && driver.live_frames % PREMISE_CHECK_EVERY == 0
                && cast_state(world).0 == 0
            {
                driver.warned_empty = true;
                eprintln!(
                    "[smash-profile] WARNING: the match ended — census rows from here on are \
                     whatever followed it, not a match"
                );
            }
            if driver
                .measure_for
                .is_some_and(|budget| since.elapsed().as_secs_f32() >= budget)
            {
                report_end_of_run(world, driver.seats_at_live);
                world.write_message(bevy::app::AppExit::Success);
                driver.stage = Stage::Done;
            }
        }
        Stage::Done => {}
    }
    world.insert_resource(driver);
}
