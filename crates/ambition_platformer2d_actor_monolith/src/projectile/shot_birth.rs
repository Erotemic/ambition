//! Where a shot may be born: clear of its shooter's feet line.
//!
//! Every shooter's fire road asks it: the player's (`systems`) and a
//! brain-driven body's (`features::ecs::brain_effects`).

use ambition_platformer2d_core as ae;

/// Pixels between a hand-born shot's box and its shooter's feet line.
const SHOT_GROUND_CLEARANCE: f32 = 1.0;

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
