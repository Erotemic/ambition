//! A body on a GRAPPLE: a held wire from wherever the line bit, which keeps
//! the momentum of the grab, swings under gravity and the stick, reels on the
//! stick, and lets go on a jump with the swing's speed (Jon, 2026-11-27: "Grapple
//! needs to be able to swing").

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, LocalAxes, Vec2};

const ROPE: crate::movement::GrappleRope = crate::movement::GrappleRope {
    max_angle: 1.9,
    swing_accel: 6.0,
    reel_speed: 240.0,
    min_length: 60.0,
    max_length: 320.0,
    release_hop: 260.0,
};

fn frame() -> crate::MotionFrame {
    crate::MotionFrame::from_direction(crate::DEFAULT_GRAVITY_DIR, TEST_TUNING.gravity)
}

fn input(x: f32, y: f32) -> InputState {
    InputState { axes: LocalAxes::new(x, y), ..Default::default() }
}

fn jump() -> InputState {
    InputState {
        movement: crate::ActionEdges::EMPTY
            .with(MovementAction::Jump, crate::Edge { pressed: true, held: true, released: false }),
        ..Default::default()
    }
}

/// The anchor: high in open air, so a 160 px rope hangs well clear of the floor.
const ANCHOR: Vec2 = Vec2::new(800.0, 480.0);

/// A body at `angle` (radians from straight down) on a `length` rope from
/// [`ANCHOR`], moving at `vel`, that has just bitten.
fn hanging(angle: f32, length: f32, vel: Vec2) -> BodyClusterScratch {
    let at = ANCHOR + Vec2::new(angle.sin(), angle.cos()) * length;
    let mut scratch = BodyClusterScratch::new_with_abilities(at, AbilitySet::sandbox_all());
    scratch.kinematics.pos = at;
    scratch.kinematics.vel = vel;
    assert!(
        crate::movement::catch_the_grapple(&mut scratch.model, at, vel, frame(), ANCHOR, ROPE),
        "the grapple did not bite"
    );
    scratch
}

fn wire(scratch: &BodyClusterScratch) -> Option<crate::WireState> {
    scratch.axis().wire
}

/// The bite takes the rope's length and angle from where she is, and the part
/// of her velocity along the swing becomes its angular speed: a running grab
/// keeps its momentum. A bite level with her or below is no swing.
#[test]
fn a_bite_hangs_her_where_she_is_and_keeps_her_momentum() {
    let scratch = hanging(0.0, 160.0, Vec2::new(300.0, 0.0));
    let w = wire(&scratch).expect("on the rope");
    assert!(w.held && (w.length - 160.0).abs() < 1e-3 && w.angle.abs() < 1e-5, "{w:?}");
    // At the bottom, moving right at 300 is the whole tangential speed.
    assert!((w.ang_vel * w.length - 300.0).abs() < 1e-2, "the run did not become the swing: {w:?}");

    let mut level = BodyClusterScratch::new_with_abilities(ANCHOR + Vec2::new(120.0, 0.0), AbilitySet::sandbox_all());
    let at = level.kinematics.pos;
    assert!(
        !crate::movement::catch_the_grapple(&mut level.model, at, Vec2::ZERO, frame(), ANCHOR + Vec2::new(0.0, 200.0), ROPE),
        "a bite below her held her up"
    );
    assert!(wire(&level).is_none());
}

/// Let go of at an angle she swings down through the bottom and up the other
/// side, staying on the rope: a pendulum, not a pull.
#[test]
fn a_hanging_body_swings_through_and_stays_on_the_rope() {
    let world = test_world();
    let mut scratch = hanging(-0.6, 160.0, Vec2::ZERO);
    let mut crossed = false;
    let mut far_side = 0.0_f32;
    for _ in 0..90 {
        step_scratch(&world, &mut scratch, input(0.0, 0.0));
        let w = wire(&scratch).expect("a held wire does not let go on its own");
        let reach = (scratch.kinematics.pos - ANCHOR).length();
        assert!((reach - w.length).abs() < 4.0, "she left the rope: {reach} from a {} rope", w.length);
        crossed |= scratch.kinematics.pos.x > ANCHOR.x;
        far_side = far_side.max(w.angle);
    }
    assert!(crossed, "she never swung through the bottom");
    assert!(far_side > 0.35, "the swing died on the way up the far side ({far_side:.2} rad)");
}

