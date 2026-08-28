use ambition_demo_twintrack::{
    beacon_alpha_position, beacon_midpoint, beacon_omega_position, pulse_emission_position,
    twintrack_room, EventOrdering, LaboratoryTwin, PulseRay, TravelerTwin, TwinTrackCharacter,
    TwinTrackDualObserverView, TwinTrackExperiment, TwinTrackIntroStep, TwinTrackLightPulseView,
    TwinTrackPhase, TwinTrackRole, TwinTrackViewMode, BEACON_HALF_SEPARATION, COURIER_ID, DJ_ID,
    DJ_POS, DRIFTER_ID, INVARIANT_SPEED, LAB_POS, LIGHT_TAG_ROUNDS, ROOM_HEIGHT, ROOM_WIDTH,
    SPEED_INVARIANCE_TOLERANCE, SPINNER_ID, TAGGER_ID, TARGET_SPEED, VIEW_CONSOLE_POS,
};
use ambition_platformer2d::actor::BodyKinematics;
use ambition_platformer2d::engine_core::BodyAbilities;
use ambition_platformer2d::relativity2d::{
    LightEmitter2d, LightReceiver2d, LightSignal2d, ProperTimeCooldown2d, ProperTimeElapsed,
    RelativisticOpticalView2d, RelativisticTargetingView2d, RelativityClockView2d,
    RelativitySignalView2d, SpacetimeCoordinateTime2d, WorldlineHistoryView2d, WorldlineTrackId,
};
use ambition_platformer2d::sim::{drive_control_frame, drive_slot_frame, ControlFrame, PlayerSlot};
use ambition_platformer2d::sim_view::{LocalView, LocalViewId, ViewPlacement, ViewSubject};
use ambition_platformer2d::world::rooms::CameraClampMode;
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

/// PROBE — what moves the laboratory twin, who is supposed to be at rest.
///
/// Five tests in this file failed on her drift from at least `a945c1de5` until
/// 2026-08-28, and this probe is what found the cause. Kept because the numbers
/// below are the reproduction, and because two earlier readings of them were
/// wrong in instructive ways.
///
/// ⭐⭐ WHAT SHE ACTUALLY DID: appeared on tick 3 already carrying
/// `vel = (-95.98, 0)`, drag ate it over seven ticks, and she stopped 6.16px LEFT
/// of the `LAB_POS` she is placed at. Her `y` reads `446.015` from the first tick
/// she is visible — 3.98px above the authored 450, with zero `y` velocity — which
/// is construction resolving a standing centre and is not part of the defect.
///
/// ⛔ FIRST WRONG READING: *"the traveler is standing inside her, and two bodies
/// separate"*. Refuted by a control — moving the room's spawn point 96px away
/// leaves her drift BYTE-IDENTICAL. That control was right and worth keeping.
///
/// ⛔⛔ SECOND WRONG READING, and it survived longer because the first control
/// made it look careful: *"ONE IMPULSE AT CONSTRUCTION, not a force and not a
/// walk — the velocity only decays"*. It IS a walk. Spawning a second body from
/// the same request shape 420px away showed it accelerating -96, -194, -294, -398,
/// -506 and pinning at the -540 cap, walking left forever: a seatless `Passive`
/// NPC is what the engine calls an "undescribed-pool STROLLER". The decay in this
/// probe was never drag on an impulse — it was drag on the ONE stroll step she
/// gets before her seat arrives.
///
/// ⇒ **the gap is a commands flush.** `adopt_the_laboratory_twin` QUEUES
/// `DrivingParticipant`, and one tick of her life happens before it lands.
/// `restore_the_laboratory_twins_mark` puts her back, and the reason it is a
/// separate system on `Added` rather than a line inside the adoption is that the
/// adoption samples her BEFORE the step it needs to undo — measured, it read
/// `720.0` with zero velocity and corrected nothing.
#[test]
#[ignore = "PROBE, print-only: what moves the laboratory twin"]
fn probe_what_moves_the_laboratory_twin() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    for tick in 0..50 {
        app.update();
        let lab = {
            let mut q = app
                .world_mut()
                .query_filtered::<&BodyKinematics, With<LaboratoryTwin>>();
            q.iter(app.world()).next().copied()
        };
        let traveler = {
            let mut q = app
                .world_mut()
                .query_filtered::<&BodyKinematics, With<TravelerTwin>>();
            q.iter(app.world()).next().copied()
        };
        if let Some(lab) = lab {
            println!(
                "TWIN {tick:>3} lab pos={:?} vel={:?} | traveler pos={:?}",
                lab.pos,
                lab.vel,
                traveler.map(|t| t.pos)
            );
        }
    }
}

