//! Where a shot is born, and so where its charge is drawn.
//!
//! A charging body's shot is born at the hand of the first frame of its
//! `shoot` row, the frame the game shows on the tick it fires
//! ([`charge_shot_birth`]). The charge it holds before is drawn at the same
//! place (`ambition_sim_view::BodyPoseView::charge_offset`): one function
//! answers both, so the charge is where the shot appears.

use ambition_platformer2d_core as ae;
use bevy::prelude::Entity;

use crate::body_landmarks::{BodyLandmarks, LandmarkPose};

/// The gap between a body's box and a shot born off its edge, for a body that
/// publishes no hand ([`box_muzzle_local_offset`]).
const MUZZLE_CLEARANCE: f32 = 4.0;

/// Pixels between a hand-born shot's box and its shooter's feet line.
const SHOT_GROUND_CLEARANCE: f32 = 1.0;

/// The sheet rows a firing body is drawn from, in preference order: the
/// `Shoot` row, then what `CharacterAnim::base_pose` falls back to. The shot
/// is born at the hand of the same rows, so it leaves the hand on screen.
pub const SHOOT_CLIPS: &[&str] = &["shoot", "idle"];

/// The part of the shoot row the shot leaves the hand in: its first frame.
/// The shot is fired on the tick the row starts, so the hand that releases
/// it is the hand of the row's first frame. A row whose hand reaches out
/// later draws the shot ahead of the hand that threw it.
pub const SHOOT_RELEASE_PHASE: f32 = 0.0;

/// The way a shot is aimed in the firer's frame (+y toward its feet): the
/// stick's aim, or straight ahead.
pub fn local_fire_dir(aim_local: ae::Vec2, facing: f32) -> ae::Vec2 {
    if aim_local.length() > 0.1 {
        aim_local.normalize_or_zero()
    } else {
        ae::Vec2::new(facing, 0.0)
    }
}

/// Where a shot aimed along `local_dir` is born for a body that publishes a
/// hand, as an offset from the body's centre in its own frame (+y toward its
/// feet). `hand` is the hand in the body's rig space (feet origin, +x the way
/// it faces). The shot's rear edge is at the hand: its centre is the hand
/// plus the shot's half extent along the aim. Ruling Q41: a launch point is a
/// landmark plus a move's own offset, never a sprite bound.
pub fn hand_local_offset(
    hand: ae::Vec2,
    local_dir: ae::Vec2,
    facing: f32,
    size: ae::Vec2,
    half_extent: ae::Vec2,
) -> ae::Vec2 {
    let feet = ae::Vec2::new(0.0, size.y * 0.5);
    let reach = local_dir.x.abs() * half_extent.x + local_dir.y.abs() * half_extent.y;
    feet + ae::Vec2::new(facing.signum() * hand.x, hand.y) + local_dir * reach
}

/// Where a shot is born for a body that publishes no hand: off the edge of
/// its box in the aim direction. The named fallback of [`hand_local_offset`].
pub fn box_muzzle_local_offset(local_dir: ae::Vec2, facing: f32, size: ae::Vec2) -> ae::Vec2 {
    let half = size * 0.5;
    if local_dir.x.abs() >= local_dir.y.abs() {
        let side = if local_dir.x.abs() > 0.001 { local_dir.x.signum() } else { facing.signum() };
        ae::Vec2::new(side * (half.x + MUZZLE_CLEARANCE), -size.y * 0.20)
    } else {
        let feet_axis = local_dir.y.signum();
        ae::Vec2::new(facing.signum() * half.x * 0.4, feet_axis * (half.y + MUZZLE_CLEARANCE))
    }
}

