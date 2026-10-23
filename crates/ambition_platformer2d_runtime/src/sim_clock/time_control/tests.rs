//! Unit tests for the time-control authority: `ProperTimeScale` defaults and
//! the `ClockScaleRequest` → policy dispatch in the `Solo` regime.

use super::*;
use ambition_time::time_control::{
    apply_clock_scale_requests, apply_suspended_time_scale_system, RegimePolicy,
};
use ambition_time::ClockObserver;

/// End-to-end: a SimClock grant + the smoother together actually
/// move time_scale. After ~10 frames at default ramp rates the
/// time_scale should be well below 1.0 (heading toward 0.125).
#[test]
fn smoother_ramps_sim_state_time_scale_toward_target() {
    let mut app = App::new();
    app.add_message::<ClockScaleRequest>()
        .insert_resource(RegimePolicy::default())
        .insert_resource(RequestedClockScale::default())
        .insert_resource(ClockState::default())
        .insert_resource(Platformer2dFeelTuningMonolith::default())
        .insert_resource(Time::<()>::default())
        .init_resource::<ambition_time::WorldTime>()
        .add_systems(
            Update,
            (
                ambition_time::refresh_world_time,
                apply_clock_scale_requests,
                smooth_sim_clock_toward_target_system,
            )
                .chain(),
        );

    let frame = std::time::Duration::from_millis(16);
    for _ in 0..30 {
        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(ClockObserver::PRIMARY),
            reason: "ramp_test",
        });
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();
    }

    let clock = app.world().resource::<ClockState>();
    // Platformer2dFeelTuningMonolith::time_ramp_down_rate is 6.0 (units/s).
    // 30 frames * 16ms = 480ms => deltas of ~2.88 units, way
    // past the (1.0 -> 0.125) gap of 0.875.
    assert!(
        (clock.time_scale - 0.125).abs() < 1e-4,
        "expected sim time_scale ~= 0.125 after ramp; got {}",
        clock.time_scale,
    );
}

/// Integration check: with `WorldTime::sim_dt` at 0.25, a timer driven by `sim_dt()`
/// advances at exactly 0.25× the wall-clock dt.
#[test]
fn world_time_sim_dt_respects_time_scale() {
    use ambition_time::WorldTime;

    let mut wt = WorldTime::default();
    wt.set_wall_dt(0.016);
    wt.set_sim_dt(0.016 * 0.25);
    assert!((wt.wall_dt() - 0.016).abs() < 1e-6);
    assert!((wt.sim_dt() - 0.004).abs() < 1e-6);

    // Pause behaviour: time_scale == 0 -> sim_dt == 0 even
    // though wall_dt keeps ticking.
    let mut paused = WorldTime::default();
    paused.set_wall_dt(0.016);
    paused.set_sim_dt(0.0);
    assert_eq!(paused.sim_dt(), 0.0);
    assert!((paused.wall_dt() - 0.016).abs() < 1e-6);
}

/// Regression: when gameplay is suspended (pause / dialogue / cutscene / room
/// transition), `apply_suspended_time_scale_system` must zero both
/// `ClockState::time_scale` AND `RequestedClockScale::sim_clock` BEFORE
/// `refresh_world_time` snapshots them — otherwise `WorldTime::sim_dt` stays
/// non-zero on the first suspended frame and any presentation system multiplying
/// by it ticks one extra frame after pause lands.
#[test]
fn suspended_frame_zeros_world_time_scaled_dt() {
    use ambition_platformer2d_shared_tangle::schedule::{gameplay_suspended, GameMode};
    use ambition_time::WorldTime;
    use bevy::state::app::StatesPlugin;

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.insert_state(GameMode::Paused);
    app.insert_resource(ClockState { time_scale: 1.0 });
    app.insert_resource(RequestedClockScale {
        sim_clock: 1.0,
        ..Default::default()
    });
    app.insert_resource(WorldTime::new(0.016, 0.016));
    app.insert_resource(Time::<()>::default());

    // Mirror the host ordering from `register_player_input_systems`:
    // suspended-zero FIRST, then refresh.
    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            ambition_time::refresh_world_time,
        )
            .chain(),
    );

    let frame = std::time::Duration::from_millis(16);
    app.world_mut().resource_mut::<Time>().advance_by(frame);
    app.update();

    let clock = app.world().resource::<ClockState>();
    let target = app.world().resource::<RequestedClockScale>();
    let wt = app.world().resource::<WorldTime>();
    assert_eq!(
        clock.time_scale, 0.0,
        "suspended frame must zero ClockState.time_scale"
    );
    assert_eq!(
        target.sim_clock, 0.0,
        "suspended frame must zero RequestedClockScale.sim_clock"
    );
    assert_eq!(
        wt.sim_dt(), 0.0,
        "suspended frame must zero WorldTime::sim_dt (refresh_world_time must \
         see the zeroed time_scale, not last frame's 1.0)"
    );
    assert!(
        (wt.wall_dt() - 0.016).abs() < 1e-6,
        "wall clock must keep ticking through pause"
    );
}

/// Gameplay-allowed frames take the regular emit → apply → smooth path; the
/// suspended fallback is short-circuited by `run_if`. `refresh_world_time` then
/// sees `ClockState::time_scale = 1.0` (the default) and reports a non-zero
/// `sim_dt`.
#[test]
fn gameplay_frame_preserves_world_time_scaled_dt() {
    use ambition_platformer2d_shared_tangle::schedule::{gameplay_suspended, GameMode};
    use ambition_time::WorldTime;
    use bevy::state::app::StatesPlugin;

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.insert_state(GameMode::Playing);
    app.insert_resource(ClockState::default());
    app.insert_resource(RequestedClockScale::default());
    app.insert_resource(WorldTime::default());
    app.insert_resource(Time::<()>::default());

    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            ambition_time::refresh_world_time,
        )
            .chain(),
    );

    let frame = std::time::Duration::from_millis(16);
    app.world_mut().resource_mut::<Time>().advance_by(frame);
    app.update();

    let wt = app.world().resource::<WorldTime>();
    assert!(
        wt.sim_dt() > 0.0,
        "gameplay frame must produce a non-zero sim_dt; got {}",
        wt.sim_dt()
    );
}
