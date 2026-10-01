//! THE RIGGED-BODY TRIAL: Mary-O's published body rig reaches her body only
//! where the composition admits rigs, and then it is on her from construction.
//!
//! The rig comes from the sprite publisher (`mary_o_v2_body_rig.ron`, solved
//! from her production SVG rig). The shipped demo does not admit it, so the
//! shipped demo's bodies carry no rig at all.

use ambition_demo_mary_o_app::{build_demo_app, build_demo_app_with_body_rigs};
use ambition_platformer2d::characters::actor::body_rig::{HAND_FAR, HAND_NEAR, HEAD};
use ambition_platformer2d::combat::body_rig::BodyRig;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt};
use bevy::prelude::*;

/// The player's rig at the first sim tick the player existed: `Some(None)` is
/// a player with no rig.
#[derive(Resource, Default)]
struct FirstTick(Option<Option<BodyRig>>);

fn probe(mut seen: ResMut<FirstTick>, player: Query<Option<&BodyRig>, With<PrimaryPlayer>>) {
    if seen.0.is_none() {
        if let Ok(rig) = player.single() {
            seen.0 = Some(rig.cloned());
        }
    }
}

fn first_tick_rig(mut app: App) -> Option<BodyRig> {
    app.init_resource::<FirstTick>();
    let sim = app.sim_schedule();
    app.add_systems(sim, probe.in_set(PlayerInputSet::Device));
    for _ in 0..600 {
        app.update();
        if let Some(rig) = app.world().resource::<FirstTick>().0.clone() {
            return rig;
        }
    }
    panic!("the demo never ran a sim tick with a playable body");
}

#[test]
fn an_admitted_rig_is_on_mary_o_before_her_first_tick() {
    let rig = first_tick_rig(build_demo_app_with_body_rigs()).expect(
        "Mary-O reached her first sim tick without her published body rig; \
         publish it with `scripts/regen/sprites.sh --target mary_o_v2`",
    );
    let joints = rig.0.joint_names();
    for joint in ["head", "torso", "near_arm", "far_arm", "near_leg", "far_leg"] {
        assert!(joints.iter().any(|name| name == joint), "no `{joint}` joint in {joints:?}");
    }
    for attachment in [HAND_NEAR, HAND_FAR, HEAD] {
        assert!(
            rig.0.attachment_index(attachment).is_some(),
            "no `{attachment}` attachment on Mary-O's rig"
        );
    }
    assert!(rig.0.clip("idle").is_some() && rig.0.clip("jump").is_some());
}

#[test]
fn the_shipped_demo_admits_no_rig() {
    assert!(
        first_tick_rig(build_demo_app()).is_none(),
        "the shipped demo built Mary-O with a body rig it does not admit"
    );
}

/// The pose is solved in the SIMULATION, from the clocks: a rigged body that
/// has run ticks carries a resolved pose — every joint and every attachment —
/// with no renderer in the app.
#[test]
fn an_admitted_rig_is_posed_by_the_simulation() {
    use ambition_platformer2d::combat::body_rig::BodyRigPose;
    let mut app = build_demo_app_with_body_rigs();
    let mut resolved = None;
    for _ in 0..600 {
        app.update();
        let mut players = app
            .world_mut()
            .query_filtered::<(&BodyRig, &BodyRigPose), With<PrimaryPlayer>>();
        if let Ok((rig, pose)) = players.single(app.world()) {
            if pose.clip.is_some() {
                resolved = Some((rig.clone(), pose.clone()));
                break;
            }
        }
    }
    let (rig, pose) = resolved.expect("Mary-O's rig was never posed by the simulation");
    assert_eq!(pose.joints.len(), rig.0.joint_names().len());
    assert_eq!(pose.attachments.len(), rig.0.attachments().len());
    let head = pose.attachment(&rig.0, HEAD).expect("a posed head");
    let hand = pose.attachment(&rig.0, HAND_NEAR).expect("a posed hand");
    // Rig space is feet-origin, +y down: the head is above the feet and above
    // the hand of a body standing on the ground.
    assert!(head.y < 0.0 && head.y < hand.y, "head {head:?}, hand {hand:?}");
}

