use super::admit_body_righting;
use crate::{BodyKinematics, SweepSample, Vec2, World};
use crate::frame::{BodyMaterial, NumericFault, thread::RefusalReason};

fn room() -> World {
    World::new("righting", Vec2::splat(1000.0), Vec2::ZERO, vec![])
}

#[test]
fn an_admitted_turn_keeps_its_physical_pose_and_record_together() {
    for reflected in [false, true] {
        let mut axes = bevy_math::Mat2::from_angle(0.7);
        if reflected { axes.x_axis = -axes.x_axis; }
        let before = BodyKinematics { material: BodyMaterial::new(2.0, axes).unwrap(), ..Default::default() };
        let mut body = BodyKinematics { pos: Vec2::new(10.0, 0.0), ..before };
        let mut record = SweepSample { curr: body.pos, ..SweepSample::at_rest(before, Vec2::Y) };
        assert!(admit_body_righting(&mut body, &mut record, Vec2::Y, 0.2, None, 1.0, &room(), None, &[], None).unwrap());
        assert_eq!(body.material, before.material.turned(-0.2).unwrap());
        assert_eq!(body.material.reflected(), reflected);
        assert_eq!(body.canonical_size, before.canonical_size);
        assert_eq!(record.material, before.material);
        assert_eq!(record.turn.unwrap().radians, -0.2);
        let encoded = crate::snapshot::encode_state(&record);
        let restored = crate::snapshot::decode_state::<SweepSample>(&encoded).unwrap();
        assert_eq!(crate::snapshot::encode_state(&restored), encoded);
    }
}

#[test]
fn a_wall_between_clear_end_poses_blocks_the_turn_and_keeps_translation() {
    let world = World::new("turn clearance", Vec2::splat(100.0), Vec2::ZERO,
        vec![crate::Block::solid("tip", Vec2::new(-3.5, 3.0), Vec2::ONE)]);
    let before = BodyKinematics { canonical_size: Vec2::new(10.0, 1.0), ..Default::default() };
    let mut body = BodyKinematics { pos: Vec2::X, ..before };
    let mut record = SweepSample { curr: body.pos, ..SweepSample::at_rest(before, Vec2::Y) };
    assert!(!admit_body_righting(&mut body, &mut record, Vec2::X, std::f64::consts::FRAC_PI_2, None,
        1.0, &world, None, &[], None).unwrap());
    assert_eq!(body.pos, Vec2::X);
    assert_eq!(body.material, before.material);
    assert_eq!(record.turn, None);
    assert_eq!(record.curr, Vec2::X);
}

#[test]
fn an_invalid_turn_duration_refuses_without_changing_the_candidate() {
    let original = BodyKinematics { material: BodyMaterial::IDENTITY.turned(0.5).unwrap(), ..Default::default() };
    for (duration, fault) in [(0.0, NumericFault::InvalidFrame), (f64::NAN, NumericFault::NonFinite)] {
        let mut body = original;
        let original_record = SweepSample::at_rest(body, Vec2::Y);
        let mut record = original_record;
        let refusal = admit_body_righting(&mut body, &mut record, Vec2::Y, 0.2, None,
            duration, &room(), None, &[], None).unwrap_err();
        assert_eq!(refusal.reason, RefusalReason::Numeric(fault));
        assert_eq!(body, original);
        assert_eq!(record, original_record);
    }
}

#[test]
fn a_quarter_turn_stops_exactly_on_the_target_axis() {
    for target in [Vec2::X, -Vec2::X, -Vec2::Y, Vec2::Y] {
        let material = BodyMaterial::IDENTITY;
        let turn = material.righting_turn(target, std::f64::consts::PI).unwrap();
        let result = material.turned(turn).unwrap();
        assert_eq!(result.at(Vec2::ZERO).unwrap().direction_wide(bevy_math::DVec2::Y), target.as_dvec2());
        assert_eq!(result.righting_turn(target, std::f64::consts::PI).unwrap(), 0.0);
    }
}
