//! Two numeric effects on one body, from two sources, end apart, and a
//! rewind across either ending gives the same fall.
//!
//! The effects are `BodyEffects` (core), rollback state (`body.effects`),
//! spent by the movement step. Both halve Alice's gravity: one for a quarter
//! second, the other for half a second. The fixed-tick world is the
//! authority. Under a GGRS sync-test session, which rewinds 4 frames each
//! tick and checks each resimulation, both expiries are rewound across many
//! times; the gravity each tick and the fall must be the fixed world's.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AmbitionSim as _, Platformer2dSimHarness, TimestepMode,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::{Entity, With};

use crate::common::base;

const ROOM: &str = "combat_calibration_lab";
const TICKS: usize = 45;
const PARASOL: ae::EffectSource = ae::EffectSource::named("test.parasol");
const FEATHER: ae::EffectSource = ae::EffectSource::named("test.feather");

fn sim(rewinding: bool) -> Platformer2dSimHarness {
    let options = crate::common::sandbox_options()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROOM);
    let options = if rewinding { options.with_sync_test_rollback_settings(4, 10) } else { options };
    Platformer2dSimHarness::new_with_options(options).expect("the calibration lab builds")
}

fn alice(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).expect("Alice's body is in the world")
}

/// Alice's gravity and height each tick after both effects are put on her
/// high in the air, or the rollback refusal that stopped the run.
fn drive(mut sim: Platformer2dSimHarness) -> (Vec<(f32, u32)>, Option<String>) {
    for _ in 0..10 {
        sim.step(base());
    }
    let alice = alice(&mut sim);
    {
        let mut entity = sim.world_mut().entity_mut(alice);
        let mut kin = entity.get_mut::<ae::BodyKinematics>().expect("Alice has kinematics");
        kin.pos.y -= 300.0;
        kin.vel = ae::Vec2::ZERO;
        let mut effects = entity.get_mut::<ae::BodyEffects>().expect("every body carries its effects");
        effects.apply(PARASOL, ae::NumericAttribute::Gravity, ae::NumericOp::Mul(0.5), 0.25);
        effects.apply(FEATHER, ae::NumericAttribute::Gravity, ae::NumericOp::Mul(0.5), 0.5);
    }
    sim.rebase_rollback_history().expect("the rollback history rebases over the effects");
    let mut timeline = Vec::new();
    for _ in 0..TICKS {
        if let Err(refusal) = sim.try_step(base()) {
            return (timeline, Some(refusal));
        }
        let world = sim.world();
        let model = world.get::<ae::MotionModel>(alice).expect("Alice has a motion model");
        let effects = world.get::<ae::BodyEffects>(alice).expect("Alice keeps her effects");
        let y = world.get::<ae::BodyKinematics>(alice).expect("Alice has kinematics").pos.y;
        timeline.push((model.gravity_modifier(effects), y.to_bits()));
    }
    (timeline, None)
}

#[test]
fn two_effects_on_one_number_expire_apart_and_rewind_across_both() {
    let (fixed, refusal) = drive(sim(false));
    assert_eq!(refusal, None, "a world with no rollback session refuses nothing");
    let gravity: Vec<f32> = fixed.iter().map(|(gravity, _)| *gravity).collect();
    let both = gravity.iter().position(|g| *g == 0.5);
    let none = gravity.iter().position(|g| *g == 1.0);
    assert!(
        gravity.first() == Some(&0.25)
            && matches!((both, none), (Some(first), Some(then)) if 10 < first && first + 10 < then)
            && gravity.iter().all(|g| [0.25, 0.5, 1.0].contains(g)),
        "precondition: both effects (x0.25), then the longer one alone (x0.5), then neither: \
         the parasol's end took the feather with it, or neither ended: {gravity:?}"
    );
    let (rewound, refusal) = drive(sim(true));
    assert_eq!(
        (rewound, refusal),
        (fixed, None),
        "(gravity and height per tick, refusal): a rewind across an effect's end gave another fall"
    );
}
