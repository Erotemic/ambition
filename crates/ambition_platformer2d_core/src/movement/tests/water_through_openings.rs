//! A body through an opening is in the water, and on the ladder, its admitted
//! material is in (world-geometry bounded delivery, steps 1 and 3).
//!
//! A body straddles a pair: its root is in front of the entry and 7 px of it
//! is past the entry plane, carried out of the far end. The root box is the
//! wrong question both ways. It misses a pool at the far end, which the
//! carried part is in, and it meets a pool behind the entry, which only the
//! cut-away part of it reaches.

use super::super::*;
use crate::body_clusters::BodyClusterScratch;
use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
use crate::frame::{MapConvention, PortalAperture, PortalFrame};
use crate::world::{ClimbableRegion, WaterKind, WaterRegion};
use crate::{AbilitySet, Aabb, AabbExt, MotionFrame, Vec2, World};

const BODY: Vec2 = Vec2::new(24.0, 48.0);
/// The root, 5 px in front of the entry plane: the body spans x 483..507.
const ROOT: Vec2 = Vec2::new(495.0, 500.0);
/// The far end's mouth faces +x, so the carried part is at x 900..907.
const EXIT: Vec2 = Vec2::new(900.0, 200.0);
/// A pool at the far end, beside the carried part.
const FAR_POOL: Aabb = Aabb { min: Vec2::new(902.0, 150.0), max: Vec2::new(960.0, 260.0) };
/// A pool behind the entry, where only the cut-away part of the root box is.
const BEHIND_ENTRY: Aabb = Aabb { min: Vec2::new(501.0, 480.0), max: Vec2::new(540.0, 520.0) };

struct Stepped { in_water: bool, drowned: bool, on_ladder: bool }

fn links() -> [PortalLink; 2] {
    let entry = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, 500.0), Vec2::NEG_X), half_length: 80.0 };
    let exit = PortalAperture { frame: PortalFrame::fixed(EXIT, Vec2::X), half_length: 80.0 };
    PortalLink::both_ways(0, (entry, exit))
}

fn admission(links: &[PortalLink], convention: MapConvention) -> MotionAdmission<'_> {
    MotionAdmission { span: Default::default(), links, first: Some(0), budget: PieceBudget::COMPOSED, convention }
}

fn step(pool: Aabb, through: bool, swim: bool) -> Stepped {
    let mut world = World::new("water through openings", Vec2::splat(2_000.0), Vec2::splat(100.0), Vec::new());
    world.water_regions.push(WaterRegion::new(pool, WaterKind::Clear, Default::default()));
    step_in(&world, through.then_some(MapConvention::Reflection), swim)
}

fn step_in(world: &World, through: Option<MapConvention>, swim: bool) -> Stepped {
    let links = links();
    let abilities = AbilitySet { swim, ..AbilitySet::default() };
    let mut scratch = BodyClusterScratch::new_with_abilities(ROOT, abilities);
    scratch.kinematics.canonical_size = BODY;
    scratch.base_size.base_size = BODY;
    let (model, mut clusters) = scratch.parts();
    switch_motion_model(model, MotionModelSpec::AxisSwept(AxisSweptParams::default()));
    let result = step_motion(model, &mut clusters, MotionStepContext {
        world,
        admission: through.map(|convention| admission(&links, convention)),
        input: InputState::default(), frame: MotionFrame::from_direction(Vec2::Y, 0.0), facing_intent: 0.0,
        dt: 1.0 / 120.0, contact: body_contact::BodyContactField::NONE, pose_owned_externally: false,
        recovery_commitment_outstanding: false,
    });
    assert!(result.events.motion_refusal.is_none(), "the pair admits this body: {:?}", result.events.motion_refusal);
    Stepped {
        in_water: scratch.env_contact.water.is_some(),
        drowned: result.events.reset == Some(ResetCause::Drowned),
        on_ladder: scratch.env_contact.climbable.is_some(),
    }
}

fn on_ladder(ladder: Aabb, through: Option<MapConvention>) -> bool {
    let mut world = World::new("ladders through openings", Vec2::splat(2_000.0), Vec2::splat(100.0), Vec::new());
    world.climbable_regions.push(ClimbableRegion::ladder(ladder));
    step_in(&world, through, false).on_ladder
}

/// Whether the part carried out of the far end keeps up as up, under
/// `convention`.
fn carried_part_keeps_up(convention: MapConvention) -> bool {
    let links = links();
    let kin = crate::BodyKinematics { pos: ROOT, canonical_size: BODY, ..Default::default() };
    let parts = kin.occupied(admission(&links, convention)).unwrap().expect("the pair cuts the body");
    let carried = parts.iter().find(|part| !part.path.is_empty()).expect("a part is carried");
    carried.to_root.direction(Vec2::NEG_Y).dot(Vec2::NEG_Y) > 0.999
}

