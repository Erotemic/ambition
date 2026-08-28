//! ⭐⭐ A ZERO-DURATION UPDATE RUNS THE SCHEDULES AND NOT THE SIMULATION.
//!
//! This is the clock half of the move-capture design. A GPU readback is
//! asynchronous: the driver has to keep calling `App::update()` until it lands.
//! Every existing multi-shot driver does that with the ORDINARY period, so the
//! simulation advances for the whole GPU wait — `capture_scene --frames` spaces
//! its shots by `stride + however long the GPU took`, which is not a spacing at
//! all. For a room burst that is invisible; for a move animation it means
//! startup or active frames pass while a PNG is in flight.
//!
//! ⭐ SO THE PUMP MUST COST NOTHING. `TimeUpdateStrategy::ManualDuration`
//! advances Bevy's clocks by exactly the duration given, so `ZERO` freezes them
//! while the rest of the frame still runs — which is what services a readback.
//!
//! ⛔ THIS TEST NEEDS NO GPU ON PURPOSE. It proves the CLOCK property, so a
//! machine with no renderer stays a supported environment; the real readback is
//! proved separately where a GPU exists.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

fn sim_tick(app: &bevy::prelude::App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

#[test]
fn a_zero_duration_pump_costs_no_simulation_and_the_canonical_period_resumes() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);

    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    // Both conditions: staged is not the same as a running session.
    let mut live = false;
    for _ in 0..900 {
        app.update();
        let staged = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            all.iter(world).count() > 0
        };
        if staged && ambition_platformer2d::rollback::session_is_active(app.world()) {
            live = true;
            break;
        }
    }
    assert!(live, "no live rollback session, so nothing below is about a running sim");

    // ── THE PUMP COSTS NOTHING ──
    let frozen_at = sim_tick(&app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    for pump in 0..30 {
        app.update();
        assert_eq!(
            sim_tick(&app),
            frozen_at,
            "a zero-duration update advanced the simulation on pump {pump} — the \
             whole point is that a GPU wait costs no simulation time, so a PNG \
             can name the exact tick it was taken on"
        );
    }

    // ── AND THE CANONICAL PERIOD RESUMES, EXACTLY ONE TICK ──
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(canonical));
    app.update();
    assert_eq!(
        sim_tick(&app),
        frozen_at + 1,
        "restoring the canonical period did not resume one tick per update — a \
         pump that cannot be un-paused is a driver that captures one frame and \
         then stops simulating"
    );
    // ⛔ AND IT KEEPS RESUMING. A single +1 could be an accumulator discharging
    // something the pumps banked rather than the clock actually running again.
    for step in 0..10 {
        let before = sim_tick(&app);
        app.update();
        assert_eq!(
            sim_tick(&app),
            before + 1,
            "step {step} after the pump advanced by something other than one tick"
        );
    }
}
