//! ACT2 acceptance: a real local-hitlag hit holds the bodies in it, and only
//! them, in a CPU-only match and with a human seat alike.
//!
//! Through a whole step of hitstop a body's travel, move time and physical
//! pose clock stand still while the match goes on. On the step its hitstop
//! runs out, its pose clock spends only the share the step released. A
//! single-room primary-body world freeze would hold every clock in a human
//! match and none of them in a CPU-only one; the per-body budget
//! (`BodyActionStep`) makes the two the same.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyActionStep, BodyCombat};
use ambition_platformer2d::combat::hurtbox_resolution::BodyPoseClock;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::sim::SimTick;
use bevy::prelude::*;

const STEP: f32 = 1.0 / 60.0;

#[derive(Clone, Debug)]
struct Sample {
    hitstop: f32,
    pos: Vec2,
    pose: Option<(String, f32)>,
    move_time: Option<(String, f32)>,
    released: Option<f32>,
}

fn sample(app: &mut App) -> (u64, Vec<(Entity, Sample)>) {
    let tick = app.world().resource::<SimTick>().get();
    let world = app.world_mut();
    let mut q = world.query_filtered::<(
        Entity,
        Option<&BodyCombat>,
        Option<&BodyKinematics>,
        Option<&BodyPoseClock>,
        Option<&MovePlayback>,
        Option<&BodyActionStep>,
    ), With<MatchSeat>>();
    let rows = q
        .iter(world)
        .map(|(entity, combat, kin, clock, playback, step)| {
            let (combat, kin) = (combat.expect("a seated fighter has combat state"), kin.expect("and a body"));
            (entity, Sample {
                hitstop: combat.hitstop_timer,
                pos: kin.pos,
                // Only a body that reads its pose clock (a rig, authored hurt
                // profiles) carries one.
                pose: clock.map(|c| (c.pose.clone(), c.elapsed_s)),
                move_time: playback.map(|p| (p.spec.id.clone(), p.t)),
                released: step.map(|s| s.released),
            })
        })
        .collect();
    (tick, rows)
}

/// What one match showed: steps held whole, and expiry steps checked.
#[derive(Default, Debug)]
struct Holds {
    held_steps: usize,
    expiries: usize,
    /// Held steps in which the other fighter, not in hitstop, moved or
    /// advanced its move: the hold is the struck body's, not the match's.
    others_ran: usize,
}

/// Seat a match, then watch every one-tick step for `ticks` updates.
fn watch(roster: ambition_platformer2d::actor::MatchParticipantRoster, label: &str, ticks: usize) -> Holds {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let countdown = roster.rules.opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut().write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
    ));
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    let mut holds = Holds::default();
    let (mut tick, mut before) = sample(&mut app);
    assert_eq!(before.len(), 2, "{label}: the match seats two fighters");
    for _ in 0..ticks {
        app.update();
        let (now, after) = sample(&mut app);
        // Only a step of exactly one tick says what one step did.
        if now == tick + 1 {
            for (entity, was) in &before {
                let Some((_, is)) = after.iter().find(|(e, _)| e == entity) else { continue };
                if was.hitstop > STEP * 1.001 {
                    // Held through the whole step.
                    holds.held_steps += 1;
                    let other_ran = before.iter().filter(|(e, _)| e != entity).any(|(other, o_was)| {
                        o_was.hitstop <= 0.0 && after.iter().find(|(e, _)| e == other).is_some_and(|(_, o_is)| {
                            o_is.pos != o_was.pos || o_is.move_time != o_was.move_time
                        })
                    });
                    holds.others_ran += usize::from(other_ran);
                    assert_eq!(is.released, Some(0.0), "{label}: a body in hitstop released action time: {was:?} -> {is:?}");
                    assert_eq!(is.pos, was.pos, "{label}: a body in hitstop travelled: {was:?} -> {is:?}");
                    if let (Some((p_was, e_was)), Some((p_is, e_is))) = (&was.pose, &is.pose) {
                        if p_was == p_is {
                            assert_eq!(e_is, e_was, "{label}: the pose clock ran through hitstop: {was:?} -> {is:?}");
                        }
                    }
                    if let (Some((id_was, t_was)), Some((id_is, t_is))) = (&was.move_time, &is.move_time) {
                        if id_was == id_is {
                            assert_eq!(t_is, t_was, "{label}: the move ran through hitstop: {was:?} -> {is:?}");
                        }
                    }
                } else if was.hitstop > 0.0 && is.hitstop <= 0.0 {
                    // The step it runs out: only the remainder is released.
                    let expected = ((STEP - was.hitstop) / STEP).clamp(0.0, 1.0);
                    let released = is.released.expect("a seated body resolves its action step");
                    assert!((released - expected).abs() < 1e-3,
                        "{label}: hitstop {} of a {STEP} step released {released}, not {expected}", was.hitstop);
                    if let (Some((p_was, e_was)), Some((p_is, e_is))) = (&was.pose, &is.pose) {
                        if p_was == p_is {
                            let spent = e_is - e_was;
                            assert!(spent <= STEP * released + 1e-4,
                                "{label}: the pose clock spent {spent} of a step that released {released}");
                        }
                    }
                    holds.expiries += 1;
                }
            }
        }
        tick = now;
        before = after;
    }
    eprintln!("[act2] {label}: {holds:?}");
    holds
}

#[test]
fn local_hitlag_holds_only_the_struck_bodies_with_cpus_and_with_a_human_seat() {
    let fighters = [ambition_demo_smash::SMASH_GEORGE_BOOUL; 2];
    let mut cpu_only = ambition_demo_smash::smash_roster_at_levels(fighters, &[9, 9]);
    cpu_only.rules.stocks = Some(25);
    // Seat 0 is a human with no controller: neutral input, struck by the CPU.
    let mut human = ambition_demo_smash::smash_roster(fighters);
    human.rules.stocks = Some(25);
    for (roster, label) in [(cpu_only, "cpu-only"), (human, "human seat")] {
        let holds = watch(roster, label, 1800);
        assert!(holds.held_steps > 0 && holds.expiries > 0 && holds.others_ran > 0,
            "{label}: the window must contain a real hit with hitlag: {holds:?}");
    }
}
