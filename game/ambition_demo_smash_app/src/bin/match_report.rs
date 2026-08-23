//! Count what two CPUs actually DO to each other over a match.
//!
//! `cargo run -p ambition_demo_smash_app --bin match_report -- [SECONDS] [CHARACTER]`
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
    let character = args
        .next()
        .unwrap_or_else(|| ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string());

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // BOTH SEATS CPU. `SmashSelect::roster` makes every locked seat a HUMAN,
    // which is right for a couch game and wrong here: a report driven through it
    // measures two fighters standing still while nobody presses anything.
    let characters = [character.as_str(), character.as_str()];
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
                    ambition_platformer2d::combat::util::body_vulnerable(
                        health.health.invulnerable,
                        facts.is_some_and(|f| f.evading()),
                        &shield.copied().unwrap_or_default(),
                        combat,
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
        if !vulnerable {
            tally.unhittable += 1;
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
