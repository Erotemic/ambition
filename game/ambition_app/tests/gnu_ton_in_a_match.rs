//! GNU-ton as a fighter, judged in a Smash match: what his moves do to a foe
//! who stands beside him, as the runtime plays them. His table's numbers were
//! set for his body and not played (`docs/planning/game/gnu-ton-fighter.md`);
//! this asks the match.
//!
//! The foe is the training sandbag (`INSPECTION_TARGET`, seated on the
//! stand-still brain). It is 48 px tall, the height of the robot, the
//! player's own fighter.

use ambition_platformer2d::actor::{ActiveSessionScope, BodyKinematics, MatchSeat};
use ambition_platformer2d::character::WornCharacter;
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::sim::ControlFrame;
use bevy::prelude::*;

const TARGET: &str = ambition_demo_smash::INSPECTION_TARGET;
/// A full stick is a smash; a tilt asks for less (`move_exercise::TILT_AXIS`).
const TILT: f32 = 0.65;
/// Centre to centre: the two bodies side by side, edge at edge.
const BESIDE_PX: f32 = 30.0;

/// A match with `fighter` in seat 0 and the sandbag in seat 1, run by one
/// update a tick.
fn a_match(fighter: &str) -> App {
    let mut app = ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    for _ in 0..30 {
        app.update();
    }
    let before = app.world().resource::<ActiveSessionScope>().current();
    app.world_mut().insert_resource(ambition_demo_smash::smash_roster_with_passive_targets([fighter, TARGET]));
    app.world_mut().write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
    ));
    for _ in 0..240 {
        app.update();
        let scope = app.world().resource::<ActiveSessionScope>().current();
        if scope.is_some() && scope != before && seat(&mut app, 0).is_some_and(|(_, id, _)| id == fighter) && seat(&mut app, 1).is_some() {
            return app;
        }
    }
    panic!("the match of {fighter} and {TARGET} did not start");
}

/// The body in `seat`, the character it wears, and its kinematics.
fn seat(app: &mut App, seat: usize) -> Option<(Entity, String, BodyKinematics)> {
    let world = app.world_mut();
    let mut seats = world.query::<(Entity, &MatchSeat, &WornCharacter, &BodyKinematics)>();
    seats.iter(world).find(|(_, at, ..)| at.0 == seat).map(|(body, _, worn, kin)| (body, worn.id().to_string(), *kin))
}

fn step(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
    app.update();
}

/// Walk the fighter to stand beside the sandbag, facing it, and stand still.
fn beside_the_sandbag(app: &mut App) {
    for _ in 0..600 {
        let gap = seat(app, 1).unwrap().2.pos.x - seat(app, 0).unwrap().2.pos.x;
        if gap.abs() <= BESIDE_PX {
            for _ in 0..30 {
                step(app, ControlFrame::default());
            }
            return;
        }
        step(app, ControlFrame { axis_x: gap.signum(), ..Default::default() });
    }
    panic!("the fighter did not come beside the sandbag");
}

/// Press Attack once with the stick at `(x, y)` (x toward the sandbag), and
/// say whether the sandbag went into hitstun in the next 40 ticks.
fn strikes(fighter: &str, x: f32, y: f32) -> bool {
    let mut app = a_match(fighter);
    beside_the_sandbag(&mut app);
    let toward = (seat(&mut app, 1).unwrap().2.pos.x - seat(&mut app, 0).unwrap().2.pos.x).signum();
    let sandbag = seat(&mut app, 1).unwrap().0;
    let press = ControlFrame { axis_x: x * toward, axis_y: y, attack_pressed: true, attack_held: true, ..Default::default() };
    step(&mut app, press);
    for _ in 0..40 {
        step(&mut app, ControlFrame { axis_x: x * toward, axis_y: y, attack_held: true, ..Default::default() });
        if app.world().get::<BodyCombat>(sandbag).is_some_and(|combat| combat.hitstun_timer > 0.0) {
            return true;
        }
    }
    false
}

/// ⭐ EACH OF GNU-TON'S GROUND STRIKES HITS A SHORT FOE WHO STANDS BESIDE
/// HIM. His up tilt's box hung 0.5 px over the sandbag's head (measured in
/// the match, 2026-10-13: `moveset_takes --verbs attack_up` at 34, 20 and 5
/// px), so against the robot it was a move that could not hit. The control
/// is the patent clerk's up tilt, whose table his was made from: it hits in
/// the same scenario, so a miss is the move and not the staging.
#[test]
fn each_ground_strike_of_gnu_ton_hits_a_short_foe_beside_him() {
    assert!(strikes("special_patent_clerk", 0.0, -TILT), "the control: the clerk's up tilt hits the sandbag beside him");
    let mut misses = Vec::new();
    for (label, x, y) in [("jab", 0.0, 0.0), ("forward tilt", TILT, 0.0), ("up tilt", 0.0, -TILT), ("down tilt", 0.0, TILT)] {
        if !strikes("npc_gnu_ton", x, y) {
            misses.push(label);
        }
    }
    assert!(misses.is_empty(), "GNU-ton's strikes that miss a foe of the robot's height beside him: {misses:?}");
}