fn laboratory_body(app: &mut App) -> BodyKinematics {
    let mut query = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<LaboratoryTwin>>();
    *query.single(app.world()).unwrap()
}

/// What each view's camera resolve decided to look at, in ascending id order.
fn follow_points(app: &mut App) -> Vec<Vec2> {
    use ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot;
    let mut query = app
        .world_mut()
        .query_filtered::<(&LocalViewId, &ResolvedCameraSnapshot), With<LocalView>>();
    let mut rows: Vec<(LocalViewId, Vec2)> = query
        .iter(app.world())
        .map(|(id, resolved)| (*id, resolved.follow_world))
        .collect();
    rows.sort_by_key(|(id, _)| *id);
    rows.into_iter().map(|(_, point)| point).collect()
}

/// Every local view as `(id, placement, subject)`, in ascending id order.
fn views(app: &mut App) -> Vec<(LocalViewId, ViewPlacement, Option<Entity>)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&LocalViewId, Option<&ViewPlacement>, Option<&ViewSubject>), With<LocalView>>();
    let mut rows: Vec<(LocalViewId, ViewPlacement, Option<Entity>)> = query
        .iter(app.world())
        .map(|(id, placement, subject)| {
            (
                *id,
                placement.copied().unwrap_or_default(),
                subject.map(|subject| subject.0),
            )
        })
        .collect();
    rows.sort_by_key(|(id, _, _)| *id);
    rows
}

