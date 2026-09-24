//! Profile a real Smash match: windowless on a machine with no GPU, or in a
//! real window on a machine that has one.
//!
//! Other paths do not profile a match. `run_game.sh smash` opens on
//! character select, and headless rooms have only one or two bodies, so
//! their baselines describe the engine's fixed overhead, not gameplay.
//!
//! This follows the path `app_it` proves: build the visible app, install a
//! roster, and route to the gameplay screen. That is the shipped composition
//! (rollback host included), not the demo shell. The two render modes differ
//! in one argument and in who drives the loop; reaching a live round is
//! shared.
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
//! Census rows are sampled on wall time. A windowless match runs far faster
//! than real time, so keep `AMBITION_PROFILE_CENSUS_HZ` high enough that the
//! run outlives the first interval. Otherwise the only row is startup, and a
//! `frames=1` row reporting `Update=127ms` is plugin build, not a frame. A
//! windowed run is paced by the display, so the default 1 Hz is right.
//!
//! Never compare or subtract the two modes. `NoWindow` selects
//! `backends: None`: no adapter, no render app, no drawing. The bundle's
//! `gpu.rendering` field (`headless` vs `hardware`) is part of the history's
//! comparability key for this reason.

use bevy::prelude::*;

use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
use ambition_platformer2d::characters::control::{ControlHolds, SlotControls};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// Frames to let the shell settle before the roster lands. A roster inserted
/// into an unbuilt shell is dropped; the integration tests wait the same 30.
const SETTLE_FRAMES: u32 = 30;

/// Frames to wait for the opening ceremony to release the cast before giving
/// up. Ten seconds at 60 Hz. The ceremony is ~3s and dev mode runs it 10x
/// fast, so this bounds a hang; it does not encode the ceremony's length.
const LIVE_DEADLINE_FRAMES: u32 = 600;

/// How often the windowed run re-checks that a match is still happening.
///
/// Not every frame: `World::query` builds a fresh `QueryState` that walks the
/// archetype set, and a live match has over two thousand entities. Checking
/// every frame would perturb the measurement.
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

