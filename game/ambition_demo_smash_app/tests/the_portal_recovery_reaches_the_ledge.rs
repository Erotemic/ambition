//! The portal up-b is a recovery: from off the side of the main platform,
//! facing it, with a neutral stick, falling into the pair puts the fighter
//! on the ledge.
//!
//! The pair is the one Alice's and GNU-ton's up-b author
//! (`movesets/alice.ron`, `movesets/gnu_ton.ron`: rise 190, lean 25). It is
//! injected, as in `the_tether_catches_a_ledge`, because the demo roster's
//! stand-ins do not author it. The geometry is read from the live stage.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};

fn a_live_stage() -> App {
    let characters = [ambition_demo_smash::SMASH_GEORGE_BOOUL, ambition_demo_smash::SMASH_GEORGE_BOOUL];
    let mut app = ambition_demo_smash_app::build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let roster = ambition_demo_smash::smash_roster(characters);
    let countdown = roster.rules.opening_countdown_ticks;
    app.world_mut().insert_resource(roster);
    app.world_mut().write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
    ));
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    app
}

/// The top-left corner of the widest solid on the live stage.
fn the_main_platforms_left_lip(app: &mut App) -> ae::Vec2 {
    let mut rooms = app.world_mut().query::<&ae::RoomGeometry>();
    let world = rooms.iter(app.world()).next().expect("a live stage has room geometry");
    let widest = world.0.blocks.iter()
        .max_by(|a, b| (a.aabb.max.x - a.aabb.min.x).partial_cmp(&(b.aabb.max.x - b.aabb.min.x)).expect("finite"))
        .expect("a stage has a solid");
    ae::Vec2::new(widest.aabb.min.x, widest.aabb.min.y)
}

/// Seat 0: the human seat with no controller, which acts only when told.
fn a_seated_fighter(app: &mut App) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &ambition_platformer2d::actor::MatchSeat)>();
    let mut seats: Vec<(Entity, usize)> = query.iter(app.world()).map(|(entity, seat)| (entity, seat.0)).collect();
    seats.sort_by_key(|(_, seat)| *seat);
    seats.first().map(|(entity, _)| *entity).expect("a live match seats fighters")
}

fn hanging(app: &App, who: Entity) -> bool {
    matches!(app.world().get::<ae::MotionModel>(who), Some(ae::MotionModel::AxisSwept(axis)) if axis.state.ledge_grab.is_some())
}

/// Where the fighter is 200 ticks after opening the pair `depth` below the
/// lip and 90 px out, falling: on the ledge, or not.
fn recovers_from(depth: f32) -> bool {
    let mut app = a_live_stage();
    let lip = the_main_platforms_left_lip(&mut app);
    let her = a_seated_fighter(&mut app);
    {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(her).unwrap();
        kin.pos = ae::Vec2::new(lip.x - 90.0, lip.y + depth);
        kin.vel = ae::Vec2::new(0.0, 300.0);
        kin.facing = 1.0;
    }
    app.update();
    let params = PortalPairParams {
        rise: 190.0,
        half_extent: (26.0, 6.0),
        lifetime_s: 2.5,
        close_on_transit: false,
        tilt_degrees: 0.0,
        aim_tilt_degrees: 32.0,
        lean_degrees: 25.0,
        channel_index: 8,
    };
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PORTAL_PAIR.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params).expect("params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor: her, request, move_instance: None });
    for _ in 0..200 {
        app.update();
        if hanging(&app, her) {
            return true;
        }
    }
    false
}

/// Measured before the lean (a pair straight up, both mouths level): none of
/// the three depths recovered; she fell through the pair in a loop under the
/// ledge until it closed.
#[test]
fn the_portal_pair_puts_a_fighter_below_the_ledge_on_it() {
    let missed: Vec<f32> = [0.0, 80.0, 160.0].into_iter().filter(|depth| !recovers_from(*depth)).collect();
    assert!(missed.is_empty(), "the portal recovery did not reach the ledge from {missed:?} px below the lip");
}