/// WHERE A PANE ACTUALLY POINTED, read off the snapshot the camera resolve
/// published for that view.
fn pane_follow_point(app: &mut App, id: LocalViewId) -> Option<Vec2> {
    let mut query = app.world_mut().query_filtered::<(
        &LocalViewId,
        &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot,
    ), With<LocalView>>();
    query
        .iter(app.world())
        .find(|(view, _)| **view == id)
        .map(|(_, resolved)| resolved.follow_world)
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

/// The traveler's own row in the per-observer targeting view.
fn traveler_aim(app: &mut App, label: &str) -> Option<Vec2> {
    let traveler = {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<TravelerTwin>>();
        query.single(app.world()).ok()?
    };
    app.world()
        .resource::<RelativisticTargetingView2d>()
        .for_observer(traveler)?
        .targets
        .iter()
        .find(|target| target.label == label)
        .map(|target| target.observer_local_emission_direction)
}

fn complete_light_tag(app: &mut App) {
    for expected_hit in 1..=LIGHT_TAG_ROUNDS {
        wait_for_transmitter(app);
        idle(app, 3);
        // the TRAVELER's aim, named. The targeting view holds one row per
        // observer, and reading it through `Deref` takes the first row in label
        // order — which is the LABORATORY twin's, since Emmy became an observer
        // too. She stands at the other end of the plaza, so her intercept
        // direction points somewhere the traveler's shot cannot land.
        let aim = traveler_aim(app, "Photon Fox")
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
fn view_console_toggles_optical_without_replacing_gameplay_with_spacetime() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    complete_introduction(&mut app);
    set_traveler_state(&mut app, VIEW_CONSOLE_POS, Vec2::ZERO);

    // The relativity exhibit is what the console offers first; the optical
    // view is the second stop on the same cycle.
    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(
        experiment(&mut app).view_mode,
        TwinTrackViewMode::SplitObservers
    );

    // Release Interact for a frame so the next press is a fresh edge rather
    // than a held button.
    idle(&mut app, 1);
    set_traveler_state(&mut app, VIEW_CONSOLE_POS, Vec2::ZERO);
    step(
        &mut app,
        ControlFrame {
            interact_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(experiment(&mut app).view_mode, TwinTrackViewMode::Optical);

    // Outside light tag, Interact exits the optical presentation directly. The
    // 3D spacetime surface is a concurrent minimap now, not a gameplay mode.
    set_traveler_state(&mut app, LAB_POS + Vec2::new(80.0, 0.0), Vec2::ZERO);
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
fn plaza_is_centered_open_and_camera_follow_is_unclamped() {
    let room = twintrack_room();
    assert!(
        room.world.blocks.is_empty(),
        "TwinTrack should not have perimeter walls"
    );
    assert_eq!(LAB_POS, Vec2::new(ROOM_WIDTH * 0.5, ROOM_HEIGHT * 0.5));
    assert!(room.world.edges.fall > 1_000_000.0);

    let open_follow = room
        .camera_zones
        .iter()
        .find(|zone| zone.id == "twintrack_open_follow")
        .expect("TwinTrack should author its open follow-camera policy");
    assert_eq!(open_follow.clamp_mode, CameraClampMode::None);
    assert!(open_follow.aabb.min.x < -1_000_000.0);
    assert!(open_follow.aabb.max.x > 1_000_000.0);

    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    for id in [COURIER_ID, DRIFTER_ID, SPINNER_ID] {
        let body = character_body(&mut app, id);
        assert!(
            body.pos.distance(LAB_POS) < 500.0,
            "clock racer {id} spawned too far from the centered lab: {:?}",
            body.pos
        );
    }
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
    assert!(
        census.last_receive_lab_time > census.last_message_departure_time,
        "the reply reached the traveler before it was sent: lab clock read \
         {} at the arrival, and the message left at coordinate time {}",
        census.last_receive_lab_time,
        census.last_message_departure_time,
    );

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

    // The TRAVELER's own intercept — the plaza has two observers, and the other
    // one is standing still at the far end of it.
    let traveler = {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<TravelerTwin>>();
        query.single(app.world()).expect("the traveler is seated")
    };
    let targeting = app.world().resource::<RelativisticTargetingView2d>();
    let target = targeting
        .for_observer(traveler)
        .expect("the traveler publishes its own aim")
        .targets
        .iter()
        .find(|target| target.label == "Photon Fox")
        .expect("Photon Fox should publish an intercept solution");
    let separation = target
        .apparent_to_intercept_angle
        .expect("Photon Fox should also have a light-delayed visible image");
    assert!(
        separation > 0.01,
        "the visible image and the intercept solution have collapsed onto each \
         other ({separation:.5} rad). That is geometry, not physics: Photon Fox \
         is flying almost straight along the observer's line of sight, so there \
         is no transverse motion to lead and the SR-3 overlay's red, green and \
         cyan markers all land on the same spot. Check that TAG_START_POS sits \
         well off LAB_POS.y."
    );
    assert!(target.optical_light_age.unwrap_or_default() > 0.0);
    assert!(target.time_to_intercept > 0.0);
}

#[test]
fn optical_view_uses_an_earlier_light_event_for_a_moving_character() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    idle(&mut app, 240);

    let traveler = {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<TravelerTwin>>();
        query.single(app.world()).expect("the traveler is seated")
    };
    let view = app
        .world()
        .resource::<RelativisticOpticalView2d>()
        .for_observer(traveler)
        .expect("the traveler publishes its own sky");
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

fn dual_observer_view(app: &mut App) -> TwinTrackDualObserverView {
    app.world().resource::<TwinTrackDualObserverView>().clone()
}

/// Pin the traveler onto the beacons' midpoint with a chosen velocity and run
/// until both observers have a reading of the same flash pair.
///
/// re-pinned every tick on purpose. Free flight drags the velocity down
/// and carries the body off the midpoint, and both of those would quietly turn
/// this into a test about distance instead of a test about frames.
fn settle_dual_observer(app: &mut App, velocity: Vec2) -> TwinTrackDualObserverView {
    let mut last = TwinTrackDualObserverView::default();
    for _ in 0..1_200 {
        set_traveler_state(app, beacon_midpoint(), velocity);
        idle(app, 1);
        last = dual_observer_view(app);
        // both panes must be reading ONE pair of events. Two panes
        // comparing two different flash pairs would "disagree" for a reason
        // that has nothing to do with relativity, so this is a precondition of
        // the measurement rather than part of it.
        if last.compares_the_same_flash_pair() {
            return last;
        }
    }
    panic!(
        "the two TwinTrack observers never read one flash pair together: lab {:?}, traveler {:?}",
        last.laboratory.as_ref().map(|row| row.compared_flash_index),
        last.traveler.as_ref().map(|row| row.compared_flash_index),
    );
}

#[test]
fn the_two_beacons_straddle_the_laboratory_twin_symmetrically() {
    // The laboratory observer's "simultaneous" answer is only meaningful
    // because it is genuinely equidistant. If a relayout moves the lab, the
    // exhibit stops being an exhibit and this says so.
    assert_eq!(beacon_midpoint(), LAB_POS);
    assert!((beacon_alpha_position().distance(LAB_POS) - BEACON_HALF_SEPARATION).abs() < 1.0e-3);
    assert!((beacon_omega_position().distance(LAB_POS) - BEACON_HALF_SEPARATION).abs() < 1.0e-3);
    assert!(beacon_alpha_position().x < beacon_omega_position().x);
}

#[test]
fn two_observers_report_different_orderings_of_the_same_flash_pair() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let view = settle_dual_observer(&mut app, Vec2::new(0.8 * INVARIANT_SPEED, 0.0));

    let lab = view.laboratory.as_ref().expect("lab twin should observe");
    let traveler = view.traveler.as_ref().expect("traveler should observe");

    // The observer at rest and equidistant: both flashes happened together.
    assert_eq!(lab.frame_order, EventOrdering::Simultaneous);
    assert_eq!(lab.seen_order, EventOrdering::Simultaneous);
    // The observer moving toward Omega: Omega happened first.
    assert_eq!(traveler.frame_order, EventOrdering::OmegaFirst);
    assert!(traveler.beta > 0.5, "traveler should be relativistic");

    assert!(
        view.frame_orders_disagree(),
        "the two panes must disagree; lab said {:?} and the traveler said {:?}",
        lab.frame_order,
        traveler.frame_order,
    );
}

#[test]
fn reversing_the_traveler_reverses_which_flash_it_says_happened_first() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let toward_omega = settle_dual_observer(&mut app, Vec2::new(0.8 * INVARIANT_SPEED, 0.0));
    let toward_alpha = settle_dual_observer(&mut app, Vec2::new(-0.8 * INVARIANT_SPEED, 0.0));

    let omega_first = toward_omega
        .traveler
        .as_ref()
        .expect("traveler should observe")
        .frame_order;
    let alpha_first = toward_alpha
        .traveler
        .as_ref()
        .expect("traveler should observe")
        .frame_order;
    assert_eq!(omega_first, EventOrdering::OmegaFirst);
    assert_eq!(alpha_first, EventOrdering::AlphaFirst);

    // ...and the observer at rest gave the same answer both times, so the flip
    // is a property of the traveler's frame rather than of the flashes.
    for view in [&toward_omega, &toward_alpha] {
        assert_eq!(
            view.laboratory
                .as_ref()
                .expect("lab twin should observe")
                .frame_order,
            EventOrdering::Simultaneous,
        );
    }
}

#[test]
fn the_frame_disagreement_is_not_a_repackaged_light_delay() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let view = settle_dual_observer(&mut app, Vec2::new(0.8 * INVARIANT_SPEED, 0.0));
    let traveler = view.traveler.as_ref().expect("traveler should observe");

    // The traveler sits where the light-delay answer is (almost) a tie, so the
    // ordering it reports cannot be coming from being nearer one beacon.
    let seen = traveler.seen_split_seconds().abs();
    let framed = traveler.frame_split_seconds().abs();
    assert!(
        seen < 0.1,
        "the traveler should be effectively equidistant, but its arrival split was {seen} s",
    );
    assert!(
        framed > 1.0,
        "the traveler's own-frame split should be a large fraction of a second, was {framed} s",
    );
}

fn light_pulse_view(app: &mut App) -> TwinTrackLightPulseView {
    app.world().resource::<TwinTrackLightPulseView>().clone()
}

/// Pin the traveler onto the emitter with a chosen velocity and run until both
/// observers are reading the SAME flare.
///
/// re-pinned every tick for the same reason `settle_dual_observer` is.
/// Free flight drags the velocity down, and a decayed velocity would turn a
/// test about the second postulate into a test about a slow observer.
fn settle_light_pulse(app: &mut App, velocity: Vec2) -> TwinTrackLightPulseView {
    let mut last = TwinTrackLightPulseView::default();
    for _ in 0..1_200 {
        set_traveler_state(app, pulse_emission_position(), velocity);
        idle(app, 1);
        last = light_pulse_view(app);
        if last.compares_the_same_pulse() {
            return last;
        }
    }
    panic!(
        "the two TwinTrack observers never read one flare together: lab {:?}, traveler {:?}",
        last.laboratory.as_ref().map(|row| row.pulse_index),
        last.traveler.as_ref().map(|row| row.pulse_index),
    );
}

#[test]
fn both_observers_measure_the_light_pulse_at_the_invariant_speed() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let view = settle_light_pulse(&mut app, Vec2::new(0.9 * INVARIANT_SPEED, 0.0));

    let lab = view.laboratory.as_ref().expect("lab twin should observe");
    let traveler = view.traveler.as_ref().expect("traveler should observe");

    // The premise. Without a real relative speed the invariance claim below is
    // a statement about two observers who agree for the boring reason.
    assert!(
        lab.beta < 1.0e-4,
        "the lab twin should be at rest, was {}",
        lab.beta
    );
    // a band, not a point. Flight drag bleeds a few units per tick off the
    // pinned velocity between the pin and the read, and pinning the number
    // exactly would make this a test about drag.
    assert!(
        (0.85..=0.95).contains(&traveler.beta),
        "the traveler should be flying near 0.9c, was {}",
        traveler.beta,
    );

    for report in [lab, traveler] {
        for ray in PulseRay::ALL {
            let measurement = report.ray(ray);
            assert!(
                (measurement.measured_speed_fraction - 1.0).abs() <= SPEED_INVARIANCE_TOLERANCE,
                "{} measured {ray:?} at {} c",
                report.label,
                measurement.measured_speed_fraction,
            );
        }
    }
    assert!(view.speed_is_invariant_for_both());

    // the falsifier that makes this more than "the pulse moved": a fast
    // projectile would hand the traveler `c - v` for the ray it chases.
    let galilean = f64::from(INVARIANT_SPEED) * (1.0 - f64::from(traveler.beta));
    let chased = traveler.ray(PulseRay::TowardOmega).measured_speed;
    assert!(
        (chased - galilean).abs() > 0.5 * f64::from(INVARIANT_SPEED),
        "the traveler measured the chased ray at {chased}, near the velocity-addition \
         answer {galilean}; the pulse is being integrated as an ordinary projectile",
    );
}

#[test]
fn the_two_observers_disagree_about_the_pulses_direction_and_colour() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let view = settle_light_pulse(&mut app, Vec2::new(0.9 * INVARIANT_SPEED, 0.0));

    let lab = view.laboratory.as_ref().expect("lab twin should observe");
    let traveler = view.traveler.as_ref().expect("traveler should observe");

    // Aberration: the crosswise ray leaves the laboratory square to the axis
    // and the traveler measures it swept far around toward its own tail.
    let lab_cross = lab.ray(PulseRay::Crosswise);
    let traveler_cross = traveler.ray(PulseRay::Crosswise);
    assert!((lab_cross.apparent_angle_degrees - 90.0).abs() < 1.0e-2);
    assert!(
        (traveler_cross.apparent_angle_degrees - lab_cross.apparent_angle_degrees).abs() > 45.0,
        "the two panes should place the crosswise ray tens of degrees apart, got {} and {}",
        lab_cross.apparent_angle_degrees,
        traveler_cross.apparent_angle_degrees,
    );
    assert!(view.directions_disagree());

    // Doppler: the same ray is one colour in one pane and another in the other,
    // blueshifted head-on and redshifted when chased.
    for measurement in &lab.rays {
        assert!((measurement.doppler_factor - 1.0).abs() < 1.0e-6);
    }
    assert!(traveler.ray(PulseRay::TowardAlpha).doppler_factor > 2.0);
    assert!(traveler.ray(PulseRay::TowardOmega).doppler_factor < 0.5);
    assert!(view.doppler_factors_disagree());

    // ...and both panes are still talking about ONE emission event, so none of
    // the above is two observers comparing two different flares.
    assert_eq!(lab.pulse_index, traveler.pulse_index);
    assert_eq!(
        lab.emission_coordinate_time,
        traveler.emission_coordinate_time
    );
}

