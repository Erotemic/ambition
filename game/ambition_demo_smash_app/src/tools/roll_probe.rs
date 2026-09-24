//! How far does a shield roll actually travel?
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- roll-probe`
//!
//! A play report said shield rolls send the character across the stage. The
//! authored numbers (530 px/s over 0.22 s against 7600 px/s² of ground
//! friction) predict about fourteen pixels. This drives a real shield roll on
//! the real stage and prints every frame. It is observational: no threshold,
//! no pass/fail.
//!
//! It drives the production input road through `drive_control_frame`, the
//! only driver that works on this host (under GGRS `ControlFrame` is an
//! output). Setting the roll's velocity directly would measure the number
//! just written.
//!
//! It prints velocity as well as position: a position that differs is not
//! a position that is still changing.
//!
//! # What makes the press land
//!
//! Each failed setup looked like "the roll does nothing":
//!
//! ```text
//! CPU seats                the BRAIN presses over the top; walk-speed drift
//! human seats              the body does not move at all
//! the brain removed        still nothing — the press reached `ActorControl`
//!                          (shield_held, burst_pressed, locomotion.x = 1.0)
//!                          and the body did not even WALK
//! the press held 4 ticks   the roll fires
//! ```
//!
//! A one-tick press is not a press: it assumes the body steps after the
//! frame is committed within the same update. A player holds the button, so
//! this holds it.
//!
//! The brain must also come off seat zero, because a seated fighter's brain
//! writes its `ControlFrame` every tick. This is a probe-side removal: it
//! changes who drives the body, not what a roll does.
//!
//! # Result on the real stage
//!
//! ```text
//! TRAVELLED 11.2px = 2.3% of the platform, peak 530px/s, still after 3 frames
//! ```
//!
//! The roll launches at its authored 530px/s and stops three frames later.
//! So `dodge_roll_speed` is not what sends a fighter across the stage. Read
//! the `state` column before trusting any run: `-` means the fighter is
//! standing still.
//!
//! # Candidates this probe has refuted
//!
//! ```text
//! the roll's own distance   11.2px, 2.3% of the platform
//! CHAINED rolls             cooldown 0.42s against a 0.22s roll, so a held
//!                           button repeats at ~26px/s — not "flying"
//! rolling OFF the lip       parked at the platform edge, the roll still stops
//!                           at 12.7px; ground friction is not what was holding
//!                           it back
//! ```
//!
//! The air dodge, measured in the kernel (where a tick is a tick), launches
//! at its authored 440px/s and travels 29.5px. `AIR_FRICTION` (650) plus
//! `AIR_STOP_ASSIST` (3750) stop it; nothing cancels its velocity.
//!
//! So nothing in the evade family moves a fighter more than about 30px. The
//! remaining candidates are knockback (a hit during or after the roll), the
//! ledge getup roll, and another fighter's authored tuning.
//!
//! `--air` jumps first so the same press resolves as an air dodge. Limit:
//! `app.update()` is a frame, not a sim tick, so a fast vertical arc (a hop)
//! can begin and end between two samples, and `--air` does not get the body
//! meaningfully airborne. Test the air dodge with a fixed-tick harness.
use crate::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use bevy::prelude::*;

/// How many frames to watch after the roll is pressed. Long enough to outlast
/// the window, the endlag, and any slide either of them leaves behind.
const WATCH_FRAMES: usize = 60;

/// Frames of shield before the roll press: the guard must be up first.
const SHIELD_FRAMES: usize = 12;

#[derive(clap::Args, Debug)]
pub struct RollProbeArgs {
    /// Probe the airborne reading instead: `air_dodge_speed` with air friction
    /// under it, which looks like a roll to a player.
    #[arg(long)]
    pub air: bool,
}

