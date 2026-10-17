use super::*;
use crate::{Aabb, AabbExt};

// y is down. A floor portal at the origin (its front is up) and its partner,
// a ceiling portal at (500, -200) (its front is down): what goes down into the
// floor comes down out of the ceiling.
fn floor_pair() -> (PortalAperture, PortalAperture) {
    let floor = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.0, -1.0)), half_length: 40.0 };
    let ceiling = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, -200.0), Vec2::new(0.0, 1.0)), half_length: 40.0 };
    (floor, ceiling)
}

fn boxed(center: Vec2, half: Vec2) -> CombatVolume {
    CombatVolume::aabb(Aabb::new(center, half))
}

fn victim(center: Vec2) -> VolumePiece {
    VolumePiece::whole(boxed(center, Vec2::splat(5.0)))
}

fn hits(pieces: &[VolumePiece], at: Vec2) -> bool {
    pieces.iter().any(|piece| piece.meets(&victim(at)))
}

const ABOVE: Vec2 = Vec2::new(0.0, -30.0);

#[test]
fn a_blade_into_a_portal_reaches_out_of_its_partner() {
    // From above, a blade 30 deep into the floor's opening.
    let blade = boxed(Vec2::new(0.0, 10.0), Vec2::new(10.0, 20.0));
    let pieces = thread_volume(&blade, ABOVE, &[floor_pair()], MapConvention::Reflection);
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    // It hurts over the floor, and under the ceiling as deep as it went in.
    assert!(hits(&pieces, Vec2::new(0.0, -6.0)));
    assert!(hits(&pieces, Vec2::new(500.0, -200.0 + 26.0)));
    assert!(!hits(&pieces, Vec2::new(500.0, -200.0 + 40.0)));
    // Not in the floor under the opening: that part of it is at the ceiling.
    assert!(!hits(&pieces, Vec2::new(0.0, 20.0)));
    // The part through says where it is, and carries its source there.
    let through = pieces.iter().find(|p| p.carried.is_some()).unwrap();
    assert!(through.center().distance(Vec2::new(500.0, -185.0)) < 0.01, "{:?}", through.center());
    assert_eq!(through.carry(ABOVE, MapConvention::Reflection), Vec2::new(500.0, -230.0));
}

#[test]
fn a_volume_beside_the_opening_or_from_behind_it_is_whole() {
    let pair = [floor_pair()];
    // Past the plane beside the opening's end.
    let beside = boxed(Vec2::new(70.0, 10.0), Vec2::new(10.0, 20.0));
    assert_eq!(thread_volume(&beside, Vec2::new(70.0, -30.0), &pair, MapConvention::Reflection).len(), 1);
    // From behind the plane: under the floor, reaching up into it.
    let up = boxed(Vec2::new(0.0, 10.0), Vec2::new(10.0, 20.0));
    assert_eq!(thread_volume(&up, Vec2::new(0.0, 40.0), &pair, MapConvention::Reflection).len(), 1);
    // Resting on it.
    let resting = boxed(Vec2::new(0.0, -20.0), Vec2::new(10.0, 20.0));
    assert_eq!(thread_volume(&resting, ABOVE, &pair, MapConvention::Reflection).len(), 1);
}

#[test]
fn a_volume_across_an_end_keeps_its_part_beside_the_opening() {
    // Half over the opening's right end, 20 deep: what is past the end is
    // beside it, here; what is over the opening is at the ceiling.
    let wide = boxed(Vec2::new(40.0, 0.0), Vec2::new(20.0, 20.0));
    let pieces = thread_volume(&wide, ABOVE, &[floor_pair()], MapConvention::Reflection);
    assert_eq!(pieces.len(), 3, "{pieces:?}");
    assert!(hits(&pieces, Vec2::new(55.0, 15.0)));
    assert!(!hits(&pieces, Vec2::new(30.0, 15.0)));
}

#[test]
fn a_cut_circle_stays_a_circle() {
    let ball = CombatVolume::circle(Vec2::new(0.0, 0.0), 15.0);
    let pieces = thread_volume(&ball, ABOVE, &[floor_pair()], MapConvention::Reflection);
    assert_eq!(pieces.len(), 2);
    // Its round part comes out of the ceiling: on its axis 15 deep, not at
    // the corner of a box.
    assert!(hits(&pieces, Vec2::new(500.0, -200.0 + 15.0 + 4.0)));
    assert!(!hits(&pieces, Vec2::new(500.0 + 16.0, -200.0 + 16.0)));
    // Two discs and a plane.
    let disc = VolumePiece::whole(CombatVolume::circle(Vec2::new(500.0, -180.0), 6.0));
    assert!(pieces.iter().any(|p| p.meets(&disc)));
    let behind_the_ceiling = VolumePiece::whole(CombatVolume::circle(Vec2::new(500.0, -212.0), 6.0));
    assert!(!pieces.iter().any(|p| p.meets(&behind_the_ceiling)));
}

#[test]
fn two_whole_pieces_meet_as_their_volumes_intersect() {
    let a = boxed(Vec2::ZERO, Vec2::splat(5.0));
    // Edge to edge is not overlap, as for the volumes.
    let touching = boxed(Vec2::new(10.0, 0.0), Vec2::splat(5.0));
    assert_eq!(VolumePiece::whole(a.clone()).meets(&VolumePiece::whole(touching.clone())), a.intersects(&touching));
}

#[test]
fn a_box_through_a_turning_pair_is_turned() {
    // Floor to a right-facing wall: a box stands up on its side.
    let floor = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.0, -1.0)), half_length: 40.0 };
    let wall = PortalAperture { frame: PortalFrame::fixed(Vec2::new(300.0, 0.0), Vec2::new(1.0, 0.0)), half_length: 40.0 };
    let mapped = carry_volume(&boxed(Vec2::new(0.0, 10.0), Vec2::new(20.0, 10.0)), &floor.frame, &wall.frame, MapConvention::Reflection);
    let CombatVolume::Aabb(b) = mapped else { panic!("{mapped:?}") };
    assert_eq!(b.half_size(), Vec2::new(10.0, 20.0));
    assert_eq!(b.center(), Vec2::new(310.0, 0.0));
    // To a slope, it is turned and stays a box.
    let slope = PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.5, -0.75f32.sqrt()));
    assert!(matches!(
        carry_volume(&boxed(Vec2::new(0.0, 10.0), Vec2::splat(5.0)), &floor.frame, &slope, MapConvention::Reflection),
        CombatVolume::Obb { .. }
    ));
}

