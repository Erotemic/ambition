//! HOW FAR DOES A SHIELD ROLL ACTUALLY TRAVEL?
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- roll-probe`
//!
//! ⭐⭐ THIS EXISTS BECAUSE A REPORT AND THE TUNING DISAGREE. Jon, 2026-08-24:
//! *"shield rolls have too much motion to them. They send the character flying
//! across the stage."* The authored numbers say otherwise — 530 px/s over a
//! 0.22s window, against 7600 px/s² of ground friction, is a roll that should be
//! motionless after about four frames and roughly fourteen pixels. One of those
//! two is wrong, and reading the constants again cannot settle it.
//!
//! So this drives a REAL shield roll on the REAL stage and prints every frame of
//! it. It is observational: no threshold, no pass/fail. What it answers is
//! "where did the body go, how fast, and for how long", which is the question a
//! play session raises and a unit test keeps failing to reach.
//!
//! ⛔ IT DRIVES THE PRODUCTION INPUT ROAD. `drive_control_frame` is the only
//! driver that works on this host — under GGRS `ControlFrame` is an OUTPUT, and
//! four other ways to press a button were measured doing nothing at all. A probe
//! that set the roll's velocity directly would measure the number it just wrote.
//!
//! ⭐ AND IT PRINTS VELOCITY, not just position. A position that DIFFERS is not
//! a position that is still CHANGING; this exact defect was misdiagnosed twice
//! by comparing two positions, and the velocity column is what separates "it
//! moved once" from "it is still moving".
//!
//! # ⭐⭐ WHAT IT TOOK TO MAKE THE PRESS LAND, measured 2026-08-25
//!
//! Four runs measured nothing before this one worked, and each failure looked
//! exactly like "the roll does nothing":
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
//! ⇒ **a one-tick press is not a press.** It assumes the body steps after the
//! frame is committed within the same update, and a probe has no business
//! modelling that ordering — a player holds the button, so this holds it.
//!
//! ⛔ AND THE BRAIN STILL HAS TO COME OFF seat zero: a seated fighter's brain
//! writes its `ControlFrame` every tick. That is a probe-side removal and it is
//! honest for an instrument — production binds a human seat through the select
//! screen, which a binary cannot click. It changes WHO drives the body, not what
//! a roll does.
//!
//! # THE ANSWER, on the real stage
//!
//! ```text
//! TRAVELLED 11.2px = 2.3% of the platform, peak 530px/s, still after 3 frames
//! ```
//!
//! ⭐ The roll launches at exactly its authored 530px/s and is motionless three
//! frames later, having crossed about a fortieth of the stage. ⛔ SO THE REPORT
//! AND THE TUNING STILL DISAGREE: whatever sends a fighter flying across the
//! stage, it is not `dodge_roll_speed`. Read the `state` column before believing
//! any run — while it says `-`, the reading is a fighter standing still.
//!
//! # WHAT THIS INSTRUMENT HAS ALREADY REFUTED, 2026-08-25
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
//! ⛔⛔ **AND THE AIR DODGE IS REFUTED TOO, measured 2026-08-25 in the KERNEL**
//! (where a tick is a tick, which is what this probe cannot promise): it
//! launches at its authored 440px/s and travels **29.5px**, stopped by frame 15.
//! `AIR_FRICTION` is 650 against ground's 7600, so the arithmetic suggested
//! ~149px of coast. ⚠ THAT ARITHMETIC WAS INCOMPLETE, not the measurement:
//! `AIR_STOP_ASSIST` is 3750 — a hands-off airborne stop assist that stacks with
//! the friction — so ~30px is the tuning working, and nothing cancels the
//! dodge's velocity.
//!
//! ⇒ **NOTHING IN THE EVADE FAMILY MOVES A FIGHTER MORE THAN ~30px.** Ground
//! roll 11.2, roll off the lip 12.7, air dodge 29.5, chained rolls ~26px/s.
//! Whatever crosses the stage is not an evade at all — the remaining candidates
//! are KNOCKBACK (a hit during or after the roll), the LEDGE getup roll, which
//! is a different mechanism entirely, or another fighter's authored tuning.
//!
//! ⇒ **THE AIR DODGE WAS THE LEADING CANDIDATE AND IS NOT THE ANSWER.** Shield + a direction
//! IN THE AIR is not a roll, it is `air_dodge_speed` (440px/s) with air friction
//! rather than ground friction under it — and it looks like a roll. `--air`
//! jumps first so the same press resolves as one.
//!
//! ⛔⛔ BUT `--air` DOES NOT YET GET THE BODY MEANINGFULLY AIRBORNE, and the
//! reason is a limit of this whole probe worth knowing before trusting any
//! number in it: **`app.update()` is a FRAME, not a sim tick.** A held jump
//! samples at 46px/s against an authored `JUMP_SPEED` of 630 and rises 0.4px —
//! not because the jump is broken, but because the hop begins and ends between
//! two samples. The ground roll survives this because its launch happens to be
//! caught (530px/s, exactly the authored value); a fast vertical arc does not.
//!
//! ⇒ testing the air dodge properly wants a fixed-tick harness rather than
//! `App::update`, or a body held airborne by something other than its own jump.
//!
//! ⛔ The other untested candidates are things a headless probe cannot reach:
//! another fighter's authored tuning, or the LEDGE getup roll, which is a
//! different mechanism from this one.
//!
use crate::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use bevy::prelude::*;

/// How many frames to watch after the roll is pressed. Long enough to outlast
/// the window, the endlag, and any slide either of them leaves behind.
const WATCH_FRAMES: usize = 60;

