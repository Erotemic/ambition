use ambition_demo_twintrack::{
    LaboratoryTwin, TravelerTwin, TwinTrackCharacter, TwinTrackExperiment, TwinTrackIntroStep,
    TwinTrackPhase, TwinTrackRole, TwinTrackViewMode, COURIER_ID, DJ_ID, DJ_POS, DRIFTER_ID,
    INVARIANT_SPEED, LAB_POS, LIGHT_TAG_ROUNDS, SPINNER_ID, TAGGER_ID, TARGET_SPEED,
    VIEW_CONSOLE_POS,
};
use ambition_platformer2d::actor::BodyKinematics;
use ambition_platformer2d::engine_core::BodyAbilities;
use ambition_platformer2d::relativity2d::{
    LightEmitter2d, LightReceiver2d, LightSignal2d, ProperTimeCooldown2d, ProperTimeElapsed,
    RelativisticOpticalView2d, RelativisticTargetingView2d, RelativityClockView2d,
    RelativitySignalView2d, SpacetimeCoordinateTime2d, WorldlineHistoryView2d, WorldlineTrackId,
};
use ambition_platformer2d::sim::{drive_control_frame, ControlFrame};
use bevy::prelude::*;

fn activate(app: &mut App) {
    for _ in 0..45 {
        app.update();
        let traveler = {
            let mut query = app
                .world_mut()
                .query_filtered::<Entity, With<TravelerTwin>>();
            query.iter(app.world()).next()
        };
        if traveler.is_some() {
            for _ in 0..3 {
                app.update();
            }
            return;
        }
    }
    panic!("TwinTrack did not activate within the test budget");
}

fn step(app: &mut App, frame: ControlFrame) {
    drive_control_frame(app.world_mut(), frame);
    app.update();
}

fn idle(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        step(app, ControlFrame::default());
    }
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

fn set_traveler_state(app: &mut App, position: Vec2, velocity: Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<&mut BodyKinematics, With<TravelerTwin>>();
    let mut body = query.single_mut(app.world_mut()).unwrap();
    body.pos = position;
    body.vel = velocity;
}

fn character_body(app: &mut App, id: u8) -> BodyKinematics {
    let mut query = app
        .world_mut()
        .query::<(&TwinTrackCharacter, &BodyKinematics)>();
    query
        .iter(app.world())
        .find(|(character, _)| character.id == id)
        .map(|(_, body)| *body)
        .unwrap_or_else(|| panic!("TwinTrack character {id} is missing"))
}

fn wait_for_phase(app: &mut App, phase: TwinTrackPhase, budget: usize) {
    for _ in 0..budget {
        if experiment(app).phase == phase {
            return;
        }
        step(app, ControlFrame::default());
    }
    panic!(
        "TwinTrack did not enter {phase:?}; current phase is {:?}",
        experiment(app).phase
    );
}

fn wait_for_transmitter(app: &mut App) {
    for _ in 0..180 {
        let ready = {
            let mut query = app
                .world_mut()
                .query_filtered::<&ProperTimeCooldown2d, With<TravelerTwin>>();
            query.single(app.world()).unwrap().ready()
        };
        if ready {
            return;
        }
        step(app, ControlFrame::default());
    }
    panic!("traveler transmitter did not recharge");
}

