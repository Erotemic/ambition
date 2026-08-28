//! Does one `step_simulation` call advance the sim by exactly one tick?
//!
//! ⛔⛔ THE QUESTION THE SEAM LIVES OR DIES ON, and it is not answerable by
//! reading: both hosts advance from a wall-clock accumulator, and whether
//! pausing the virtual clock and handing it exactly one tick's worth of time
//! yields exactly one advance depends on how `bevy_ggrs` rounds, on what
//! `TimePlugin` does to a paused clock, and on whether anything else feeds the
//! accumulator. This asks the running app.
//!
//! Prints the tick delta per step, and how long the whole thing took in REAL
//! time — the second number is the point: a deterministic stepper should beat
//! the wall clock, not track it.

use ambition_platformer2d::runtime::{step_simulation, take_manual_control};

/// ⛔⛤ `SimTick` IS NOT THIS HOST'S COUNTER. It is advanced by
/// `ambition_time::advance_sim_tick`, installed in the PLAYER schedule, and it
/// stayed at zero through free updates of a live smash match — so a probe built
/// on it reports the seam broken when the seam has not been asked anything.
/// `bevy_ggrs::RollbackFrameCount` is the rollback host's own frame number: the
/// thing that increments once per world advance, which IS the question.
fn frame(app: &bevy::prelude::App) -> i32 {
    ambition_platformer2d::sim::rollback_frame(app.world()).unwrap_or(-1)
}

fn main() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ⛔⛔ A BOOTED APP IS NOT A RUNNING SIMULATION. The first version measured
    // an app sitting in the shell with no session and reported zero ticks for
    // BOTH arms — which looks like the seam failing and is actually the probe
    // measuring a world where nothing simulates. Seat a real match first, the
    // way every driver does.
    //
    // ⛔ AND BOOT BEFORE SEATING. A `GoTo` written before the first `update()`
    // is written into a world whose readers do not exist yet; every working
    // driver in this repo runs ~30 frames first.
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // ⛔⛔ WAIT FOR A LIVE ROUND, NOT FOR A FRAME NUMBER. The rollback frame is
    // the thing being measured, so waiting on it is circular — and it stays at
    // zero until a match is actually staged, which is why the first version of
    // this probe timed an app that was not simulating at all (1.75ms an update
    // against a live match's ~7ms). The observable condition is the one every
    // test here uses: a cast exists, and nothing in it is still held by the
    // opening ceremony.
    let mut live = false;
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&ambition_platformer2d::actor::MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actor::MatchSeat,
                bevy::prelude::With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            live = true;
            break;
        }
    }
    assert!(live, "the opening ceremony never released the cast");
    println!("[step-probe] rollback frame at start: {}", frame(&app));

    let free_before = frame(&app);
    let wall = std::time::Instant::now();
    for _ in 0..120 {
        app.update();
    }
    let free_ticks = frame(&app) - free_before;
    let free_secs = wall.elapsed().as_secs_f32();

    take_manual_control(&mut app);
    let stepped_before = frame(&app);
    let wall = std::time::Instant::now();
    step_simulation(&mut app, 120);
    let stepped_ticks = frame(&app) - stepped_before;
    let stepped_secs = wall.elapsed().as_secs_f32();

    println!("[step-probe] 120 free updates   -> {free_ticks:>4} sim ticks in {free_secs:.2}s");
    println!("[step-probe] 120 stepped ticks  -> {stepped_ticks:>4} sim ticks in {stepped_secs:.2}s");
    if stepped_ticks == 120 {
        println!("[step-probe] PASS — one call, one tick");
    } else {
        println!("[step-probe] FAIL — asked for 120 ticks and the sim ran {stepped_ticks}");
    }
}