#[test]
fn the_two_observers_time_one_light_cone_arrival_differently() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let view = settle_light_pulse(&mut app, Vec2::new(0.9 * INVARIANT_SPEED, 0.0));

    let lab = view.laboratory.as_ref().expect("lab twin should observe");
    let traveler = view.traveler.as_ref().expect("traveler should observe");

    // Whether the light got to Omega is not up for debate.
    assert_eq!(
        lab.omega_arrival_coordinate_time,
        traveler.omega_arrival_coordinate_time,
    );
    let crossing = f64::from(BEACON_HALF_SEPARATION) / f64::from(INVARIANT_SPEED);
    assert!(
        (lab.omega_arrival_coordinate_time - lab.emission_coordinate_time - crossing).abs()
            < 1.0e-6,
    );
    // WHEN it got there is.
    assert!(
        (lab.omega_arrival_frame_seconds - traveler.omega_arrival_frame_seconds).abs() > 0.5,
        "the two observers timed one arrival at {} s and {} s",
        lab.omega_arrival_frame_seconds,
        traveler.omega_arrival_frame_seconds,
    );
}

/// TWO PARTICIPANTS, TWO BODIES, ONE SIMULATION — and each seat moves
/// only its own.
///
/// The exhibit's whole claim is that two observers of one Minkowski simulation
/// disagree, and until only one of those observers was a person: the
/// laboratory twin was a bare entity with a clock and no way to be driven.
///
/// the falsifier is the OTHER body in every direction. A composition that
/// routed both seats through one control frame — which is what a second
/// participant that never got its own `InputParticipant` would degrade to —
/// passes "the twin moved" and fails here, because the traveler moves with her.
/// Both halves are asserted both ways round for that reason.
#[test]
fn each_seat_moves_its_own_body_and_leaves_the_others_alone() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let twin_start = laboratory_body(&mut app).pos;
    let traveler_start = traveler_body(&mut app).pos;

    // Seat one presses right; seat zero presses nothing.
    for _ in 0..45 {
        drive_slot_frame(
            app.world_mut(),
            PlayerSlot(1),
            ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        step(&mut app, ControlFrame::default());
    }
    let twin_moved = laboratory_body(&mut app).pos;
    let traveler_held = traveler_body(&mut app).pos;
    assert!(
        twin_moved.x - twin_start.x > 20.0,
        "seat one pressed right for 45 ticks and the laboratory twin went from \
         {twin_start} to {twin_moved}: the second participant is not driving her",
    );
    assert!(
        (traveler_held - traveler_start).length() < 1.0,
        "seat one's input moved the TRAVELER as well ({traveler_start} → \
         {traveler_held}): the two seats are sharing one control frame",
    );

    // And back the other way, in a FRESH plaza. not a continuation of the run
    // above: free flight has drag rather than a brake, so a twin that was just
    // pushed for 45 ticks is still coasting, and "did the other body move" then
    // measures the previous phase instead of this one.
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    let twin_start = laboratory_body(&mut app).pos;
    let traveler_start = traveler_body(&mut app).pos;
    for _ in 0..45 {
        step(
            &mut app,
            ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
    }
    let traveler_moved = traveler_body(&mut app).pos;
    assert!(
        traveler_moved.x - traveler_start.x > 20.0,
        "seat zero pressed right and the traveler went from {traveler_start} to \
         {traveler_moved}",
    );
    let twin_held = laboratory_body(&mut app).pos;
    assert!(
        (twin_held - twin_start).length() < 1.0,
        "seat zero's input moved the LABORATORY TWIN as well ({twin_start} → \
         {twin_held}): the two seats are sharing one control frame",
    );
}

/// THE SPLIT IS THE SHAPE OF THE GAME, not a view mode you have to find.
///
/// existed, as `TwinTrackViewMode::SplitObservers`, reachable only by walking to
/// an in-world console and cycling it — and even then it was an opaque diagram
/// over the top of one gameplay camera, not two gameplay views.
///
/// the two placements must be DISJOINT, which is the part a shared rectangle
/// passes. Two views both claiming the whole display draw on top of each other
/// and look, from any single-view assertion, entirely healthy.
#[test]
fn the_plaza_opens_split_between_its_two_participants() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let rows = views(&mut app);
    assert_eq!(
        rows.len(),
        2,
        "TwinTrack seats two participants, so it publishes two local views; found {rows:?}",
    );
    let (left, right) = (rows[0].1, rows[1].1);
    assert!(
        left.max.x <= right.min.x + f32::EPSILON,
        "the two panes overlap ({left:?} and {right:?}), so both observers are \
         drawing into the same rectangle",
    );
    assert!(
        left.min.x <= f32::EPSILON && right.max.x >= 1.0 - f32::EPSILON,
        "the two panes leave part of the display unclaimed ({left:?} and {right:?})",
    );

    let twin_pos = laboratory_body(&mut app).pos;
    let framed =
        pane_follow_point(&mut app, LocalViewId(1)).expect("the right pane resolved a camera");
    assert!(
        (framed - twin_pos).length() < 1.0,
        "the right pane framed {framed} while the laboratory twin is at \
         {twin_pos} — it is following whatever the session is following, not \
         its own participant",
    );
    // and the LEFT pane names nobody on purpose: a view with no `ViewSubject`
    // frames the session's controlled body, which is seat zero's — including
    // while that seat is possessing something else. Naming it here would be a
    // second answer to a question the engine already answers.
    assert_eq!(
        rows[0].2, None,
        "the traveler's pane names its own subject, which is a second authority \
         on who seat zero is driving",
    );

    // Two views naming two bodies must resolve two follow points.
    set_traveler_state(&mut app, LAB_POS + Vec2::new(900.0, 0.0), Vec2::ZERO);
    idle(&mut app, 4);
    let follows = follow_points(&mut app);
    let separation = (follows[0] - follows[1]).length();
    assert!(
        separation > 400.0,
        "the two panes resolved follow points {} and {} — {separation} apart — \
         while their subjects are {} apart: the camera resolve is still \
         answering \"who is everybody watching\" once for every view",
        follows[0],
        follows[1],
        (traveler_body(&mut app).pos - laboratory_body(&mut app).pos).length(),
    );
    assert!(
        (follows[0] - traveler_body(&mut app).pos).length() < 64.0,
        "the traveler's pane is not framing the traveler",
    );
    assert!(
        (follows[1] - laboratory_body(&mut app).pos).length() < 64.0,
        "the laboratory twin's pane is not framing the laboratory twin",
    );
}

