//! Count what two CPUs actually DO to each other over a match.
//!
//! `cargo run -p ambition_demo_smash_app --bin match_report -- [SECONDS] [CHARACTER] [--runs N]`
//!
//! Every mechanic in this demo is authored, tuned and reachable; the question
//! that keeps going unanswered is whether anybody USES it. Three separate
//! slices — the smash charge, directional influence, the tech — shipped green
//! and inert, and each one was caught by counting in a real match rather than by
//! a unit test. This is that counting, made cheap enough to run after any change
//! that claims to affect how a fight goes.
//!
//! It is observational and has no pass/fail threshold. The one guard that DOES
//! assert lives in `tests/the_repertoire_gets_used.rs`; this prints the whole
//! vocabulary so a number that moved can be seen next to the ones that did not.
//!
//! ⛔ `--runs N` IS NOT DECORATION, AND ONE RUN IS NOT A MEASUREMENT. Two
//! fighters carry an execution-noise stream each, and a single thirty-second
//! sample of a fight is noisy enough that tuning against it makes things worse:
//! measured 2026-08-23, an option-scorer change judged on one run took the smash
//! suite from two failures to four. With `--runs` the spread is printed as
//! `min–median–max`, which is the shape a threshold should be picked off.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// One seat's tally. Ticks unless the name says otherwise.
#[derive(Default, Clone)]
struct Tally {
    damage: i32,
    hitstun: usize,
    tumbling: usize,
    knocked_down: usize,
    evading: usize,
    /// Ticks this body could not be struck at all — the damage rule's own
    /// answer, inverted. The number that separates "the CPUs are defensive" from
    /// "the CPUs are unhittable".
    unhittable: usize,
    /// WHICH of the four terms in `body_vulnerable` was false, counted
    /// separately. "A quarter of the match is untouchable" is a symptom; which
    /// term owns it is the fix.
    unhit_invuln: usize,
    unhit_evading: usize,
    /// The LEDGE's share of `unhit_evading` — a refinement of it, not a
    /// sibling, so the two columns do not add up to `unhittable`.
    ///
    /// Worth its own column because the ledge was invisible until its
    /// intangibility was split off the dodge roll's timer: a body camped on an
    /// edge and a body mid-evade both read as `dodge_rolling`, so "evading 659"
    /// could have been either, and nobody could tune one without the other.
    unhit_ledge: usize,
    unhit_parry_window: usize,
    unhit_iframes: usize,
    shielding: usize,
    parries_caught: usize,
    tech_armed: usize,
    charge_held: usize,
    /// The highest charge fraction this seat ever reached.
    best_charge: f32,
    /// Distinct move starts, so a match that throws one move reads as one.
    moves_started: usize,
    /// The fastest launch this body was ever handed, and the speed its own
    /// tuning says a launch has to beat to become a tumble. Printed together
    /// because "nobody tumbled" has two very different causes and only these two
    /// numbers separate them.
    ///
    /// ⛔ hitstun-gated on purpose. Plain top speed is not a launch: every attack
    /// in this engine lunges, and George's lunge alone reads 1500 px/s against a
    /// 500 px/s tumble threshold — a number that says a body was thrown when
    /// nothing threw it.
    top_speed: f32,
    tumble_speed: f32,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seconds: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(30);
    let mut character = ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string();
    let mut runs = 1usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--runs" => {
                runs = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|n| *n > 0)
                    .unwrap_or(1);
            }
            other => character = other.to_string(),
        }
    }

    let all: Vec<Vec<Tally>> = (0..runs)
        .map(|i| {
            run_one(
                &character,
                seconds,
                0x5F37_7A11_u64.wrapping_mul(i as u64 + 1),
            )
        })
        .collect();

    if runs == 1 {
        report_one(&character, seconds, &all[0]);
    } else {
        report_spread(&character, seconds, &all);
    }
}

