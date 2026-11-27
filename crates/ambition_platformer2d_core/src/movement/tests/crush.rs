//! Crushing C0 and C1 (crushing-and-obstruction-response.md): what the kernel
//! does today when solids pinch a body, pinned before any response changes
//! it, and the obstruction fact read from the same conflict built across a
//! portal pair.

use super::*;
use crate::collision_semantics::{Axis, ConflictSide, ContactSource};
use crate::world::Block;
use crate::AbilitySet;
use crate::geometry::AabbExt;

fn body_at(pos: Vec2) -> BodyClusterScratch {
    let mut body = BodyClusterScratch::new_with_abilities(pos, AbilitySet::sandbox_all());
    body.kinematics.pos = pos;
    body.kinematics.vel = Vec2::ZERO;
    body
}

fn named<'a>(source: &ContactSource, name: &str, blocks: impl IntoIterator<Item = &'a Block>) -> bool {
    match source {
        ContactSource::Block { id, .. } => blocks.into_iter().any(|block| block.id == *id && block.name == name),
        ContactSource::Chain { .. } | ContactSource::Rim { .. } => false,
    }
}

/// Advance each moving block by its per-tick motion: the world is read at
/// the tick's end, its `velocity` the travel that brought it there.
fn advance(world: &mut World) {
    for block in &mut world.blocks {
        let step = block.velocity;
        block.aabb.min += step;
        block.aabb.max += step;
    }
}

/// C4's first piece (C0 found it missing): a press that descends onto a body
/// standing on a floor is one Y conflict per step, whose claimants are the
/// press and the floor, whose overconstraint is the overlap the press's step
/// made, and whose driver is the press. The repair met the pinch one side at
/// a time (the press pushed the body into the floor, the floor pushed it back
/// under the press) and reported nothing; a one-sided repair that would land
/// the body in an opposing claimant now reports the pinch and leaves the body
/// where it stands, for the body's response to act on.
#[test]
fn a_press_closing_on_a_standing_body_is_one_conflict_of_the_overlap() {
    let floor = Block::solid("floor", Vec2::new(0.0, 300.0), Vec2::new(1600.0, 40.0));
    let probe = body_at(Vec2::ZERO);
    let half = probe.kinematics.aabb().half_size().y;
    let stand = Vec2::new(400.0, 300.0 - half);
    // The press's face starts 2 px over the body's head.
    let press = Block::solid("press", Vec2::new(300.0, stand.y - half - 2.0 - 40.0), Vec2::new(200.0, 40.0));
    let mut world = World::new("press", Vec2::new(1600.0, 900.0), stand, vec![floor, press]);
    let mut body = body_at(stand);
    for _ in 0..3 { step_scratch(&world, &mut body, InputState::default()); }
    assert!(body.ground.on_ground, "it stands: {:?}", body.kinematics);
    let stood = body.kinematics.pos;
    world.blocks[1].velocity = Vec2::new(0.0, 6.0);
    for tick in 1..=3 {
        advance(&mut world);
        let events = step_scratch(&world, &mut body, InputState::default());
        let [conflict] = events.constraint_conflicts.as_slice() else {
            panic!("tick {tick}: one conflict: {:?}", events.constraint_conflicts)
        };
        assert_eq!(conflict.axis, Axis::Y);
        let sources = [&conflict.min_source, &conflict.max_source];
        assert!(sources.iter().any(|source| named(*source, "press", world.blocks.iter())), "{conflict:?}");
        assert!(sources.iter().any(|source| named(*source, "floor", world.blocks.iter())), "{conflict:?}");
        let overlap = 6.0 * tick as f32 - 2.0;
        assert!((conflict.overconstraint() - overlap).abs() < 0.05, "tick {tick}: the overlap: {conflict:?}");
        assert_eq!(conflict.driver(), Some(ConflictSide::Min), "the press closes from above: {conflict:?}");
        assert!((body.kinematics.pos - stood).length() < 0.05, "tick {tick}: nothing resolves it yet: {:?}", body.kinematics.pos);
    }
}

