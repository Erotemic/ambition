use std::collections::BTreeMap;

use ambition_characters::actor::body_rig::{
    BodyRigDefinition, JointPose, RigAttachment, RigClip, RigJoint, HAND_NEAR,
};
use ambition_characters::actor::landmarks::{LandmarkClip, LandmarkFrame};

use super::*;

/// A table whose near hand is at a different x in every frame of every clip,
/// so the clip and frame an answer came from can be read off the answer.
fn table() -> BodyLandmarkTable {
    let clip = |xs: &[f32]| LandmarkClip {
        frame_duration_s: 0.1,
        frames: xs
            .iter()
            .map(|&x| {
                let mut frame = LandmarkFrame::default();
                frame[Landmark::HandNear.slot()] = Some(Vec2::new(x, -30.0));
                frame
            })
            .collect(),
    };
    BodyLandmarkTable {
        frame_height: 100.0,
        clips: BTreeMap::from([
            ("idle".to_string(), clip(&[10.0, 11.0])),
            ("pet".to_string(), clip(&[20.0, 21.0, 22.0, 23.0])),
            ("walk".to_string(), clip(&[40.0, 41.0, 42.0])),
        ]),
    }
}

/// One arm joint with the hand 3 ahead of it, at a different height per clip.
fn rig() -> PreparedBodyRig {
    let clip = |ys: &[f32]| RigClip {
        looping: true,
        frame_duration_s: 0.1,
        frames: ys
            .iter()
            .map(|&y| {
                vec![JointPose {
                    translation: (0.0, y),
                    rotation: 0.0,
                    scale: (1.0, 1.0),
                }]
            })
            .collect(),
    };
    BodyRigDefinition {
        joints: vec![RigJoint {
            name: "arm".to_string(),
            parent: None,
        }],
        attachments: vec![RigAttachment {
            name: HAND_NEAR.to_string(),
            joint: "arm".to_string(),
            offset: (3.0, 0.0),
        }],
        hurt_parts: Vec::new(),
        clips: BTreeMap::from([
            ("idle".to_string(), clip(&[-10.0])),
            ("pet".to_string(), clip(&[-50.0, -51.0, -52.0, -53.0])),
        ]),
    }
    .prepare()
    .expect("valid rig")
}

#[test]
fn a_package_answers_a_named_clip_at_its_phase_in_world_units() {
    let table = table();
    let pet = LandmarkPose::Clip { chain: &["pet", "idle"], phase: 0.5 };
    assert_eq!(
        package_landmark(Landmark::HandNear, pet, &table, 0.5, PoseClocks::default()),
        Some(Vec2::new(11.0, -15.0)),
        "frame 2 of 4, at half a world unit per pixel",
    );
    // No such point, and no such clip: None, never the origin.
    assert_eq!(package_landmark(Landmark::Head, pet, &table, 0.5, PoseClocks::default()), None);
    let wave = LandmarkPose::Clip { chain: &["wave"], phase: 0.5 };
    assert_eq!(package_landmark(Landmark::HandNear, wave, &table, 0.5, PoseClocks::default()), None);
    // The first clip of the chain the table has: the row the gesture is drawn from.
    let interact = LandmarkPose::Clip { chain: &["interact", "walk", "idle"], phase: 0.0 };
    assert_eq!(
        package_landmark(Landmark::HandNear, interact, &table, 1.0, PoseClocks::default()).map(|p| p.x),
        Some(40.0),
    );
}

#[test]
fn a_package_answers_this_tick_from_the_body_clocks() {
    let table = table();
    let ask = |clocks| package_landmark(Landmark::HandNear, LandmarkPose::ThisTick, &table, 1.0, clocks).map(|p| p.x);
    // A walking body: the walk clip, on the gait clock.
    let walking = PoseClocks {
        pose: Some(("idle", 9.0)),
        gait: Some((Gait::Walking, 0.25)),
        ..Default::default()
    };
    assert_eq!(ask(walking), Some(42.0));
    // A playing move outranks the pose, at the move's progress.
    let pet = ambition_entity_catalog::ClipBinding {
        clip: "pet".to_string(),
        fallbacks: Vec::new(),
    };
    let petting = PoseClocks {
        active_move: Some((&pet, 0.25)),
        ..walking
    };
    assert_eq!(ask(petting), Some(21.0));
    // A body with no clock stands in the first idle frame.
    assert_eq!(ask(PoseClocks::default()), Some(10.0));
}

#[test]
fn a_rig_answers_this_tick_from_its_resolved_pose_and_a_named_clip_by_solving_it() {
    let rig = rig();
    let mut resolved = BodyRigPose::default();
    rig.solve("idle", 0, &mut resolved.joints);
    resolved.attachments = vec![Vec2::new(3.0, -10.0)];
    assert_eq!(
        rig_landmark(Landmark::HandNear, LandmarkPose::ThisTick, &rig, Some(&resolved)),
        Some(Vec2::new(3.0, -10.0)),
    );
    // The body is idle, and the script asks where the hand will be in the pet.
    let pet = LandmarkPose::Clip { chain: &["pet", "idle"], phase: 0.5 };
    assert_eq!(
        rig_landmark(Landmark::HandNear, pet, &rig, Some(&resolved)),
        Some(Vec2::new(3.0, -52.0)),
    );
    // A rig with no such attachment, clip or resolved pose does not answer.
    assert_eq!(rig_landmark(Landmark::Head, pet, &rig, Some(&resolved)), None);
    let wave = LandmarkPose::Clip { chain: &["wave"], phase: 0.5 };
    assert_eq!(rig_landmark(Landmark::HandNear, wave, &rig, Some(&resolved)), None);
    assert_eq!(rig_landmark(Landmark::HandNear, LandmarkPose::ThisTick, &rig, None), None);
}

#[test]
fn the_scale_is_the_posed_bodys_own_or_its_quad_over_the_frame_height() {
    let posed = ambition_sprite_sheet::character::SpritePosedBody {
        target: "any".to_string(),
        world_per_pixel: 0.5,
    };
    let quad = crate::components::ActorRenderSize(Vec2::new(151.0, 126.0) * 0.75);
    assert_eq!(drawn_world_per_pixel(Some(&posed), Some(&quad), 126.0), Some(0.5));
    assert_eq!(drawn_world_per_pixel(None, Some(&quad), 126.0), Some(0.75));
    // A body that states neither has no scale: no guess from its box.
    assert_eq!(drawn_world_per_pixel(None, None, 126.0), None);
}

#[test]
fn the_feet_are_the_middle_of_the_face_toward_down() {
    let kin = ae::BodyKinematics {
        pos: Vec2::new(100.0, 200.0),
        size: Vec2::new(30.0, 48.0),
        ..Default::default()
    };
    assert_eq!(feet_of(&kin, Vec2::new(0.0, 1.0)), Vec2::new(100.0, 224.0));
    assert_eq!(feet_of(&kin, Vec2::new(0.0, -1.0)), Vec2::new(100.0, 176.0));
}
