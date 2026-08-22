//! The act can actually be finished. One scripted headless run to the goal.
//!
//! This is Sanic's completion proof, and it exists because nothing else could
//! give it. The goal, the results capture, and the act cycle each have focused
//! coverage, but all of that is reachable only if a player can physically get
//! to `GOAL_X` — and Sanic rides the momentum kernel, so his position is derived
//! from his surface parameter rather than set. You cannot teleport him to the
//! goal to check; the only way to know the finish line is reachable is to run
//! there.
//! ## Driving the stick under either composition
//!
//! This ran in `PreUpdate` behind `#![cfg(not(feature = "input"))]`, and that
//! cfg reads THIS crate's `input` feature while the thing that erases a scripted
//! write is `ambition_platformer2d/input` — the participant pipeline in the dependency, which
//! workspace feature unification turns on regardless. The first test grew a
//! runtime SKIP for it; the second never did, and failed in the gate the moment
//! the earlier `two_rooms` failure stopped masking it.
//!
//! The SKIP is gone with the guard — a skipped proof is a silent pass, which is what let this
//! rot.

use ambition_demo_sanic::{SanicActPhase, SanicActState, GOAL_X};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn player_x(app: &mut App) -> f32 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .map(|k| k.pos.x)
        .unwrap_or(f32::NAN)
}

fn phase(app: &mut App) -> Option<SanicActPhase> {
    let mut query = app.world_mut().query::<&SanicActState>();
    query.iter(app.world()).next().map(|s| s.phase)
}

#[test]
fn holding_right_reaches_the_goal_and_clears_the_act() {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..8 {
        app.update();
    }

    // Prove the stick reaches the sim before spending 2400 frames on a run that
    // would otherwise fail as "unreachable level geometry" — the misreading that
    // cost someone an afternoon here once already ("furthest x reached was 160 of
    // a goal at 6000").
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = {
        let mut frame = ControlFrame::default();
        frame.axis_x = 1.0;
        frame.right_pressed = true;
        frame
    };
    // One `app.update` was enough only while this read the resource the scripted stage writes
    // directly.
    let mut arrived = false;
    for _ in 0..20 {
        app.update();
        if app.world().resource::<ControlFrame>().axis_x >= 0.5 {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "the scripted stick must survive into the sim's `ControlFrame`; if this \
         fails the ordering above no longer reaches the authority that writes it"
    );

    let start = player_x(&mut app);
    assert!(
        start < GOAL_X,
        "the act starts before its goal ({start} vs {GOAL_X})"
    );

    // Hold right, and JUMP on the approach to the authored pit.
    let stick = |jump: bool| {
        let mut frame = ControlFrame::default();
        frame.axis_x = 1.0;
        frame.right_pressed = true;
        frame.jump_pressed = jump;
        frame.jump_held = jump;
        frame
    };
    let approaching_pit = |x: f32| {
        x > ambition_demo_sanic::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::PIT_RIGHT_X
    };

    // 40 seconds of sim at 60Hz.
    let mut cleared = None;
    let mut max_x = f32::MIN;
    for frame in 0..2400 {
        let x = player_x(&mut app);
        max_x = max_x.max(x);
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = stick(approaching_pit(x));
        app.update();
        if let Some(SanicActPhase::Cleared { time, rings, .. }) = phase(&mut app) {
            cleared = Some((frame, time, rings));
            break;
        }
    }

    let (frame, time, rings) = cleared.unwrap_or_else(|| {
        panic!(
            "played for 40s and never cleared the act; furthest x reached was \
             {max_x:.0} of a goal at {GOAL_X}. If the furthest point is SHORT of \
             the goal, the goal is somewhere the body cannot go — which is how \
             this shipped: the runnable extent tops out near {} and the goal sat \
             past it, so the act was uncompletable.",
            ambition_demo_sanic::LEVEL_WIDTH - 270.0
        )
    });

    assert!(
        time > 0.0,
        "the clear captures the elapsed run time, not zero"
    );
    assert!(rings >= 0, "and the rings held at the line");
    // The clock must have STOPPED — that is what turns elapsed time into a
    // result rather than leaving a stopwatch running under the card.
    let held = time;
    for _ in 0..30 {
        app.update();
    }
    let Some(SanicActPhase::Cleared { time: still, .. }) = phase(&mut app) else {
        panic!("the act stayed cleared while the results card is up");
    };
    assert_eq!(still, held, "the clock stops on a clear");
    eprintln!("sanic completed the act at frame {frame} in {time:.2}s with {rings} rings");
}

/// He must survive his own results card.
///
/// Room-replay triage §1: `GOAL_X` sits 400px from the right edge and clearing
/// the act neither braked him nor closed the course, so he crossed the line at
/// speed, ran out of level, and died well inside the four-second dwell his own
/// card was still counting down.
#[test]
fn clearing_the_act_does_not_kill_him_before_the_card_retires() {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..8 {
        app.update();
    }
    let stick = |jump: bool| {
        let mut frame = ControlFrame::default();
        frame.axis_x = 1.0;
        frame.right_pressed = true;
        frame.jump_pressed = jump;
        frame.jump_held = jump;
        frame
    };
    let approaching_pit = |x: f32| {
        x > ambition_demo_sanic::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::PIT_RIGHT_X
    };

    let mut cleared = false;
    for _ in 0..2400 {
        let x = player_x(&mut app);
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = stick(approaching_pit(x));
        app.update();
        if matches!(phase(&mut app), Some(SanicActPhase::Cleared { .. })) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "never reached the goal, so this proves nothing");

    let dwell_frames = (ambition_demo_sanic::ACT_CLEAR_DWELL * 60.0).ceil() as usize;
    let mut furthest_back = f32::MAX;
    for _ in 0..dwell_frames {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = stick(false);
        app.update();
        furthest_back = furthest_back.min(player_x(&mut app));
    }
    assert!(
        furthest_back > GOAL_X - 400.0,
        "he ended up at x={furthest_back:.0} during his own {}s results card, \
         from a goal at {GOAL_X} — he ran off the end of the course, died, and \
         is replaying the act while the card still shows his time",
        ambition_demo_sanic::ACT_CLEAR_DWELL,
    );
}
