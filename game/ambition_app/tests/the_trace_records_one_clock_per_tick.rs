//! Does a trace row state one clock?
//!
//! ⭐ A ROW WRITES `real_dt`, `sim_dt` AND `time_scale`, and a reader divides
//! them. All three must come from the tick-head `WorldTime`: the smoother
//! rewrites `ClockState::time_scale` later in the same tick with no ordering
//! edge against `Trace`, so a row that read `ClockState` would pair this tick's
//! dts with whatever scale the smoother had already moved to.
//!
//! ⛔ THE ORDER IS PINNED, NOT HOPED FOR. Which of the two runs first is not
//! part of the schedule contract, so the composition adds a clock writer
//! ordered between `CoreSimulation` and `Trace` — the smoother winning the race
//! on every tick.

use ambition_app::{Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::gameplay_trace::{ActorTraceBuffer, GameplayTraceBuffer};
use ambition_platformer2d::sim::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};
use ambition_platformer2d::time::ClockState;
use bevy::prelude::*;

/// A ramp step taken after the tick head and before the recorders.
fn move_the_clock_before_the_trace(mut clock: ResMut<ClockState>) {
    clock.time_scale = (clock.time_scale * 0.9).max(0.1);
}

#[test]
fn every_trace_row_states_the_scale_its_dts_were_stepped_at() {
    let mut sim = Platformer2dSimHarness::build(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz()),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let sim = app.sim_schedule();
            app.add_systems(
                sim,
                move_the_clock_before_the_trace
                    .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                    .before(Platformer2dSimulationPhaseMonolith::Trace),
            );
            Ok(())
        },
    )
    .expect("sandbox sim builds");

    let mut moved_after_the_head = 0;
    for _ in 0..40 {
        sim.step_frame(Default::default());
        let world = sim.world();
        let row = world
            .resource::<GameplayTraceBuffer>()
            .frames()
            .last()
            .cloned()
            .expect("the gameplay trace recorded this tick");
        let actor_row = world
            .resource::<ActorTraceBuffer>()
            .frames()
            .last()
            .cloned()
            .expect("the actor trace recorded this tick");
        assert!(
            (row.sim_dt - row.real_dt * row.time_scale).abs() < 1e-6,
            "tick {}: the gameplay trace row says real_dt={} sim_dt={} but \
             time_scale={}, so its scale is not the one its dts were stepped at",
            row.tick,
            row.real_dt,
            row.sim_dt,
            row.time_scale,
        );
        assert_eq!(
            (row.real_dt, row.sim_dt, row.time_scale),
            (actor_row.real_dt, actor_row.sim_dt, actor_row.time_scale),
            "tick {}: the gameplay trace and the actor trace disagree about the clock",
            row.tick,
        );
        if (world.resource::<ClockState>().time_scale - row.time_scale).abs() > 1e-6 {
            moved_after_the_head += 1;
        }
    }

    // ⛔ THE PREMISE: the live clock moved after the tick head. At a clock that
    // never moves, every source agrees and nothing above could have failed.
    assert!(
        moved_after_the_head > 0,
        "the live clock never left the scale the tick head recorded"
    );
}
