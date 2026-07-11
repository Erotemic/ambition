//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::super::observation::CrowdingSignal;
use super::*;

fn obs_at(distance_x: f32) -> ObservationFrame {
    ObservationFrame {
        self_pos: ae::Vec2::ZERO,
        self_vel: ae::Vec2::ZERO,
        self_facing: 1.0,
        self_on_ground: true,
        self_aerial: false,
        self_alive: true,
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
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::Walk { dir: 1.0 },
        &obs_at(300.0),
        &mut frame,
    );
    assert!(frame.locomotion.x > 0.0);
    assert_eq!(frame.locomotion.y, 0.0);
    assert!(frame.facing > 0.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
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
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    emit_inputs(
        SpecificAction::MeleeAttack {
            dir: ae::Vec2::new(1.0, 0.0),
        },
        &obs_at(40.0),
        &mut frame,
    );
    assert!(frame.melee_pressed);
    assert_eq!(frame.attack_axis, ae::Vec2::new(1.0, 0.0));
    assert!(frame.facing > 0.0);
}

#[test]
fn ranged_attack_sets_fire_with_dir() {
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
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
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    emit_inputs(SpecificAction::Jump, &obs_at(200.0), &mut frame);
    assert!(frame.jump_pressed);
}

#[test]
fn idle_zeros_locomotion_but_keeps_facing() {
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    // Target on the left → expect actor to face left.
    emit_inputs(SpecificAction::Idle, &obs_at(-200.0), &mut frame);
    assert_eq!(frame.locomotion, ae::Vec2::ZERO);
    assert!(frame.facing < 0.0, "facing should point at target");
}
