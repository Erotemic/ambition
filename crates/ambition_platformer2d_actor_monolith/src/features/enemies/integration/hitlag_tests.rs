//! So the freeze was a property of WHO GOT HIT rather than of the hit: a player felt every connect,
//! and a hit whose two parties were both actors froze neither of them. On a platform-fighter stage
//! that is every CPU-versus-CPU exchange, and every seat past slot 0.
//!
//! these drive the REAL actor spine (`ActorMut::update` → `integrate_body`),
//! the same road `dash_tests` uses, so they measure what a body DOES rather than
//! which field is set. `BodyCombat` is a parameter of that call already — the
//! freeze had a seat at the table and nobody sat in it.

use super::*;
use crate::features::ecs::actor_clusters::SeedActorMut;
use ambition_body_seed::{ActorBody, ActorClusterSeed};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_entity_catalog::placements::CharacterBrain;

/// A wide solid floor; bodies rest on its top face at y = 100.
fn floored_world() -> ae::World {
    ae::World::new(
        "hitlag_test",
        ae::Vec2::new(4000.0, 800.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(-2000.0, 100.0),
            ae::Vec2::new(4000.0, 80.0),
        )],
    )
}

/// Walk a grounded actor right for `ticks`, carrying `hitstop` seconds of
/// freeze, and return how far it actually traveled along +x.
///
/// The control frame is IDENTICAL in both arms — full-throttle locomotion — so
/// the only thing that differs between a call with hitstop and one without is
/// the body's own combat state. That is the whole claim.
fn walk_distance_with_hitstop(hitstop: f32, ticks: u32) -> f32 {
    let world = floored_world();
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "struck".to_string(),
        "Struck".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    let half_h = seed.kin.size.y * 0.5;
    seed.kin.pos = ae::Vec2::new(0.0, 100.0 - half_h);
    seed.kin.vel = ae::Vec2::ZERO;
    seed.kin.facing = 1.0;
    seed.surface.gravity_scale = 1.0;
    seed.body = ActorBody::from_kit(
        ae::AbilitySet {
            move_horizontal: true,
            ..ae::AbilitySet::NONE
        },
        false,
        seed.kin.size,
    );
    seed.body.0.ground.on_ground = true;

    let mut combat = ambition_characters::actor::BodyCombat::default();
    combat.hitstop_timer = hitstop;

    let start_x = seed.kin.pos.x;
    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let mut frame = ActorControlFrame::neutral();
    frame.locomotion = ae::LocalAxes::new(1.0, 0.0);
    frame.facing = 1.0;
    let dt = 1.0 / 60.0;
    for _ in 0..ticks {
        // it also pins that the freeze is fed by WORLD time, not by the
        // per-body `sim_dt` the hitlag branch zeroes — those being the same value
        // would be a deadlock, and this is where that would show.
        combat.decay_reaction_timers(dt);
        em.update(
            &world,
            ae::Vec2::new(2000.0, em.kin.pos.y),
            FeatureCombatTuning::default(),
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
            // No move playing: these fixtures are about movement, not helplessness.
            None,
            ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
            None,
            &mut combat,
            // Not tumbling — this fixture is not about the floor game.
            false,
            // In play — a body waiting out a death window does not move at all.
            false,
            ae::BodyContactField::NONE,
        );
    }
    em.kin.pos.x - start_x
}

/// A struck actor stops, and an unstruck one does not.
///
/// both terms are asserted, and that is deliberate. A gate that froze
/// every body — or a fixture whose actor could not walk in the first place —
/// would satisfy "the frozen one did not move" on its own. The moving arm is
/// what makes the frozen arm mean something.
#[test]
fn a_hit_between_two_actors_freezes_them_both() {
    // 20 ticks ≈ 0.33s.
    let free = walk_distance_with_hitstop(0.0, 20);
    let frozen = walk_distance_with_hitstop(0.50, 20);

    assert!(
        free > 20.0,
        "the unfrozen actor barely moved ({free:.2}px), so this fixture cannot \
         tell a freeze from a body that never walks"
    );
    assert!(
        frozen.abs() < 0.001,
        "an actor carrying hitstop advanced {frozen:.2}px — the actor road is \
         stepping the simulation during a freeze it was handed. Hitstop is armed \
         on BOTH parties to a hit; if only the avatar road spends it, a \
         CPU-versus-CPU exchange freezes nobody"
    );
}

/// The freeze ENDS. A body that never resumes is a different bug wearing the
/// same symptom, and `frozen.abs() < 0.001` above cannot tell them apart.
#[test]
fn hitlag_stops_the_body_without_stranding_it() {
    // 0.10s is 6 ticks at 60Hz — well short of the 30-tick window, so one run
    // covers both the freeze and the recovery.
    let frozen_then_free = walk_distance_with_hitstop(0.10, 30);
    let never_frozen = walk_distance_with_hitstop(0.0, 30);

    assert!(
        frozen_then_free > 20.0,
        "a body given a BRIEF hitstop never resumed over 30 ticks \
         ({frozen_then_free:.2}px) — the freeze is not decaying on the actor road"
    );
    assert!(
        frozen_then_free < never_frozen,
        "a hitstop cost the actor no ground at all (froze {frozen_then_free:.2}px \
         vs free {never_frozen:.2}px), so the branch is not being taken"
    );
}
