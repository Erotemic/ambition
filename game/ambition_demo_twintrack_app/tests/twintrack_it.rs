use ambition_demo_twintrack::{
    LaboratoryTwin, TravelerTwin, TwinTrackExperiment, TwinTrackPhase, DOPPLER_PASSBAND_MAX,
    DOPPLER_PASSBAND_MIN, INVARIANT_SPEED, TARGET_SPEED,
};
use ambition_platformer2d::actor::BodyKinematics;
use ambition_platformer2d::relativity2d::{
    LightReceiver2d, LightSignal2d, ProperTimeElapsed, RelativitySignalView2d,
    SpacetimeCoordinateTime2d, WorldlineHistoryView2d,
};
use ambition_platformer2d::sim::{drive_control_frame, ControlFrame};
use bevy::prelude::*;

fn activate(app: &mut App) {
    for _ in 0..30 {
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

fn experiment(app: &mut App) -> TwinTrackExperiment {
    let mut query = app
        .world_mut()
        .query_filtered::<&TwinTrackExperiment, With<LaboratoryTwin>>();
    *query.single(app.world()).unwrap()
}

fn traveler_body(app: &mut App) -> BodyKinematics {
    let mut query = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<TravelerTwin>>();
    *query.single(app.world()).unwrap()
}

#[test]
fn provider_installs_clocks_signals_worldlines_and_minkowski_spacetime() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    for _ in 0..3 {
        app.update();
    }

    let clock_view = app
        .world()
        .resource::<ambition_platformer2d::relativity2d::RelativityClockView2d>();
    assert_eq!(clock_view.model_id, Some("minkowski"));
    assert!(clock_view.clocks.contains_key("traveler"));
    assert!(clock_view.clocks.contains_key("laboratory"));

    let signal_view = app.world().resource::<RelativitySignalView2d>();
    assert_eq!(signal_view.invariant_speed, f64::from(INVARIANT_SPEED));
    assert_eq!(signal_view.receivers.len(), 3);
    assert_eq!(signal_view.emitters.len(), 1);

    let worldline_view = app.world().resource::<WorldlineHistoryView2d>();
    assert!(worldline_view.tracks.contains_key("traveler"));
    assert!(worldline_view.tracks.contains_key("laboratory"));

    let coordinate_time_count = {
        let mut query = app.world_mut().query::<&SpacetimeCoordinateTime2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(coordinate_time_count, 1);

    let signal_pool_count = {
        let mut query = app.world_mut().query::<&LightSignal2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(signal_pool_count, 8);

    let receiver_count = {
        let mut query = app.world_mut().query::<&LightReceiver2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(receiver_count, 3);
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
            &mut BodyKinematics,
            With<TravelerTwin>,
        >();
        let mut body = query.single_mut(app.world_mut()).unwrap();
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
    assert!(
        (ratio - expected).abs() < 2.0e-3,
        "ratio={ratio}, expected={expected}"
    );
}

#[test]
fn playable_signal_course_doppler_locks_reflects_and_reunites() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let mut fired = false;
    for _ in 0..900 {
        let state = experiment(&mut app);
        if state.phase == TwinTrackPhase::Complete {
            break;
        }
        let body = traveler_body(&mut app);
        let should_fire = !fired
            && state.phase == TwinTrackPhase::DopplerLock
            && body.vel.x >= TARGET_SPEED * 0.98;
        fired |= should_fire;
        let axis_x = if state.phase == TwinTrackPhase::Inbound {
            -1.0
        } else {
            1.0
        };
        drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x,
                interact_pressed: should_fire,
                ..Default::default()
            },
        );
        app.update();
    }

    let state = experiment(&mut app);
    assert!(fired, "the scripted participant never reached emission speed");
    assert_eq!(state.phase, TwinTrackPhase::Complete);
    assert_ne!(state.qualified_packet_id, 0);
    assert!(state.doppler_frequency >= DOPPLER_PASSBAND_MIN);
    assert!(state.doppler_frequency <= DOPPLER_PASSBAND_MAX);
    assert!(state.doppler_arrival_time > 0.0);
    assert!(state.radar_arrival_time > state.doppler_arrival_time);
    assert!(state.echo_arrival_time > state.radar_arrival_time);
    assert!(state.echo_receiver_proper_time > 0.0);
    assert!(state.result_lab_time > state.result_traveler_time);

    let signal_view = app.world().resource::<RelativitySignalView2d>();
    let qualified_arrivals: Vec<_> = signal_view
        .recent_arrivals
        .iter()
        .filter(|arrival| arrival.packet_id == state.qualified_packet_id)
        .collect();
    assert!(qualified_arrivals.iter().any(|arrival| {
        arrival.receiver_label == "doppler_passband"
            && arrival.accepted
            && !arrival.signal_was_reflected
    }));
    assert!(qualified_arrivals.iter().any(|arrival| {
        arrival.receiver_label == "radar_reflector" && arrival.reflected
    }));
    assert!(qualified_arrivals.iter().any(|arrival| {
        arrival.receiver_label == "traveler_echo_receiver" && arrival.signal_was_reflected
    }));
}
