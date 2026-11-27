//! The rim as contact: the cases the post-step snag decided, met in the cast.

use super::*;
use crate::frame::thread::{MaterialPrimitive, PieceBudget, PortalLink};
use crate::frame::{ApertureStep, BodyMaterial, MapConvention, PortalAperture, PortalFrame};
use crate::world::MaterialCollider;
use crate::{AabbExt, BodyKinematics};

fn aperture(origin: Vec2, normal: Vec2, half_length: f32) -> PortalAperture {
    PortalAperture { frame: PortalFrame::fixed(origin, normal), half_length }
}

/// A floor portal at the origin (front above, `+y` down), its opening
/// `x ∈ [-12, 12]`, and its partner far off.
fn floor_links() -> [PortalLink; 2] {
    PortalLink::both_ways(0, (aperture(Vec2::ZERO, -Vec2::Y, 12.0), aperture(Vec2::new(400.0, 0.0), -Vec2::Y, 12.0)))
}

fn body(center: Vec2, size: Vec2) -> BodyKinematics {
    BodyKinematics { pos: center, canonical_size: size, ..Default::default() }
}

/// The first rim the body meets moving by `delta`, through the same query
/// a movement system asks of its room.
fn first_rim(body: BodyKinematics, delta: Vec2, links: &[PortalLink]) -> Option<(RimObstacle, MaterialCastHit)> {
    let domain = body.canonical_box().unwrap();
    let cast = MaterialCast::new(MaterialPrimitive { domain: &domain, to_world: body.material_map().unwrap() },
        body.pos, delta, 0.0..1.0);
    let admission = MotionAdmission { span: Default::default(), links, first: None,
        budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection };
    let world = crate::World::new("rim", Vec2::splat(2000.0), Vec2::ZERO, vec![]);
    let cells = cast.cells(Some(admission)).unwrap();
    let hit = world.first_material_contact(&cells, |_, _, _| Ok(true)).unwrap()?;
    match hit.collider {
        MaterialCollider::Rim(rim) => Some((rim, hit.contact)),
        other => panic!("an empty room has only rims: {other:?}"),
    }
}

/// The plan's example (portal P3): a 20 by 2 body turned 45 degrees, its
/// long axis along the mouth normal and 4 px along the face, comes in from
/// 12 px to 1 px through a ±12 opening and meets no rim. Its world box
/// (22 px) was snagged. Turned to lie along the face, 30 long, it lands on
/// both ends.
#[test]
fn a_turned_body_that_fits_passes_and_one_that_does_not_lands_on_the_ends() {
    let turn = std::f64::consts::FRAC_PI_4;
    let material = BodyMaterial::IDENTITY.turned(turn).unwrap();
    let normal = Vec2::new(-(turn.sin() as f32), turn.cos() as f32);
    let ap = aperture(Vec2::ZERO, normal, 12.0);
    let links = PortalLink::both_ways(0, (ap, aperture(Vec2::new(400.0, 0.0), normal, 12.0)));
    let tangent = ap.frame.tangent();
    let at = |size: Vec2, depth: f32| BodyKinematics { material, pos: tangent * 4.0 + normal * depth,
        canonical_size: size, ..Default::default() };
    let thin = at(Vec2::new(2.0, 20.0), 12.0);
    assert!(thin.aabb().half_size().x > 6.9, "premise: its world box is wide: {:?}", thin.aabb());
    assert_eq!(first_rim(thin, -normal * 11.0, &links), None, "a body that fits comes in");
    let wide = at(Vec2::new(30.0, 2.0), 12.0);
    let (rim, hit) = first_rim(wide, -normal * 12.0, &links).expect("30 along a 24 opening does not fit");
    assert_eq!(rim.kind, RimKind::End);
    assert!((hit.toi - 11.0 / 12.0).abs() < 1e-4, "its face reaches the plane: {}", hit.toi);
    assert!(hit.normal.dot(normal) > 0.99, "it is held out along the mouth normal: {:?}", hit.normal);
}