fn complete_introduction(app: &mut App) {
    if experiment(app).phase != TwinTrackPhase::Introduction {
        return;
    }
    set_traveler_state(app, LAB_POS + Vec2::new(40.0, 0.0), Vec2::ZERO);
    step(
        app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(experiment(app).intro_step, TwinTrackIntroStep::Drift);

    set_traveler_state(app, LAB_POS + Vec2::new(260.0, 0.0), Vec2::ZERO);
    step(app, ControlFrame::default());
    assert_eq!(experiment(app).intro_step, TwinTrackIntroStep::Accelerate);

    set_traveler_state(
        app,
        LAB_POS + Vec2::new(320.0, 0.0),
        Vec2::new(INVARIANT_SPEED * 0.55, 0.0),
    );
    step(app, ControlFrame::default());
    wait_for_phase(app, TwinTrackPhase::ClockCensus, 10);
}

fn ask_character_for_clock(app: &mut App, id: u8, expected_mask: u8) {
    wait_for_transmitter(app);
    let target = character_body(app, id);
    set_traveler_state(app, target.pos - Vec2::new(80.0, 0.0), Vec2::ZERO);
    step(
        app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..240 {
        if experiment(app).clock_report_mask & expected_mask != 0 {
            return;
        }
        step(app, ControlFrame::default());
    }
    panic!("clock report from character {id} did not return over light");
}

fn complete_clock_census(app: &mut App) {
    complete_introduction(app);
    ask_character_for_clock(app, COURIER_ID, 0b001);
    ask_character_for_clock(app, DRIFTER_ID, 0b010);
    ask_character_for_clock(app, SPINNER_ID, 0b100);
    wait_for_phase(app, TwinTrackPhase::DopplerDance, 30);
}

fn complete_doppler_dance(app: &mut App) {
    wait_for_transmitter(app);
    set_traveler_state(app, DJ_POS - Vec2::new(150.0, 0.0), Vec2::ZERO);
    step(
        app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..90 {
        if experiment(app).doppler_frequency > 0.0 {
            break;
        }
        step(app, ControlFrame::default());
    }
    let rejected = experiment(app);
    assert_eq!(rejected.phase, TwinTrackPhase::DopplerDance);
    assert!(rejected.doppler_frequency < 190.0);

    wait_for_transmitter(app);
    set_traveler_state(app, DJ_POS - Vec2::new(150.0, 0.0), Vec2::new(360.0, 0.0));
    step(
        app,
        ControlFrame {
            axis_x: 0.667,
            interact_pressed: true,
            ..Default::default()
        },
    );
    wait_for_phase(app, TwinTrackPhase::LightTag, 180);
}

fn complete_light_tag(app: &mut App) {
    for expected_hit in 1..=LIGHT_TAG_ROUNDS {
        wait_for_transmitter(app);
        idle(app, 3);
        let aim = app
            .world()
            .resource::<RelativisticTargetingView2d>()
            .targets
            .iter()
            .find(|target| target.label == "Photon Fox")
            .map(|target| target.observer_local_emission_direction)
            .expect("Photon Fox should publish a local light-intercept direction");
        step(
            app,
            ControlFrame {
                aim_x: aim.x,
                aim_y: -aim.y,
                interact_pressed: true,
                ..Default::default()
            },
        );
        for _ in 0..240 {
            if experiment(app).tag_hits >= expected_hit {
                break;
            }
            step(app, ControlFrame::default());
        }
        assert_eq!(experiment(app).tag_hits, expected_hit);
    }
    wait_for_phase(app, TwinTrackPhase::Reunion, 30);
}

#[test]
fn guided_opening_requires_sync_drift_and_relativistic_speed() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    assert_eq!(experiment(&mut app).phase, TwinTrackPhase::Introduction);
    assert_eq!(
        experiment(&mut app).intro_step,
        TwinTrackIntroStep::Synchronize
    );
    complete_introduction(&mut app);
    assert_eq!(experiment(&mut app).phase, TwinTrackPhase::ClockCensus);
}

#[test]
fn provider_installs_a_two_dimensional_relativity_plaza() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let clock_view = app.world().resource::<RelativityClockView2d>();
    assert_eq!(clock_view.model_id, Some("minkowski"));
    for label in [
        "traveler",
        "laboratory",
        "Courier",
        "Drifter",
        "Spinner",
        "DJ Blue Shift",
        "Photon Fox",
    ] {
        assert!(
            clock_view.clocks.contains_key(label),
            "missing clock {label}"
        );
    }

    let signal_view = app.world().resource::<RelativitySignalView2d>();
    assert_eq!(signal_view.invariant_speed, f64::from(INVARIANT_SPEED));
    assert_eq!(signal_view.receivers.len(), 6);
    assert_eq!(signal_view.emitters.len(), 6);

    let worldline_view = app.world().resource::<WorldlineHistoryView2d>();
    for label in [
        "traveler",
        "laboratory",
        "Courier",
        "Drifter",
        "Spinner",
        "Photon Fox",
    ] {
        // Keyed by identity, not by caption (`a301a79a0`).
        assert!(
            worldline_view
                .tracks
                .contains_key(&WorldlineTrackId(label.to_owned())),
            "missing track {label}"
        );
    }

    let abilities = {
        let mut query = app
            .world_mut()
            .query_filtered::<&BodyAbilities, With<TravelerTwin>>();
        query.single(app.world()).unwrap().abilities
    };
    assert!(abilities.fly);
    assert!(abilities.interact);
    assert!(!abilities.jump);
    assert!(!abilities.fly_toggle);

    let coordinate_time_count = {
        let mut query = app.world_mut().query::<&SpacetimeCoordinateTime2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(coordinate_time_count, 1);

    let signal_pool_count = {
        let mut query = app.world_mut().query::<&LightSignal2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(signal_pool_count, 32);

    let receiver_count = {
        let mut query = app.world_mut().query::<&LightReceiver2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(receiver_count, 6);

    let emitter_count = {
        let mut query = app.world_mut().query::<&LightEmitter2d>();
        query.iter(app.world()).count()
    };
    assert_eq!(emitter_count, 6);
}

#[test]
fn shared_free_flight_moves_diagonally_and_remains_subluminal() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    set_traveler_state(&mut app, Vec2::new(300.0, 350.0), Vec2::ZERO);
    let start = traveler_body(&mut app).pos;

    for _ in 0..70 {
        step(
            &mut app,
            ControlFrame {
                axis_x: 1.0,
                axis_y: 0.65,
                ..Default::default()
            },
        );
    }

    let body = traveler_body(&mut app);
    assert!(body.pos.x > start.x + 40.0);
    assert!((body.pos.y - start.y).abs() > 25.0);
    assert!(body.vel.length() <= TARGET_SPEED + 1.0e-3);
    assert!(body.vel.length() < INVARIANT_SPEED);
}

#[test]
fn teaching_views_cycle_without_leaving_the_participant_trapped() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    complete_introduction(&mut app);
    set_traveler_state(&mut app, VIEW_CONSOLE_POS, Vec2::ZERO);

    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(experiment(&mut app).view_mode, TwinTrackViewMode::Optical);

    set_traveler_state(&mut app, Vec2::new(600.0, 400.0), Vec2::ZERO);
    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(experiment(&mut app).view_mode, TwinTrackViewMode::Spacetime);

    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(
        experiment(&mut app).view_mode,
        TwinTrackViewMode::Laboratory
    );
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
        set_traveler_state(
            &mut app,
            Vec2::new(600.0, 450.0),
            Vec2::new(TARGET_SPEED, 0.0),
        );
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
fn complete_plaza_course_exchanges_clock_dialogue_dances_tags_and_reunites() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    complete_clock_census(&mut app);
    let census = experiment(&mut app);
    assert_eq!(census.clock_report_mask, 0b111);
    assert!(census.last_report_clock > 0.0);
    assert!(census.last_receive_lab_time > census.last_message_departure_time);

    complete_doppler_dance(&mut app);
    let dance = experiment(&mut app);
    assert!((190.0..=202.0).contains(&dance.doppler_frequency));

    complete_light_tag(&mut app);
    assert_eq!(experiment(&mut app).tag_hits, LIGHT_TAG_ROUNDS);

    set_traveler_state(
        &mut app,
        Vec2::new(360.0, 450.0),
        Vec2::new(TARGET_SPEED, 0.0),
    );
    for _ in 0..60 {
        step(
            &mut app,
            ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
    }
    set_traveler_state(&mut app, LAB_POS + Vec2::new(60.0, 0.0), Vec2::ZERO);
    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );

    let final_state = experiment(&mut app);
    assert_eq!(final_state.phase, TwinTrackPhase::Complete);
    assert!(final_state.result_lab_time > final_state.result_traveler_time);

    let arrivals = &app
        .world()
        .resource::<RelativitySignalView2d>()
        .recent_arrivals;
    assert!(arrivals
        .iter()
        .any(|arrival| arrival.receiver_label.contains("Courier")));
    assert!(arrivals
        .iter()
        .any(|arrival| arrival.receiver_label.contains("Drifter")));
    assert!(arrivals
        .iter()
        .any(|arrival| arrival.receiver_label.contains("Spinner")));
    assert!(arrivals
        .iter()
        .any(|arrival| arrival.receiver_label.contains("DJ Blue Shift")));
    assert!(arrivals
        .iter()
        .any(|arrival| arrival.receiver_label.contains("Photon Fox")));
}

#[test]
fn photon_fox_visible_image_and_future_light_intercept_are_distinct() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    idle(&mut app, 180);

    let targeting = app.world().resource::<RelativisticTargetingView2d>();
    let target = targeting
        .targets
        .iter()
        .find(|target| target.label == "Photon Fox")
        .expect("Photon Fox should publish an intercept solution");
    let separation = target
        .apparent_to_intercept_angle
        .expect("Photon Fox should also have a light-delayed visible image");
    assert!(separation > 0.01);
    assert!(target.optical_light_age.unwrap_or_default() > 0.0);
    assert!(target.time_to_intercept > 0.0);
}

#[test]
fn optical_view_uses_an_earlier_light_event_for_a_moving_character() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    idle(&mut app, 240);

    let view = app.world().resource::<RelativisticOpticalView2d>();
    let observer = view
        .observer
        .as_ref()
        .expect("traveler should be the optical observer");
    let fox = view
        .sources
        .iter()
        .find(|source| source.label == "Photon Fox")
        .expect("Photon Fox should intersect the retained past light cone");
    assert!(fox.emission_event.coordinate_time < observer.coordinate_time);
    assert!(fox.light_age > 0.0);
    assert!(fox.apparent_range > 0.0);
    assert!(fox.doppler_factor.is_finite() && fox.doppler_factor > 0.0);
    assert!((fox.apparent_source_direction.length() - 1.0).abs() < 1.0e-3);
}