/// One match, under one execution-noise stream.
fn run_one(character: &str, seconds: usize, noise_seed: u64) -> Vec<Tally> {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // BOTH SEATS CPU. `SmashSelect::roster` makes every locked seat a HUMAN,
    // which is right for a couch game and wrong here: a report driven through it
    // measures two fighters standing still while nobody presses anything.
    let characters = [character, character];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Past the ceremony: every fighter carries scripted control for the whole
    // 3-2-1-GO, so ticks inside the hold measure bodies that are forbidden to
    // act. Read the count from the ruleset rather than restating it.
    let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    // THE STREAM IS FORCED, and this rig supplies it rather than modelling how a
    // live fighter gets one — the point is the SPREAD across streams, exactly as
    // `ladder_probe` documents for the same reason.
    {
        use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
        let world = app.world_mut();
        let mut q = world.query::<&mut Brain>();
        for (index, mut brain) in q.iter_mut(world).enumerate() {
            if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
                state.noise = noise_seed.wrapping_mul(index as u64 + 1).wrapping_add(1);
            }
        }
    }
    let ticks = seconds * 60;
    let mut totals: Vec<Tally> = vec![Tally::default(); 4];
    let mut live_move: Vec<Option<(String, f32)>> = vec![None; 4];
    let mut parry_was: Vec<f32> = vec![0.0; 4];
    let mut hitstun_was: Vec<f32> = vec![0.0; 4];
    for _ in 0..ticks {
        app.update();
        sample(
            &mut app,
            &mut totals,
            &mut live_move,
            &mut parry_was,
            &mut hitstun_was,
        );
    }


    totals
}

fn report_one(character: &str, seconds: usize, totals: &[Tally]) {
    println!("match_report: {character} vs {character}, {seconds}s of CPU-versus-CPU\n");
    println!(
        "{:<6} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "seat",
        "damage",
        "moves",
        "hitstun",
        "tumbling",
        "downed",
        "evading",
        "unhit",
        "shield",
        "parries",
        "techs",
        "charge",
        "launch",
        "tumble@",
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.damage == 0 && tally.moves_started == 0 {
            continue;
        }
        println!(
            "{:<6} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>6.2} {:>8.0} {:>8.0}",
            seat,
            tally.damage,
            tally.moves_started,
            tally.hitstun,
            tally.tumbling,
            tally.knocked_down,
            tally.evading,
            tally.unhittable,
            tally.shielding,
            tally.parries_caught,
            tally.tech_armed,
            tally.best_charge,
            tally.top_speed,
            tally.tumble_speed,
        );
    }
    println!("\nwhy each body could not be struck, by the term that refused:");
    println!(
        "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
        "seat", "invuln", "evading", "of-ledge", "parry-window", "i-frames"
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.unhittable == 0 {
            continue;
        }
        println!(
            "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
            seat,
            tally.unhit_invuln,
            tally.unhit_evading,
            // A SHARE of the column before it, not a sibling: these do not sum
            // to `unhittable`.
            tally.unhit_ledge,
            tally.unhit_parry_window,
            tally.unhit_iframes
        );
    }
    println!(
        "\nticks are counts of SAMPLED TICKS in that state; damage is final percent, \
         parries and techs are events, charge is the best fraction reached."
    );
    // THE ONE READING WORTH SAYING OUT LOUD. A match where nobody is ever
    // launched is not a match, however much damage it accumulates, and that
    // exact state shipped once already.
    if totals.iter().all(|t| t.tumbling == 0) {
        println!(
            "\n⚠ NOBODY TUMBLED. Hits are landing and nothing is being launched — \
             check the tumble threshold against the launches actually resolved."
        );
    }
    if totals.iter().all(|t| t.best_charge <= 0.0) {
        println!("\n⚠ NOBODY CHARGED A SMASH. The multiplier is authored and unpaid.");
    }
}

