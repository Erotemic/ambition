//! A body through an opening touches a chest or switch with its admitted
//! material (world-geometry bounded delivery, step 2). Facing and preference
//! read the target where the body meets it, carried back to the body's own
//! chart.
//!
//! The body straddles a pair: its root is 5 px in front of the entry, and
//! 7 px of it is past the entry plane and out of the far end, which is far
//! to the left of the body.

use super::InteractReach;
use ambition_platformer2d_core::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture, PortalFrame};
use ambition_platformer2d_core::{AabbExt, BodyKinematics, CenteredAabb, Vec2};

const ROOT: Vec2 = Vec2::new(495.0, 500.0);
/// A switch beside the far end, where only the carried part is.
const FAR: CenteredAabb = CenteredAabb { center: Vec2::new(116.0, 200.0), half_size: Vec2::new(14.0, 10.0) };
/// A switch behind the entry, where only the cut-away part of the root box is.
const BEHIND: CenteredAabb = CenteredAabb { center: Vec2::new(510.0, 500.0), half_size: Vec2::new(9.0, 10.0) };

fn reach(facing: f32, through: bool) -> InteractReach {
    let entry = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, 500.0), Vec2::NEG_X), half_length: 80.0 };
    let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(100.0, 200.0), Vec2::X), half_length: 80.0 };
    let links = PortalLink::both_ways(0, (entry, exit));
    let kin = BodyKinematics { pos: ROOT, canonical_size: Vec2::new(24.0, 48.0), facing, ..Default::default() };
    InteractReach::of(&kin, None, None, None, through.then_some(MotionAdmission {
        span: Default::default(), links: &links, first: Some(0),
        budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection,
    }))
}

#[test]
fn a_straddler_touches_with_its_material_and_meets_the_target_in_its_own_chart() {
    // The premise: the root box gives the opposite answer for each switch.
    let control = reach(1.0, false);
    assert!(!FAR.aabb().strict_intersects(control.collision_box()));
    assert!(control.touches(None, &BEHIND, false), "no opening: the root box touches the switch behind the entry");

    let through = reach(1.0, true);
    assert_eq!(through.refuses_touch(None, &BEHIND, false), Some("not touched"), "no material is behind the entry");
    // The far switch is 16 px in front of the far end, which is 16 px ahead
    // of the entry in the body's chart: faced by a body facing +x though it
    // is 380 px to its left in the world, and near its front.
    assert_eq!(through.refuses_touch(None, &FAR, true), None, "the carried part touches the far switch and faces it");
    let preference = through.touch_preference(None, &FAR, true).unwrap();
    assert!(preference < 20.0, "preference is measured where the body meets the switch, not across the world: {preference}");
    assert_eq!(reach(-1.0, true).refuses_touch(None, &FAR, true), Some("not faced"), "a body facing away from the opening");
}
