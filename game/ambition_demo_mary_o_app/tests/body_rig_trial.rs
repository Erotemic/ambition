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