/// ONE CONTROLLER IS A COMPLETE, SUPPORTED SESSION.
///
/// do nothing and have the character be uncontrolled"* — and then *"it will
/// still be useful to watch it as an observer."* Both sentences are asserted
/// here: nothing is driving the twin, so she does not move, and her pane keeps
/// framing her regardless.
///
/// the failure this forbids is a seat with no pad reading somebody ELSE's
/// pad. `assign_local_seat_devices` clears an association it cannot satisfy
/// rather than falling back to any-pad, which is exactly the leafwing default
/// that would have made player one's stick move both bodies.
#[test]
fn with_nobody_in_the_second_seat_the_twin_stands_still_and_stays_watched() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);

    let twin = laboratory_body(&mut app);
    idle(&mut app, 90);
    let after = laboratory_body(&mut app);
    assert!(
        (after.pos - twin.pos).length() < 1.0,
        "an unattended laboratory twin drifted from {} to {}",
        twin.pos,
        after.pos,
    );

    let framed =
        pane_follow_point(&mut app, LocalViewId(1)).expect("the second pane resolved a camera");
    assert!(
        (framed - after.pos).length() < 1.0,
        "the second pane framed {framed} while the unattended twin is at {} — \
         it stopped watching her because nobody was driving her, and an \
         unattended observer is still an observer",
        after.pos,
    );
}

