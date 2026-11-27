//! GRAPPLE G2, in the running game: the grapple hangs its body on a rope.
//!
//! Scout holds the grapple and jumps under a beam. At the top of the jump,
//! Attack aimed up and to the right casts the line; it bites the beam's
//! underside above the body, and the `grapple` module asks the host to hang
//! the body there (`ambition.motion.hang_on_rope`). The kernel's held wire
//! then swings Scout down and through, beneath the bite, staying on the
//! rope; the rope is a motion fact the renderer draws. Jump lets go, with the
//! swing's run and a hop. Under a GGRS sync-test session, which rewinds and
//! replays the bite, the swing and the release.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::Entity;

use crate::common::base;

/// The beam's underside, above the basement floor (Scout's centre stands at
/// about y = 1920).
const BEAM_UNDERSIDE: f32 = 1680.0;
const BEAM_LEFT: f32 = 240.0;

fn sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        crate::common::sandbox_options()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the sandbox builds under a GGRS sync-test session")
}

fn scout(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world.query_filtered::<Entity, PrimaryPlayerOnly>().iter(world).next().expect("a primary body")
}

fn wire(sim: &Platformer2dSimHarness, body: Entity) -> Option<ae::WireState> {
    match sim.world().get::<ae::movement::MotionModel>(body)? {
        ae::movement::MotionModel::AxisSwept(axis) => axis.state.wire,
        _ => None,
    }
}

fn kin(sim: &Platformer2dSimHarness, body: Entity) -> ae::BodyKinematics {
    *sim.world().get::<ae::BodyKinematics>(body).expect("a body")
}

/// ⭐ A bite above hangs Scout on the rope; she swings through beneath it on
/// the rope's length, the rope is drawn from her motion facts, and a jump
/// lets go with the swing's run and a hop. A rewinding session agrees.
#[test]
fn a_grapple_bite_above_swings_and_a_jump_lets_go() {
    let mut sim = sim();
    let body = scout(&mut sim);
    let spec = ambition_platformer2d::characters::brain::held_item_by_id("grapple").expect("the grapple");
    sim.world_mut().entity_mut(body).insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
    // On the basement's solid floor, as `a_wielded_grapple_runs_on_the_extension_host`.
    sim.world_mut().get_mut::<ae::BodyKinematics>(body).expect("a body").pos = ae::Vec2::new(200.0, 1920.0);
    sim.add_block(ae::Block::solid("grapple_beam", ae::Vec2::new(BEAM_LEFT, BEAM_UNDERSIDE - 40.0), ae::Vec2::new(320.0, 40.0)));
    sim.rebase_rollback_history().expect("the rollback history rebases over the beam");
    sim.step_n(base(), 30);
    let floor_y = kin(&sim, body).pos.y;

    // Jump, and at the top of it, bite up and to the right.
    sim.step(AgentAction { jump: true, jump_held: true, ..base() });
    let mut ticks = 0;
    while kin(&sim, body).vel.y < -40.0 && ticks < 60 {
        sim.step(AgentAction { jump_held: true, ..base() });
        ticks += 1;
    }
    let top = kin(&sim, body);
    assert!(top.pos.y < floor_y - 40.0, "premise: the jump rose ({:?} from {floor_y})", top.pos);
    sim.step(AgentAction { attack: true, move_x: 0.7, move_y: -0.7, ..base() });
    let rope = wire(&sim, body).expect("the line bit the beam and hung Scout on it");
    assert!(rope.held, "a grapple's rope is held, not a timed lift: {rope:?}");
    assert!(
        (rope.anchor.y - BEAM_UNDERSIDE).abs() < 2.0 && rope.anchor.x > top.pos.x,
        "the bite is on the beam's underside, up and to the right: {rope:?} from {:?}",
        top.pos
    );
    let facts = sim.world().get::<ae::movement::BodyMotionFacts>(body).copied();
    // The facts are published by the step; the next tick publishes the rope.
    sim.step(base());
    let drawn = sim.world().get::<ae::movement::BodyMotionFacts>(body).and_then(|facts| facts.wire_anchor);
    assert_eq!(drawn, Some(rope.anchor), "the rope is a motion fact the renderer draws (before: {facts:?})");

    // The swing: down and through beneath the bite, on the rope's length.
    let mut through = false;
    for _ in 0..40 {
        sim.step(base());
        let now = kin(&sim, body);
        let on = wire(&sim, body).expect("a held rope does not let go on its own");
        let reach = now.pos.distance(on.anchor);
        assert!((reach - on.length).abs() < 12.0, "Scout left the rope: {reach:.1} from a {:.1} rope", on.length);
        through |= now.pos.x > on.anchor.x;
        if through && now.vel.x > 60.0 {
            break;
        }
    }
    assert!(through, "Scout never swung beneath the bite: {:?}", kin(&sim, body));
    let swinging = kin(&sim, body).vel;

    // Jump lets go with the swing's run and a hop.
    sim.step(AgentAction { jump: true, jump_held: true, ..base() });
    assert!(wire(&sim, body).is_none(), "the jump let go of the rope");
    let left = kin(&sim, body);
    assert!(left.vel.x > 40.0, "the release kept the swing's run ({swinging:?} -> {:?})", left.vel);
    assert!(left.vel.y < -100.0, "the release hopped: {:?}", left.vel);
    sim.step_n(base(), 10);
    assert!(kin(&sim, body).vel.x > 20.0, "the air stop assist ate the swing: {:?}", kin(&sim, body).vel);
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
}
