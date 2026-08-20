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
    stepped(
        body,
        combat,
        ae::MovementTuning::default(),
        ae::InputState::default(),
    )
    .length()
}

/// The same step, with the tuning and the input named — for SDI, which is a
/// property of both.
fn stepped(
    body: &mut ae::BodyClusterScratch,
    combat: &BodyCombat,
    tuning: ae::MovementTuning,
    input: ae::InputState,
) -> ae::Vec2 {
    let world = open_world();
    let before = body.kinematics.pos;
    {
        let (model, mut clusters) = body.parts();
        step_body(
            model,
            &mut clusters,
            combat,
            tuning,
            ae::MotionStepContext {
                world: &world,
                input,
                frame: frame(),
                facing_intent: 0.0,
                dt: DT,
            },
        );
    }
    body.kinematics.pos - before
}

/// **SDI IS THE ONE THING A FROZEN BODY MAY STILL DO.**
///
/// ⛔ three assertions and each kills a different wrong version: a body with no
/// authored budget must not move (every body in Ambition), one with a budget and
/// a held stick must, and the shift must be the STICK's direction rather than
/// the body's own velocity — a version that nudged along `vel` would look right
/// in a test where the two happen to agree and be a different mechanic.
#[test]
fn a_frozen_body_can_still_influence_where_the_next_hit_finds_it() {
    let frozen = BodyCombat {
        hitstop_timer: 0.08,
        ..BodyCombat::default()
    };
    assert!(frozen.is_in_hitlag(), "the fixture must arm the freeze");
    // ⚠ the body travels along +x; the stick is held along -y (up), so the two
    // cannot be confused for each other.
    let held = ae::InputState {
        axes: ae::LocalAxes::new(0.0, -1.0),
        ..Default::default()
    };
    let mut tuning = ae::MovementTuning::default();
    tuning.sdi_step = 3.0;

    let unauthored = stepped(
        &mut body_travelling(),
        &frozen,
        ae::MovementTuning::default(),
        held.clone(),
    );
    assert_eq!(
        unauthored,
        ae::Vec2::ZERO,
        "a body that authored no SDI budget influenced anyway"
    );

    let shifted = stepped(&mut body_travelling(), &frozen, tuning.clone(), held);
    assert_eq!(
        shifted,
        ae::Vec2::new(0.0, -3.0),
        "SDI did not move the frozen body one step along the held stick"
    );

    // ⛔ and a null stick is still a freeze: the budget alone must not drift a
    // body that is asking for nothing.
    let idle = stepped(
        &mut body_travelling(),
        &frozen,
        tuning,
        ae::InputState::default(),
    );
    assert_eq!(idle, ae::Vec2::ZERO, "a budget alone drifted a frozen body");
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