/// A PANE BELONGS TO A PERSON, NOT TO A BODY.
///
/// the seat is moved directly rather than through a possession, because
/// what is under test is the VIEW's resolution, not any particular way of
/// changing who drives what. `DrivingParticipant` is the one authority either
/// road goes through.
///
/// the two positions must DIFFER, or a pane that never moved would pass.
#[test]
fn the_second_pane_follows_its_participant_to_a_new_body() {
    use ambition_demo_twintrack::LAB_TWIN_SLOT;
    use ambition_platformer2d::characters::control::DrivingParticipant;

    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    idle(&mut app, 4);

    let twin_pos = laboratory_body(&mut app).pos;
    let framed = pane_follow_point(&mut app, LocalViewId(1)).expect("the second pane has a camera");
    assert!(
        (framed - twin_pos).length() < 1.0,
        "precondition: the second pane starts on the laboratory twin",
    );

    // Somebody else in the plaza, standing somewhere else.
    let (target, target_pos) = {
        let mut query = app
            .world_mut()
            .query::<(Entity, &TwinTrackCharacter, &BodyKinematics)>();
        query
            .iter(app.world())
            .find(|(_, character, body)| {
                character.id == DJ_ID && (body.pos - twin_pos).length() > 32.0
            })
            .map(|(entity, _, body)| (entity, body.pos))
            .expect("the DJ stands somewhere other than the laboratory twin")
    };

    // Participant one takes over that body.
    let twin = {
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<LaboratoryTwin>>();
        query.single(app.world()).unwrap()
    };
    app.world_mut()
        .entity_mut(twin)
        .remove::<DrivingParticipant>();
    app.world_mut()
        .entity_mut(target)
        .insert(DrivingParticipant(LAB_TWIN_SLOT));
    idle(&mut app, 4);

    let framed = pane_follow_point(&mut app, LocalViewId(1)).expect("the second pane has a camera");
    assert!(
        (framed - target_pos).length() < 4.0,
        "the second pane framed {framed} after its participant moved to a body \
         at {target_pos}; the laboratory twin is at {twin_pos}. The pane is \
         following a BODY rather than the person it belongs to",
    );
}