/// `origin` moved against gravity just far enough that a shot of
/// `half_extent` born there clears the feet line of the body at `body_pos`,
/// `height` tall, by `SHOT_GROUND_CLEARANCE`; unchanged when it already does.
///
/// ⛔ A SHOT BORN TOUCHING THE FLOOR DIES ON ITS FIRST TICK. A hand muzzle fires
/// from the hand the rig puts the weapon in, and the pirates hold the
/// gun-sword at the hip: their hand is about 8 px above their feet and the
/// shot is 16 px tall. The redrawn admiral (renderer `7bd8024`) holds it 2 px
/// lower than the old sheet's fit did; his side-B shot was born 0.45 px into
/// the ground and never seen (`admiral_gun_sword`, 2026-10-04). The old art
/// had cleared it by 0.6 px, by chance.
pub fn clear_of_the_feet(
    origin: ae::Vec2,
    body_pos: ae::Vec2,
    height: f32,
    gravity_dir: ae::Vec2,
    half_extent: ae::Vec2,
) -> ae::Vec2 {
    let feet = body_pos + gravity_dir * (height * 0.5);
    let above_feet = (feet - origin).dot(gravity_dir);
    let half_along_gravity = (half_extent.x * gravity_dir.x).abs() + (half_extent.y * gravity_dir.y).abs();
    let lift = (half_along_gravity + SHOT_GROUND_CLEARANCE - above_feet).max(0.0);
    origin - gravity_dir * lift
}

/// The world centre of a shot of `half_extent` that the charging `body` (at
/// `kin`) fires along `local_dir`: at the gesture hand of the first frame of
/// its `shoot` row ([`SHOOT_CLIPS`], [`SHOOT_RELEASE_PHASE`]), else off the
/// edge of its box, and clear of its feet. `to_world` takes a direction of
/// the body's frame to the world and `down` is the body's unit down.
pub fn charge_shot_birth(
    landmarks: &BodyLandmarks,
    body: Entity,
    kin: &ae::BodyKinematics,
    to_world: impl Fn(ae::Vec2) -> ae::Vec2,
    down: ae::Vec2,
    local_dir: ae::Vec2,
    half_extent: ae::Vec2,
) -> ae::Vec2 {
    let facing = if kin.facing.abs() < f32::EPSILON { 1.0 } else { kin.facing.signum() };
    let shoot_row = LandmarkPose::Clip { chain: SHOOT_CLIPS, phase: SHOOT_RELEASE_PHASE };
    let Some(hand) = landmarks.gesture_hand(body, shoot_row) else {
        return kin.pos + to_world(box_muzzle_local_offset(local_dir, facing, kin.size));
    };
    let local = hand_local_offset(hand, local_dir, facing, kin.size, half_extent);
    clear_of_the_feet(kin.pos + to_world(local), kin.pos, kin.size.y, down, half_extent)
}

#[cfg(test)]
mod tests {
    use super::clear_of_the_feet;
    use ambition_platformer2d_core as ae;

    const HEIGHT: f32 = 48.0;
    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

    /// A shot born from a hand near the feet is lifted clear of the feet line,
    /// along whatever gravity is; one born higher is left where it is.
    #[test]
    fn a_hand_born_shot_clears_the_feet_line() {
        let half = ae::Vec2::new(10.0, 8.0);
        // Feet at y = 224: a hand 7.5 above them is lifted to 9 above.
        let low = clear_of_the_feet(ae::Vec2::new(5.0, 216.5), ae::Vec2::new(0.0, 200.0), HEIGHT, DOWN, half);
        assert_eq!(low, ae::Vec2::new(5.0, 215.0));
        let high = ae::Vec2::new(5.0, 190.0);
        assert_eq!(clear_of_the_feet(high, ae::Vec2::new(0.0, 200.0), HEIGHT, DOWN, half), high);
        // Gravity to the right: the feet line is at x = 24 and the shot's half
        // width (10) is what must clear it.
        let right = ae::Vec2::new(1.0, 0.0);
        let side = clear_of_the_feet(ae::Vec2::new(20.0, 3.0), ae::Vec2::ZERO, HEIGHT, right, half);
        assert_eq!(side, ae::Vec2::new(13.0, 3.0));
    }
}