fn sample(
    app: &mut App,
    totals: &mut [Tally],
    live_move: &mut [Option<(String, f32)>],
    parry_was: &mut [f32],
    hitstun_was: &mut [f32],
) {
    let world = app.world_mut();
    let mut q = world.query::<(
        &MatchSeat,
        &BodyHealth,
        &BodyCombat,
        &ae::BodyKinematics,
        Option<&MovePlayback>,
        Option<&ae::BodyMotionFacts>,
        Option<&ae::BodyShieldState>,
        Option<&ambition_platformer2d::actors::features::MotionModel>,
    )>();
    let rows: Vec<_> = q
        .iter(world)
        .map(
            |(seat, health, combat, kin, playback, facts, shield, model)| {
                (
                    seat.0,
                    health.damage_taken(),
                    // THE DAMAGE RULE'S OWN ANSWER, asked here rather than
                    // reconstructed: a report that guessed at eligibility would
                    // be a second opinion about the thing it measures.
                    (
                        health.health.invulnerable.any(),
                        facts.is_some_and(|f| f.evading()),
                        // The ledge's own intangibility, which used to be
                        // spelled as a dodge roll and so could not be counted.
                        facts.is_some_and(|f| f.ledge_intangible),
                        shield.is_some_and(|s| s.parrying()),
                        !combat.vulnerable(),
                    ),
                    combat.hitstun_timer,
                    kin.vel.length(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => {
                            axis.params.abilities.tumble_speed
                        }
                        _ => 0.0,
                    },
                    playback.map(|p| (p.spec.id.clone(), p.t, p.smash_charge_fraction())),
                    facts.copied(),
                    shield.copied(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => Some(axis.state.tech_press_timer),
                        _ => None,
                    },
                )
            },
        )
        .collect();
    for (
        seat,
        damage,
        vulnerable,
        hitstun,
        speed,
        tumble_speed,
        playback,
        facts,
        shield,
        tech_timer,
    ) in rows
    {
        let Some(tally) = totals.get_mut(seat) else {
            continue;
        };
        tally.damage = damage;
        // ON THE RISING EDGE OF HITSTUN, which is the tick the launch was
        // written. Sampling any later reads gravity's work as the attacker's:
        // a body launched downward is faster every tick it falls, and the
        // threshold it had to beat was the one it left with.
        if hitstun > 0.0 && hitstun_was[seat] <= 0.0 {
            tally.top_speed = tally.top_speed.max(speed);
        }
        hitstun_was[seat] = hitstun;
        tally.tumble_speed = tumble_speed;
        if hitstun > 0.0 {
            tally.hitstun += 1;
        }
        if let Some(facts) = facts {
            if facts.tumbling {
                tally.tumbling += 1;
            }
            if facts.knocked_down {
                tally.knocked_down += 1;
            }
            if facts.evading() {
                tally.evading += 1;
            }
        }
        let (invuln, evading, ledge, parry_window, iframes) = vulnerable;
        if invuln || evading || parry_window || iframes {
            tally.unhittable += 1;
        }
        if invuln {
            tally.unhit_invuln += 1;
        }
        if evading {
            tally.unhit_evading += 1;
        }
        if ledge {
            tally.unhit_ledge += 1;
        }
        if parry_window {
            tally.unhit_parry_window += 1;
        }
        if iframes {
            tally.unhit_iframes += 1;
        }
        if let Some(shield) = shield {
            if shield.active {
                tally.shielding += 1;
            }
            // An EVENT, not a state: the timer is counted on the tick it rises.
            if shield.parry_caught_timer > parry_was[seat] {
                tally.parries_caught += 1;
            }
            parry_was[seat] = shield.parry_caught_timer;
        }
        if tech_timer.is_some_and(|t| t > 0.0) {
            tally.tech_armed += 1;
        }
        if let Some((id, t, charge)) = playback {
            let fresh = match &live_move[seat] {
                Some((last_id, last_t)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                tally.moves_started += 1;
            }
            live_move[seat] = Some((id, t));
            if let Some(fraction) = charge {
                tally.charge_held += 1;
                tally.best_charge = tally.best_charge.max(fraction);
            }
        } else {
            live_move[seat] = None;
        }
    }
}

/// `min–median–max` across runs, which is the shape a threshold should be picked
/// off. One number from one run is a sample of a noisy process, and this rig
/// exists because a change judged on one made the suite worse.
fn report_spread(character: &str, seconds: usize, all: &[Vec<Tally>]) {
    println!(
        "match_report: {character} vs {character}, {seconds}s × {} runs, per-run TOTALS across both seats\n",
        all.len()
    );
    let spread = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).sum::<f32>())
            .collect();
        values.sort_by(f32::total_cmp);
        let median = values[values.len() / 2];
        format!(
            "{:.0}–{:.0}–{:.0}",
            values.first().copied().unwrap_or(0.0),
            median,
            values.last().copied().unwrap_or(0.0)
        )
    };
    let peak = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).fold(0.0f32, f32::max))
            .collect();
        values.sort_by(f32::total_cmp);
        format!(
            "{:.2}–{:.2}–{:.2}",
            values.first().copied().unwrap_or(0.0),
            values[values.len() / 2],
            values.last().copied().unwrap_or(0.0)
        )
    };
    println!("  damage      {}", spread(|t| t.damage as f32));
    println!("  moves       {}", spread(|t| t.moves_started as f32));
    println!("  hitstun     {}", spread(|t| t.hitstun as f32));
    println!("  tumbling    {}", spread(|t| t.tumbling as f32));
    println!("  downed      {}", spread(|t| t.knocked_down as f32));
    println!("  evading     {}", spread(|t| t.evading as f32));
    println!("  unhittable  {}", spread(|t| t.unhittable as f32));
    println!("  shielding   {}", spread(|t| t.shielding as f32));
    println!("  parries     {}", spread(|t| t.parries_caught as f32));
    println!("  techs       {}", spread(|t| t.tech_armed as f32));
    println!("  best charge {}", peak(|t| t.best_charge));
    println!("  peak launch {}", peak(|t| t.top_speed));
    println!(
        "\nmin–median–max across runs. Counts are summed over both seats; charge and \
         launch are the best either seat reached."
    );
}