/// C0: a one-way platform that raises a body into a ceiling drops it. The
/// one-way claims a body only while its feet were within the crossing band
/// over its top (`material_axis_hits`, `prev_feet_coord`); held under the
/// ceiling, the body stays and the platform rises on past its feet, so its
/// claim lapses and the body falls through it; a one-way is never a claimant
/// of a reported pinch (it yields). This is the kernel's rule for every game today (Mary-O's
/// reported behaviour); C4 names it `one_way_yields`.
#[test]
fn a_one_way_that_raises_a_body_into_a_ceiling_drops_it_through() {
    let ceiling = Block::solid("ceiling", Vec2::new(0.0, 160.0), Vec2::new(1600.0, 40.0));
    let lift = Block::one_way("lift", Vec2::new(300.0, 300.0), Vec2::new(200.0, 16.0));
    let probe = body_at(Vec2::ZERO);
    let half = probe.kinematics.aabb().half_size().y;
    let stand = Vec2::new(400.0, 300.0 - half);
    let mut world = World::new("lift", Vec2::new(1600.0, 900.0), stand, vec![ceiling, lift]);
    let mut body = body_at(stand);
    for _ in 0..3 { step_scratch(&world, &mut body, InputState::default()); }
    assert!(body.ground.on_ground, "it stands on the lift: {:?}", body.kinematics);
    world.blocks[1].velocity = Vec2::new(0.0, -4.0);
    let mut held = None;
    let mut dropped = None;
    for tick in 0..60 {
        advance(&mut world);
        let events = step_scratch(&world, &mut body, InputState::default());
        assert!(events.constraint_conflicts.is_empty(), "tick {tick}: the one-way yields: {:?}", events.constraint_conflicts);
        if held.is_none() && body.kinematics.aabb().min.y <= 200.0 + 0.05 {
            held = Some(tick);
        }
        if body.kinematics.aabb().max.y > world.blocks[1].aabb.min.y + 1.0 {
            dropped = Some(tick);
            break;
        }
    }
    let held = held.expect("the lift raises the body to the ceiling");
    let dropped = dropped.expect("the held body falls through the one-way");
    assert!(dropped >= held, "{held} then {dropped}");
}

/// C4: a game whose one-ways do not yield (`World::one_way_yields` false)
/// sees the same lift's pinch reported, with the lift and the ceiling as its
/// claimants, for the body's crush response to answer.
#[test]
fn a_one_way_that_does_not_yield_reports_its_pinch() {
    let ceiling = Block::solid("ceiling", Vec2::new(0.0, 160.0), Vec2::new(1600.0, 40.0));
    let lift = Block::one_way("lift", Vec2::new(300.0, 300.0), Vec2::new(200.0, 16.0));
    let probe = body_at(Vec2::ZERO);
    let half = probe.kinematics.aabb().half_size().y;
    let stand = Vec2::new(400.0, 300.0 - half);
    let mut world = World::new("lift", Vec2::new(1600.0, 900.0), stand, vec![ceiling, lift]).with_one_way_yields(false);
    let mut body = body_at(stand);
    for _ in 0..3 { step_scratch(&world, &mut body, InputState::default()); }
    assert!(body.ground.on_ground, "it stands on the lift: {:?}", body.kinematics);
    world.blocks[1].velocity = Vec2::new(0.0, -4.0);
    let mut pinched = None;
    for tick in 0..60 {
        advance(&mut world);
        let events = step_scratch(&world, &mut body, InputState::default());
        if let Some(conflict) = events.constraint_conflicts.first() {
            pinched = Some((tick, conflict.clone()));
            break;
        }
    }
    let (_, conflict) = pinched.expect("the lift's pinch is reported");
    assert_eq!(conflict.axis, Axis::Y);
    let sources = [&conflict.min_source, &conflict.max_source];
    assert!(sources.iter().any(|source| named(*source, "lift", world.blocks.iter())), "{conflict:?}");
    assert!(sources.iter().any(|source| named(*source, "ceiling", world.blocks.iter())), "{conflict:?}");
}

