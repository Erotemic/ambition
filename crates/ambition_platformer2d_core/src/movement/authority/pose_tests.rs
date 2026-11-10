use super::*;
use crate::{BodyKinematics, LinearTravel};
use crate::frame::{ApertureStep, MapConvention, NumericFault, PortalAperture, PortalFrame};
use crate::frame::thread::{MotionAdmission, PieceBudget, PortalLink, RefusalReason};

fn body() -> (BodyKinematics, SweepSample) {
    let body = BodyKinematics { pos: Vec2::new(-30.0,0.0), vel: Vec2::X * 3.0,
        canonical_size: Vec2::splat(16.0), ..Default::default() };
    let sample = SweepSample { prev: body.pos - Vec2::X * 5.0, curr: body.pos,
        vel: body.vel, half: body.size()*0.5, down: Vec2::Y, ..Default::default() };
    (body,sample)
}

fn links(scale: f32) -> [PortalLink;1] {
    [PortalLink::new(
        PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::X),half_length:80.0 },
        PortalAperture { frame:PortalFrame::fixed(Vec2::new(500.0,0.0),Vec2::X),half_length:80.0*scale },7)]
}

fn admission(links: &[PortalLink], hops: u8) -> MotionAdmission<'_> {
    MotionAdmission { links,first:None,budget:PieceBudget { max_hops:hops,..PieceBudget::COMPOSED },
        convention:MapConvention::Reflection }
}

#[test]
fn a_refused_pin_retains_the_body_and_its_prior_accepted_motion() {
    let (mut body,mut sample) = body();
    let before = body;
    let history = sample;
    let desired = BodyKinematics { pos: Vec2::new(-2.0,0.0),vel:Vec2::X*200.0,
        canonical_size: Vec2::splat(32.0),facing:-1.0,..body };
    let links = links(1.0);
    let refusal = constrain_body_pose(&mut body,Some(&mut sample),desired,Some(admission(&links,0)),&[]).unwrap_err();
    assert_eq!(refusal.reason,RefusalReason::HopLimit);
    assert_eq!(body,before);
    assert_eq!(sample,history,"an earlier accepted operation remains recorded");
    constrain_body_pose(&mut body,Some(&mut sample),desired,Some(admission(&links,4)),&[]).unwrap();
    assert_eq!(body,desired);
    assert_eq!(sample.prev,history.prev);
    assert_eq!(sample.curr,desired.pos);
    assert_eq!(sample.vel,desired.vel);
    assert!(sample.ending_at(body.pos).is_some());
}

#[test]
fn a_pin_refuses_carried_force_overflow_before_it_changes_any_physical_state() {
    let (mut body,mut sample) = body();
    let before = body;
    let history = sample;
    let desired = BodyKinematics { pos:Vec2::new(-2.0,0.0),vel:Vec2::ZERO,..body };
    let links = links(2.0);
    let refusal = constrain_body_pose(&mut body,Some(&mut sample),desired,Some(admission(&links,4)),
        &[Vec2::X * (f32::MAX*0.75)]).unwrap_err();
    assert_eq!(refusal.reason,RefusalReason::Numeric(NumericFault::Overflow));
    assert_eq!(body,before);
    assert_eq!(sample,history);
}

#[test]
fn a_new_pin_replaces_a_prior_prefix_with_its_own_complete_record() {
    let (mut body,mut sample) = body();
    sample.prev = body.pos - Vec2::X * 10.0;
    sample.travel = LinearTravel::Prefix { delta:Vec2::X*20.0,until:0.5 };
    let desired = BodyKinematics { pos:body.pos + Vec2::X*10.0,vel:Vec2::X*50.0,..body };
    constrain_body_pose(&mut body,Some(&mut sample),desired,None,&[]).unwrap();
    assert_eq!(sample.travel,LinearTravel::Complete);
    assert_eq!(sample.time,[0.0,1.0]);
    assert!(sample.ending_at(body.pos).is_some());
}

#[test]
fn invalid_recorded_travel_is_refused_instead_of_becoming_a_placement() {
    let (mut body,mut sample) = body();
    sample.travel = LinearTravel::Prefix { delta:Vec2::X*20.0,until:0.5 };
    let before = body;
    let history = sample;
    let desired = BodyKinematics { pos:Vec2::new(-2.0,0.0),..body };
    let refusal = constrain_body_pose(&mut body,Some(&mut sample),desired,None,&[]).unwrap_err();
    assert_eq!(refusal.reason,RefusalReason::Numeric(NumericFault::InvalidFrame));
    assert_eq!(body,before);
    assert_eq!(sample,history);
}

fn turning_links() -> [PortalLink;1] {
    let mut links = links(1.0);
    links[0].enter.frame.normal = -Vec2::Y;
    links[0].enter_step = ApertureStep::ending_at(links[0].enter.frame,Vec2::ZERO,std::f32::consts::TAU);
    links
}

#[test]
fn a_birth_pin_does_not_exist_during_earlier_aperture_motion() {
    let mut body = BodyKinematics { pos:Vec2::new(30.0,-30.0),canonical_size:Vec2::splat(4.0),..Default::default() };
    let mut sample = SweepSample::at_tick_end(body,Vec2::Y);
    let desired = BodyKinematics { pos:Vec2::new(1.0,-30.0),..body };
    let links = turning_links();
    constrain_body_pose(&mut body,Some(&mut sample),desired,Some(admission(&links,0)),&[]).unwrap();
    assert_eq!(sample.time,[1.0,1.0]);
    assert_eq!(sample.prev,desired.pos);
    assert_eq!(sample.curr,desired.pos);
}

#[test]
fn an_instant_pin_retains_its_actual_world_time() {
    let mut body = BodyKinematics { pos:Vec2::new(1.0,-30.0),canonical_size:Vec2::splat(4.0),..Default::default() };
    let mut sample = SweepSample { time:[0.25,0.25],..SweepSample::at_tick_end(body,Vec2::Y) };
    let before = body;
    let history = sample;
    let links = turning_links();
    let refusal = constrain_body_pose(&mut body,Some(&mut sample),before,Some(admission(&links,0)),&[]).unwrap_err();
    assert_eq!(refusal.reason,RefusalReason::HopLimit);
    assert_eq!(body,before);
    assert_eq!(sample,history);
}