/// A SEAT COUNT WITHOUT AN ASSIGNMENT POLICY IS A DEAD SECOND SEAT.
///
/// this asserts the DECLARATION, and says so. The integration suite builds
/// without the `input` feature: there are no `InputParticipant`s, no gamepads and
/// no device-assignment pass here, so it cannot watch a pad reach Emmy. What it
/// can do is refuse to let the declaration go missing again. The mechanism the
/// declaration buys is pinned upstream, in `ambition_input::local_seats`
/// (`a_single_pad_beside_a_keyboard_player_drives_the_second_seat`): under
/// `JoinToClaim`, one keyboard player beside one pad player puts the pad on
/// SEAT TWO and makes the keyboard seat deaf to it.
///
/// and the retirement half is asserted too. Both resources are process-global
/// and TwinTrack is one route in a host that also runs Mary-O and Smash.
#[test]
fn the_plaza_declares_two_seats_and_a_couch_policy_only_while_it_is_live() {
    use ambition_platformer2d::input::{InputAssignmentPolicy, LocalSeatOffer};

    fn offer(app: &App) -> LocalSeatOffer {
        app.world()
            .get_resource::<LocalSeatOffer>()
            .cloned()
            .expect("the plaza inits the offer it claims")
    }

    let mut app = ambition_demo_twintrack_app::build_demo_app();
    app.update();
    assert_eq!(
        offer(&app).seats(),
        0,
        "TwinTrack offered seats before its session existed",
    );
    assert_eq!(
        offer(&app).policy(),
        InputAssignmentPolicy::default(),
        "TwinTrack partitioned the room's controllers before anybody was playing",
    );

    activate(&mut app);
    assert_eq!(
        offer(&app).seats(),
        2,
        "the exhibit is two observers, so the session seats two participants",
    );
    assert_eq!(
        offer(&app).policy(),
        InputAssignmentPolicy::JoinToClaim,
        "the seats are declared but every device still drives seat zero: a \
         keyboard and one controller are two people at this exhibit",
    );
}

