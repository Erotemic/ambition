use super::*;
use crate::{Aabb, AabbExt};

#[test]
fn opposed_portal_planes_leave_no_area_at_any_position_or_angle() {
    for center in [Vec2::ZERO, Vec2::new(2200.0, 876.0)] {
        for degree in 0..360 {
            let normal = Vec2::from_angle((degree as f32).to_radians());
            let plane = HalfPlane { point: center, inward: normal };
            let volume = CombatVolume::aabb(crate::Aabb::new(center, Vec2::splat(24.0)));
            let clipped = clip_convex(volume.polygon().unwrap(), &[plane, plane.flipped()]);
            assert!(clipped.is_none(), "center={center:?}, degree={degree}, clipped={clipped:?}");
        }
    }
}

#[test]
fn an_isolated_rotated_pair_cuts_a_translated_body_once() {
    for center in [Vec2::ZERO, Vec2::new(2200.0, 876.0)] {
        for degree in 0..360 {
            let normal = Vec2::from_angle((degree as f32).to_radians());
            let pair = (
                PortalAperture { frame: PortalFrame::fixed(center, normal), half_length: 100.0 },
                PortalAperture {
                    frame: PortalFrame::fixed(center + Vec2::new(350.0, -196.0), Vec2::from_angle((degree as f32 * 0.7).to_radians())),
                    half_length: 100.0,
                },
            );
            let root = center + normal * 3.0;
            let volume = CombatVolume::convex(box_corners(crate::Aabb::new(center, Vec2::splat(24.0))).to_vec());
            for convention in [MapConvention::Reflection, MapConvention::Rotation] {
                for budget in [PieceBudget::SINGLE_HOP, PieceBudget::COMPOSED] {
                    let cut = partition(crate::frame::thread::MaterialPrimitive::world(&volume), root, &links(&[pair]), Some(0), budget, convention)
                        .unwrap_or_else(|reason| panic!("center={center:?}, degree={degree}, convention={convention:?}, budget={budget:?}: {reason:?}"));
                    assert!(
                        cut.len() == 2
                            && cut.iter().all(|piece| piece.bounds.len() <= 3 && piece.path.len() <= 1),
                        "center={center:?}, degree={degree}, {convention:?}, {budget:?}: pieces={}, bounds={:?}",
                        cut.len(), cut.iter().map(|p| p.bounds.len()).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn a_small_real_region_keeps_its_area_away_from_the_world_origin() {
    for center in [Vec2::ZERO, Vec2::new(2200.0, 876.0), Vec2::new(20_000.0, 10_000.0)] {
        let square = box_corners(crate::Aabb::new(center, Vec2::splat(0.125))).to_vec();
        assert_eq!(clip_convex(square.clone(), &[]), Some(square), "center={center:?}");
    }
}

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

/// One pair, one hop: today's rule.
fn thread_volume(
    volume: &CombatVolume,
    root: Vec2,
    pairs: &[(PortalAperture, PortalAperture)],
    convention: MapConvention,
) -> Vec<VolumePiece> {
    partition(crate::frame::thread::MaterialPrimitive::world(volume), root, &links(pairs), None, PieceBudget::SINGLE_HOP, convention).unwrap()
}

fn links(pairs: &[(PortalAperture, PortalAperture)]) -> Vec<PortalLink> {
    pairs.iter().enumerate().flat_map(|(i, pair)| PortalLink::both_ways(i, *pair)).collect()
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
    let through = pieces.iter().find(|p| p.is_carried()).unwrap();
    assert!(through.center().distance(Vec2::new(500.0, -185.0)) < 0.01, "{:?}", through.center());
    assert_eq!(through.carry(ABOVE).unwrap(), Vec2::new(500.0, -230.0));
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
    // Half over the opening's right end, 20 deep, reaching from 30 over the
    // middle: what the root sees past the end (the line from it crosses the
    // floor beside the opening) is beside it, here; the rest is at the
    // ceiling. The wedge from the root through the right end is x = 40 + y·4/3.
    let wide = boxed(Vec2::new(40.0, 0.0), Vec2::new(20.0, 20.0));
    let pieces = thread_volume(&wide, ABOVE, &[floor_pair()], MapConvention::Reflection);
    assert_eq!(pieces.len(), 3, "{pieces:?}");
    assert!(hits(&pieces, Vec2::new(55.0, 15.0)));
    assert!(!hits(&pieces, Vec2::new(30.0, 15.0)));
    // Reaching from the same root, the root sees part of what is past the end
    // through the opening (the wedge through the right end is x = 40 + y·4/3).
    let reached = partition_reaching(crate::frame::thread::MaterialPrimitive::world(&wide), ABOVE, &links(&[floor_pair()]), None, PieceBudget::SINGLE_HOP, MapConvention::Reflection).unwrap();
    let at = |p: Vec2| reached.iter().any(|piece| piece.meets(&victim_small(p)));
    assert!(at(Vec2::new(58.0, 3.0)), "outside the wedge: beside, here");
    assert!(!at(Vec2::new(52.0, 15.0)), "in the wedge: through, not here");
}

/// A swing reaches through an opening in the wedge from its root through the
/// opening's ends, as an eye sees through a window: what the blade swept
/// beside the opening met the floor and stays here, and what it swept inside
/// it comes out of the partner, unclipped by the partner's ends.
#[test]
fn a_swing_goes_through_in_the_wedge_from_its_root() {
    // The floor opening is 80 wide. A slash 120 wide and 20 deep, from a
    // shoulder 20 over the floor's middle: the wedge widens past the plane
    // (x = ±40·(1 + y/20)), so at 10 deep it spans ±60.
    let shoulder = Vec2::new(0.0, -20.0);
    let slash = boxed(Vec2::new(0.0, 5.0), Vec2::new(60.0, 15.0));
    let pieces = partition_reaching(crate::frame::thread::MaterialPrimitive::world(&slash), shoulder, &links(&[floor_pair()]), None, PieceBudget::SINGLE_HOP, MapConvention::Reflection).unwrap();
    let at = |p: Vec2| pieces.iter().any(|piece| piece.meets(&victim_small(p)));
    let through = pieces.iter().find(|p| p.is_carried()).expect("a part goes through");
    // 10 deep and 55 to the side: past the end, in the wedge. It comes out
    // of the ceiling 10 under it and 55 from its middle (mirrored or not).
    let out = [Vec2::new(500.0 + 55.0, -190.0), Vec2::new(500.0 - 55.0, -190.0)];
    assert!(out.iter().any(|p| through.meets(&victim_small(*p))), "{through:?}");
    assert!(!at(Vec2::new(55.0, 10.0)), "in the wedge, not here");
    // Just under the floor and 58 to the side: outside the wedge, here.
    assert!(at(Vec2::new(58.0, 1.5)));
    // The parts tile the swing: nothing is lost or drawn twice.
    assert_tiles(&slash, &pieces);
}

fn victim_small(center: Vec2) -> VolumePiece {
    VolumePiece::whole(boxed(center, Vec2::splat(1.0)))
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
    let mapped = carry_volume(&boxed(Vec2::new(0.0, 10.0), Vec2::new(20.0, 10.0)), &floor.frame, &wall.frame, MapConvention::Reflection).unwrap();
    let CombatVolume::Aabb(b) = mapped else { panic!("{mapped:?}") };
    assert_eq!(b.half_size(), Vec2::new(10.0, 20.0));
    assert_eq!(b.center(), Vec2::new(310.0, 0.0));
    // To a slope, it is turned and stays a box.
    let slope = PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.5, -0.75f32.sqrt()));
    assert!(matches!(
        carry_volume(&boxed(Vec2::new(0.0, 10.0), Vec2::splat(5.0)), &floor.frame, &slope, MapConvention::Reflection).unwrap(),
        CombatVolume::Obb { .. }
    ));
}


// ---- the partitioner (G1) ----

fn aperture(origin: Vec2, normal: Vec2, half_length: f32) -> PortalAperture {
    PortalAperture { frame: PortalFrame::fixed(origin, normal.normalize()), half_length }
}

const UP: Vec2 = Vec2::new(0.0, -1.0);
const DOWN: Vec2 = Vec2::new(0.0, 1.0);

fn area(poly: &[Vec2]) -> f32 {
    (0..poly.len()).map(|i| poly[i].perp_dot(poly[(i + 1) % poly.len()])).sum::<f32>().abs() / 2.0
}

/// The common part of two convex polygons.
fn overlap(a: &[Vec2], b: &[Vec2]) -> f32 {
    let mid = b.iter().copied().sum::<Vec2>() / b.len() as f32;
    let planes: Vec<HalfPlane> = (0..b.len())
        .map(|i| {
            let (p, q) = (b[i], b[(i + 1) % b.len()]);
            let n = (q - p).perp();
            HalfPlane { point: p, inward: if (mid - p).dot(n) >= 0.0 { n } else { -n } }
        })
        .collect();
    clip_convex(a.to_vec(), &planes).map_or(0.0, |c| area(&c))
}

/// Each piece's part, back in the volume's own chart.
fn source_regions(pieces: &[VolumePiece]) -> Vec<Vec<Vec2>> {
    pieces
        .iter()
        .map(|piece| {
            let back = piece.transform().inverse().unwrap();
            piece.region().unwrap().into_iter().map(|p| back.point(p).unwrap()).collect()
        })
        .collect()
}

/// The pieces tile the volume: their parts, taken back to its chart, cover
/// it once.
fn assert_tiles(volume: &CombatVolume, pieces: &[VolumePiece]) {
    let whole = area(&volume.polygon().unwrap());
    let parts = source_regions(pieces);
    let sum: f32 = parts.iter().map(|p| area(p)).sum();
    assert!((sum - whole).abs() < 0.05 * whole.max(1.0) / 100.0 + 0.05, "areas {sum} vs {whole}: {pieces:#?}");
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            assert!(overlap(&parts[i], &parts[j]) < 0.05, "pieces {i} and {j} overlap");
        }
        // Each part is in the volume.
        assert!((overlap(&parts[i], &volume.polygon().unwrap()) - area(&parts[i])).abs() < 0.05);
    }
}

#[test]
fn the_parts_of_a_cut_tile_the_volume() {
    let (floor, ceiling) = floor_pair();
    let slope = aperture(Vec2::new(-300.0, 0.0), Vec2::new(0.4, -1.0), 40.0);
    for convention in [MapConvention::Reflection, MapConvention::Rotation] {
        for pair in [(floor, ceiling), (floor, slope)] {
            for volume in [
                boxed(Vec2::new(0.0, 10.0), Vec2::new(10.0, 20.0)),
                boxed(Vec2::new(40.0, 0.0), Vec2::new(20.0, 20.0)),
                boxed(Vec2::new(0.0, 10.0), Vec2::new(70.0, 20.0)),
                CombatVolume::obb(Vec2::new(5.0, 5.0), Vec2::new(30.0, 6.0), 0.7),
                CombatVolume::convex(vec![Vec2::new(-20.0, -10.0), Vec2::new(25.0, -5.0), Vec2::new(0.0, 45.0)]),
            ] {
                let pieces = thread_volume(&volume, ABOVE, &[pair], convention);
                assert_tiles(&volume, &pieces);
                for piece in &pieces {
                    assert!((piece.transform().orthogonal().determinant().abs() - 1.0).abs() < 1e-5);
                    // Area is kept by every map.
                    let region = piece.region().unwrap();
                    let back: Vec<Vec2> = region.iter().map(|p| piece.uncarry(*p).unwrap()).collect();
                    assert!((area(&region) - area(&back)).abs() < 0.05);
                }
            }
        }
    }
}

#[test]
fn a_square_through_a_pair_that_turns_it_by_45_degrees_is_where_it_is() {
    // A floor portal to a 45° face: the part through is the square turned,
    // not the box around it.
    let floor = aperture(Vec2::ZERO, UP, 40.0);
    let tilted = aperture(Vec2::new(400.0, 0.0), Vec2::new(1.0, -1.0), 40.0);
    let square = boxed(Vec2::new(0.0, 10.0), Vec2::splat(10.0));
    let pieces = thread_volume(&square, ABOVE, &[(floor, tilted)], MapConvention::Reflection);
    assert_tiles(&square, &pieces);
    let through = pieces.iter().find(|p| p.is_carried()).unwrap();
    let region = through.region().unwrap();
    assert!((area(&region) - 400.0).abs() < 0.1, "{region:?}");
    // A corner of the box around the turned square is not in it.
    let centre = Vec2::new(400.0, 0.0) + Vec2::new(1.0, -1.0).normalize() * 10.0;
    // The box around it reaches 14.1 each way; along its own axes this is
    // 17 out.
    let corner_of_its_box = centre + Vec2::new(12.0, 12.0);
    assert!(!through.meets(&VolumePiece::whole(boxed(corner_of_its_box, Vec2::splat(0.5)))));
    assert!(through.meets(&VolumePiece::whole(boxed(centre, Vec2::splat(0.5)))));
}

#[test]
fn a_body_across_four_pairs_has_a_piece_through_each() {
    // Four floor openings side by side, each to a ceiling of its own.
    let pairs: Vec<_> = (0..4)
        .map(|i| {
            let x = i as f32 * 100.0;
            (aperture(Vec2::new(x, 0.0), UP, 40.0), aperture(Vec2::new(x + 1000.0, -300.0), DOWN, 40.0))
        })
        .collect();
    let giant = boxed(Vec2::new(150.0, 10.0), Vec2::new(200.0, 20.0));
    let p = partition(crate::frame::thread::MaterialPrimitive::world(&giant), Vec2::new(150.0, -50.0), &links(&pairs), None, PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    assert_tiles(&giant, &p);
    let carried: Vec<_> = p.iter().filter(|p| p.is_carried()).collect();
    assert_eq!(carried.len(), 4, "{:#?}", p);
    for i in 0..4 {
        let below = Vec2::new(i as f32 * 100.0 + 1000.0, -300.0 + 15.0);
        assert!(hits(&p, below), "under ceiling {i}");
        assert!(!hits(&p, Vec2::new(i as f32 * 100.0, 15.0)), "in floor {i}");
    }
    // Between the openings, the floor's part is where it is.
    assert!(hits(&p, Vec2::new(50.0, 15.0)));
}

#[test]
fn a_piece_through_two_pairs_is_in_the_chart_of_both() {
    // Into the floor at A, out of the ceiling at B, down into the floor at C,
    // out of the ceiling at D.
    let a = aperture(Vec2::ZERO, UP, 40.0);
    let b = aperture(Vec2::new(500.0, -200.0), DOWN, 40.0);
    let c = aperture(Vec2::new(500.0, -100.0), UP, 40.0);
    let d = aperture(Vec2::new(900.0, -200.0), DOWN, 40.0);
    // 150 deep: 100 of it comes out of B down to C, and 50 through C.
    let arm = boxed(Vec2::new(0.0, 75.0), Vec2::new(10.0, 75.0));
    let all = links(&[(a, b), (c, d)]);
    let p = partition(crate::frame::thread::MaterialPrimitive::world(&arm), Vec2::new(0.0, -10.0), &all, None, PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    assert_tiles(&arm, &p);
    let deepest = p.iter().max_by_key(|p| p.path.len()).unwrap();
    assert_eq!(deepest.path, vec![0, 2]);
    let t_ab = Iso2::through(&a.frame, &b.frame, MapConvention::Reflection);
    let t_cd = Iso2::through(&c.frame, &d.frame, MapConvention::Reflection);
    let composed = t_ab.then(&t_cd);
    assert!((deepest.transform().point(Vec2::new(3.0, 140.0)).unwrap() - composed.point(Vec2::new(3.0, 140.0))).length() < 1e-3);
    assert!(hits(&p, Vec2::new(900.0, -200.0 + 45.0)));
    assert!(!hits(&p, Vec2::new(900.0, -200.0 + 56.0)));
    assert!(!hits(&p, Vec2::new(500.0, -100.0 + 20.0)), "nothing in C's floor");
    assert!(hits(&p, Vec2::new(500.0, -100.0 - 10.0)), "between B and C");
    let one = partition(crate::frame::thread::MaterialPrimitive::world(&arm), Vec2::new(0.0, -10.0), &all, None, PieceBudget::SINGLE_HOP, MapConvention::Reflection);
    assert_eq!(one.unwrap_err(), Refusal { path: vec![0], link: Some(2), reason: RefusalReason::HopLimit });
}

#[test]
fn a_mirror_pair_and_a_turn_pair_differ_only_along_the_face() {
    let floor = aperture(Vec2::ZERO, UP, 40.0);
    let wall = aperture(Vec2::new(300.0, 0.0), Vec2::X, 40.0);
    let blade = boxed(Vec2::new(15.0, 10.0), Vec2::new(5.0, 20.0));
    let mirror = thread_volume(&blade, ABOVE, &[(floor, wall)], MapConvention::Reflection);
    let turn = thread_volume(&blade, ABOVE, &[(floor, wall)], MapConvention::Rotation);
    let through = |pieces: &[VolumePiece]| pieces.iter().find(|p| p.is_carried()).unwrap().clone();
    assert!((through(&mirror).transform().orthogonal().determinant() + 1.0).abs() < 1e-6);
    assert!((through(&turn).transform().orthogonal().determinant() - 1.0).abs() < 1e-6);
    // Depth is the same; the side along the wall is opposite.
    let (m, r) = (through(&mirror).center(), through(&turn).center());
    assert!((m.x - r.x).abs() < 1e-3 && (m.x - 315.0).abs() < 1e-3, "{m:?} {r:?}");
    assert!((m.y + r.y).abs() < 1e-3 && m.y.abs() > 10.0, "{m:?} {r:?}");
}

#[test]
fn the_order_links_are_given_in_does_not_change_the_pieces() {
    let pairs: Vec<_> = (0..3)
        .map(|i| {
            let x = i as f32 * 60.0;
            (aperture(Vec2::new(x, 0.0), UP, 25.0), aperture(Vec2::new(x, -400.0), DOWN, 25.0))
        })
        .collect();
    let all = links(&pairs);
    let mut reversed = all.clone();
    reversed.reverse();
    let mut shuffled = all.clone();
    shuffled.rotate_left(2);
    let body = boxed(Vec2::new(60.0, 0.0), Vec2::new(90.0, 30.0));
    let at = |links: &[PortalLink]| partition(crate::frame::thread::MaterialPrimitive::world(&body), Vec2::new(60.0, -40.0), links, None, PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    assert_eq!(at(&all), at(&reversed));
    assert_eq!(at(&all), at(&shuffled));
}

#[test]
fn a_volume_on_the_plane_or_grazing_an_end_is_not_cut() {
    let pair = [floor_pair()];
    // Its bottom on the plane.
    let resting = boxed(Vec2::new(0.0, -10.0), Vec2::splat(10.0));
    assert_eq!(thread_volume(&resting, ABOVE, &pair, MapConvention::Reflection), vec![VolumePiece::whole(resting.clone())]);
    // A positive material region remains a part, even when it is thin.
    let grazing = boxed(Vec2::new(0.0, -10.0 + 1e-5), Vec2::splat(10.0));
    let grazing_parts = thread_volume(&grazing, ABOVE, &pair, MapConvention::Reflection);
    assert_eq!(grazing_parts.len(), 2);
    assert_tiles(&grazing, &grazing_parts);
    // Past the plane, its edge on the opening's end: beside it, whole.
    let at_the_end = boxed(Vec2::new(50.0, 10.0), Vec2::splat(10.0));
    assert_eq!(thread_volume(&at_the_end, Vec2::new(50.0, -30.0), &pair, MapConvention::Reflection).len(), 1);
}

#[test]
fn a_pair_facing_itself_stops_at_its_budget() {
    // A ceiling straight over the floor it is linked to: what goes down into
    // the floor comes down from the ceiling, into the floor again.
    let floor = aperture(Vec2::ZERO, UP, 40.0);
    let ceiling = aperture(Vec2::new(0.0, -200.0), DOWN, 40.0);
    // 4000 deep: twenty times round.
    let pole = boxed(Vec2::new(0.0, 1990.0), Vec2::new(5.0, 2000.0));
    let all = links(&[(floor, ceiling)]);
    for budget in [PieceBudget::SINGLE_HOP, PieceBudget::COMPOSED, PieceBudget { max_scale_exponent: 12, max_hops: 50, max_pieces: 12 }] {
        let refusal = partition(crate::frame::thread::MaterialPrimitive::world(&pole), Vec2::new(0.0, -10.0), &all, None, budget, MapConvention::Reflection).unwrap_err();
        assert!(matches!(refusal.reason, RefusalReason::HopLimit | RefusalReason::PieceLimit | RefusalReason::WorkLimit));
        assert!(refusal.path.len() <= usize::from(budget.max_hops));
        assert!(refusal.path.iter().all(|key| *key == 0));
    }
}

#[test]
fn the_pair_a_body_straddles_cuts_it_first_whatever_side_its_centre_is_on() {
    let pair = floor_pair();
    let body = boxed(Vec2::new(0.0, 0.0), Vec2::splat(12.0));
    let all = links(&[pair]);
    // Its centre on the plane: from neither side.
    let free = partition(crate::frame::thread::MaterialPrimitive::world(&body), Vec2::ZERO, &all, None, PieceBudget::SINGLE_HOP, MapConvention::Reflection).unwrap();
    assert_eq!(free.len(), 1);
    let straddling = partition(crate::frame::thread::MaterialPrimitive::world(&body), Vec2::ZERO, &all, Some(0), PieceBudget::SINGLE_HOP, MapConvention::Reflection).unwrap();
    assert_eq!(straddling.len(), 2);
    assert_tiles(&body, &straddling);
}

/// Cost of a partition, pure: `cargo test --release -p ambition_platformer2d_core partition_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn partition_cost() {
    let run = |name: &str, volume: &CombatVolume, root: Vec2, links: &[PortalLink], first: Option<u32>, budget: PieceBudget| {
        let n = 20_000;
        let start = std::time::Instant::now();
        let mut pieces = 0;
        for _ in 0..n {
            pieces += std::hint::black_box(partition(crate::frame::thread::MaterialPrimitive::world(volume), root, links, first, budget, MapConvention::Reflection)).map_or(0, |parts| parts.len());
        }
        let per = start.elapsed().as_secs_f64() / n as f64 * 1e6;
        println!("{name:<32} {per:>8.2} µs  ({} pieces)", pieces / n);
    };
    let body = boxed(Vec2::new(0.0, -20.0), Vec2::new(12.0, 18.0));
    let none: Vec<PortalLink> = Vec::new();
    run("no portals", &body, Vec2::new(0.0, -20.0), &none, None, PieceBudget::COMPOSED);
    let one = links(&[floor_pair()]);
    run("one pair, not reached", &boxed(Vec2::new(0.0, -100.0), Vec2::splat(12.0)), Vec2::new(0.0, -100.0), &one, None, PieceBudget::COMPOSED);
    run("straddling one pair", &boxed(Vec2::new(0.0, 0.0), Vec2::splat(12.0)), Vec2::new(0.0, -1.0), &one, Some(0), PieceBudget::COMPOSED);
    let four: Vec<_> = (0..4)
        .map(|i| (aperture(Vec2::new(i as f32 * 100.0, 0.0), UP, 40.0), aperture(Vec2::new(i as f32 * 100.0 + 1000.0, -300.0), DOWN, 40.0)))
        .collect();
    run("a giant across four pairs", &boxed(Vec2::new(150.0, 10.0), Vec2::new(200.0, 20.0)), Vec2::new(150.0, -50.0), &links(&four), None, PieceBudget::COMPOSED);
    let chain = links(&[
        (aperture(Vec2::ZERO, UP, 40.0), aperture(Vec2::new(500.0, -200.0), DOWN, 40.0)),
        (aperture(Vec2::new(500.0, -100.0), UP, 40.0), aperture(Vec2::new(900.0, -200.0), DOWN, 40.0)),
    ]);
    run("an arm through two pairs", &boxed(Vec2::new(0.0, 75.0), Vec2::new(10.0, 75.0)), Vec2::new(0.0, -10.0), &chain, None, PieceBudget::COMPOSED);
    let cycle = links(&[(aperture(Vec2::ZERO, UP, 40.0), aperture(Vec2::new(0.0, -200.0), DOWN, 40.0))]);
    run("a pair facing itself", &boxed(Vec2::new(0.0, 1990.0), Vec2::new(5.0, 2000.0)), Vec2::new(0.0, -10.0), &cycle, None, PieceBudget::COMPOSED);
}

#[test]
fn a_link_that_goes_so_deep_keeps_what_is_deeper_where_it_is() {
    // A picture 60 deep into the floor's opening; its limb went 25 deep.
    let (floor, ceiling) = floor_pair();
    let link = PortalLink { enter: floor, exit: ceiling, key: 0, depth: Some(25.0) };
    let picture = boxed(Vec2::new(0.0, 20.0), Vec2::new(20.0, 40.0));
    let p = partition(crate::frame::thread::MaterialPrimitive::world(&picture), ABOVE, &[link], Some(0), PieceBudget::SINGLE_HOP, MapConvention::Reflection).unwrap();
    assert_tiles(&picture, &p);
    // 25 out of the ceiling, and no more.
    assert!(hits(&p, Vec2::new(500.0, -200.0 + 20.0)));
    assert!(!hits(&p, Vec2::new(500.0, -200.0 + 31.0)));
    // Deeper than that it is under the floor, where it is; shallower it is not.
    assert!(hits(&p, Vec2::new(0.0, 45.0)));
    assert!(!hits(&p, Vec2::new(0.0, 15.0)));
}

/// A rejected partition exposes no material that another consumer can admit.
#[test]
fn a_partition_is_never_larger_than_its_budget() {
    let (floor, ceiling) = floor_pair();
    let link = PortalLink { enter: floor, exit: ceiling, key: 0, depth: Some(25.0) };
    let picture = boxed(Vec2::new(0.0, 20.0), Vec2::new(60.0, 40.0));
    let cut = |max_pieces| partition(crate::frame::thread::MaterialPrimitive::world(&picture), ABOVE, &[link], Some(0),
        PieceBudget { max_scale_exponent: 12, max_hops: 1, max_pieces }, MapConvention::Reflection);
    let free = cut(16).unwrap();
    assert_eq!(free.len(), 5);
    assert_tiles(&picture, &free);
    for max_pieces in 0..5 {
        assert_eq!(cut(max_pieces).unwrap_err().reason, RefusalReason::PieceLimit);
    }
    let four: Vec<PortalLink> = (0..4).flat_map(|i| {
        let pair = (aperture(Vec2::new(i as f32 * 100.0, 0.0), UP, 40.0),
            aperture(Vec2::new(i as f32 * 100.0 + 1000.0, -300.0), DOWN, 40.0));
        PortalLink::both_ways(i, pair).map(|link| PortalLink { depth: Some(25.0), ..link })
    }).collect();
    let giant = boxed(Vec2::new(150.0, 10.0), Vec2::new(200.0, 40.0));
    for max_pieces in 0..=24 {
        match partition(crate::frame::thread::MaterialPrimitive::world(&giant), Vec2::new(150.0, -50.0), &four, None,
            PieceBudget { max_scale_exponent: 12, max_hops: 4, max_pieces }, MapConvention::Reflection) {
            Ok(parts) => {
                assert!(parts.len() <= usize::from(max_pieces));
                assert_tiles(&giant, &parts);
            }
            Err(refusal) => assert_eq!(refusal.reason, RefusalReason::PieceLimit),
        }
    }
}

#[test]
fn the_planes_a_piece_is_drawn_clipped_by_are_the_ones_that_cut_it() {
    let pieces = thread_volume(&boxed(Vec2::new(0.0, 10.0), Vec2::new(10.0, 20.0)), ABOVE, &[floor_pair()], MapConvention::Reflection);
    let through = pieces.iter().find(|p| p.is_carried()).unwrap();
    // In front of the exit cuts it; the opening's ends do not (it is inside them).
    assert_eq!(through.bounds.len(), 3);
    assert_eq!(through.active_bounds().len(), 1);
    let here = pieces.iter().find(|p| !p.is_carried()).unwrap();
    assert_eq!(here.active_bounds().len(), 1);
}

#[test]
fn a_pair_facing_itself_is_a_ring_and_a_pair_side_by_side_is_not() {
    let floor = aperture(Vec2::ZERO, UP, 40.0);
    let over = aperture(Vec2::new(0.0, -200.0), DOWN, 40.0);
    assert_eq!(recursive_links(&links(&[(floor, over)])), vec![0, 1]);
    // The partner far to the side: what comes down out of it does not come
    // down onto the floor's opening.
    let aside = aperture(Vec2::new(500.0, -200.0), DOWN, 40.0);
    assert!(recursive_links(&links(&[(floor, aside)])).is_empty());
    // Two doors in line, facing each other: an endless corridor.
    let wall_l = aperture(Vec2::new(-300.0, -100.0), Vec2::X, 40.0);
    let wall_r = aperture(Vec2::new(300.0, -100.0), -Vec2::X, 40.0);
    assert_eq!(recursive_links(&links(&[(floor, aside), (wall_l, wall_r)])), vec![2, 3]);
}

#[test]
fn unequal_apertures_preserve_source_material_and_scale_world_area() {
    for ratio in [0.5, 1.0, 2.0] {
        let (enter, mut exit) = floor_pair();
        exit.half_length *= ratio;
        let volume = boxed(Vec2::new(0.0, 5.0), Vec2::new(10.0, 20.0));
        for convention in [MapConvention::Reflection, MapConvention::Rotation] {
            let parts = partition(crate::frame::thread::MaterialPrimitive::world(&volume), ABOVE, &links(&[(enter, exit)]), None, PieceBudget::COMPOSED, convention).unwrap();
            assert_tiles(&volume, &parts);
            for piece in &parts {
                let source = piece.source_region().unwrap();
                let world = piece.region().unwrap();
                let scale = piece.transform().scale();
                assert!((area(&world) - area(&source) * scale * scale).abs() < 0.1);
                assert_eq!(piece.transform().reflected(), piece.is_carried() && matches!(convention, MapConvention::Reflection));
            }
        }
    }
}

#[test]
fn scaled_circle_parts_keep_their_analytic_radius() {
    let (enter, mut exit) = floor_pair();
    exit.half_length *= 0.5;
    let ball = CombatVolume::circle(Vec2::ZERO, 20.0);
    let parts = partition(crate::frame::thread::MaterialPrimitive::world(&ball), ABOVE, &links(&[(enter, exit)]), None, PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    let through = parts.iter().find(|part| part.is_carried()).unwrap();
    assert!(matches!(through.world_volume(), CombatVolume::Circle { radius, .. } if (*radius - 10.0).abs() < 1.0e-5));
    assert!(!through.meets(&victim_small(Vec2::new(509.0, -191.0))));
    assert!(through.meets(&victim_small(Vec2::new(500.0, -195.0))));
}

#[test]
fn canonical_parts_include_root_parity_and_keep_world_geometry_on_rebase() {
    let domain = boxed(Vec2::ZERO, Vec2::new(10.0, 20.0));
    let root = Sim2::new(2.0, bevy_math::Mat2::from_cols(-Vec2::X, Vec2::Y), Vec2::new(0.0, -5.0)).unwrap();
    let (enter, mut exit) = floor_pair();
    exit.half_length *= 0.5;
    let link = PortalLink::both_ways(0, (enter, exit))[0];
    let parts = partition(MaterialPrimitive { domain: &domain, to_world: root },
        root.point(Vec2::ZERO).unwrap(), &[link], Some(link.key), PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    assert_eq!(parts.len(), 2);
    let canonical_area: f32 = parts.iter().map(|part| area(&part.source_region().unwrap())).sum();
    assert!((canonical_area - area(&domain.polygon().unwrap())).abs() < 0.001);
    for part in parts {
        assert_eq!(part.source(), &domain);
        assert_eq!(part.transform().reflected(), !part.is_carried());
        let world = part.region().unwrap();
        let before_path = part.is_carried();
        let rebased = part.rebased_through(&link, MapConvention::Reflection).unwrap();
        assert_eq!(rebased.region().unwrap(), world);
        assert_eq!(rebased.transform(), part.transform());
        assert_ne!(rebased.is_carried(), before_path);
        let canonical = part.source_region().unwrap()[0];
        let new_root = root.then(link.map(MapConvention::Reflection).unwrap()).unwrap().point(canonical).unwrap();
        assert!((rebased.carry(new_root).unwrap() - part.transform().point(canonical).unwrap()).length() < 0.001);
    }
}

#[test]
fn cumulative_scale_refuses_by_policy_before_a_numeric_fault() {
    for (root_scale, ratio) in [(4.0, 2.0), (0.25, 0.5)] {
        let domain = boxed(Vec2::ZERO, Vec2::splat(10.0));
        let root = Sim2::new(root_scale, bevy_math::Mat2::IDENTITY, Vec2::new(0.0, -1.0)).unwrap();
        let (enter, mut exit) = floor_pair();
        exit.half_length *= ratio;
        let budget = PieceBudget { max_scale_exponent: 2, ..PieceBudget::COMPOSED };
        let refusal = partition(MaterialPrimitive { domain: &domain, to_world: root },
            root.point(Vec2::ZERO).unwrap(), &links(&[(enter, exit)]), Some(0), budget, MapConvention::Reflection).unwrap_err();
        assert_eq!(refusal.reason, RefusalReason::ScaleLimit);
        assert_eq!(refusal.link, Some(0));
    }
}


#[test]
fn small_world_primitives_keep_their_through_material() {
    let enter = aperture(Vec2::ZERO, UP, 0.04);
    let exit = aperture(Vec2::new(0.5, -0.2), DOWN, 0.04);
    let domain = boxed(Vec2::new(0.0, -0.0005), Vec2::splat(0.002));
    assert!(reaches_through(&domain, &enter));
    let parts = partition(MaterialPrimitive::world(&domain), domain.center(), &links(&[(enter, exit)]),
        Some(0), PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|part| part.source_region().is_some()));
    let covered: f32 = parts.iter().map(|part| area(&part.source_region().unwrap())).sum();
    assert!((covered - area(&domain.polygon().unwrap())).abs() < 1.0e-10);
}

#[test]
fn canonical_coverage_does_not_depend_on_the_world_scale() {
    let domain = boxed(Vec2::ZERO, Vec2::splat(20.0));
    for exponent in [-12, -8, 0, 8, 12] {
        let scale = 2.0_f32.powi(exponent);
        let root = Sim2::new(scale, bevy_math::Mat2::IDENTITY, Vec2::new(0.0, -5.0 * scale)).unwrap();
        let enter = aperture(Vec2::ZERO, UP, 40.0 * scale);
        let exit = aperture(Vec2::new(500.0, -200.0) * scale, DOWN, 40.0 * scale);
        let parts = partition(MaterialPrimitive { domain: &domain, to_world: root }, root.point(Vec2::ZERO).unwrap(),
            &links(&[(enter, exit)]), Some(0), PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
        assert_eq!(parts.len(), 2, "scale exponent {exponent}");
        assert_tiles(&domain, &parts);
    }
}


#[test]
fn clipped_circles_have_no_world_size_contact_halo() {
    for exponent in [-12, -8, 0, 8, 12] {
        let scale = 2.0_f32.powi(exponent);
        let domain = CombatVolume::circle(Vec2::ZERO, 20.0);
        let root = Sim2::new(scale, bevy_math::Mat2::IDENTITY, Vec2::ZERO).unwrap();
        let enter = aperture(Vec2::ZERO, UP, 40.0 * scale);
        let exit = aperture(Vec2::new(500.0, -200.0) * scale, DOWN, 40.0 * scale);
        let parts = partition(MaterialPrimitive { domain: &domain, to_world: root }, Vec2::new(0.0, -10.0) * scale,
            &links(&[(enter, exit)]), Some(0), PieceBudget::COMPOSED, MapConvention::Reflection).unwrap();
        let through = parts.iter().find(|part| part.is_carried()).unwrap();
        for (offset, touches) in [(-2.0, false), (2.0, true)] {
            let target = VolumePiece::whole(CombatVolume::circle(exit.frame.origin + Vec2::Y * (offset * scale), 0.25 * scale));
            assert_eq!(through.meets(&target), touches, "exponent {exponent}, offset {offset}");
            assert_eq!(target.meets(through), touches, "contact must be symmetric");
        }
    }
}

#[test]
fn a_small_rotation_does_not_erase_the_end_of_a_long_primitive() {
    let source=CombatVolume::aabb(crate::Aabb::new(Vec2::ZERO,Vec2::new(1.0e8,1.0)));
    let map=Sim2::new(1.0,bevy_math::Mat2::from_angle(5.0e-7),Vec2::ZERO).unwrap();
    let mapped=map_volume(&source,&map).unwrap();
    assert!(mapped.bounds().max.y>50.0,"{mapped:?}");
}
