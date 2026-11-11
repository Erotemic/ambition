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

/// Resolve reference-frame aim, or the body's physical facing for neutral aim.
pub fn world_fire_dir(kin: &ae::BodyKinematics, aim_world: ae::Vec2) -> Result<ae::Vec2, ae::frame::NumericFault> {
    if !aim_world.is_finite() || !kin.facing.is_finite() { return Err(ae::frame::NumericFault::NonFinite); }
    if aim_world.as_dvec2().length() > 0.1 {
        Ok(aim_world.as_dvec2().normalize().as_vec2())
    } else {
        let facing = if kin.facing < 0.0 { -1.0 } else { 1.0 };
        Ok(kin.material_map()?.direction(ae::Vec2::X * facing))
    }
}

/// Where a shot is born for a body that publishes no hand: off the edge of
/// its box in the aim direction. The named fallback of [`charge_shot_birth`].
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

/// Lift a shot clear of the body's physical feet plane. Shot extents are
/// in the shot's root chart; the gap is one canonical unit of the body.
pub fn clear_of_the_feet(
    origin: ae::Vec2,
    kin: &ae::BodyKinematics,
    feet: ae::Vec2,
    half_extent: ae::Vec2,
) -> Result<ae::Vec2, ae::frame::NumericFault> {
    use ae::frame::{NumericFault, Sim2};
    if !origin.is_finite() || !feet.is_finite() || !half_extent.is_finite() { return Err(NumericFault::NonFinite); }
    if half_extent.min_element() < 0.0 { return Err(NumericFault::InvalidScale); }
    let map = kin.material_map()?;
    let down = map.direction(ae::Vec2::Y).as_dvec2().normalize();
    let above_feet = (feet.as_dvec2() - origin.as_dvec2()).dot(down);
    let support = half_extent.as_dvec2().dot(down.abs());
    let lift = (support + f64::from(map.length(SHOT_GROUND_CLEARANCE)?) - above_feet).max(0.0);
    Sim2::IDENTITY.point_from_wide(origin.as_dvec2() - down * lift)
}

/// Place a shot at the firing hand or the canonical box fallback. The hand
/// and feet use body material. The shot keeps its own authored root extents.
pub fn charge_shot_birth(
    landmarks: &BodyLandmarks,
    body: Entity,
    kin: &ae::BodyKinematics,
    world_dir: ae::Vec2,
    half_extent: ae::Vec2,
) -> Result<ae::Vec2, ae::frame::NumericFault> {
    use ae::frame::{NumericFault, Sim2};
    if !world_dir.is_finite() || !half_extent.is_finite() { return Err(NumericFault::NonFinite); }
    if half_extent.min_element() < 0.0 { return Err(NumericFault::InvalidScale); }
    let dir = world_dir.as_dvec2().try_normalize().ok_or(NumericFault::InvalidFrame)?;
    let shoot_row = LandmarkPose::Clip { chain: SHOOT_CLIPS, phase: SHOOT_RELEASE_PHASE };
    match landmarks.gesture_hand_in_world(body, shoot_row, kin)? {
        Some(hand) => {
            let reach = half_extent.as_dvec2().dot(dir.abs());
            let origin = Sim2::IDENTITY.point_from_wide(hand.as_dvec2() + dir * reach)?;
            let feet = landmarks.place_rig_point(body, ae::Vec2::ZERO, kin)?;
            clear_of_the_feet(origin, kin, feet, half_extent)
        }
        None => {
            let map = kin.material_map()?;
            let local_dir = map.inverse()?.direction(dir.as_vec2());
            map.point(box_muzzle_local_offset(local_dir, kin.facing, kin.canonical_size))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_clears_transformed_feet_without_scaling_its_own_extent() {
        let mut body = ae::BodyKinematics { pos: ae::Vec2::new(100.0, 200.0),
            canonical_size: ae::Vec2::new(20.0, 48.0), ..Default::default() };
        let half = ae::Vec2::new(10.0, 8.0);
        let feet = ae::Vec2::new(100.0, 224.0);
        let origin = ae::Vec2::new(105.0, 216.5);
        assert_eq!(clear_of_the_feet(origin, &body, feet, half).unwrap(), ae::Vec2::new(105.0, 215.0));
        body.material = ae::frame::BodyMaterial::new(0.5,
            bevy::math::Mat2::from_cols(-ae::Vec2::Y, -ae::Vec2::X)).unwrap();
        // Physical down is left. The gap halves; the shot's half width stays 10.
        let feet = ae::Vec2::new(88.0, 200.0);
        assert_eq!(clear_of_the_feet(ae::Vec2::new(91.0, 197.5), &body, feet, half).unwrap(),
            ae::Vec2::new(98.5, 197.5));
        assert_eq!(world_fire_dir(&body, ae::Vec2::ZERO).unwrap(), -ae::Vec2::Y);
        assert_eq!(world_fire_dir(&body, ae::Vec2::X).unwrap(), ae::Vec2::X);
        assert_eq!(clear_of_the_feet(ae::Vec2::splat(f32::NAN), &body, feet, half),
            Err(ae::frame::NumericFault::NonFinite));
    }
}
