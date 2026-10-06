//! A transit moves the body and does not turn it.
//!
//! The record of a step states the box the step moved
//! ([`SweepSample::half`]). A transit collapses the record to a zero-length
//! sample at the arrival, and it has no frame to turn a box with. It wrote the
//! level half, so for one tick a reader of the record (a loading zone, an ECS
//! hazard) saw a body in turned gravity with the box of a body in normal
//! gravity.
//!
//! The record carries the DOWN of its step. A collapse keeps that DOWN and
//! turns the body's PRESENT size to it. The next step turns the body.

use super::super::*;
use super::hazard_footprint::{normal_gravity, room, sideways_gravity, BODY, STEP_HALF};
use crate::body_clusters::{BodyClusterScratch, SweepSample};
use crate::{AbilitySet, MotionFrame, Vec2};

const DT: f32 = 1.0 / 60.0;
const START: Vec2 = Vec2::new(1000.0, 1200.0);
const ARRIVAL: Vec2 = Vec2::new(2000.0, 600.0);

/// The halves of the record after a step, after a transit, and after the
/// next step at the arrival.
struct Halves {
    stepped: Vec2,
    collapsed: SweepSample,
    next: Vec2,
}

/// Step a body once, transit it, and step it again. `before_transit` runs
/// between the step and the transit.
fn step_transit_step(
    spec: MotionModelSpec,
    attached_to: Option<Vec2>,
    start: Vec2,
    frame: MotionFrame,
    before_transit: impl FnOnce(&mut crate::body_clusters::BodyClustersMut<'_>),
) -> Halves {
    let world = room(None);
    let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::default());
    scratch.kinematics.size = BODY;
    scratch.base_size.base_size = BODY;
    scratch.kinematics.facing = -1.0;
    let mut sample = SweepSample::default();
    let (model, mut clusters) = scratch.parts();
    switch_motion_model(model, spec);
    if let (MotionModel::AdhesiveCrawler(crawler), Some(normal)) = (&mut *model, attached_to) {
        crawler.state = CrawlerState::attached(normal);
    }
    clusters.sweep = Some(&mut sample);
    let context = MotionStepContext {
        world: &world,
        input: InputState::default(),
        frame,
        facing_intent: -1.0,
        dt: DT,
        contact: body_contact::BodyContactField::NONE,
        pose_owned_externally: false,
        recovery_commitment_outstanding: false,
    };
    step_motion(model, &mut clusters, context);
    let stepped = clusters.sweep.as_deref().expect("the body has a record").half;

    before_transit(&mut clusters);
    transit_body(model, &mut clusters, ARRIVAL, TransitVelocity::Zero);
    let collapsed = *clusters.sweep.as_deref().expect("the body has a record");
    assert_eq!(collapsed.prev, ARRIVAL, "premise: a transit collapses the record at the arrival");
    assert_eq!(collapsed.curr, ARRIVAL);

    step_motion(model, &mut clusters, context);
    let next = clusters.sweep.as_deref().expect("the body has a record").half;
    Halves { stepped, collapsed, next }
}

fn axis() -> MotionModelSpec {
    MotionModelSpec::AxisSwept(AxisSweptParams::default())
}

#[track_caller]
fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 1.0e-3
}

#[test]
fn a_transit_in_sideways_gravity_keeps_the_box_of_the_body() {
    let halves = step_transit_step(axis(), None, START, sideways_gravity(), |_| {});
    assert_eq!(halves.stepped, STEP_HALF, "premise: the step moved the box that lies along the gravity");
    assert_eq!(halves.next, STEP_HALF, "premise: the next step moves the same box");
    assert_eq!(
        halves.collapsed.half, STEP_HALF,
        "the record at the arrival must state the box of the body, not its level box"
    );
}

#[test]
fn a_transit_in_oblique_gravity_keeps_the_box_of_the_body() {
    let frame = MotionFrame::from_acceleration(Vec2::new(0.6, 0.8) * GRAVITY).expect("gravity is not zero");
    let halves = step_transit_step(axis(), None, START, frame, |_| {});
    assert!(close(halves.stepped, Vec2::new(25.8, 23.2)), "premise: the bound of the turned box: {:?}", halves.stepped);
    assert!(close(halves.next, halves.stepped), "premise: the next step moves the same box");
    assert!(
        close(halves.collapsed.half, halves.stepped),
        "the record at the arrival must state the box of the body: {:?}",
        halves.collapsed.half
    );
}

/// A reset gives the body a new size and then transits it. The record states
/// the PRESENT size, turned as the body was.
#[test]
fn a_transit_after_a_change_of_size_turns_the_new_size() {
    let resized = Vec2::new(30.0, 60.0);
    let halves = step_transit_step(axis(), None, START, sideways_gravity(), |clusters| {
        clusters.kinematics.size = resized;
    });
    assert_eq!(halves.next, Vec2::new(30.0, 15.0), "premise: the next step moves the new size along the gravity");
    assert_eq!(
        halves.collapsed.half, halves.next,
        "the record at the arrival must state the new size, turned as the body is"
    );
}

/// The one case where the record at the arrival is not the box of the next
/// step. A crawler on a wall is detached by a transit, and it arrives with
/// the turn it had. The next step, in the frame, turns it level.
#[test]
fn a_crawler_that_transits_off_a_wall_arrives_with_the_turn_it_had() {
    let on_wall = Vec2::new(48.0 + STEP_HALF.x, 1000.0);
    let halves = step_transit_step(
        MotionModelSpec::AdhesiveCrawler(CrawlerParams::default()),
        Some(Vec2::new(1.0, 0.0)),
        on_wall,
        normal_gravity(),
        |_| {},
    );
    assert_eq!(halves.stepped, STEP_HALF, "premise: the slug on the wall lies along the wall");
    assert_eq!(halves.next, BODY * 0.5, "premise: detached, the next step moves the level box");
    assert_eq!(halves.collapsed.half, STEP_HALF, "a transit does not turn the body");
}

#[test]
fn a_transit_in_normal_gravity_keeps_the_level_box() {
    let halves = step_transit_step(axis(), None, START, normal_gravity(), |_| {});
    assert_eq!(halves.stepped, BODY * 0.5);
    assert_eq!(halves.collapsed.half, BODY * 0.5);
    assert_eq!(halves.next, BODY * 0.5);
}

/// A body that no step has moved has no turn. Its record at an arrival states
/// the level box.
#[test]
fn a_body_no_step_has_moved_arrives_with_its_level_box() {
    let mut scratch = BodyClusterScratch::new_with_abilities(START, AbilitySet::default());
    scratch.kinematics.size = BODY;
    let mut sample = SweepSample::default();
    let (model, mut clusters) = scratch.parts();
    clusters.sweep = Some(&mut sample);
    transit_body(model, &mut clusters, ARRIVAL, TransitVelocity::Zero);
    assert_eq!(clusters.sweep.as_deref().expect("the body has a record").half, BODY * 0.5);
}

/// The rollback snapshot of the record holds its DOWN, so a restored record
/// collapses to the same box, and two peers that disagree on it differ.
#[test]
fn the_snapshot_of_a_record_holds_its_down() {
    use crate::snapshot::{Reader, SnapshotState};
    let turned = SweepSample {
        down: Vec2::new(1.0, 0.0),
        ..SweepSample::default()
    };
    let mut bytes = Vec::new();
    turned.encode(&mut bytes);
    let mut level = Vec::new();
    SweepSample::default().encode(&mut level);
    assert_ne!(bytes, level, "two records that differ in their DOWN only must not encode alike");
    assert_eq!(SweepSample::decode(&mut Reader::new(&bytes)), Some(turned));
}