/// Rig packet 3: an admitted rig is Mary-O's DEFAULT hurt geometry. Standing,
/// she is struck through her rig's four parts — a head, a torso, two legs —
/// not through her coarse body box, and the parts sit where a body stands:
/// the head on top, the legs down at her feet.
#[test]
fn an_admitted_rig_is_mary_os_default_hurt_geometry() {
    use ambition_platformer2d::combat::components::{CenteredAabb, DamageableVolumes};
    use ambition_platformer2d::combat::hurtbox_resolution::{HurtboxSelection, ResolvedHurtboxes};

    let mut app = build_demo_app_with_body_rigs();
    let mut found = None;
    for _ in 0..600 {
        app.update();
        let mut players = app.world_mut().query_filtered::<
            (&ResolvedHurtboxes, &DamageableVolumes, &CenteredAabb),
            With<PrimaryPlayer>,
        >();
        if let Ok((resolved, damageable, body)) = players.single(app.world()) {
            if resolved.source == HurtboxSelection::RigDefault && damageable.published() {
                found = Some((damageable.volumes.clone(), body.aabb()));
                break;
            }
        }
    }
    let (volumes, body) = found.expect("Mary-O's hurt geometry never came from her rig");
    assert_eq!(volumes.len(), 4, "head, torso and two legs: {volumes:?}");
    let bounds: Vec<_> = volumes.iter().map(|volume| volume.bounds()).collect();
    let (head, legs) = (&bounds[0], &bounds[2..]);
    // y is down. The head is the highest part; the legs end at her feet.
    assert!(bounds[1..].iter().all(|part| head.min.y < part.min.y), "{bounds:?}");
    let feet = body.max.y;
    for leg in legs {
        assert!(
            (leg.max.y - feet).abs() < (body.max.y - body.min.y) * 0.15,
            "a leg ends {} from her feet ({leg:?}, body {body:?})",
            leg.max.y - feet
        );
    }
    // Articulated, not inflated: every part is centred inside her body box's
    // horizontal span.
    for part in &bounds {
        let x = (part.min.x + part.max.x) * 0.5;
        assert!(body.min.x <= x && x <= body.max.x, "{part:?} is outside {body:?}");
    }
}

/// Review finding [P3]: a walking body is posed from its walk clip, not from
/// idle. Her rig's walk frames move her near hand away from where idle puts it,
/// so a body solved from the wrong clip shows here.
///
/// She stands on the flat test course, then walks right. The gait is a
/// simulation fact (`BodyPoseClock::gait`); the rig clip follows it and its
/// frame advances on the gait clock.
#[test]
fn a_walking_mary_o_is_posed_from_her_walk_clip() {
    use ambition_platformer2d::combat::body_rig::BodyRigPose;
    use ambition_platformer2d::combat::hurtbox_resolution::{BodyPoseClock, Gait};
    use ambition_platformer2d::input::ControlFrame;

    let mut app = build_demo_app_with_body_rigs();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID.to_string(),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    let step = |app: &mut App, frame: ControlFrame| {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
            .0 = frame;
        app.update();
    };
    let read = |app: &mut App| {
        let mut players = app
            .world_mut()
            .query_filtered::<(&BodyRig, &BodyRigPose, &BodyPoseClock), With<PrimaryPlayer>>();
        players.single(app.world()).ok().map(|(rig, pose, clock)| {
            (
                pose.clip.clone(),
                pose.frame,
                clock.gait,
                pose.attachment(&rig.0, HAND_NEAR),
            )
        })
    };

    // Standing still on the ground: the idle clip.
    // Thirty ticks in a row in the idle clip: she is settled on the floor.
    let mut standing = (0, None);
    let mut last = None;
    for _ in 0..600 {
        step(&mut app, ControlFrame::default());
        last = read(&mut app);
        match &last {
            Some((Some(clip), _, Gait::Standing, hand)) if clip == "idle" => standing = (standing.0 + 1, *hand),
            _ => standing = (0, None),
        }
        if standing.0 >= 30 {
            break;
        }
    }
    let standing_hand = standing
        .1
        .filter(|_| standing.0 >= 30)
        .unwrap_or_else(|| panic!("Mary-O never stood in her idle clip; last read {last:?}"));

    let walk = ControlFrame {
        axis_x: 1.0,
        aim_x: 1.0,
        right_pressed: true,
        ..ControlFrame::default()
    };
    let mut walk_frames = std::collections::BTreeSet::new();
    let mut moved_hand = false;
    let mut gaits = Vec::new();
    for _ in 0..120 {
        step(&mut app, walk.clone());
        let Some((clip, frame, gait, hand)) = read(&mut app) else { continue };
        gaits.push(gait);
        if matches!(gait, Gait::Walking | Gait::Running) {
            assert_eq!(clip.as_deref(), Some("walk"), "moving with gait {gait:?} and posed from {clip:?}");
            walk_frames.insert(frame);
            moved_hand |= hand.is_some_and(|hand| hand.distance(standing_hand) > 1.0);
        }
    }
    assert!(!walk_frames.is_empty(), "she never had a moving gait; gaits: {gaits:?}");
    assert!(walk_frames.len() >= 2, "the walk clip never advanced: frames {walk_frames:?}");
    assert!(moved_hand, "her near hand stayed at its idle place for the whole walk");
}