/// Sliding along the face from within the opening, a body stops at the
/// end: the end faces back into the opening.
#[test]
fn a_body_sliding_along_the_face_stops_at_the_end() {
    let (rim, hit) = first_rim(body(Vec2::new(0.0, -1.0), Vec2::splat(8.0)), Vec2::new(20.0, 0.0), &floor_links())
        .expect("its right side reaches the end at x 12");
    assert_eq!((rim.end, rim.kind), (1, RimKind::End));
    assert!((hit.toi - 0.4).abs() < 1e-6, "{}", hit.toi);
    assert!((hit.normal - -Vec2::X).length() < 1e-6, "back along the face: {:?}", hit.normal);
}

/// Coming in from the front over an end, a body lands on the end; wholly
/// beside the opening it meets no rim (the host there is the room's).
#[test]
fn a_body_falling_over_an_end_lands_on_it_and_one_beside_the_opening_does_not() {
    let links = floor_links();
    let (_, hit) = first_rim(body(Vec2::new(10.0, -10.0), Vec2::splat(8.0)), Vec2::new(0.0, 20.0), &links)
        .expect("x 6..14 holds the end at 12");
    assert!((hit.toi - 0.3).abs() < 1e-6, "its feet reach the plane: {}", hit.toi);
    assert!((hit.normal - -Vec2::Y).length() < 1e-6, "held out of the plane: {:?}", hit.normal);
    assert_eq!(first_rim(body(Vec2::new(30.0, -10.0), Vec2::splat(8.0)), Vec2::new(0.0, 20.0), &links), None);
    assert_eq!(first_rim(body(Vec2::new(0.0, -10.0), Vec2::splat(8.0)), Vec2::new(0.0, 20.0), &links), None,
        "control: within the opening it goes in");
}

/// A body behind the portal is the host's (its backside), not the rim's.
#[test]
fn a_body_behind_the_opening_meets_no_rim() {
    assert_eq!(first_rim(body(Vec2::new(10.0, 10.0), Vec2::splat(8.0)), Vec2::new(0.0, -20.0), &floor_links()), None);
}

/// A body already past an end (its feet into a slope beside the opening)
/// may go along, or back, but no deeper.
#[test]
fn a_body_past_an_end_goes_no_deeper() {
    let links = floor_links();
    let feet_in = body(Vec2::new(20.0, -3.0), Vec2::splat(8.0));
    let (rim, hit) = first_rim(feet_in, Vec2::new(0.0, 5.0), &links).expect("deeper is held");
    assert_eq!((rim.end, rim.kind), (1, RimKind::Floor));
    assert!(hit.toi < 1e-3, "at once: {}", hit.toi);
    assert!((hit.normal - -Vec2::Y).length() < 1e-6, "{:?}", hit.normal);
    assert_eq!(first_rim(feet_in, Vec2::new(-20.0, 0.0), &links), None, "along the face, onto the opening");
    assert_eq!(first_rim(feet_in, Vec2::new(0.0, -5.0), &links), None, "back out");
}

/// A moving opening meets a still body: its end comes to the body.
#[test]
fn a_moving_end_meets_a_still_body() {
    let end = aperture(Vec2::new(-20.0, 0.0), -Vec2::Y, 12.0);
    let exit = aperture(Vec2::new(400.0, 0.0), -Vec2::Y, 12.0);
    let links = PortalLink::both_ways_with_steps(0, [
        (end, ApertureStep { origin: Vec2::ZERO, normal: -Vec2::Y, displacement: Vec2::new(-20.0, 0.0), turn: 0.0 }),
        (exit, ApertureStep::fixed(exit.frame.origin, exit.frame.normal)),
    ]);
    let (rim, hit) = first_rim(body(Vec2::new(0.0, -1.0), Vec2::splat(8.0)), Vec2::ZERO, &links)
        .expect("the end at 12 travels to -8 through the body's side at 4");
    assert_eq!(rim.end, 1);
    assert!((hit.toi - 0.4).abs() < 1e-6, "{}", hit.toi);
    assert!((hit.normal - -Vec2::X).length() < 1e-6, "{:?}", hit.normal);
}