/// Spawn `count` plain sprites into the live match, spread over the stage.
///
/// This varies one dimension (sprite count) on the real stack: same app,
/// schedules, and render path.
///
/// The sprites are plain (`Sprite`, `Transform`, `Visibility`): no gameplay
/// components, which would measure gameplay instead. They share one colour
/// and no texture, so this measures the per-sprite path, not batch breaking.
/// Batch breaking needs a separate knob.
fn spawn_scaling_sprites(world: &mut World, count: usize) {
    // Anchor on a fighter, where the world camera looks. A sprite with
    // `ViewVisibility` set can be a HUD sprite in screen space. The fighter's
    // position is in `BodyKinematics`, not a `GlobalTransform`: the sim body
    // carries kinematics, and its `Transform` lives on a separate presentation
    // entity.
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

    // A deterministic grid around the anchor: the same placement every run at
    // the same count.
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

/// Report loudly when a scaling population was spawned and then culled.
///
/// `spawn_scaling_sprites` cannot check visibility, which is computed later
/// in the frame. Note: a raw `Sprite + Transform + Visibility` is never drawn
/// in this composition. Presentation is projected per view
/// (`[census] draws per_view_projections`), so a synthetic sprite benchmark
/// must go through that projection.
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

/// The cast, as the round's own state: how many seats exist, and how many are
/// still held by the opening ceremony's scripted control.
///
/// Wait for the round, not for a frame count. A count encodes the ceremony's
/// length, and dev mode runs the ceremony 10x fast. The condition is
/// observable: a cast exists, and nothing in it is held.
fn cast_state(world: &mut World) -> (usize, usize) {
    let seated = world.query::<&MatchSeat>().iter(world).count();
    let held = world
        .query_filtered::<&MatchSeat, With<ControlHolds>>()
        .iter(world)
        .count();
    (seated, held)
}

/// Install the roster and ask the shell for the gameplay route. Both in one
/// tick, in this order: the route activation reads the roster.
///
/// Any character the composition carries, not only the default stand-in. This
/// bin composes the full app (`composition_can_seat=` reports 21, including
/// `npc_pirate_admiral`), while the smash rigs reach only the demo shell's
/// three ids (D189).
///
/// An unseatable id must fail loudly. Otherwise the match has no fighter
/// brain and every number is zero, which reads as a quiet fight.
fn seat_the_match(world: &mut World, fighters: usize, character: &str) {
    world.insert_resource(ambition_demo_smash::smash_roster(vec![character; fighters]));
    world.write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
}

/// Check the premise. A profile of a match that ended is a profile of a
/// results screen, which looks like a cheap frame.
fn report_end_of_run(world: &mut World, seats_at_start: usize) {
    let (seats, _) = cast_state(world);
    // A KO during the measured window changes the population, so the mean
    // averages two different matches. `seats == 0` is the extreme case.
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
    // The startup anchor, first line. `StartupProfiler` measures from here;
    // without it the report starts mid-plugin-build ("app construction NOT
    // MEASURED"). Plugin build scales with registered systems, so this prices
    // composition changes the frame cannot see.
    ambition_platformer2d::dev_tools::profiling::note_process_start();
    let args: Vec<String> = std::env::args().collect();
    let ticks: u32 = arg_value(&args, "--ticks")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    // Four is the cap: `SlotControls::MAX_SLOTS` is 4, and a longer roster
    // would be silently clamped.
    let fighters: usize = arg_value(&args, "--fighters")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
        .clamp(2, SlotControls::MAX_SLOTS);
    // Which fighter. The default is the stand-in this bin has always seated,
    // so existing invocations measure the same thing.
    let character: String = arg_value(&args, "--character").unwrap_or_else(|| "performer".to_string());
    // Wall seconds of live match to measure before quitting, windowed only.
    // It starts when the round goes live, not at process start, because cold
    // launch time varies by machine. Zero (the default) means "until the
    // window closes".
    let seconds: f32 = arg_value(&args, "--seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    // The scaling dimension. Zero (the default) adds nothing.
    let scaling_sprites: usize = arg_value(&args, "--sprites")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    if arg_flag(&args, "--window") {
        run_windowed(fighters, seconds, character);
    } else {
        run_windowless(fighters, ticks, scaling_sprites, character);
    }
}

/// The no-GPU arm: build the app with no window, step it by hand, measure a
/// fixed number of ticks after the round goes live.
fn run_windowless(fighters: usize, ticks: u32, scaling_sprites: usize, character: String) {
    // One App, one process, so a global tracing subscriber is safe here.
    // `build_visible_app` drops `LogPlugin` from windowless modes, and Tracy's
    // recorder is a layer on that subscriber, so without this a
    // `--features profile` capture records no zones.
    // Add it in the compose hook: a `NoWindow` build finishes its plugins
    // before returning, and Bevy 0.19 panics on `add_plugins` after that. The
    // `#[cfg]` stays inside the closure so it cannot attach to the next
    // statement.
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_app| {
            #[cfg(feature = "profile")]
            _app.add_plugins(bevy::log::LogPlugin::default());
        },
    );

    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    // Which cast this composition can seat. `SmashRoster` is `SMASH_ROSTER`
    // filtered at `Startup` to ids in the assembled catalog. Printed so a
    // profile names its cast (see D189).
    let seatable = app
        .world()
        .get_resource::<ambition_demo_smash::select::SmashRoster>()
        .map(|roster| roster.0.clone())
        .unwrap_or_default();
    eprintln!(
        "[smash-profile] composition_can_seat={} ids{}",
        seatable.len(),
        if seatable.is_empty() {
            String::new()
        } else {
            format!(" — {}", seatable.join(", "))
        }
    );
    seat_the_match(app.world_mut(), fighters, &character);

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
        // Name the real cause. The common reason to get here is a character
        // this composition does not carry, not a stuck ceremony.
        if !seatable.is_empty() && !seatable.iter().any(|id| id == &character) {
            eprintln!(
                "[smash-profile] ABORT: '{character}' is not one of the {} ids this \
                 composition carries, so no fighter was ever seated and the ceremony had \
                 nothing to release. Carried: {}",
                seatable.len(),
                seatable.join(", ")
            );
        } else {
            eprintln!(
                "[smash-profile] ABORT: the opening ceremony never released the cast, so \
                 nothing below would have measured a match"
            );
        }
        std::process::exit(3);
    };
    // The roster asked for is not always the roster that seated (for
    // example `--fighters 4` can seat 3). A different count answers a
    // different question, so check it.
    let (seated_now, _) = cast_state(app.world_mut());
    if seated_now != fighters {
        eprintln!(
            "[smash-profile] ⛔ ROSTER MISMATCH: asked for {fighters} fighters, {seated_now} are \
             seated. This run does NOT measure a {fighters}-fighter match, and comparing it \
             against another arm compares different rosters."
        );
    }
    // The entity count at the quiet moment. Later, combat VFX make the live
    // count swing by ~40 within one run, as much as a whole fighter. Right
    // after the round goes live, two runs differ only by roster.
    let live_entities = app
        .world_mut()
        .query::<bevy::prelude::Entity>()
        .iter(app.world())
        .count();
    // Sprites too, at the same moment: what the pipeline draws, not only what
    // was spawned.
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

    // After the round goes live: sprites spawned during the opening ceremony
    // are removed by the teardown between lobby and stage.
    if scaling_sprites > 0 {
        spawn_scaling_sprites(app.world_mut(), scaling_sprites);
        // Visibility is computed later in the frame, so check on the next one.
        app.update();
        warn_if_scaling_sprites_were_culled(app.world_mut(), scaling_sprites);
    }

    // Count how much of the measured window had a match in it. A long run
    // outlives the match, and post-match frames cost much less than match
    // frames (1.84ms vs 4.31ms), which pulls the mean down. The coverage goes
    // in the summary line, beside the numbers it qualifies.
    //
    // Sampled every 50 ticks, so the instrument does not join the population
    // it measures.
    let mut live_samples = 0usize;
    let mut total_samples = 0usize;
    for tick in 0..ticks {
        app.update();
        if tick % 50 == 0 {
            total_samples += 1;
            if cast_state(app.world_mut()).0 > 0 {
                live_samples += 1;
            }
        }
    }
    let coverage = if total_samples == 0 {
        100.0
    } else {
        100.0 * live_samples as f64 / total_samples as f64
    };
    if coverage < 95.0 {
        eprintln!(
            "[smash-profile] ⛔ ONLY {coverage:.0}% OF THE MEASURED WINDOW HAD A LIVE CAST \
             ({live_samples}/{total_samples} samples). The rest is a results screen at roughly \
             HALF the frame cost, so every mean below is dragged down and every rate is \
             diluted. Re-run with fewer --ticks, or filter to intervals with bodies>=2."
        );
    }
    eprintln!("[smash-profile] measured_window_live_cast={coverage:.0}%");

    report_end_of_run(app.world_mut(), seated_now);
}

