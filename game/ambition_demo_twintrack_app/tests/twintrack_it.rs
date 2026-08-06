use ambition_demo_twintrack::{
    LaboratoryTwin, TravelerTwin, TwinTrackExperiment, TwinTrackPhase, DOPPLER_PASSBAND_MAX,
    DOPPLER_PASSBAND_MIN, INVARIANT_SPEED, TARGET_SPEED,
};
use ambition_platformer2d::actor::BodyKinematics;
use ambition_platformer2d::relativity2d::{
    LightReceiver2d, LightSignal2d, ProperTimeElapsed, RelativisticTargetingView2d,
    RelativitySignalView2d, SpacetimeCoordinateTime2d, WorldlineHistoryView2d,
};
use ambition_platformer2d::sim::{drive_control_frame, ControlFrame};
use bevy::prelude::*;

fn activate(app: &mut App) {
    for _ in 0..30 {
        app.update();
        let traveler = {
            let mut query = app
                .world_mut()
                .query_filtered::<Entity, With<TravelerTwin>>();
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
    assert_eq!(signal_view.receivers.len(), 4);
    assert_eq!(signal_view.emitters.len(), 1);

    let worldline_view = app.world().resource::<WorldlineHistoryView2d>();
    assert!(worldline_view.tracks.contains_key("traveler"));
    assert!(worldline_view.tracks.contains_key("laboratory"));

    let targeting = app.world().resource::<RelativisticTargetingView2d>();
    let target = targeting
        .targets
        .iter()
        .find(|target| target.label == "chase_beacon")
        .expect("the moving beacon should publish a causal targeting solution");
    assert!(target.time_to_intercept > 0.0);
    assert!((target.observer_local_emission_direction.length() - 1.0).abs() < 1.0e-3);

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
    assert_eq!(receiver_count, 4);
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
        let mut query = app
            .world_mut()
            .query_filtered::<&mut BodyKinematics, With<TravelerTwin>>();
        let mut body = query.single_mut(app.world_mut()).unwrap();
        body.vel = Vec2::new(TARGET_SPEED, 0.0);
        app.update();
    }

    let (traveler, laboratory) = {
        let mut traveler_query = app
            .world_mut()
            .query_filtered::<&ProperTimeElapsed, With<TravelerTwin>>();
        let traveler = traveler_query.single(app.world()).unwrap().seconds;
        let mut lab_query = app
            .world_mut()
            .query_filtered::<&ProperTimeElapsed, With<LaboratoryTwin>>();
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
fn playable_signal_course_doppler_locks_leads_the_optical_target_and_reunites() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let mut doppler_fired = false;
    let mut pursuit_fired = false;
    for _ in 0..1_500 {
        let state = experiment(&mut app);
        if state.phase == TwinTrackPhase::Complete {
            break;
        }
        let body = traveler_body(&mut app);
        let should_fire_doppler = !doppler_fired
            && state.phase == TwinTrackPhase::DopplerLock
            && body.vel.x >= TARGET_SPEED * 0.98;
        doppler_fired |= should_fire_doppler;

        let pursuit_aim = if state.phase == TwinTrackPhase::Pursuit {
            app.world()
                .resource::<RelativisticTargetingView2d>()
                .targets
                .iter()
                .find(|target| target.label == "chase_beacon")
                .map(|target| target.observer_local_emission_direction)
        } else {
            None
        };
        let should_fire_pursuit = pursuit_aim.is_some();
        pursuit_fired |= should_fire_pursuit;
        let axis_x = if state.phase == TwinTrackPhase::Inbound {
            -1.0
        } else if state.phase == TwinTrackPhase::Pursuit {
            0.0
        } else {
            1.0
        };
        let (aim_x, aim_y) = pursuit_aim
            .map(|direction| (direction.x, -direction.y))
            .unwrap_or((0.0, 0.0));
        drive_control_frame(
            app.world_mut(),
            ControlFrame {
                axis_x,
                aim_x,
                aim_y,
                interact_pressed: should_fire_doppler,
                projectile_pressed: should_fire_pursuit,
                ..Default::default()
            },
        );
        app.update();
    }

    let state = experiment(&mut app);
    assert!(
        doppler_fired,
        "the scripted participant never reached emission speed"
    );
    assert!(
        pursuit_fired,
        "the scripted participant never received a targeting solution"
    );
    assert_eq!(state.phase, TwinTrackPhase::Complete);
    assert_ne!(state.qualified_packet_id, 0);
    assert!(state.doppler_frequency >= DOPPLER_PASSBAND_MIN);
    assert!(state.doppler_frequency <= DOPPLER_PASSBAND_MAX);
    assert!(state.doppler_arrival_time > 0.0);
    assert!(state.radar_arrival_time > state.doppler_arrival_time);
    assert!(state.echo_arrival_time > state.radar_arrival_time);
    assert!(state.echo_receiver_proper_time > 0.0);
    assert_eq!(state.pursuit_hits, 1);
    assert!(state.pursuit_hit_time > state.echo_arrival_time);
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
    assert!(qualified_arrivals
        .iter()
        .any(|arrival| { arrival.receiver_label == "radar_reflector" && arrival.reflected }));
    assert!(qualified_arrivals.iter().any(|arrival| {
        arrival.receiver_label == "traveler_echo_receiver" && arrival.signal_was_reflected
    }));
    assert!(signal_view
        .recent_arrivals
        .iter()
        .any(|arrival| arrival.receiver_label == "pursuit_target"));
}

#[test]
fn causal_targeting_separates_the_retarded_image_from_the_future_intercept() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let coordinate_time = {
        let mut query = app.world_mut().query::<&SpacetimeCoordinateTime2d>();
        query.single(app.world()).unwrap().seconds
    };
    {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut TwinTrackExperiment, With<LaboratoryTwin>>();
        let mut state = query.single_mut(app.world_mut()).unwrap();
        state.phase = TwinTrackPhase::Pursuit;
        state.pursuit_start_time = coordinate_time;
    }
    for _ in 0..180 {
        app.update();
    }

    let targeting = app.world().resource::<RelativisticTargetingView2d>();
    let target = targeting
        .targets
        .iter()
        .find(|target| target.label == "chase_beacon")
        .expect("the pursuit target should publish an intercept solution");
    let separation = target
        .apparent_to_intercept_angle
        .expect("the target should also have a retained retarded image");
    assert!(
        separation > 0.01,
        "retarded and intercept directions unexpectedly coincide"
    );
    assert!(target.optical_light_age.unwrap_or_default() > 0.0);
    assert!(target.time_to_intercept > 0.0);
}

#[test]
fn optical_view_solves_retarded_events_for_the_moving_beacon() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    for _ in 0..240 {
        app.update();
    }
    let view = app
        .world()
        .resource::<ambition_platformer2d::relativity2d::RelativisticOpticalView2d>();
    let observer = view
        .observer
        .as_ref()
        .expect("traveler should be the optical observer");
    let beacon = view
        .sources
        .iter()
        .find(|source| source.label == "chase_beacon")
        .expect("moving beacon should intersect the retained past light cone");
    assert!(beacon.emission_event.coordinate_time < observer.coordinate_time);
    assert!(beacon.light_age > 0.0);
    assert!(beacon.apparent_range > 0.0);
    assert!(beacon.doppler_factor.is_finite() && beacon.doppler_factor > 0.0);
    assert!((beacon.apparent_source_direction.length() - 1.0).abs() < 1.0e-3);
}
