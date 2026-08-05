use ambition_demo_twintrack::{
    LaboratoryTwin, TravelerTwin, TwinTrackExperiment, TwinTrackPhase, INVARIANT_SPEED, LAB_X,
    TARGET_SPEED, TURNAROUND_X,
};
use ambition_platformer2d::actor::BodyKinematics;
use ambition_platformer2d::relativity2d::ProperTimeElapsed;
use bevy::prelude::*;

fn activate(app: &mut App) {
    for _ in 0..20 {
        app.update();
        let traveler = {
            let mut query = app.world_mut().query_filtered::<Entity, With<TravelerTwin>>();
            query.iter(app.world()).next()
        };
        if traveler.is_some() {
            return;
        }
    }
    panic!("TwinTrack did not activate within the test budget");
}

#[test]
fn provider_installs_two_clocks_and_minkowski_spacetime() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    for _ in 0..2 {
        app.update();
    }
    let view = app
        .world()
        .resource::<ambition_platformer2d::relativity2d::RelativityClockView2d>();
    assert_eq!(view.model_id, Some("minkowski"));
    assert!(view.clocks.contains_key("traveler"));
    assert!(view.clocks.contains_key("laboratory"));
}

#[test]
fn engine_clock_at_point_nine_c_matches_the_sr_rate() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    {
        let mut query = app.world_mut().query::<&mut ProperTimeElapsed>();
        for mut clock in query.iter_mut(app.world_mut()) {
            clock.reset();
        }
    }

    for _ in 0..120 {
        let mut query = app.world_mut().query_filtered::<
            (&mut BodyKinematics, &mut ProperTimeElapsed),
            With<TravelerTwin>,
        >();
        let (mut body, _clock) = query.single_mut(app.world_mut()).unwrap();
        body.pos.x = LAB_X;
        body.vel = Vec2::new(TARGET_SPEED, 0.0);
        app.update();
    }

    let (traveler, laboratory) = {
        let mut traveler_query = app.world_mut().query_filtered::<
            &ProperTimeElapsed,
            With<TravelerTwin>,
        >();
        let traveler = traveler_query.single(app.world()).unwrap().seconds;
        let mut lab_query = app.world_mut().query_filtered::<
            &ProperTimeElapsed,
            With<LaboratoryTwin>,
        >();
        let lab = lab_query.single(app.world()).unwrap().seconds;
        (traveler, lab)
    };
    let ratio = traveler / laboratory;
    let expected = (1.0 - f64::from(TARGET_SPEED / INVARIANT_SPEED).powi(2)).sqrt();
    assert!((ratio - expected).abs() < 2.0e-3, "ratio={ratio}, expected={expected}");
}

#[test]
fn forced_round_trip_reaches_a_reunion_result() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let force = |app: &mut App, x: f32, vx: f32| {
        let mut query = app.world_mut().query_filtered::<
            &mut BodyKinematics,
            With<TravelerTwin>,
        >();
        let mut body = query.single_mut(app.world_mut()).unwrap();
        body.pos.x = x;
        body.vel.x = vx;
    };

    force(&mut app, LAB_X + 100.0, TARGET_SPEED);
    app.update();
    force(&mut app, TURNAROUND_X + 1.0, TARGET_SPEED);
    app.update();
    force(&mut app, LAB_X + 10.0, -TARGET_SPEED);
    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<&TwinTrackExperiment, With<LaboratoryTwin>>();
    let experiment = query.single(app.world()).unwrap();
    assert_eq!(experiment.phase, TwinTrackPhase::Complete);
    assert!(experiment.result_lab_time > experiment.result_traveler_time);
}