/// C0: a body that walks into a still wall is stopped by it and reports no
/// conflict: nothing closes on it, so there is nothing to crush.
#[test]
fn a_body_walking_into_a_still_wall_reports_no_conflict() {
    let floor = Block::solid("floor", Vec2::new(0.0, 300.0), Vec2::new(1600.0, 40.0));
    let wall = Block::solid("wall", Vec2::new(500.0, 100.0), Vec2::new(40.0, 200.0));
    let mut probe = body_at(Vec2::ZERO);
    let half = probe.kinematics.aabb().half_size();
    let stand = Vec2::new(500.0 - half.x - 20.0, 300.0 - half.y);
    probe.kinematics.pos = stand;
    let world = World::new("wall", Vec2::new(1600.0, 900.0), stand, vec![floor, wall]);
    let mut body = body_at(stand);
    let right = InputState { axes: crate::LocalAxes::new(1.0, 0.0), ..InputState::default() };
    for _ in 0..40 {
        let events = step_scratch(&world, &mut body, right);
        assert!(events.constraint_conflicts.is_empty(), "{:?}", events.constraint_conflicts);
    }
    assert!(body.kinematics.aabb().max.x <= 500.0 + 0.05, "the wall stops it: {:?}", body.kinematics);
    assert!(body.kinematics.aabb().max.x > 500.0 - 1.0, "it reached the wall: {:?}", body.kinematics);
}

/// C1: the pinch of `a_pinch_names_the_claimant_that_closes_on_the_body`
/// built across a static portal pair. The ceiling is not over the body: a
/// mouth is, facing down, and its exit is a floor mouth far away, covered by
/// a solid. The body's head goes through the pair and meets that solid,
/// which claims it as the local ceiling did, with the same numbers; the
/// rising platform drives, and the other claimant is the far-side solid.
#[test]
fn a_pinch_across_a_portal_pair_is_the_local_pinch() {
    use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink};
    use crate::frame::{MapConvention, PortalAperture, PortalFrame};
    let platform = || {
        let mut block = Block::solid("rising platform", Vec2::new(300.0, 272.0), Vec2::new(200.0, 40.0));
        block.velocity = Vec2::new(0.0, -4.0);
        block
    };
    let start = Vec2::new(400.0, 256.0);
    let conflict = |blocks: Vec<Block>, mut admission: Option<MotionAdmission<'_>>| {
        let world = World::new("pinch", Vec2::new(1600.0, 900.0), start, blocks);
        let mut scratch = body_at(start);
        let gravity_dir = Vec2::new(0.0, 1.0);
        let prev_feet_coord = scratch.kinematics.aabb().feet_coord(gravity_dir);
        super::super::collision::sweep_player_axis_clusters(
            &world, &mut scratch.kinematics, &mut scratch.ground, &mut scratch.wall,
            &scratch.body_mode, &scratch.env_contact, Axis::Y,
            0.0, prev_feet_coord, false, gravity_dir, &mut Vec::new(), &mut admission, &mut None, None,
        )
    };
    let local = vec![Block::solid("ceiling", Vec2::new(0.0, 200.0), Vec2::new(1600.0, 40.0)), platform()];
    let local = conflict(local, None).expect("a 32 px gap cannot hold a 48 px body");

    // A mouth where the ceiling's face was, facing down into the room; its
    // exit is a floor mouth at (1200, 640) facing up, covered by a solid
    // whose underside is on the exit's plane.
    let mouth = PortalAperture { frame: PortalFrame::fixed(Vec2::new(400.0, 240.0), Vec2::Y), half_length: 60.0 };
    let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(1200.0, 640.0), -Vec2::Y), half_length: 60.0 };
    let links = [PortalLink::new(mouth, exit, 0), PortalLink::new(exit, mouth, 1)];
    let far = Block::solid("far ceiling", Vec2::new(1100.0, 600.0), Vec2::new(200.0, 40.0));
    let blocks = vec![far, platform()];
    let admission = MotionAdmission { links: &links, first: None, budget: PieceBudget::COMPOSED,
        convention: MapConvention::Reflection, span: Default::default() };
    let across = conflict(blocks.clone(), Some(admission)).expect("the far-side solid pinches the body");
    assert_eq!(across.axis, local.axis);
    assert!((across.overconstraint() - local.overconstraint()).abs() < 1.0e-3, "{across:?} vs {local:?}");
    assert_eq!(across.driver(), Some(ConflictSide::Max), "{across:?}");
    let obstruction = across.obstruction();
    let (driver, closed) = obstruction.driver.clone().expect("a driver");
    assert!(named(&driver, "rising platform", &blocks), "{obstruction:?}");
    assert!(named(&obstruction.other, "far ceiling", &blocks), "the far-side solid: {obstruction:?}");
    assert!((closed - 4.0).abs() < 1.0e-3, "{closed}");
}
