//! A scripted whiffer against a CPU: does the CPU punish a predictable miss?
//!
//! Seat 0 is driven through ordinary control frames (the same presses
//! `moveset_render` makes): it stands its ground, faces the CPU, and throws
//! one verb on a fixed cadence. Seat 1 is a CPU fighter at the asked rung. It
//! decides where to stand; nothing in the brain knows this is a drill.
//!
//! Each swing is classed by what the engine resolved:
//!
//! * `hit`      the swing connected (the CPU stood in it);
//! * `stuffed`  the whiffer took damage before its active window ended;
//! * `punished` the swing missed and the whiffer took damage during its own
//!   recovery (from the end of the active window to the end of the move,
//!   plus a short grace for a hit already in flight);
//! * `missed`   the swing missed and nothing answered it.
//!
//! The question it answers is the tactical-intent experiment's fixture A
//! (`docs/planning/engine/fighter-tactical-intent.md`): with the window
//! there (`scripts/smash_punish_windows.py`), does the CPU take it?
//!
//! ```bash
//! cargo run -p ambition_app_tools --bin whiff_drill -- \
//!     --whiffer pugnacious_polygon --verb smash_forward --cpu pointed_polygon --level 9 --seconds 60
//! ```
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use ambition_sim_harness::move_exercise;
use bevy::prelude::*;
use clap::Parser;

/// Ticks past the end of the whiffer's move in which a landing hit still
/// counts as the punish: a strike already in flight.
const PUNISH_GRACE_TICKS: u32 = 4;

#[derive(Parser)]
#[command(name = "whiff_drill", about = "A scripted whiffer against a CPU fighter")]
struct Args {
    /// The fighter that throws the verb on a cadence (seat 0, scripted).
    #[arg(long)]
    whiffer: String,
    /// The repertoire verb it throws (see `move_exercise::VERBS`).
    #[arg(long)]
    verb: String,
    /// The CPU fighter under test (seat 1).
    #[arg(long)]
    cpu: String,
    /// The CPU's ladder rung.
    #[arg(long, default_value_t = 9)]
    level: u8,
    /// Seconds of drill after the match goes live.
    #[arg(long, default_value_t = 60.0)]
    seconds: f32,
    /// Ticks between the end of one swing and the next press.
    #[arg(long, default_value_t = 40)]
    rest_ticks: u32,
}

#[derive(Default)]
struct Swing {
    started: u64,
    active_end_s: f32,
    duration_s: f32,
    connected: bool,
    stuffed: bool,
    punished: bool,
    gap_at_end: f32,
}

#[derive(Default)]
struct Tally {
    swings: usize,
    hit: usize,
    stuffed: usize,
    punished: usize,
    missed: usize,
    whiffer_damage: i32,
    cpu_damage: i32,
    miss_gaps: Vec<f32>,
}

fn seat_state(app: &mut App, seat: usize) -> Option<(f32, f32, i32, Option<(f32, f32, f32, bool)>)> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::actor::BodyKinematics,
        &ambition_platformer2d::characters::actor::BodyHealth,
        Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
    )>();
    q.iter(world).find(|(at, ..)| at.0 == seat).map(|(_, kin, health, playback)| {
        let move_state = playback.map(|p| {
            let active_end = p
                .spec
                .windows
                .iter()
                .filter(|w| matches!(w.tag, ambition_platformer2d::entity_catalog::WindowTag::Active))
                .map(|w| w.end_s)
                .fold(0.0f32, f32::max);
            (p.t, active_end, p.spec.duration_s, p.connected_hit)
        });
        (kin.pos.x, kin.facing, health.damage_taken(), move_state)
    })
}