pub fn run(args: RollProbeArgs) {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }

    // Human seats, not CPUs: `smash_roster_at_levels` gives every seat a brain
    // that presses its own buttons. `smash_roster` locks the seats as human.
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let roster = ambition_demo_smash::smash_roster(characters);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    let Some(body) = seat_zero(&mut app) else {
        println!("[roll_probe] no seated fighter — nothing to roll");
        return;
    };

    // The stage, so the answer can be stated as a fraction of it.
    let stage = ambition_demo_smash::smash_stage().world;
    let platform = stage.blocks[0].aabb;
    let platform_width = platform.max.x - platform.min.x;

    println!(
        "[roll_probe] stage platform is {platform_width:.0}px wide, world is {:.0}px",
        stage.size.x
    );
    println!(
        "[roll_probe] authored: dodge_roll_speed={:.0}px/s  dodge_roll_time={:.2}s  \
         ground_friction={:.0}px/s^2",
        ambition_platformer2d::engine_core::DODGE_ROLL_SPEED,
        ambition_platformer2d::engine_core::DODGE_ROLL_TIME,
        ambition_platformer2d::engine_core::GROUND_FRICTION,
    );

    // Report who drives this body before pressing anything. A press nobody
    // listens for reads as "the roll does nothing".
    report_binding(&mut app, body);

    // Take the brain off seat zero so the press lands: the brain writes the
    // body's `ControlFrame` every tick. Production binds a human seat through
    // the select screen, which a binary cannot click. The kernel, tuning, and
    // out-of-shield policy are untouched.
    app.world_mut()
        .entity_mut(body)
        .remove::<ambition_platformer2d::characters::brain::Brain>();
    app.update();

    // `--air` jumps first, so the same press resolves as an air dodge.
    let airborne = args.air;
    if airborne {
        // Held, not tapped, as for the roll press.
        for tick in 0..8 {
            drive(
                &mut app,
                ControlFrame {
                    // One rising edge, then held, as a pad does. The jump reads the edge.
                    jump_pressed: tick == 0,
                    jump_held: true,
                    ..ControlFrame::default()
                },
            );
            app.update();
        }
        let grounded = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(body)
            .map(|g| g.on_ground);
        println!("[roll_probe] jumped; grounded={grounded:?}");
    }

    // Guard up first. `shield_held` with no direction is a shield; the roll
    // is the burst out of it.
    for _ in 0..if airborne { 1 } else { SHIELD_FRAMES } {
        drive(
            &mut app,
            ControlFrame {
                shield_held: true,
                ..ControlFrame::default()
            },
        );
        app.update();
    }
    let start = position(&app, body);
    println!("[roll_probe] shield up at x={:.2}; rolling right", start.x);

    // The roll: shield held, burst pressed, stick right. A burst with no
    // guard is a dash; the direction makes it a roll, not a spot dodge.
    // Supply both the axis and the digital direction, as a real pad does.
    // Held for several ticks.
    for _ in 0..4 {
        drive(
            &mut app,
            ControlFrame {
                shield_held: true,
                burst_pressed: true,
                axis_x: 1.0,
                right_pressed: true,
                ..ControlFrame::default()
            },
        );
        app.update();
    }

    println!("[roll_probe] frame  x         dx(px)   vel.x(px/s)  travelled(px)  %platform  state");
    let mut peak_speed = 0.0f32;
    let mut previous = start.x;
    let mut still_frame: Option<usize> = None;
    for frame in 0..WATCH_FRAMES {
        let (pos, vel) = sample(&app, body);
        let travelled = (pos.x - start.x).abs();
        peak_speed = peak_speed.max(vel.x.abs());
        let step = pos.x - previous;
        previous = pos.x;
        // The first frame the body is still, which is what the report is about.
        if still_frame.is_none() && frame > 0 && vel.x.abs() < 1.0 {
            still_frame = Some(frame);
        }
        // Whether a roll is happening: without this column, "moved 15px" can be
        // no roll at all.
        println!(
            "[roll_probe] {frame:>5}  {:>8.2}  {step:>7.2}  {:>11.1}  {travelled:>13.2}  \
             {:>8.1}%  {}",
            pos.x,
            vel.x,
            100.0 * travelled / platform_width,
            state_of(&app, body),
        );
        drive(&mut app, ControlFrame::default());
        app.update();
    }

    let end = position(&app, body);
    let travelled = (end.x - start.x).abs();
    println!(
        "[roll_probe] TRAVELLED {travelled:.1}px = {:.1}% of the platform, \
         peak {peak_speed:.0}px/s, still after {} frames",
        100.0 * travelled / platform_width,
        still_frame
            .map(|f| f.to_string())
            .unwrap_or_else(|| format!(">{WATCH_FRAMES}")),
    );
    println!(
        "[roll_probe] ⇒ if that percentage is small and the report says otherwise, \
         the roll is not what is moving the body — look at what else the press \
         starts."
    );
}

/// Seat zero's body, or `None` if the cast never got built.
fn seat_zero(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &MatchSeat)>();
    let mut seats: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
    seats.sort_by_key(|(seat, _)| *seat);
    seats.first().map(|(_, e)| *e)
}

fn sample(app: &App, body: Entity) -> (Vec2, Vec2) {
    let kin = app
        .world()
        .get::<BodyKinematics>(body)
        .expect("the fighter still has a body");
    (
        Vec2::new(kin.pos.x, kin.pos.y),
        Vec2::new(kin.vel.x, kin.vel.y),
    )
}

fn position(app: &App, body: Entity) -> Vec2 {
    sample(app, body).0
}

/// Which slot drives this body, and whether it also carries a brain. With
/// CPU seats the brain presses over the roll; with no driver the press has
/// no listener. Both look like "the roll barely does anything".
fn report_binding(app: &mut App, body: Entity) {
    let slot = app
        .world()
        .get::<ambition_platformer2d::characters::control::DrivingParticipant>(body)
        .map(|d| d.0 .0);
    let has_brain = app
        .world()
        .get::<ambition_platformer2d::characters::brain::Brain>(body)
        .is_some();
    println!(
        "[roll_probe] seat 0 body is driven by slot {} and {} a brain — \
         the press below goes to the PRIMARY slot",
        slot.map(|s| s.to_string())
            .unwrap_or_else(|| "NOBODY".to_string()),
        if has_brain { "carries" } else { "carries no" },
    );
    if slot.is_none() {
        println!(
            "[roll_probe] ⛔ NO DRIVER: nothing will read the press, and every \
             number below will be a body standing still. The seat was never \
             claimed to a local slot."
        );
    }
}

/// What the body says it is DOING, from the published movement facts.
///
/// Reads `BodyMotionFacts`, the seam animation and combat also read, not the
/// policy's private timers.
fn state_of(app: &App, body: Entity) -> String {
    let Some(facts) = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyMotionFacts>(body)
    else {
        return "no-facts".to_string();
    };
    let mut on: Vec<&str> = Vec::new();
    if facts.dodge_rolling && !facts.spot_dodging {
        on.push("ROLL");
    }
    if facts.spot_dodging {
        on.push("spot");
    }
    if facts.dodge_roll_endlag {
        on.push("endlag");
    }
    if facts.running {
        on.push("run");
    }
    if facts.dashing {
        on.push("dash");
    }
    if on.is_empty() {
        "-".to_string()
    } else {
        on.join("+")
    }
}

/// The only driver that works on this host. Under GGRS the session rewrites
/// `ControlFrame` from its confirmed inputs every advance, so writing it
/// between updates does nothing.
fn drive(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
}
