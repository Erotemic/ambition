//! Profile an ACTUAL Smash match, headlessly, in the shipped composition.
//!
//! ⛔⛔ THIS EXISTS BECAUSE NOTHING ELSE PROFILES A MATCH. Measured 2026-08-29:
//! `run_game.sh smash` builds the standalone demo, passes `--window` AND
//! `--headless`, and opens on CHARACTER SELECT — so it profiles a menu. And
//! every headless room measures a two-body world: sweeping `--start-room` over
//! `goblin_encounter`, `central_hub_complex` and the default all report
//! `entities=64, bodies=1-2`. Every frame baseline taken before this therefore
//! describes the engine's FIXED OVERHEAD, not gameplay.
//!
//! ⭐ The road is the one `app_it` already proves works — build the visible app
//! with no window, install a roster, and route to the gameplay screen. That is
//! the SHIPPED composition (rollback host and all), not the demo shell, so what
//! it measures is what Jon plays.
//!
//! ```bash
//! AMBITION_PROFILE_CENSUS=1 AMBITION_PROFILE_CENSUS_HZ=20 \
//!   cargo run -p ambition_app_tools --bin smash_match_profile -- --ticks 3000
//! ```
//!
//! ⚠ Census rows are sampled on WALL time. A headless match runs far faster than
//! real time, so leave `AMBITION_PROFILE_CENSUS_HZ` high enough that the run
//! outlives the first interval — otherwise the only row you get is startup, and
//! a `frames=1` row reporting `Update=127ms` is PLUGIN BUILD, not a frame.

use bevy::prelude::*;

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ticks: u32 = arg_value(&args, "--ticks")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    // Four is the cap: `SlotControls::MAX_SLOTS` is 4, so a roster longer than
    // that is not a scaling axis — it is a silently clamped one.
    let fighters: usize = arg_value(&args, "--fighters")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
        .clamp(2, ambition_platformer2d::characters::control::SlotControls::MAX_SLOTS);

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);

    // Let the shell settle before the roster lands, exactly as the integration
    // tests do; a roster inserted into an unbuilt shell is dropped.
    for _ in 0..30 {
        app.update();
    }

    let roster = ambition_demo_smash::smash_roster(vec!["actor"; fighters]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // ⭐ THE ROUND GOING LIVE IS OBSERVABLE, and waiting a fixed frame count
    // would encode the opening ceremony's length instead. The cast is live when
    // seats exist and none is still held by the ceremony's scripted control.
    let mut live_at = None;
    for tick in 0..600u32 {
        app.update();
        let world = app.world_mut();
        let seated = world
            .query::<&ambition_platformer2d::actor::MatchSeat>()
            .iter(world)
            .count();
        let held = world
            .query_filtered::<
                &ambition_platformer2d::actor::MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >()
            .iter(world)
            .count();
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
    eprintln!("[smash-profile] fighters={fighters} live_after_ticks={live_at} measuring={ticks}");

    for _ in 0..ticks {
        app.update();
    }

    // ⛔ THE PREMISE, CHECKED RATHER THAN ASSUMED. A profile of a match that
    // quietly ended is a profile of a results screen.
    let seats = {
        let world = app.world_mut();
        world
            .query::<&ambition_platformer2d::actor::MatchSeat>()
            .iter(world)
            .count()
    };
    if seats == 0 {
        eprintln!(
            "[smash-profile] WARNING: no seats remain — the match ended during the measured \
             window, so the census rows above mix a match with whatever followed it"
        );
    }
    eprintln!("[smash-profile] done seats_at_end={seats}");
}
