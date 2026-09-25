//! A lunge's authored windup step, through the real movement kernel.

use super::*;
use crate::actor_clusters::SeedActorMut;
use ambition_body_seed::{ActorBody, ActorClusterSeed};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::action_set::{LungeSpec, MeleeActionSpec, SwipeSpec};
use ambition_entity_catalog::placements::CharacterBrain;

fn floored_world() -> ae::World {
    ae::World::new(
        "lunge_test",
        ae::Vec2::new(4000.0, 800.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(-2000.0, 100.0),
            ae::Vec2::new(4000.0, 80.0),
        )],
    )
}

/// How far a standing body travels over its swing's windup, starting the move
/// the way `trigger_moveset_moves` does (the move's start impulse, mirrored by
/// facing) and holding a neutral stick while it plays.
fn windup_travel(melee: MeleeActionSpec) -> f32 {
    let attack = ambition_characters::moveset_prefabs::attack_move_from_melee(&melee);
    let (windup, ..) = melee.timeline();
    let world = floored_world();
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "lunger".to_string(),
        "Lunger".to_string(),
        aabb,
        CharacterBrain::Passive,
        &[],
    );
    let half_h = seed.kin.size.y * 0.5;
    seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
    seed.kin.vel = ae::Vec2::ZERO;
    seed.kin.facing = 1.0;
    seed.surface.gravity_scale = 1.0;
    seed.body = ActorBody::from_abilities(
        ae::AbilitySet::classic_actor(),
        false,
        seed.kin.size,
    );
    seed.body.0.ground.on_ground = true;
    // The move's opening beats, applied as `advance_move_playback` applies
    // them: a velocity write on the owner and a hold on its movement policy.
    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    for event in attack.events.iter().filter(|event| event.at_s == 0.0) {
        match &event.kind {
            ambition_entity_catalog::MoveEventKind::Impulse { local, mode } => {
                let v = ae::Vec2::new(local.0 * seed.kin.facing, local.1);
                seed.kin.vel = match mode {
                    ambition_entity_catalog::ImpulseMode::Add => seed.kin.vel + v,
                    ambition_entity_catalog::ImpulseMode::Set => v,
                };
            }
            ambition_entity_catalog::MoveEventKind::HoldVelocity { seconds } => {
                assert!(model.hold_velocity(*seconds));
            }
            _ => {}
        }
    }
    let playback = ambition_combat::moveset::MovePlayback::new(attack, seed.kin.facing);

    let mut combat = ambition_characters::actor::BodyCombat::default();
    let start_x = seed.kin.pos.x;
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    frame.facing = 1.0;
    let dt = 1.0 / 60.0;
    let ticks = (windup / dt).round() as u32;
    for _ in 0..ticks {
        em.update(
            &world,
            FeatureCombatTuning::default(),
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            Some(&playback),
            ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
            None,
            &mut combat,
            false,
            false,
            ae::BodyContactField::NONE,
        );
    }
    em.kin.pos.x - start_x
}

/// A lunge steps its authored distance while it winds up, and a swipe, which
/// authors no step, stands still.
///
/// Measured through the movement kernel because the kernel is what used to
/// eat it: the same speed as a bare impulse, with nothing holding it, travelled
/// 0.92px of an authored 18 before ground friction stopped it.
#[test]
fn a_lunge_steps_its_authored_distance_during_its_windup() {
    let authored = LungeSpec::BRUTE_DEFAULT.step_px;
    let lunge = windup_travel(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT));
    let swipe = windup_travel(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT));
    assert!(
        (lunge - authored).abs() <= 1.0,
        "the brute lunge authors a {authored}px windup step and travelled {lunge}px"
    );
    assert_eq!(swipe, 0.0, "a swing that authors no step moved {swipe}px");
}

/// Only the grounded forward attack steps. The directional variants are the
/// base swing turned, and an aerial that inherited the step would drift.
#[test]
fn a_lunges_derived_variants_do_not_inherit_its_step() {
    let melee = MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT);
    let moveset = ambition_characters::moveset_prefabs::build_actor_moveset(None, Some(&melee), None, None)
        .expect("a melee builds a moveset");
    let steps = |spec: &ambition_entity_catalog::MoveSpec| {
        spec.events.iter().any(|event| {
            matches!(event.kind, ambition_entity_catalog::MoveEventKind::HoldVelocity { .. })
        })
    };
    let attack_id = &moveset.verbs[ambition_entity_catalog::ATTACK_VERB];
    let mut variants = 0;
    for spec in &moveset.moves {
        if &spec.id == attack_id {
            assert!(steps(spec), "the base attack lost the lunge's step");
        } else {
            variants += 1;
            assert!(!steps(spec), "the derived `{}` inherited the lunge's step", spec.id);
        }
    }
    assert!(variants > 0, "no derived variants, so this checked nothing");
}