/// The stick pumps the swing: pushing with the swing carries her farther round
/// than hanging does.
#[test]
fn the_stick_pumps_the_swing_higher() {
    let world = test_world();
    let mut peak = [0.0_f32; 2];
    for (pump, peak) in [false, true].into_iter().zip(&mut peak) {
        let mut scratch = hanging(-0.3, 160.0, Vec2::ZERO);
        for _ in 0..240 {
            let along = wire(&scratch).map_or(0.0, |w| w.ang_vel.signum());
            step_scratch(&world, &mut scratch, input(if pump { along } else { 0.0 }, 0.0));
            *peak = peak.max(wire(&scratch).expect("on the rope").angle.abs());
        }
    }
    assert!(peak[1] > peak[0] + 0.3, "pumping reached {:.2} rad, hanging {:.2}", peak[1], peak[0]);
}

/// Up reels the rope in and down pays it out, within its bounds.
#[test]
fn the_reel_follows_the_stick_within_its_bounds() {
    let world = test_world();
    let mut scratch = hanging(0.0, 160.0, Vec2::ZERO);
    for _ in 0..120 {
        step_scratch(&world, &mut scratch, input(0.0, -1.0));
    }
    assert!((wire(&scratch).unwrap().length - ROPE.min_length).abs() < 1e-3, "{:?}", wire(&scratch));
    let reach = (scratch.kinematics.pos - ANCHOR).length();
    assert!((reach - ROPE.min_length).abs() < 6.0, "the reel shortened the rope but not her: {reach}");
    for _ in 0..240 {
        step_scratch(&world, &mut scratch, input(0.0, 1.0));
    }
    assert!((wire(&scratch).unwrap().length - ROPE.max_length).abs() < 1e-3, "{:?}", wire(&scratch));
}

/// A jump lets go with the swing's speed and the rope's hop, and spends the
/// press: no air jump on top. A let-go keeps the swing's speed alone.
#[test]
fn a_jump_lets_go_with_the_swing_and_a_hop() {
    let world = test_world();
    let mut scratch = hanging(0.0, 160.0, Vec2::new(300.0, 0.0));
    let swing = wire(&scratch).unwrap().exit_velocity(frame(), 0.0);
    let jumps_before = scratch.jump.clone();
    step_scratch(&world, &mut scratch, jump());
    assert!(wire(&scratch).is_none(), "the jump did not let go");
    // One tick of air after the release: gravity has had one step.
    let up = -scratch.kinematics.vel.y;
    assert!(up > ROPE.release_hop * 0.8 && up < ROPE.release_hop + 40.0, "the hop: {up:.0}");
    assert!((scratch.kinematics.vel.x - swing.x).abs() < 30.0, "the swing's speed: {:?} vs {swing:?}", scratch.kinematics.vel);
    assert_eq!(scratch.jump, jumps_before, "the release also spent a jump");

    let mut scratch = hanging(0.0, 160.0, Vec2::new(300.0, 0.0));
    let leave = crate::movement::let_go_of_the_wire(&mut scratch.model, frame()).expect("on a held wire");
    assert!((leave.x - 300.0).abs() < 1e-2 && leave.y.abs() < 1e-2, "{leave:?}");
    assert!(wire(&scratch).is_none());
}

/// The flyline's timed wire is not held: it keeps its clock and lets go on
/// its own, and a jump does not end it.
#[test]
fn the_flylines_wire_is_not_held() {
    let world = test_world();
    let at = Vec2::new(800.0, 700.0);
    let mut scratch = BodyClusterScratch::new_with_abilities(at, AbilitySet::sandbox_all());
    scratch.kinematics.pos = at;
    assert!(crate::movement::catch_the_wire(&mut scratch.model, at, frame(), 720.0, 0.55, 900.0, 0.3, 3.4, 90.0));
    assert!(!wire(&scratch).unwrap().held);
    assert!(crate::movement::let_go_of_the_wire(&mut scratch.model, frame()).is_none(), "a let-go ended a lift");
    step_scratch(&world, &mut scratch, jump());
    assert!(wire(&scratch).is_some(), "a jump ended a lift");
}
