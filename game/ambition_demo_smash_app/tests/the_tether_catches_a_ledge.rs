//! ⭐⭐ THE ONE CLAIM THE TETHER'S UNIT TESTS REFUSE TO MAKE: that she ends up
//! HANGING.
//!
//! `ambition_demo_smash::tether`'s own tests say so in their header. They prove
//! the reel arrives, that it releases at a point the ledge authority's probe
//! accepts, and that it hands her over falling rather than rising — but the
//! catching itself happens inside the movement kernel, which a headless
//! two-system fixture does not run. ⇒ This test runs a REAL MATCH on a REAL
//! STAGE and asks the only question that matters to a player: after the line
//! bites, is she on the ledge?
//!
//! ⛔ IT INJECTS THE TECHNIQUE RATHER THAN PRESSING UP-B, and that is deliberate
//! rather than a shortcut. The demo app's roster is three stand-ins who do not
//! author a tether; the fighter who does is content-side. A counter's response
//! and a move's event both arrive as an ordinary `ActorActionMessage`, so
//! injecting one is the same road the authored move takes — and it keeps this
//! test about the ENGINE composition rather than about one fighter's numbers.
//!
//! ⚠ THE GEOMETRY IS READ FROM THE LIVE STAGE, never hardcoded. A stage whose
//! layout changed would otherwise turn this into a test of two stale constants.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_tether::{TetherPullParams, TETHER_PULL};
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

/// The top-left corner of the widest solid on the live stage — the main
/// platform's left lip, whatever the stage is.
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
    // +Y is down in this engine, so `min` IS the top-left corner.
    ae::Vec2::new(widest.aabb.min.x, widest.aabb.min.y)
}

/// Seat 0, and the seat number matters.
///
/// ⛔⛔ THE FIRST VERSION TOOK WHATEVER THE QUERY YIELDED FIRST and got a
/// CPU-DRIVEN fighter: she was being walked left at 270px/s with her facing
/// flipped by her own brain, so the line was thrown away from the stage and bit
/// nothing. `reel=None` on every tick is what gave it away. ⇒ Seat 0 is the
/// HUMAN seat with no controller attached — it acts only when this test tells it
/// to, which is the only way to attribute anything that happens to the tether.
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

/// ⭐⭐ SHE THROWS A LINE FROM OFF THE SIDE AND ENDS UP ON THE LEDGE.
#[test]
fn a_tether_thrown_at_a_ledge_ends_in_a_hang() {
    let mut app = a_live_stage();
    let lip = the_main_platforms_left_lip(&mut app);
    let her = a_seated_fighter(&mut app);

    // Off the left edge and a little below the lip, facing the stage: the
    // position a recovering player is actually in when they reach for a ledge.
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
        .write_message(ActorActionMessage { actor: her, request });

    // ⛔⛔ PROMPTLY, AND THE DEADLINE IS THE ASSERTION. A 40-tick window cannot
    // tell this row's work from the absence of it: measured on this stage, a
    // reel that chases the anchor instead of asking the authority pins against
    // the wall, rides out its whole 0.35s timeout, releases on the clock, and
    // she is caught at tick 22 anyway. Asking the authority catches her at tick
    // 6. ⇒ A generous window is green either way, which is exactly what the
    // poison found: the test passed with the fix reverted.
    //
    // ⚠ 12 is chosen between the two measurements with room on both sides, not
    // fitted to the good one.
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

/// ⛔⛔ THE PAIRED MISS, and without it the test above proves nothing about the
/// TETHER. A fighter dropped beside a ledge in a live match has a ledge
/// authority running every frame that will happily catch them on its own — the
/// Smash-style auto-snap needs only that they fall past the lip. So the same
/// position, the same 40 ticks, and a line thrown the OTHER WAY must NOT end in
/// a hang, or "she hung" was never evidence of anything this row built.
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
        // Facing AWAY from the stage: the line goes left, over open air.
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
        .write_message(ActorActionMessage { actor: her, request });

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
