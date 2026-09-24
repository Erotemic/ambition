#![cfg(feature = "rl_sim")]
//! Morph Ball is the Ambition protagonist's in the Ambition game, and only
//! there. The grant is the game's home-body declaration, not a character fact:
//! the robot's authored kit carries no Morph Ball, so the same robot seated by
//! an experience that grants nothing cannot morph.

use crate::common::{base, fixed_60hz_sim};
use ambition_app::AgentAction;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::body::PrimaryBody;
use bevy::prelude::*;

fn primary(sim: &mut ambition_app::Platformer2dSimHarness) -> (ae::BodyMode, ae::BodyKinematics) {
    let mut q = sim.world_mut().query_filtered::<
        (&ae::body_clusters::BodyModeState, &ae::BodyKinematics),
        With<PrimaryBody>,
    >();
    let (mode, kin) = q.single(sim.world()).expect("one primary body");
    (mode.body_mode, *kin)
}

fn feet(kin: &ae::BodyKinematics) -> f32 {
    kin.pos.y + kin.size.y * 0.5
}

/// **THE PROTAGONIST IN AMBITION CURLS INTO THE BALL AND STANDS BACK UP.**
///
/// Entry and exit both go through the collision-checked mode change: the feet
/// stay where they were in both directions, so neither shape change sinks the
/// body into the floor nor lifts it off it.
#[test]
fn the_protagonist_in_ambition_morphs_and_unmorphs_in_place() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    let (mode, standing) = primary(&mut sim);
    assert_eq!(mode, ae::BodyMode::Standing, "the protagonist did not settle standing");

    let tap = AgentAction { move_y: 1.0, down_pressed: true, ..base() };
    let mut morphed = None;
    for _ in 0..2 {
        sim.step(tap.clone());
        sim.step_n(base(), 3);
        let (mode, kin) = primary(&mut sim);
        if mode == ae::BodyMode::MorphBall {
            morphed = Some(kin);
        }
    }
    let ball = morphed.expect("a grounded double-tap DOWN never curled the protagonist into the ball");
    assert!(ball.size.y < standing.size.y, "the ball is not smaller than the standing body");
    assert!(
        (feet(&ball) - feet(&standing)).abs() < 0.5,
        "entering the ball moved the feet from {:.2} to {:.2}",
        feet(&standing),
        feet(&ball)
    );

    sim.step(AgentAction { move_y: -1.0, up_pressed: true, ..base() });
    sim.step_n(base(), 3);
    let (mode, stood) = primary(&mut sim);
    assert_eq!(mode, ae::BodyMode::Standing, "Up in open space did not stand the body back up");
    assert!((stood.size.y - standing.size.y).abs() < 0.01, "standing up did not restore the body");
    assert!(
        (feet(&stood) - feet(&standing)).abs() < 0.5,
        "leaving the ball moved the feet from {:.2} to {:.2}",
        feet(&standing),
        feet(&stood)
    );
}

/// **THE SAME ROBOT, SEATED BY AN EXPERIENCE THAT GRANTS NOTHING, CANNOT MORPH.**
///
/// Versus seats the protagonist's character through the match road, which
/// declares no Morph Ball; the robot's own kit has none to bring.
#[test]
fn the_protagonist_seated_in_versus_cannot_morph() {
    use ambition_platformer2d::actor::{BodyAbilities, MatchSeat};
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster(["player_robot_v3", "player_robot_v3"]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    let mut seats = Vec::new();
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &BodyAbilities)>();
        seats = q.iter(world).map(|(seat, a)| (seat.0, a.abilities)).collect();
        if !seats.is_empty() {
            break;
        }
    }
    assert!(!seats.is_empty(), "the match never seated a fighter with an ability set");
    for (seat, abilities) in seats {
        // ANTI-VACUITY: the seat carries a real fighting kit, so "no morph" is
        // a grant withheld and not an empty set.
        assert!(abilities.jump && abilities.attack, "seat {seat} carries no kit: {abilities:?}");
        assert!(!abilities.morph, "seat {seat} can morph in Versus: {abilities:?}");
    }
}