#[test]
fn completed_spacetime_view_can_scrub_the_replay_cursor() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    complete_clock_census(&mut app);
    complete_doppler_dance(&mut app);
    complete_light_tag(&mut app);
    set_traveler_state(&mut app, LAB_POS + Vec2::new(60.0, 0.0), Vec2::ZERO);
    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(experiment(&mut app).phase, TwinTrackPhase::Complete);

    {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut TwinTrackExperiment, With<LaboratoryTwin>>();
        let mut state = query.single_mut(app.world_mut()).unwrap();
        state.view_mode = TwinTrackViewMode::Spacetime;
        state.replay_cursor = 1.0;
    }
    for _ in 0..20 {
        step(
            &mut app,
            ControlFrame {
                axis_x: -1.0,
                ..Default::default()
            },
        );
    }
    assert!(experiment(&mut app).replay_cursor < 0.9);
}

#[test]
fn every_plaza_character_has_a_distinct_role_and_receiver() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let rows = {
        let mut query = app.world_mut().query::<&TwinTrackCharacter>();
        query.iter(app.world()).cloned().collect::<Vec<_>>()
    };
    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter()
            .filter(|row| row.role == TwinTrackRole::ClockCitizen)
            .count(),
        3
    );
    assert!(rows
        .iter()
        .any(|row| row.id == DJ_ID && row.role == TwinTrackRole::DopplerDj));
    assert!(rows
        .iter()
        .any(|row| row.id == TAGGER_ID && row.role == TwinTrackRole::LightTagger));
    let mut channels = rows
        .iter()
        .map(|row| row.receiver_channel)
        .collect::<Vec<_>>();
    channels.sort_unstable();
    channels.dedup();
    assert_eq!(channels.len(), rows.len());
}
