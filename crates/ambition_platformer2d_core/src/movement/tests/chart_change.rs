//! Portal M2a (portals-are-moving-frames.md): the kernel interrupts a step
//! where the body's root crosses a pair and continues it in the exit's chart.

use super::super::*;
use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink, Reroot, StepSpan};
use crate::frame::{MapConvention, PortalAperture, PortalFrame, RootTransfer};
use crate::test_support::*;
use crate::{AbilitySet, Vec2, World};

const DT: f32 = 1.0 / 60.0;
const MOUTH: Vec2 = Vec2::new(100.0, 300.0);

fn open_air() -> World {
    World::new("open air", Vec2::splat(2000.0), Vec2::ZERO, vec![])
}

/// A small body 1.5 px above the floor mouth, falling into it.
fn falling_body() -> crate::body_clusters::BodyClusterScratch {
    let mut body = crate::body_clusters::BodyClusterScratch::new_with_abilities(MOUTH - Vec2::new(0.0, 1.5), AbilitySet::default());
    body.kinematics.canonical_size = Vec2::splat(16.0);
    body.kinematics.vel = Vec2::new(0.0, 300.0);
    body
}

fn step(body: &mut crate::body_clusters::BodyClusterScratch, world: &World, admission: Option<MotionAdmission<'_>>, dt: f32) -> MotionStepResult {
    let (model, mut clusters) = body.parts();
    let result = step_motion(model, &mut clusters, MotionStepContext {
        admission, world, input: InputState::default(), frame: TEST_TUNING.frame(),
        facing_intent: 0.0, dt, contact: BodyContactField::NONE,
        pose_owned_externally: false, recovery_commitment_outstanding: false,
    });
    assert!(result.events.motion_refusal.is_none(), "{:?}", result.events.motion_refusal);
    result
}

fn admitted(links: &[PortalLink]) -> MotionAdmission<'_> {
    MotionAdmission { links, first: None, budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection,
        span: StepSpan { window: [0.0, 1.0], reroot: Some(Reroot { closed: &[], min_exit_speed: 0.0 }) } }
}

fn pair(exit: PortalAperture) -> [PortalLink; 2] {
    let entry = PortalAperture { frame: PortalFrame::fixed(MOUTH, -Vec2::Y), half_length: 40.0 };
    [PortalLink::new(entry, exit, 0), PortalLink::new(exit, entry, 1)]
}

/// Through a quarter turn (a floor mouth to a wall exit facing +x), a body
/// that crosses early in the tick ends where the step up to the crossing,
/// the map at the crossing point and the rest of the tick stepped in the exit
/// chart put it. The rest of the tick's gravity pulls it DOWN there: the
/// whole step integrated before the map turned it sideways, so the body left
/// with no fall at all (the derived 53 r px/s error).
#[test]
fn a_crossing_through_a_turned_pair_falls_under_the_exit_sides_gravity() {
    let links = pair(PortalAperture { frame: PortalFrame::fixed(Vec2::new(600.0, 100.0), Vec2::X), half_length: 40.0 });
    let world = open_air();
    let mut body = falling_body();
    let (start, start_vel) = (body.kinematics.pos, body.kinematics.vel);
    let result = step(&mut body, &world, Some(admitted(&links)), DT);
    let [change] = result.events.chart_changes.as_slice() else {
        panic!("one chart change: {:?}", result.events.chart_changes)
    };
    let at = change.world_time as f32;
    assert!(at > 0.05 && at < 0.5, "the root crosses early in the tick: {at}");
    assert_eq!(change.link.key, 0);

    // The same law, stepped by hand: up to the crossing in open air...
    let mut alone = falling_body();
    step(&mut alone, &world, None, DT * at);
    assert_eq!(change.transfer.entry_vel, alone.kinematics.vel, "the velocity at the crossing is the policy's by then");
    // ...through the pair at the crossing point...
    let transfer = RootTransfer::through(&change.pose.enter, &change.pose.exit, MapConvention::Reflection,
        change.point, alone.kinematics.vel, alone.kinematics.material, 0.0).unwrap();
    alone.kinematics.pos = transfer.exit_pos;
    alone.kinematics.vel = transfer.exit_vel;
    alone.kinematics.material = alone.kinematics.material.carried(transfer.map).unwrap();
    // What comes out along the run is carried by the world, not run by the
    // controller, as generic transit has it.
    let side = TEST_TUNING.frame().basis().side;
    alone.flight.carried_run = transfer.carried_run(side, side, alone.flight.carried_run).unwrap();
    // ...and the rest of the tick in the exit's chart.
    step(&mut alone, &world, None, DT - DT * at);
    assert!((body.kinematics.pos - alone.kinematics.pos).length() < 1e-3,
        "ends where the continuation puts it: {:?} vs {:?}", body.kinematics.pos, alone.kinematics.pos);
    assert!((body.kinematics.vel - alone.kinematics.vel).length() < 1e-3,
        "{:?} vs {:?}", body.kinematics.vel, alone.kinematics.vel);

    let g = TEST_TUNING.base.gravity;
    let fall = g * (DT - DT * at);
    assert!((body.kinematics.vel.y - fall).abs() < 1e-2,
        "the rest of the tick falls down in the exit chart: {:?}, expected vy {fall}", body.kinematics.vel);
    assert!(body.kinematics.vel.x > start_vel.y, "and the fall into the floor comes out of the wall: {:?}", body.kinematics.vel);
    assert!(body.kinematics.pos.x > 600.0 && (body.kinematics.pos.y - 100.0).abs() < 8.0,
        "out of the wall exit: from {start:?} to {:?}", body.kinematics.pos);
}

