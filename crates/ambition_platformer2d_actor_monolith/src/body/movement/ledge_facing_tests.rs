//! A body that holds a ledge faces the ledge, whatever its control frame says.
//!
//! The ledge tick turns the body toward the platform. The actor road then
//! commits the control frame's facing, and a fighter's brain (or a human's
//! stick) always sends one, so before the fix the grab never turned anyone
//! around. This drives the REAL `ActorMut::update` with a frame that faces
//! away from the ledge.
use super::*;
use ambition_body_seed::clusters::SeedActorMut;
use ambition_body_seed::{ActorBody, ActorClusterSeed};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_entity_catalog::placements::CharacterBrain;

#[test]
fn a_body_that_grabs_a_ledge_turns_to_face_it() {
    // A block whose left wall a body hangs on; its top-left corner is the ledge.
    let world = ae::World::new(
        "ledge_facing_test",
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(80.0, 160.0),
        vec![ae::Block::solid("ledge", ae::Vec2::new(100.0, 100.0), ae::Vec2::new(200.0, 200.0))],
    );
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(12.0, 20.0));
    let mut seed = ActorClusterSeed::new(
        "climber".to_string(),
        "Climber".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    let mut abilities = ae::AbilitySet::classic_actor();
    abilities.ledge_grab = true;
    seed.body = ActorBody::from_abilities(abilities, false, seed.kin.size);
    seed.kin.pos = ae::Vec2::new(86.0, 110.0);
    seed.kin.vel = ae::Vec2::new(30.0, 20.0);
    seed.kin.facing = -1.0;
    seed.surface.gravity_scale = 1.0;
    seed.body.0.wall.on_wall = true;
    seed.body.0.wall.wall_normal_x = -1.0;

    let mut model = ae::MotionModel::axis_swept(ae::DEFAULT_AXIS_SWEPT_PARAMS);
    if let ae::MotionModel::AxisSwept(axis) = &mut model {
        axis.state.wall_clinging = true;
    }
    // Pushing into the wall, but FACING AWAY from it: what a brain chasing a
    // target behind it, or a human drifting back, sends.
    let mut frame = ActorControlFrame::neutral();
    frame.locomotion = ae::LocalAxes::new(1.0, 0.0);
    frame.facing = -1.0;

    let mut em = seed.as_actor_mut();
    let mut held = false;
    for _ in 0..4 {
        em.update(
            &world,
            FeatureCombatTuning::default(),
            1.0 / 60.0,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            None,
            ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
            None,
            &mut ambition_characters::actor::BodyCombat::default(),
            false,
            false,
            ae::BodyContactField::NONE,
        );
        held |= model.holds_a_ledge();
        if held {
            assert_eq!(em.kin.facing, 1.0, "a body on a ledge faces into the platform");
        }
    }
    assert!(held, "premise: the body grabbed the ledge");
}
