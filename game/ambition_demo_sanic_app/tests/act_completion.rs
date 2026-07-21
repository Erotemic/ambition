//! **The act can actually be finished.** One scripted headless run to the goal.
//!
//! This is Sanic's completion proof, and it exists because nothing else could
//! give it. The goal, the results capture, and the act cycle each have focused
//! coverage, but all of that is reachable only if a player can physically get
//! to `GOAL_X` — and Sanic rides the momentum kernel, so his position is derived
//! from his surface parameter rather than set. You cannot teleport him to the
//! goal to check; the only way to know the finish line is reachable is to run
//! there.
#![cfg(not(feature = "input"))]

use ambition::engine_core as ae;
use ambition::input::ControlFrame;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_demo_sanic::{SanicActPhase, SanicActState, GOAL_X};
use ambition_demo_sanic_app::build_demo_app;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy, Default)]
struct ScriptedStick(ControlFrame);

fn apply_scripted_stick(stick: Res<ScriptedStick>, mut frame: ResMut<ControlFrame>) {
    *frame = stick.0;
}

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
    // A fixed-tick host without a pinned clock runs a machine-speed-dependent
    // number of ticks per update — the same script would then cover a different
    // distance on every run.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<ScriptedStick>();
    // `PreUpdate`: Bevy runs the fixed-timestep loop BEFORE `Update`, so intent
    // written later is not seen by the tick it is meant to drive.
    app.add_systems(PreUpdate, apply_scripted_stick);
    for _ in 0..8 {
        app.update();
    }

    // The crate-level `cfg(not(feature = "input"))` guard is NOT sufficient: it
    // reads THIS crate's `input` feature, while the thing that erases a scripted
    // write is `ambition/input` — the participant pipeline in the dependency.
    // Under `cargo test --workspace` cargo unifies features across the graph, so
    // `ambition` is built WITH `input` while this crate's own flag stays off;
    // the guard passes and every scripted frame is then overwritten from device
    // state. That is how this failed in the gate: "furthest x reached was 160 of
    // a goal at 6000", which reads like unreachable level geometry rather than
    // like discarded input. Reproduce with
    // `--features ambition/input`. Ask the composition, not the feature flag.
    app.world_mut().resource_mut::<ScriptedStick>().0 = {
        let mut frame = ControlFrame::default();
        frame.axis_x = 1.0;
        frame.right_pressed = true;
        frame
    };
    app.update();
    if app.world().resource::<ControlFrame>().axis_x < 0.5 {
        eprintln!(
            "SKIP: a participant pipeline owns `ControlFrame` in this build \
             (`ambition/input` is on, likely via workspace feature unification), \
             so scripted input never reaches the sim. A completion run is only \
             meaningful in the headless sim composition."
        );
        return;
    }

    let start = player_x(&mut app);
    assert!(
        start < GOAL_X,
        "the act starts before its goal ({start} vs {GOAL_X})"
    );

    // Hold right, and JUMP on the approach to the authored pit. Holding right
    // alone runs straight into it, dies, and respawns forever — which is
    // correct level design, not a bug, and is exactly why a completion proof
    // has to play rather than hold one button.
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

    // 40 seconds of sim at 60Hz. Generous: if the speedway cannot be run in
    // that, the goal is not reachable by playing and the act cannot be
    // finished — which is the failure this exists to catch.
    let mut cleared = None;
    let mut max_x = f32::MIN;
    for frame in 0..2400 {
        let x = player_x(&mut app);
        max_x = max_x.max(x);
        app.world_mut().resource_mut::<ScriptedStick>().0 = stick(approaching_pit(x));
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
