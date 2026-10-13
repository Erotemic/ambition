//! [`MotionModel::ground_stop_distance`] and
//! [`MotionModel::ground_coast_distance`] against the axis-swept laws they
//! read. A brain spends these numbers at a ledge, so each law must stop inside
//! the first when the body brakes in the strongest way that law has, and
//! inside the second with no stick.

use super::super::*;
use super::{test_world, TEST_TUNING};
use crate::body_clusters::BodyClusterScratch;
use crate::test_support::{update_player_with_tuning_scratch, TestTuning};
use crate::{AbilitySet, LocalAxes};

const DT: f32 = 1.0 / 60.0;
/// Faster than any run, as after a launch or a dash: the speed at which the
/// margin matters.
const SPEED: f32 = 900.0;

/// The floor (px) a grounded body moving at `SPEED` covers before it stops,
/// with the stick held at `stick` (negative is against the motion).
fn floor_to_stop(tuning: TestTuning, stick: f32) -> f32 {
    let world = test_world();
    let mut scratch = BodyClusterScratch::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    for _ in 0..120 {
        update_player_with_tuning_scratch(&world, &mut scratch, InputState::default(), DT, tuning);
    }
    assert!(scratch.ground.on_ground, "the body did not come to rest on the floor");
    scratch.kinematics.vel.x = SPEED;
    let start = scratch.kinematics.pos.x;
    for _ in 0..240 {
        let input = InputState {
            axes: LocalAxes::new(stick, 0.0),
            ..Default::default()
        };
        update_player_with_tuning_scratch(&world, &mut scratch, input, DT, tuning);
        assert!(scratch.ground.on_ground, "the body left the floor while it braked");
        if scratch.kinematics.vel.x <= 0.0 {
            return scratch.kinematics.pos.x - start;
        }
    }
    panic!("the body did not stop in four seconds");
}

/// The stop distance the model says, and the two ways to brake: with the
/// stick against the motion, and hands off. The hands-off slide is also held
/// to the coast distance the model says.
fn measure(tuning: TestTuning) -> (f32, f32, f32) {
    let model = MotionModel::axis_swept(tuning.params());
    let said = model.ground_stop_distance(SPEED);
    let hands_off = floor_to_stop(tuning, 0.0);
    let coast = model.ground_coast_distance(SPEED);
    assert!((hands_off - coast).abs() <= SPEED * DT, "hands off: {hands_off} px, the model's coast is {coast} px");
    (said, floor_to_stop(tuning, -1.0), hands_off)
}

/// ⭐ THE RESPONSIVE LAW STOPS HARDEST HANDS OFF: it approaches the zero
/// target at `run_accel` and applies `ground_friction` in the same tick, and
/// it stops inside the distance the model says, give or take one tick of
/// travel. The stick against the motion stops
/// later: that is the control which shows the model took the stronger rate.
#[test]
fn the_responsive_law_stops_inside_the_distance_the_model_says() {
    let (said, against, hands_off) = measure(TEST_TUNING);
    assert!((hands_off - said).abs() <= SPEED * DT, "hands off: {hands_off} px, the model says {said} px");
    assert!(against > said + SPEED * DT, "against the motion: {against} px, the model says {said} px");
}

/// ⭐ THE MOMENTUM LAW STOPS HARDEST WITH THE STICK AGAINST THE MOTION when
/// its reverse rate is the larger one, as on Mary-O Classic.
#[test]
fn the_momentum_law_stops_inside_the_distance_the_model_says() {
    let mut tuning = TEST_TUNING;
    tuning.horizontal_law = AxisHorizontalLaw::Momentum(MomentumHorizontalTuning {
        ground_reverse_accel: 1500.0,
        ground_coast_decel: 600.0,
        air_reverse_accel: 900.0,
        air_coast_decel: 0.0,
    });
    let (said, against, hands_off) = measure(tuning);
    assert!((against - said).abs() <= SPEED * DT, "against the motion: {against} px, the model says {said} px");
    assert!(hands_off > said + SPEED * DT, "hands off: {hands_off} px, the model says {said} px");
}
