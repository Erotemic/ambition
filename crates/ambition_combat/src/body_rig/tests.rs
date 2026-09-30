use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_characters::actor::body_rig::{
    BodyRigDefinition, JointPose, RigAttachment, RigClip, RigJoint, HAND_NEAR,
};

use super::*;

fn pose(x: f32, y: f32) -> JointPose {
    JointPose {
        translation: (x, y),
        rotation: 0.0,
        scale: (1.0, 1.0),
    }
}

/// One arm joint whose hand is at a different height in every clip, so the
/// clip a body resolves can be read off its hand.
fn rig() -> Arc<PreparedBodyRig> {
    let clip = |ys: &[f32], looping: bool| RigClip {
        looping,
        frame_duration_s: 0.1,
        frames: ys.iter().map(|&y| vec![pose(0.0, y)]).collect(),
    };
    Arc::new(
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
                ("idle".to_string(), clip(&[-10.0, -11.0], true)),
                ("jump".to_string(), clip(&[-20.0], false)),
                ("slash".to_string(), clip(&[-30.0, -31.0, -32.0], false)),
            ]),
        }
        .prepare()
        .expect("valid rig"),
    )
}

fn binding(clip: &str) -> ambition_entity_catalog::ClipBinding {
    ambition_entity_catalog::ClipBinding {
        clip: clip.to_string(),
        fallbacks: Vec::new(),
    }
}

#[test]
fn a_playing_move_outranks_the_body_pose_and_is_slaved_to_its_progress() {
    let rig = rig();
    let slash = binding("slash");
    assert_eq!(
        select_rig_frame(&rig, Some((&slash, 0.5)), Some(("airborne", 0.0))),
        Some(("slash", 1))
    );
    // A move whose clip the rig does not have falls through to the pose.
    let taunt = binding("taunt");
    assert_eq!(
        select_rig_frame(&rig, Some((&taunt, 0.5)), Some(("airborne", 0.0))),
        Some(("jump", 0))
    );
    // A pose whose clip the rig does not have falls back to idle, on its clock.
    assert_eq!(select_rig_frame(&rig, None, Some(("crouch", 0.15))), Some(("idle", 1)));
}

fn app_with_body(clock: BodyPoseClock) -> (App, Entity) {
    let mut app = App::new();
    app.add_systems(Update, resolve_body_rig_poses);
    let body = app
        .world_mut()
        .spawn((BodyRig(rig()), clock, BodyRigPose::default()))
        .id();
    (app, body)
}

fn hand(app: &App, body: Entity) -> Vec2 {
    let world = app.world();
    let pose = world.get::<BodyRigPose>(body).unwrap();
    pose.attachment(&world.get::<BodyRig>(body).unwrap().0, HAND_NEAR)
        .expect("resolved hand")
}

#[test]
fn a_headless_body_resolves_its_hand_from_the_pose_clock() {
    let (mut app, body) = app_with_body(BodyPoseClock::new("airborne", 0.0));
    app.update();
    assert_eq!(hand(&app, body), Vec2::new(3.0, -20.0));
    app.world_mut().get_mut::<BodyPoseClock>(body).unwrap().pose = "idle".to_string();
    app.world_mut().get_mut::<BodyPoseClock>(body).unwrap().elapsed_s = 0.1;
    app.update();
    assert_eq!(hand(&app, body), Vec2::new(3.0, -11.0));
}

#[test]
fn a_discarded_pose_is_rebuilt_identically_from_the_same_clocks() {
    // A rollback restore brings back the clocks and drops derived state. One
    // resolver pass must rebuild the pose exactly.
    let (mut app, body) = app_with_body(BodyPoseClock::new("idle", 0.1));
    app.update();
    let resolved = app.world().get::<BodyRigPose>(body).unwrap().clone();
    *app.world_mut().get_mut::<BodyRigPose>(body).unwrap() = BodyRigPose::default();
    app.update();
    assert_eq!(app.world().get::<BodyRigPose>(body).unwrap(), &resolved);
}

#[test]
fn a_body_facing_left_mirrors_its_rig_about_its_feet() {
    let down = Vec2::Y;
    let hand = Vec2::new(3.0, -20.0);
    assert_eq!(BodyRigPose::to_body(hand, 1.0, down), Vec2::new(3.0, -20.0));
    assert_eq!(BodyRigPose::to_body(hand, -1.0, down), Vec2::new(-3.0, -20.0));
    // Under reversed gravity the head side of the rig points toward -down.
    assert_eq!(BodyRigPose::to_body(hand, 1.0, -down).y, 20.0);
}

#[test]
fn hurt_volumes_are_placed_from_the_feet_as_bounds_of_their_parts() {
    use ambition_characters::actor::body_rig::{RigHurtPart, RigShape};
    let mut definition = BodyRigDefinition {
        joints: vec![RigJoint {
            name: "arm".to_string(),
            parent: None,
        }],
        attachments: Vec::new(),
        hurt_parts: vec![RigHurtPart {
            name: "fist".to_string(),
            joint: "arm".to_string(),
            shape: RigShape::Circle {
                center: (0.0, 0.0),
                radius: 2.0,
            },
        }],
        clips: BTreeMap::from([(
            "idle".to_string(),
            RigClip {
                looping: true,
                frame_duration_s: 0.1,
                frames: vec![vec![pose(4.0, -10.0)]],
            },
        )]),
    };
    let rig = definition.clone().prepare().unwrap();
    let mut rig_pose = BodyRigPose::default();
    assert_eq!(rig_pose.hurt_volumes(&rig, 8.0), None, "an unresolved pose has no hurt shape");
    rig.solve("idle", 0, &mut rig_pose.joints);
    let volumes = rig_pose.hurt_volumes(&rig, 8.0).expect("one part");
    // The fist is 10 above the feet, and the feet are 8 below the centre.
    assert_eq!(
        volumes[0].shape,
        ambition_entity_catalog::VolumeShape::Rect {
            offset: (4.0, -2.0),
            half_extents: (2.0, 2.0),
        }
    );
    definition.hurt_parts.clear();
    let bare = definition.prepare().unwrap();
    assert_eq!(rig_pose.hurt_volumes(&bare, 8.0), None);
}
