use super::*;

fn pose(x: f32, y: f32, rotation: f32) -> JointPose {
    JointPose {
        translation: (x, y),
        rotation,
        scale: (1.0, 1.0),
    }
}

fn joint(name: &str, parent: Option<&str>) -> RigJoint {
    RigJoint {
        name: name.to_string(),
        parent: parent.map(str::to_string),
    }
}

/// An arm whose CHILD is declared before its parent, with a hand at the tip.
fn arm() -> BodyRigDefinition {
    BodyRigDefinition {
        joints: vec![joint("forearm", Some("upper")), joint("upper", None)],
        attachments: vec![RigAttachment {
            name: HAND_NEAR.to_string(),
            joint: "forearm".to_string(),
            offset: (0.0, 5.0),
        }],
        hurt_parts: vec![RigHurtPart {
            name: "forearm".to_string(),
            joint: "forearm".to_string(),
            shape: RigShape::Capsule {
                a: (0.0, 0.0),
                b: (0.0, 5.0),
                radius: 1.0,
            },
        }],
        clips: BTreeMap::from([(
            "idle".to_string(),
            RigClip {
                looping: true,
                frame_duration_s: 0.1,
                frames: vec![
                    // forearm hangs 10 below the shoulder; the shoulder is at (0, -20).
                    vec![pose(0.0, 10.0, 0.0), pose(0.0, -20.0, 0.0)],
                    // the shoulder turns a quarter clockwise: the arm points -x.
                    vec![
                        pose(0.0, 10.0, 0.0),
                        pose(0.0, -20.0, std::f32::consts::FRAC_PI_2),
                    ],
                ],
            },
        )]),
    }
}

fn hand(rig: &PreparedBodyRig, frame: usize) -> Vec2 {
    let mut joints = Vec::new();
    assert!(rig.solve("idle", frame, &mut joints));
    let attachment = &rig.attachments()[rig.attachment_index(HAND_NEAR).unwrap()];
    joints[usize::from(attachment.joint)].transform_point2(attachment.offset)
}

#[test]
fn a_child_declared_before_its_parent_is_solved_through_it() {
    let rig = arm().prepare().expect("valid rig");
    assert_eq!(rig.joint_names(), ["upper", "forearm"]);
    assert!((hand(&rig, 0) - Vec2::new(0.0, -5.0)).length() < 1e-4);
    // Clockwise on a y-down screen takes +y (down) to -x.
    assert!((hand(&rig, 1) - Vec2::new(-15.0, -20.0)).length() < 1e-4);
}

#[test]
fn scaling_moves_lengths_and_leaves_angles() {
    let rig = arm().scaled(2.0).prepare().expect("valid rig");
    assert!((hand(&rig, 0) - Vec2::new(0.0, -10.0)).length() < 1e-4);
    assert!((hand(&rig, 1) - Vec2::new(-30.0, -40.0)).length() < 1e-4);
}

#[test]
fn preparation_refuses_each_malformed_rig() {
    let refused = |edit: fn(&mut BodyRigDefinition)| {
        let mut rig = arm();
        edit(&mut rig);
        rig.prepare().expect_err("malformed rig was admitted")
    };
    assert!(matches!(
        refused(|rig| rig.joints.push(joint("upper", None))),
        BodyRigError::DuplicateJoint(_)
    ));
    assert!(matches!(
        refused(|rig| rig.joints[1].parent = Some("pelvis".to_string())),
        BodyRigError::UnknownParent { .. }
    ));
    assert!(matches!(
        refused(|rig| rig.joints[1].parent = Some("forearm".to_string())),
        BodyRigError::ParentCycle(_)
    ));
    assert!(matches!(
        refused(|rig| rig.attachments[0].joint = "wrist".to_string()),
        BodyRigError::UnknownJoint { what: "attachment", .. }
    ));
    assert!(matches!(
        refused(|rig| rig.hurt_parts[0].joint = "wrist".to_string()),
        BodyRigError::UnknownJoint { what: "hurt part", .. }
    ));
    assert!(matches!(
        refused(|rig| rig.attachments[0].offset.0 = f32::NAN),
        BodyRigError::NonFinite(_)
    ));
    assert!(matches!(
        refused(|rig| rig.clips.get_mut("idle").unwrap().frames[1][0].rotation = f32::INFINITY),
        BodyRigError::NonFinite(_)
    ));
    assert!(matches!(
        refused(|rig| rig.clips.get_mut("idle").unwrap().frames[0].pop().map(drop).unwrap()),
        BodyRigError::BadClip { .. }
    ));
    assert!(matches!(
        refused(|rig| rig.clips.clear()),
        BodyRigError::NoClips
    ));
}

#[test]
fn a_clip_loops_or_holds_by_its_own_flag() {
    let clip = RigClip {
        looping: true,
        frame_duration_s: 0.1,
        frames: vec![Vec::new(); 3],
    };
    assert_eq!(clip.frame_at_time(0.35), 0);
    let held = RigClip { looping: false, ..clip.clone() };
    assert_eq!(held.frame_at_time(0.35), 2);
    assert_eq!(clip.frame_at_phase(0.5), 1);
    assert_eq!(clip.frame_at_phase(1.0), 2);
}

#[test]
fn a_published_rig_parses_in_the_publishers_shape() {
    let text = r#"
        // Auto-emitted body rig.
        (
            schema_version: 1,
            target: "demo",
            joints: [(name: "torso", parent: None), (name: "head", parent: Some("torso"))],
            attachments: [(name: "head", joint: "head", offset: (0.0, -4.0))],
            hurt_parts: [
                (name: "head", joint: "head", shape: Circle(center: (0.0, -4.0), radius: 3.0)),
                (name: "torso", joint: "torso", shape: Rect(center: (0.0, 0.0), half_extents: (4.0, 6.0))),
            ],
            clips: {
                "idle": (looping: true, frame_duration_s: 0.16, frames: [
                    [(translation: (0.0, -10.0), rotation: 0.0, scale: (1.0, 1.0)),
                     (translation: (0.0, -6.0), rotation: 0.0, scale: (-1.0, 1.0))],
                ]),
            },
        )
    "#;
    let rig = BodyRigDefinition::from_published_ron(text).expect("parses");
    let rig = rig.prepare().expect("valid");
    assert_eq!(rig.joint_names(), ["torso", "head"]);
    let wrong_schema = text.replace("schema_version: 1", "schema_version: 2");
    assert!(BodyRigDefinition::from_published_ron(&wrong_schema).is_err());
}

#[test]
fn a_rotated_rect_is_bounded_not_shrunk() {
    let shape = RigShape::Rect {
        center: (0.0, 0.0),
        half_extents: (2.0, 1.0),
    };
    let (min, max) = shape.bounds_in(Affine2::from_angle(std::f32::consts::FRAC_PI_2));
    assert!((min - Vec2::new(-1.0, -2.0)).length() < 1e-4);
    assert!((max - Vec2::new(1.0, 2.0)).length() < 1e-4);
}
