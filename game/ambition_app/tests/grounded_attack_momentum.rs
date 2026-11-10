//! GROUNDED-ATTACK-MOMENTUM: what a grounded attack does to a running body.
//!
//! The robot runs at full speed, then attacks on the ground. Each case
//! records the speed when the move starts, the travel while it plays and
//! after it, and whether the body is still on the ground:
//! - no attack, hands off: the law's brake, the control;
//! - an attack, hands off;
//! - an attack with the stick held forward: does the move take the stick?
//! - an attack with the stick held back;
//! - an attack, hands off, started just before a ledge: does it carry the
//!   body off?
//!
//! A measurement, not a contract. The owner is
//! `docs/planning/engine/unified-movement-kernel.md` (Q169).

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

use crate::common::fixed_60hz_room_options;

/// A long flat floor: the hub's main hall.
const ROOM: &str = "central_hub_complex";
const RUN_FRAMES: i32 = 45;
const AFTER_FRAMES: i32 = 90;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stick {
    Off,
    Forward,
    Back,
}

#[derive(Debug, Default)]
struct Run {
    speed_at_press: f32,
    move_frames: i32,
    travel_during_move: f32,
    travel_total: f32,
    speeds: Vec<f32>,
    left_the_ground: bool,
    left_the_ground_in_the_move: bool,
    move_name: Option<String>,
}

fn playing(sim: &mut Platformer2dSimHarness) -> Option<String> {
    let world = sim.world_mut();
    world
        .query_filtered::<&ambition_platformer2d::combat::moveset::MovePlayback, bevy::prelude::With<PrimaryPlayer>>()
        .iter(world)
        .next()
        .map(|playback| format!("{:?}", playback.spec.id))
}

/// Run right from `from` (when given) for [`RUN_FRAMES`], then do `attack`
/// with `stick`, and follow for [`AFTER_FRAMES`]. `press_at` presses when
/// the body reaches that x instead of after the run.
fn run(from: Option<(f32, f32)>, attack: bool, stick: Stick, press_at: Option<f32>) -> Run {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    for _ in 0..10 {
        sim.app_mut().update();
    }
    if let Some(at) = from {
        sim.teleport_player(at);
        for _ in 0..10 {
            sim.drive_seat(0, ControlFrame::default());
            sim.app_mut().update();
        }
    }
    let right = ControlFrame { axis_x: 1.0, right_pressed: true, ..Default::default() };
    let mut frames = 0;
    loop {
        sim.drive_seat(0, right);
        sim.app_mut().update();
        frames += 1;
        let x = sim.observation().player_pos.0;
        let done = match press_at {
            Some(edge) => x >= edge || frames > 600,
            None => frames >= RUN_FRAMES,
        };
        if done {
            break;
        }
    }
    let start = sim.observation();
    assert!(start.on_ground, "premise: the run is on the ground");
    let mut out = Run { speed_at_press: start.player_vel.0, ..Default::default() };
    let axis = match stick {
        Stick::Off => 0.0,
        Stick::Forward => 1.0,
        Stick::Back => -1.0,
    };
    let mut moving = true;
    for frame in 0..AFTER_FRAMES {
        let before = sim.observation().player_pos.0;
        sim.drive_seat(
            0,
            ControlFrame {
                axis_x: axis,
                attack_pressed: attack && frame == 0,
                attack_held: attack && frame < 3,
                attack_released: attack && frame == 3,
                ..Default::default()
            },
        );
        sim.app_mut().update();
        let now = sim.observation();
        let step = now.player_pos.0 - before;
        out.travel_total += step;
        out.speeds.push(now.player_vel.0);
        out.left_the_ground |= !now.on_ground;
        let move_now = playing(&mut sim);
        if move_now.is_some() && moving {
            out.left_the_ground_in_the_move |= !now.on_ground;
            out.move_frames += 1;
            out.travel_during_move += step;
            out.move_name = move_now;
        } else if out.move_frames > 0 {
            moving = false;
        }
    }
    out
}

fn line(label: &str, run: &Run) {
    let sample: Vec<String> = run.speeds.iter().step_by(6).take(10).map(|v| format!("{v:.0}")).collect();
    println!(
        "[momentum] {label}: {:.0} px/s at the press; move {:?} for {} frames, {:.1} px during it, {:.1} px in all; left the ground: {} (in the move: {}); speed every 6 frames: {}",
        run.speed_at_press,
        run.move_name,
        run.move_frames,
        run.travel_during_move,
        run.travel_total,
        run.left_the_ground,
        run.left_the_ground_in_the_move,
        sample.join(" ")
    );
}

#[test]
#[ignore = "measurement: prints what a grounded attack does to a running body; run with --ignored --nocapture"]
fn probe_what_a_grounded_attack_does_to_a_running_body() {
    line("no attack, hands off (the brake)", &run(None, false, Stick::Off, None));
    line("attack, hands off", &run(None, true, Stick::Off, None));
    line("attack, stick forward", &run(None, true, Stick::Forward, None));
    line("attack, stick back", &run(None, true, Stick::Back, None));
    line("no attack, stick forward (the run goes on)", &run(None, false, Stick::Forward, None));
}
