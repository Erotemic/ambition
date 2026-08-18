//! The freeze, measured through the one seam every road now takes.
//!
//! ⚠ **the pair is the point.** A body that never moves passes the first test
//! for the wrong reason, so the second is its non-vacuity control: the SAME
//! body, the SAME step, hitlag cleared, must travel. Measuring only the freeze
//! is how you ship a body that is frozen forever.

use super::*;

const DT: f32 = 1.0 / 60.0;
/// Fast enough that one tick's travel is unmistakable against float noise.
const SPEED: f32 = 600.0;

fn open_world() -> ae::World {
    // No blocks: the body is in free flight, so the only thing that can stop it
    // is the step's own `dt`. A floor would give the test a second explanation.
    ae::World::new(
        "hitlag probe",
        ae::Vec2::new(20_000.0, 20_000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
}

fn body_travelling() -> ae::BodyClusterScratch {
    ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::basic())
        .with_velocity(ae::Vec2::new(SPEED, 0.0))
}

fn frame() -> ae::MotionFrame {
    ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY))
        .expect("a non-zero gravity is a valid frame")
}

/// Step `body` once and answer how far it actually travelled.
fn travel(body: &mut ae::BodyClusterScratch, combat: &BodyCombat) -> f32 {
    let world = open_world();
    let before = body.kinematics.pos;
    {
        let (model, mut clusters) = body.parts();
        step_body(
            model,
            &mut clusters,
            combat,
            ae::MovementTuning::default(),
            ae::MotionStepContext {
                world: &world,
                input: ae::InputState::default(),
                frame: frame(),
                facing_intent: 0.0,
                dt: DT,
            },
        );
    }
    (body.kinematics.pos - before).length()
}

#[test]
fn a_body_in_hitlag_does_not_travel_through_its_own_freeze() {
    let mut body = body_travelling();
    let combat = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    assert!(
        combat.is_in_hitlag(),
        "the fixture must actually arm the freeze, or this asserts nothing"
    );

    let moved = travel(&mut body, &combat);

    assert_eq!(
        moved, 0.0,
        "a body carrying hitstop walked {moved}px through its own freeze"
    );
}

#[test]
fn and_the_same_body_travels_once_the_freeze_clears() {
    let mut body = body_travelling();
    let combat = BodyCombat::default();
    assert!(
        !combat.is_in_hitlag(),
        "the control must NOT be frozen, or it proves nothing about the other test"
    );

    let moved = travel(&mut body, &combat);

    // One tick at `SPEED` is 10px; assert a floor rather than the exact figure so
    // gravity's contribution to the same tick is not a false failure.
    assert!(
        moved > 1.0,
        "an unfrozen body should have travelled; it moved {moved}px, so the \
         freeze test above may be passing because nothing moves at all"
    );
}
