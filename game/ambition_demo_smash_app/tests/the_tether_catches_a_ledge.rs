//! After the tether line bites, the fighter ends up hanging on the ledge.
//!
//! The unit tests in `ambition_demo_smash::tether` prove the reel arrives,
//! releases at a point the ledge authority's probe accepts, and hands her
//! over falling. The catch happens inside the movement kernel, which their
//! fixture does not run. This test runs a real match on a real stage.
//!
//! It injects the technique instead of pressing up-B: the demo roster's
//! stand-ins do not author a tether. A move's event arrives as an ordinary
//! `ActorActionMessage`, so injection takes the same road as the authored
//! move, and the test stays about the engine composition.
//!
//! The geometry is read from the live stage, never hardcoded.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_tether::{TetherPullParams, TETHER_PULL};
use ambition_platformer2d::engine_core as ae;

fn a_live_stage() -> App {
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let mut app = ambition_demo_smash_app::build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let roster = ambition_demo_smash::smash_roster(characters);
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Past the 3-2-1-GO: a fighter held by the countdown is forbidden to act.
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    app
}

/// The top-left corner of the widest solid on the live stage: the main
/// platform's left lip.
fn the_main_platforms_left_lip(app: &mut App) -> ae::Vec2 {
    let mut rooms = app.world_mut().query::<&ae::RoomGeometry>();
    let world = rooms
        .iter(app.world())
        .next()
        .expect("a live stage has room geometry");
    let widest = world
        .0
        .blocks
        .iter()
        .max_by(|a, b| {
            (a.aabb.max.x - a.aabb.min.x)
                .partial_cmp(&(b.aabb.max.x - b.aabb.min.x))
                .expect("stage geometry is finite")
        })
        .expect("a stage has at least one solid");
    // +Y is down in this engine, so `min` is the top-left corner.
    ae::Vec2::new(widest.aabb.min.x, widest.aabb.min.y)
}

/// Seat 0, which is the human seat with no controller. It acts only when
/// this test tells it to, so everything that happens can be attributed to
/// the tether. A CPU-driven seat would walk and turn on its own.
fn a_seated_fighter(app: &mut App) -> Entity {
    let mut query = app
        .world_mut()
        .query::<(Entity, &ambition_platformer2d::actor::MatchSeat, &ae::BodyKinematics)>();
    let mut seats: Vec<(Entity, usize)> = query
        .iter(app.world())
        .map(|(entity, seat, _)| (entity, seat.0))
        .collect();
    seats.sort_by_key(|(_, seat)| *seat);
    seats
        .first()
        .map(|(entity, _)| *entity)
        .expect("a live match seats fighters")
}

fn hanging(app: &mut App, who: Entity) -> bool {
    match app.world().get::<ae::MotionModel>(who) {
        Some(ae::MotionModel::AxisSwept(axis)) => axis.state.ledge_grab.is_some(),
        other => panic!("the fighter is not on an axis-swept model: {other:?}"),
    }
}

/// She throws a line from off the side and ends up on the ledge.
#[test]
fn a_tether_thrown_at_a_ledge_ends_in_a_hang() {
    let mut app = a_live_stage();
    let lip = the_main_platforms_left_lip(&mut app);
    let her = a_seated_fighter(&mut app);

    // Off the left edge, a little below the lip, facing the stage: where a
    // recovering player reaches for a ledge.
    let start = ae::Vec2::new(lip.x - 70.0, lip.y + 24.0);
    {
        let world = app.world_mut();
        let mut kin = world.get_mut::<ae::BodyKinematics>(her).unwrap();
        kin.pos = start;
        kin.vel = ae::Vec2::ZERO;
        kin.facing = 1.0;
    }
    app.update();
    assert!(
        !hanging(&mut app, her),
        "she was already hanging before the line was thrown, so this test cannot \
         attribute the hang to the tether",
    );

    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(TETHER_PULL.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
            &TetherPullParams {
                reach: 150.0,
                speed: 900.0,
                timeout_s: 0.35,
            },
        )
        .expect("tether params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor: her, request, move_instance: None });

    // Promptly: the deadline is the assertion. A reel that chases the anchor
    // instead of asking the authority pins against the wall, times out, and
    // is still caught at about tick 22. Asking the authority catches her at
    // about tick 6. 12 sits between the two with room on both sides.
    for _ in 0..12 {
        app.update();
        if hanging(&mut app, her) {
            return;
        }
    }
    let kin = app.world().get::<ae::BodyKinematics>(her).unwrap().clone();
    panic!(
        "12 ticks after the line was thrown she is at {:?} and not hanging; the \
         ledge lip is {lip:?} and she started at {start:?}. A catch that arrives \
         later than this is the reel timing out and the authority picking her \
         up, not the tether delivering her",
        kin.pos,
    );
}

/// The paired miss. The ledge authority auto-snaps a fighter who falls past
/// the lip, without any tether. So the same position and window, with the
/// line thrown the other way, must not end in a hang; otherwise "she hung"
/// is no evidence about the tether.
#[test]
fn a_line_thrown_away_from_the_stage_does_not_end_in_a_hang() {
    let mut app = a_live_stage();
    let lip = the_main_platforms_left_lip(&mut app);
    let her = a_seated_fighter(&mut app);

    {
        let world = app.world_mut();
        let mut kin = world.get_mut::<ae::BodyKinematics>(her).unwrap();
        kin.pos = ae::Vec2::new(lip.x - 70.0, lip.y + 24.0);
        kin.vel = ae::Vec2::ZERO;
        // Facing away from the stage: the line goes left, over open air.
        kin.facing = -1.0;
    }
    app.update();

    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(TETHER_PULL.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
            &TetherPullParams {
                reach: 150.0,
                speed: 900.0,
                timeout_s: 0.35,
            },
        )
        .expect("tether params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor: her, request, move_instance: None });

    for _ in 0..40 {
        app.update();
        assert!(
            !hanging(&mut app, her),
            "she ended up on the ledge with her line thrown the other way, so \
             the hang in the test above is the AUTHORITY catching a falling \
             body and not the tether delivering her",
        );
    }
}
