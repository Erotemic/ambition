//! The training reset puts every fighter back where the match seated it, at
//! rest, on the percent it was asked for, in the shipped composition.

use ambition_demo_smash::training::{reset_fighters, TrainingReset};
use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::engine_core::{BodyKinematics, RoomGeometry};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

/// Two CPUs, two seconds past the countdown, so both have moved.
fn a_fight_under_way() -> App {
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let roster = ambition_demo_smash::smash_roster_at_levels(&ambition_demo_smash::smash_pack::PACK.prepared(), characters, &[5, 5]);
    let countdown = roster.rules.opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..(countdown as usize + 150) {
        app.update();
    }
    app
}

/// Each seat's (seat, position, velocity, meter), in seat order.
fn fighters(app: &mut App) -> Vec<(usize, Vec2, Vec2, i32)> {
    let mut rows: Vec<_> = app
        .world_mut()
        .query::<(&MatchSeat, &BodyKinematics, &BodyHealth)>()
        .iter(app.world())
        .map(|(seat, kin, health)| (seat.0, kin.pos, kin.vel, health.damage_taken()))
        .collect();
    rows.sort_by_key(|row| row.0);
    rows
}

#[test]
fn a_training_reset_puts_the_fighters_back_on_the_percent_asked() {
    let mut app = a_fight_under_way();
    let spawn = app
        .world_mut()
        .query::<&RoomGeometry>()
        .iter(app.world())
        .next()
        .expect("the stage is live")
        .0
        .spawn;
    let before = fighters(&mut app);
    assert_eq!(before.len(), 2, "both seats are in play: {before:?}");
    // Otherwise the position check below could not fail.
    assert!(
        before.iter().any(|(seat, pos, _, _)| {
            let (at, _) = ambition_platformer2d::versus_match::seat_placement(*seat, spawn);
            (pos.x - at.x).abs() >= 1.0
        }),
        "nobody left their seat in the fight, so the reset has nothing to undo: {before:?}"
    );
    // Something to undo: the meters, whatever the fight has done to them.
    for mut health in app.world_mut().query::<&mut BodyHealth>().iter_mut(app.world_mut()) {
        health.damage(70);
    }

    app.world_mut()
        .run_system_once_with(reset_fighters, TrainingReset::at_percent(80))
        .expect("the reset runs");
    for (seat, pos, vel, meter) in fighters(&mut app) {
        let (at, _) = ambition_platformer2d::versus_match::seat_placement(seat, spawn);
        assert!(
            (pos.x - at.x).abs() < 1.0,
            "seat {seat} stands at x {} and was seated at {}",
            pos.x,
            at.x
        );
        assert_eq!(vel, Vec2::ZERO, "seat {seat} kept moving through the reset");
        assert_eq!(meter, 80, "seat {seat} is on {meter}, not the 80% asked");
    }

    // Back to the start: no handicap was set, so no meter.
    app.world_mut()
        .run_system_once_with(reset_fighters, TrainingReset::START)
        .expect("the reset runs");
    let meters: Vec<i32> = fighters(&mut app).iter().map(|row| row.3).collect();
    assert_eq!(meters, vec![0, 0]);
    // And the match goes on from there.
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(fighters(&mut app).len(), 2);
}