/// Control: an aligned pair (a floor mouth to a ceiling exit, so down stays
/// down) gives what mapping the whole step's end gave: the continuation
/// changes nothing where the map does not turn the policy.
#[test]
fn an_aligned_pair_ends_where_the_whole_step_mapped_ends() {
    let exit = PortalAperture { frame: PortalFrame::fixed(Vec2::new(600.0, 100.0), Vec2::Y), half_length: 40.0 };
    let links = pair(exit);
    let world = open_air();
    let mut body = falling_body();
    let result = step(&mut body, &world, Some(admitted(&links)), DT);
    assert_eq!(result.events.chart_changes.len(), 1, "{:?}", result.events.chart_changes);

    let mut whole = falling_body();
    step(&mut whole, &world, None, DT);
    let map = links[0].snapshot_at(1.0).unwrap().map(MapConvention::Reflection).unwrap();
    let mapped = map.point(whole.kinematics.pos).unwrap();
    assert!((body.kinematics.pos - mapped).length() < 1e-3, "{:?} vs {mapped:?}", body.kinematics.pos);
    assert!((body.kinematics.vel - map.vector(whole.kinematics.vel).unwrap()).length() < 1e-3,
        "{:?} vs {:?}", body.kinematics.vel, whole.kinematics.vel);
}

/// Without the kernel's charge to move the root, the step is the old one:
/// integrated whole in the chart it started in, crossing nothing.
#[test]
fn a_step_whose_transit_moves_the_root_does_not_cross_in_the_kernel() {
    let links = pair(PortalAperture { frame: PortalFrame::fixed(Vec2::new(600.0, 100.0), Vec2::X), half_length: 40.0 });
    let world = open_air();
    let mut body = falling_body();
    let admission = MotionAdmission { span: StepSpan::WHOLE, ..admitted(&links) };
    let result = step(&mut body, &world, Some(admission), DT);
    assert!(result.events.chart_changes.is_empty());
    assert!((body.kinematics.pos.x - MOUTH.x).abs() < 1e-3 && body.kinematics.pos.y > MOUTH.y,
        "the root went on past the mouth in its own chart: {:?}", body.kinematics.pos);
}

/// Portal M2b: a body that goes in and out of two pairs in one tick crosses
/// both in its step, each where it reaches it. Pairs A (a mouth at x 0 to an
/// exit at x 100) and B (x 150 to x 300), side by side; a body from x -10 at
/// 80 px a tick crosses A a tenth of the way in and B as it reaches x 150 in
/// A's exit chart, and ends 20 px past B's exit. Control: with A admitted
/// alone, it crosses A and goes on 70 px past its exit.
#[test]
fn a_body_crosses_two_pairs_in_one_step() {
    let mouth = |at: Vec2, normal: Vec2| PortalAperture { frame: PortalFrame::fixed(at, normal), half_length: 40.0 };
    let a = [mouth(Vec2::ZERO, Vec2::NEG_X), mouth(Vec2::new(100.0, 0.0), Vec2::X)];
    let b = [mouth(Vec2::new(150.0, 0.0), Vec2::NEG_X), mouth(Vec2::new(300.0, 0.0), Vec2::X)];
    let links = [PortalLink::new(a[0], a[1], 0), PortalLink::new(a[1], a[0], 1),
        PortalLink::new(b[0], b[1], 2), PortalLink::new(b[1], b[0], 3)];
    let world = open_air();
    let run = |links: &[PortalLink]| {
        let mut body = crate::body_clusters::BodyClusterScratch::new_with_abilities(Vec2::new(-10.0, 0.0), AbilitySet::default());
        body.kinematics.canonical_size = Vec2::splat(16.0);
        body.kinematics.vel = Vec2::new(80.0 * 60.0, 0.0);
        // Run the world gave it, not the controller's: no air control drags it.
        body.flight.carried_run = body.kinematics.vel.x;
        let (model, mut clusters) = body.parts();
        let result = step_motion(model, &mut clusters, MotionStepContext {
            admission: Some(admitted(links)), world: &world,
            input: InputState::default(), frame: crate::MotionFrame::from_direction(Vec2::Y, 0.0),
            facing_intent: 0.0, dt: DT, contact: BodyContactField::NONE,
            pose_owned_externally: false, recovery_commitment_outstanding: false,
        });
        assert!(result.events.motion_refusal.is_none(), "{:?}", result.events.motion_refusal);
        (body.kinematics.pos, result.events.chart_changes.iter().map(|c| (c.link.key, c.world_time)).collect::<Vec<_>>())
    };
    let (end, changes) = run(&links);
    assert_eq!(changes.iter().map(|(key, _)| *key).collect::<Vec<_>>(), vec![0, 2], "A then B: {changes:?}");
    assert!((changes[0].1 - 0.125).abs() < 1e-6 && (changes[1].1 - 0.75).abs() < 1e-6, "{changes:?}");
    assert!(end.distance(Vec2::new(320.0, 0.0)) < 1e-2, "ends 20 px past B's exit: {end:?}");
    let (end, changes) = run(&links[..2]);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!(end.distance(Vec2::new(170.0, 0.0)) < 1e-2, "A alone: 70 px past its exit: {end:?}");
}