fn main() {
    let args = Args::parse();
    let Some(verb) = move_exercise::verb_named(&args.verb) else {
        eprintln!("whiff_drill: unknown verb '{}'", args.verb);
        std::process::exit(2);
    };
    let mut app = ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let timestep = app.world().resource::<bevy::time::Time<bevy::time::Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    for _ in 0..30 {
        app.update();
    }
    let mut roster = ambition_demo_smash::smash_roster([args.whiffer.as_str(), args.cpu.as_str()]);
    if let Some(cpu) = roster.participants.get_mut(1) {
        cpu.controller = ambition_platformer2d::versus_match::ControllerBinding::Cpu {
            brain_profile: Some(format!("{}_l{}", ambition_demo_smash::SMASH_DUELIST_BRAIN, args.level)),
        };
    }
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE)));
    let mut live = false;
    for _ in 0..1200 {
        app.update();
        let held = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actor::MatchSeat,
                With<ambition_platformer2d::characters::control::ControlHolds>,
            >();
            q.iter(world).count()
        };
        if held == 0
            && move_exercise::seat_character(&mut app, 0).as_deref() == Some(args.whiffer.as_str())
            && move_exercise::seat_character(&mut app, 1).as_deref() == Some(args.cpu.as_str())
        {
            live = true;
            break;
        }
    }
    if !live {
        eprintln!("whiff_drill: the match never went live (is '{}' / '{}' on the smash grid?)", args.whiffer, args.cpu);
        std::process::exit(1);
    }

    let ticks = (args.seconds * 60.0).round() as u64;
    let mut tally = Tally::default();
    let mut swing: Option<Swing> = None;
    let mut press_tick: Option<u64> = None;
    let mut rest = args.rest_ticks;
    let mut last_damage = (0i32, 0i32);
    for tick in 0..ticks {
        let (Some(me), Some(cpu)) = (seat_state(&mut app, 0), seat_state(&mut app, 1)) else {
            move_exercise::step(&mut app, Default::default());
            continue;
        };
        let (my_x, my_facing, my_damage, my_move) = me;
        let (cpu_x, _, cpu_damage, _) = cpu;
        let took = my_damage - last_damage.0;
        let dealt = cpu_damage - last_damage.1;
        last_damage = (my_damage, cpu_damage);
        tally.whiffer_damage += took.max(0);
        tally.cpu_damage += dealt.max(0);

        // Classify the swing in flight.
        if let Some(open) = swing.as_mut() {
            let elapsed_s = (tick - open.started) as f32 / 60.0;
            if let Some((_, _, _, connected)) = my_move {
                open.connected |= connected;
            }
            if took > 0 {
                if elapsed_s <= open.active_end_s {
                    open.stuffed = true;
                } else if !open.connected {
                    open.punished = true;
                }
            }
            let over = elapsed_s > open.duration_s + PUNISH_GRACE_TICKS as f32 / 60.0;
            if over {
                let done = swing.take().expect("open");
                tally.swings += 1;
                if done.connected {
                    tally.hit += 1;
                } else if done.stuffed {
                    tally.stuffed += 1;
                } else if done.punished {
                    tally.punished += 1;
                } else {
                    tally.missed += 1;
                    tally.miss_gaps.push(done.gap_at_end);
                }
                rest = args.rest_ticks;
            } else if elapsed_s <= open.active_end_s {
                open.gap_at_end = (cpu_x - my_x).abs();
            }
        }

        // Drive seat 0: a press on the cadence, a turn toward the CPU between.
        let toward = (cpu_x - my_x).signum();
        let frame = if let Some(at) = press_tick {
            let n = (tick - at) as usize;
            if n == 1 {
                if let Some((_, active_end, duration, _)) = my_move {
                    swing = Some(Swing {
                        started: at,
                        active_end_s: active_end,
                        duration_s: duration,
                        ..Default::default()
                    });
                }
            }
            if n >= 2 {
                press_tick = None;
            }
            // One press, released at once: an uncharged swing (the exercise's
            // own schedule holds the button long enough to charge a smash).
            verb.frame(n == 0, my_facing)
        } else if swing.is_none() && my_move.is_none() {
            if toward != 0.0 && toward != my_facing.signum() {
                let mut turn = ambition_platformer2d::sim::ControlFrame::default();
                turn.axis_x = toward * 0.3;
                turn
            } else if rest == 0 {
                press_tick = Some(tick);
                verb.frame(true, my_facing)
            } else {
                rest -= 1;
                Default::default()
            }
        } else {
            Default::default()
        };
        move_exercise::step(&mut app, frame);
    }

    let whiffs = tally.punished + tally.missed;
    let mut gaps = tally.miss_gaps.clone();
    gaps.sort_by(f32::total_cmp);
    println!(
        "whiff_drill: {} throws {} at {} (rung {}), {} s",
        args.whiffer, args.verb, args.cpu, args.level, args.seconds
    );
    println!(
        "  swings {}  hit {}  stuffed {}  whiffed {}  punished {} ({}% of whiffs)  unanswered {}",
        tally.swings,
        tally.hit,
        tally.stuffed,
        whiffs,
        tally.punished,
        if whiffs > 0 { 100 * tally.punished / whiffs } else { 0 },
        tally.missed
    );
    println!(
        "  damage: whiffer took {}  cpu took {}  median gap at an unanswered whiff {} px",
        tally.whiffer_damage,
        tally.cpu_damage,
        gaps.get(gaps.len() / 2).map_or("-".to_string(), |g| format!("{g:.0}"))
    );
}