/// Frames of shield BEFORE the roll press. A roll out of shield is the thing
/// being measured, so the guard has to actually be up first.
const SHIELD_FRAMES: usize = 12;

#[derive(clap::Args, Debug)]
pub struct RollProbeArgs {
    /// Probe the AIRBORNE reading instead of the grounded one — `air_dodge_speed`
    /// with air friction under it, which looks like a roll to a player.
    #[arg(long)]
    pub air: bool,
}

pub fn run(args: RollProbeArgs) {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }

    // ⛔ HUMAN SEATS, NOT CPUs. `smash_roster_at_levels` gives every seat a
    // fighter BRAIN, and the first run of this probe measured exactly that: the
    // brain pressed its own buttons over the top of the roll, the body drifted
    // at walking speed, and no roll ever fired. `smash_roster` locks the seats
    // as human, so the only input in the reading is the one below.
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

    // The stage, so the answer can be stated as a FRACTION of it. "Flying across
    // the stage" is a claim about the stage, and a pixel count alone cannot
    // confirm or refute it.
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

    // ⛔ WHO IS DRIVING THIS BODY? A probe that presses a button nobody is
    // listening for measures a fighter standing still and reads as "the roll
    // does nothing". Say the binding out loud before pressing anything.
    report_binding(&mut app, body);

    // ⭐⭐ TAKE THE BRAIN OFF SEAT ZERO, which is what makes the press land.
    //
    // ⛔⛔ THE FIRST TWO RUNS OF THIS PROBE MEASURED NOTHING because of this: a
    // seated fighter carries a `Brain`, the brain writes the body's
    // `ControlFrame` every tick, and a frame delivered from outside is
    // overwritten before the kernel reads it. Run one (CPU seats) drifted at
    // walking speed; run two (human seats) did not move at all. Neither was
    // about the roll.
    //
    // ⛔ A PROBE-SIDE REMOVAL, and it is honest for an instrument: production
    // binds a human seat through the select screen, which a binary cannot click.
    // What this changes is WHO drives the body, not what a roll does — the
    // movement kernel, the tuning and the out-of-shield policy are untouched.
    app.world_mut()
        .entity_mut(body)
        .remove::<ambition_platformer2d::characters::brain::Brain>();
    app.update();

    // ⭐ THE AIRBORNE VARIANT, on request: `--air` jumps first, so the same press
    // resolves as an AIR DODGE rather than a roll. That is the candidate the
    // ground readings point at — `air_dodge_speed` with air friction under it,
    // and it looks like a roll to a player.
    let airborne = args.air;
    if airborne {
        // ⛔ HELD, not tapped, for the same reason the roll press is: a one-tick
        // press assumes an ordering this probe has no business modelling. An
        // earlier attempt teleported the body upward instead and it simply
        // landed before the press.
        for tick in 0..8 {
            drive(
                &mut app,
                ControlFrame {
                    // ⛔ ONE RISING EDGE, then HELD — which is what a pad does.
                    // Re-pressing every tick is not a longer press; it is eight
                    // presses, and the jump law reads the edge.
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

    // GUARD UP FIRST. `shield_held` with no direction is a shield; the roll is
    // the burst that comes out of it.
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

    // THE ROLL: shield still held, burst pressed, stick right. All three, because
    // that is what a player does — a burst with no guard is a dash, and the
    // direction is what makes it a roll rather than a spot dodge.
    // ⛔ BOTH THE AXIS AND THE DISCRETE DIRECTION. `axis_x` is the analogue
    // reading and `right_pressed` the digital edge; which one a gate consults is
    // not this probe's business to model, so it supplies the press a real pad
    // would supply and lets the kernel choose.
    // ⚠ HELD FOR SEVERAL TICKS, not one. A single-tick press assumes the body
    // steps after the frame is committed within the same update, and the probe
    // has no business modelling that ordering — a player holds the button.
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
        // ⭐ THE FIRST FRAME THE BODY IS ACTUALLY STILL, which is the number the
        // report is really about — not the frame the timer says the roll ended.
        if still_frame.is_none() && frame > 0 && vel.x.abs() < 1.0 {
            still_frame = Some(frame);
        }
        // ⭐ WHETHER THE ROLL IS ACTUALLY HAPPENING, which is the column the
        // first run of this probe lacked and needed: without it, "the body moved
        // 15px" reads as a short roll when it was really no roll at all.
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

/// WHICH SLOT drives this body, and whether it also carries a brain.
///
/// ⛔⛔ THE FIRST TWO RUNS OF THIS PROBE WERE BOTH MEASURING THE WRONG THING,
/// and this line is what would have said so immediately. Run one seated CPUs, so
/// a fighter brain pressed its own buttons over the roll and the body drifted at
/// walking speed. Run two seated humans, and the body did not move AT ALL —
/// a press with no listener. Neither reading is about the roll; both look like
/// "the roll barely does anything".
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
/// ⛔ THE FACTS, not the policy's private timers. `BodyMotionFacts` is the seam
/// every other reader outside the kernel uses, so a probe reading it is reading
/// what animation and combat read — and a roll that is invisible here is
/// invisible to them too.
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

/// ⛔ THE ONLY DRIVER THAT WORKS ON THIS HOST. Writing `ControlFrame` between
/// updates, or from a system ordered before the commit set, was measured doing
/// nothing: under GGRS the frame is an OUTPUT that the session rewrites from its
/// confirmed inputs every advance.
fn drive(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
}