/// A ladder is met as water is, and climbed only where the map that
/// carries the part keeps up as up: the climb moves the body along its own
/// chart's axes. The same pair under the two conventions gives one map of
/// each kind.
#[test]
fn a_straddler_climbs_the_ladder_its_far_part_is_on_only_where_up_stays_up() {
    let [keeps, flips] = [MapConvention::Reflection, MapConvention::Rotation];
    let (keeps, flips) = if carried_part_keeps_up(keeps) { (keeps, flips) } else { (flips, keeps) };
    assert!(carried_part_keeps_up(keeps) && !carried_part_keeps_up(flips), "premise: one convention flips up");

    assert!(!on_ladder(FAR_POOL, None), "no opening: the root box misses the far ladder");
    assert!(on_ladder(BEHIND_ENTRY, None), "no opening: the root box is on the ladder behind the entry");
    assert!(on_ladder(FAR_POOL, Some(keeps)), "the carried part is on the far ladder, up kept");
    assert!(!on_ladder(BEHIND_ENTRY, Some(keeps)), "no material is behind the entry");
    assert!(!on_ladder(FAR_POOL, Some(flips)), "a ladder met through a map that flips up admits no climb");

    // Inspection keeps what the response drops: the flipped touch is
    // reported, and refused for its map.
    let touched = |convention| {
        let mut world = World::new("ladders through openings", Vec2::splat(2_000.0), Vec2::splat(100.0), Vec::new());
        world.climbable_regions.push(ClimbableRegion::ladder(FAR_POOL));
        world.climbable_regions.push(ClimbableRegion::ladder(FAR_POOL));
        let links = links();
        let kin = crate::BodyKinematics { pos: ROOT, canonical_size: BODY, ..Default::default() };
        world.regions_touched(&kin.occupied(admission(&links, convention)).unwrap().unwrap())
    };
    use crate::{RegionTouch, TouchResponse::*, TouchedRegion::Climbable};
    let touch = |index, response| RegionTouch { region: Climbable(index), response };
    assert_eq!(touched(keeps), [touch(0, Answers), touch(1, Shadowed)], "up kept: the first ladder answers");
    assert_eq!(touched(flips), [touch(0, UpTurned), touch(1, UpTurned)], "up flipped: both are touched, neither climbed");
}

#[test]
fn a_straddler_is_in_the_water_its_far_part_is_in_and_not_in_water_its_cut_part_left() {
    let root_box = Aabb::new(ROOT, BODY * 0.5);
    assert!(!root_box.strict_intersects(FAR_POOL), "premise: the root box misses the far pool");
    assert!(root_box.strict_intersects(BEHIND_ENTRY), "premise: the root box meets the pool behind the entry");

    // The control: with no opening the root box is the body, both ways.
    assert!(!step(FAR_POOL, false, true).in_water);
    let control = step(BEHIND_ENTRY, false, false);
    assert!(control.in_water && control.drowned, "no opening: the body behind the plane is in that pool");

    let far = step(FAR_POOL, true, true);
    assert!(far.in_water, "the part carried out of the far end is in the far pool");
    assert!(step(FAR_POOL, true, false).drowned, "and a body that cannot swim drowns in it");

    let behind = step(BEHIND_ENTRY, true, false);
    assert!(!behind.in_water && !behind.drowned, "no material is behind the entry: no water, no drowning");
}

/// Membership is overlap by more than a touch: a turned part whose bounds
/// overlap a region but whose face only meets its corner is not in it.
#[test]
fn a_turned_part_that_only_touches_a_region_is_not_in_it() {
    let region = Aabb { min: Vec2::ZERO, max: Vec2::splat(10.0) };
    // A diamond whose lower-left face is the line x + y = 20, through (10, 10).
    let diamond = |shift: f32| [Vec2::new(15.0, 5.0), Vec2::new(25.0, 15.0), Vec2::new(15.0, 25.0), Vec2::new(5.0, 15.0)]
        .map(|p| p - Vec2::splat(shift));
    assert!(region.strict_intersects(Aabb { min: Vec2::splat(5.0), max: Vec2::splat(25.0) }), "premise: the bounds overlap");
    assert!(!crate::world::convex_strictly_overlaps(&diamond(0.0), region));
    assert!(crate::world::convex_strictly_overlaps(&diamond(0.5), region));
}