/// THE PLAZA HAS TWO OBSERVERS, AND THEY SEE DIFFERENT SKIES.
///
/// The per-observer optical and targeting views landed with ZERO adopters —
/// every consumer read the first row through `Deref`, which is one observer's
/// sky drawn for everybody. This is the adoption: the laboratory twin carries
/// her own `RelativisticObserver2d`, so the resources publish two rows and the
/// exhibit's claim is made of measured numbers rather than of a diagram.
///
/// They disagree because they must: the traveler is moving and the laboratory twin is not, so
/// they disagree about the light's arrival time, its direction and its colour.
#[test]
fn both_observers_publish_their_own_sky_and_the_two_disagree() {
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    activate(&mut app);
    set_traveler_state(
        &mut app,
        LAB_POS + Vec2::new(240.0, 0.0),
        Vec2::new(TARGET_SPEED, 0.0),
    );
    idle(&mut app, 120);

    let (traveler, laboratory) = {
        let world = app.world_mut();
        let mut travelers = world.query_filtered::<Entity, With<TravelerTwin>>();
        let traveler = travelers.single(world).expect("the traveler is seated");
        let mut twins = world.query_filtered::<Entity, With<LaboratoryTwin>>();
        let laboratory = twins.single(world).expect("the laboratory twin is seated");
        (traveler, laboratory)
    };

    let optics = app.world().resource::<RelativisticOpticalView2d>();
    assert_eq!(
        optics.len(),
        2,
        "the plaza publishes {} optical image(s) — the exhibit compares two \
         observers, so both of them have to BE observers",
        optics.len(),
    );

    let source = "Photon Fox";
    let seen = |observer: Entity| {
        optics
            .for_observer(observer)
            .and_then(|view| view.sources.iter().find(|s| s.label == source))
            .map(|s| (s.emission_event.coordinate_time, s.doppler_factor))
    };
    let (Some((traveler_time, traveler_doppler)), Some((lab_time, lab_doppler))) =
        (seen(traveler), seen(laboratory))
    else {
        panic!("both observers should have {source} inside their retained past light cone");
    };

    assert!(
        (traveler_time - lab_time).abs() > 1.0e-4,
        "both observers read the same emission event ({traveler_time}) for \
         {source}: they are at different places, so the light that reaches them \
         now left at different times",
    );
    assert!(
        (traveler_doppler - lab_doppler).abs() > 1.0e-3,
        "both observers measure the same Doppler factor ({traveler_doppler}) — \
         one of them is moving at {TARGET_SPEED} and the other is at rest, so \
         this is one image being copied rather than two being computed",
    );
}
