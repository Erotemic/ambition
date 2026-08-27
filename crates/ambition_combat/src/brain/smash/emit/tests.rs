use super::*;
use ambition_characters::brain::smash::CrowdingSignal;

fn obs_at(distance_x: f32) -> ObservationFrame {
    ObservationFrame {
        self_pos: ae::Vec2::ZERO,
        self_vel: ae::Vec2::ZERO,
        self_facing: 1.0,
        self_on_ground: true,
        self_aerial: false,
        self_alive: true,
        self_captured: false,
        self_holding_captive: false,
        self_pummels_landed: 0,
        self_attacking: false,
        self_air_jumps_remaining: 0,
        attack_cooldown_remaining: 0.0,
        stun_remaining: 0.0,
        self_health_fraction: 1.0,
        target_pos: ae::Vec2::new(distance_x, 0.0),
        target_alive: true,
        to_target_x: distance_x,
        to_target_y: 0.0,
        distance_to_target: distance_x.abs(),
        down: ae::Vec2::new(0.0, 1.0),
        crowding: CrowdingSignal::default(),
        terrain: Default::default(),
        sim_time: 1.0,
        dt: 1.0 / 60.0,
    }
}

#[test]
fn walk_emits_locomotion_along_dir() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::Walk { dir: 1.0 },
        &obs_at(300.0),
        &mut frame,
    );
    assert!(frame.locomotion.x > 0.0);
    assert_eq!(frame.locomotion.y, 0.0);
    assert!(frame.facing > 0.0);
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::Walk { dir: -1.0 },
        &obs_at(300.0),
        &mut frame,
    );
    assert!(frame.locomotion.x < 0.0);
    assert!(frame.facing < 0.0);
}

#[test]
fn melee_attack_sets_melee_pressed_and_attack_axis() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::MeleeAttack {
            dir: ae::Vec2::new(1.0, 0.0),
        },
        &obs_at(40.0),
        &mut frame,
    );
    assert!(frame.melee_pressed);
    assert_eq!(frame.attack_axis, ae::LocalAxes::new(1.0, 0.0));
    assert!(frame.facing > 0.0);
}

#[test]
fn ranged_attack_sets_fire_with_dir() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::RangedAttack {
            dir: ae::Vec2::new(0.0, -1.0),
        },
        &obs_at(200.0),
        &mut frame,
    );
    match frame.fire {
        Some(req) => {
            assert!((req.dir.y + 1.0).abs() < 1e-3);
            assert_eq!(req.dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
        }
        None => panic!("expected fire request"),
    }
}

#[test]
fn jump_emits_jump_pressed_edge() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(SpecificAction::Jump, &obs_at(200.0), &mut frame);
    assert!(frame.jump_pressed);
}

#[test]
fn idle_zeros_locomotion_but_keeps_facing() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    // Target on the left → expect actor to face left.
    emit_inputs(SpecificAction::Idle, &obs_at(-200.0), &mut frame);
    assert_eq!(frame.locomotion, ae::LocalAxes::ZERO);
    assert!(frame.facing < 0.0, "facing should point at target");
}

/// A CHOSEN SHIELD REACHES THE BODY'S GUARD BIT.
///
///  nothing chooses `Shield` yet, and that is the point of pinning it here.
/// The remaining gap is upstream: `ObservationFrame` carries no channel for what
/// the TARGET is doing, so the brain cannot see an incoming swing and has no
/// condition on which to guard. This test is what makes that a one-layer job
/// instead of two, and it fails the moment somebody restores the drop-to-Idle.
#[test]
fn a_chosen_shield_presses_the_guard_and_stops_moving() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    assert!(
        !frame.shield_held,
        "a neutral frame already guards, so this test cannot tell whether \
         `emit_inputs` did anything"
    );
    emit_inputs(SpecificAction::Shield, &obs_at(120.0), &mut frame);
    assert!(
        frame.shield_held,
        "a brain that chose to shield emitted no guard — the action is a no-op \
         again and any brain-side work above it is invisible"
    );
    assert_eq!(
        frame.locomotion,
        ae::LocalAxes::ZERO,
        "shielding while walking: a guard is a commitment, not a modifier"
    );
}

///  `ActorControlFrame` has no dodge bit.
#[test]
fn a_chosen_dodge_is_still_reserved_and_says_so() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::Dodge {
            dir: ae::Vec2::new(1.0, 0.0),
        },
        &obs_at(120.0),
        &mut frame,
    );
    assert!(
        !frame.shield_held,
        "a dodge raised a GUARD — the two arms were merged again, and they are \
         different verbs with different bodies of rules"
    );
    assert!(
        !frame.burst_pressed,
        "a dodge emitted a dash. That is not a shortcut, it is the P5.38 defect: \
         `apply_dodge` claims the dash buffer first, so this would produce a \
         burst the brain believes is a dodge and the body resolves as a dash"
    );
}
