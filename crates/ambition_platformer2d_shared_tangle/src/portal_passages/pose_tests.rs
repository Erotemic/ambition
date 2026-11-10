use super::*;
use bevy::prelude::*;
use ambition_platformer2d_core::{BodyKinematics, BodyFlightState, SweepSample};
use ambition_platformer2d_core::frame::PortalFrame;
use crate::frame_env::CarriedAcceleration;

fn pin(poses: PoseAdmission, mut bodies: Query<(Entity,&mut BodyKinematics,&mut SweepSample)>) {
    for (entity,mut body,mut sample) in &mut bodies {
        let desired = BodyKinematics { pos:Vec2::new(-2.0,0.0),vel:Vec2::X*100.0,
            canonical_size:Vec2::splat(32.0),facing:-1.0,..*body };
        let _ = poses.constrain(entity,&mut body,Some(&mut sample),desired);
    }
}

fn app(scale: f32) -> (App,[Entity;2],BodyKinematics,SweepSample) {
    let mut app = App::new();
    app.add_systems(Update,pin);
    let body = BodyKinematics { pos:Vec2::new(-30.0,0.0),canonical_size:Vec2::splat(16.0),..Default::default() };
    let sample = SweepSample { prev:body.pos-Vec2::X*5.0,curr:body.pos,down:Vec2::Y,..Default::default() };
    let bodies = [app.world_mut().spawn((body,sample)).id(),app.world_mut().spawn((body,sample)).id()];
    let mut passages = PortalPassages::default();
    passages.add_pair(None,(
        PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::X),half_length:80.0 },
        PortalAperture { frame:PortalFrame::fixed(Vec2::new(500.0,0.0),Vec2::X),half_length:80.0*scale }));
    app.insert_resource(passages);
    (app,bodies,body,sample)
}


#[test]
fn a_refused_body_policy_does_not_stop_another_owners_pose() {
    let (mut app,[refused,accepted],before,history) = app(1.0);
    app.world_mut().resource_mut::<PortalPassages>().set_body_budget(refused,
        PieceBudget { max_hops:0,..PieceBudget::COMPOSED });
    app.update();
    assert_eq!(*app.world().get::<BodyKinematics>(refused).unwrap(),before);
    assert_eq!(*app.world().get::<SweepSample>(refused).unwrap(),history);
    let moved = app.world().get::<BodyKinematics>(accepted).unwrap();
    assert_eq!(moved.pos,Vec2::new(-2.0,0.0));
    assert_eq!(moved.canonical_size,Vec2::splat(32.0));
    assert_eq!(moved.facing,-1.0);
}

#[test]
fn pose_admission_reads_the_bodys_carried_acceleration() {
    let (mut app,[refused,accepted],before,history) = app(2.0);
    app.world_mut().entity_mut(refused).insert(CarriedAcceleration(Vec2::X*(f32::MAX*0.75)));
    app.update();
    assert_eq!(*app.world().get::<BodyKinematics>(refused).unwrap(),before);
    assert_eq!(*app.world().get::<SweepSample>(refused).unwrap(),history);
    assert_eq!(app.world().get::<BodyKinematics>(accepted).unwrap().pos,Vec2::new(-2.0,0.0));
}

#[test]
fn far_parts_do_not_get_separate_controller_velocity_state() {
    let (mut app,[body,other],_,_) = app(2.0);
    let carried_run = f32::MAX*0.75;
    app.world_mut().entity_mut(body).insert(BodyFlightState { carried_run,..Default::default() });
    app.update();
    assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos,Vec2::new(-2.0,0.0));
    assert_eq!(app.world().get::<BodyFlightState>(body).unwrap().carried_run,carried_run);
    assert_eq!(app.world().get::<BodyKinematics>(other).unwrap().pos,Vec2::new(-2.0,0.0));
}
