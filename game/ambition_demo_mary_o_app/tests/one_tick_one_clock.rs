//! ONE TICK, ONE CLOCK: everything a sim tick runs reads the clock that tick
//! published.
//!
//! The gravity-zone snapshot runs ahead of `CoreSimulation`, and it oscillates
//! zones by `SimDt`. The clock (timeline advance, time scale, `WorldTime` and
//! its `SimDt` mirror) used to be published inside `PlayerInput`, a phase
//! later, so the snapshot read the PREVIOUS tick's dt and timeline — two
//! answers to "how much time does this tick advance". This probe records the
//! timeline each side sees on the same tick.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::actors::gravity::GravitySet;
use ambition_platformer2d::platformer::schedule::SimScheduleExt;
use ambition_platformer2d::time::SimTick;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Seen {
    snapshot: Vec<u64>,
    input: Vec<u64>,
}

#[test]
fn the_gravity_snapshot_reads_the_clock_its_own_tick_published() {
    let mut app = build_demo_app();
    app.init_resource::<Seen>();
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        (|tick: Res<SimTick>, mut seen: ResMut<Seen>| seen.snapshot.push(tick.get()))
            .in_set(GravitySet::ZoneSnapshot),
    );
    app.add_systems(
        sim,
        (|tick: Res<SimTick>, mut seen: ResMut<Seen>| seen.input.push(tick.get()))
            .in_set(ambition_platformer2d::sim::Platformer2dSimulationPhaseMonolith::PlayerSimulation),
    );
    for _ in 0..600 {
        app.update();
        if app.world().resource::<Seen>().input.len() >= 10 {
            break;
        }
    }
    let seen = app.world().resource::<Seen>();
    assert!(
        seen.input.len() >= 10,
        "the premise: the demo ran at least ten sim ticks ({} seen)",
        seen.input.len()
    );
    assert!(
        seen.snapshot.len() >= 10,
        "the premise: the gravity snapshot ran on those ticks ({} seen) — an empty \
         side compares nothing",
        seen.snapshot.len()
    );
    let n = seen.snapshot.len().min(seen.input.len());
    assert_eq!(
        &seen.snapshot[seen.snapshot.len() - n..],
        &seen.input[seen.input.len() - n..],
        "the gravity snapshot ran against a different tick's clock than the rest \
         of the tick: the clock is published after it",
    );
}