/// The GPU arm: a real window, winit's event loop, hardware rendering.
///
/// The loop is not ours here: `app.run()` returns only when the window
/// closes. [`MatchDriver`] runs the windowless arm's settle / seat /
/// wait-for-live sequence one frame at a time, with the same conditions.
fn run_windowed(fighters: usize, seconds: f32, character: String) {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::Windowed,
        true,
        |app| {
            // A profiling run must not write the developer's save. A normal
            // windowed app does; this process is an instrument.
            app.insert_resource(ambition_platformer2d::persistence::PersistenceRoot::isolated());
        },
    );
    // No startup ceremony: its cards are ten seconds of measured logo.
    app.insert_resource(MatchDriver {
        fighters,
        character,
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
    /// The catalog id every seat wears. See `seat_the_match`.
    character: String,
    /// `None` means "until the window closes".
    measure_for: Option<f32>,
    /// Frames since the round went live, for [`PREMISE_CHECK_EVERY`].
    live_frames: u32,
    warned_empty: bool,
    /// Seats when the round went live: the baseline for detecting a KO in
    /// `report_end_of_run`.
    seats_at_live: usize,
    stage: Stage,
}

/// An exclusive system: seating a match inserts a resource, writes a shell
/// command, and queries two populations. It also lets this arm call the same
/// helpers as the windowless arm.
fn drive_match(world: &mut World) {
    // Taken out and put back so the body can mutate the world freely; the
    // alternative is threading a resource borrow through every helper.
    let Some(mut driver) = world.remove_resource::<MatchDriver>() else {
        return;
    };
    match driver.stage {
        Stage::Settling(0) => {
            seat_the_match(world, driver.fighters, &driver.character);
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
            // Check the premise while it is still checkable. A developer who
            // closes the window never reaches the end-of-run report.
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
