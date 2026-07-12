//! Blink mechanics: press / hold / release, precision aim, soft-wall
//! pass, post-blink gravity grace, downward-velocity clamp.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::world::BlinkWallTier;
use crate::{AbilitySet, Vec2};

fn scratch_at(spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn held_blink_arms_when_cooldown_clears_without_new_press() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.blink.cooldown = 0.02;

    // Pressing slightly early should not arm yet.
    let _ = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(!scratch.blink.hold_active);

    // Cooldown clears in simulation time.
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        0.03,
        TEST_TUNING,
    );
    assert_eq!(scratch.blink.cooldown, 0.0);

    // The user is still holding the button, so control time can arm blink
    // without requiring another just-pressed edge.
    let _ = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(scratch.blink.hold_active);
}

#[test]
fn blink_ability_gates_teleport() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.blink = false;
    abilities.precision_blink = false;
    let mut scratch = scratch_with(abilities, world.spawn);
    let start = scratch.kinematics.pos;
    let input = InputState {
        axes: crate::LocalAxes::new(1.0, 0.0),
        blink_pressed: true,
        blink_held: true,
        ..Default::default()
    };
    let _ = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        input,
        1.0 / 60.0,
        TEST_TUNING,
    );
    let input = InputState {
        axes: crate::LocalAxes::new(1.0, 0.0),
        blink_released: true,
        ..Default::default()
    };
    let events = update_player_control_with_tuning_scratch(
        &world,
        &mut scratch,
        input,
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert_eq!(scratch.kinematics.pos, start);
    assert!(events.blinks.is_empty());
}

#[test]
fn quick_blink_moves_on_release() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let start = scratch.kinematics.pos;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axes: crate::LocalAxes::new(1.0, 0.0),
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
    );
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            blink_quick_dir: crate::WorldVec2(Vec2::new(1.0, 0.0)),
            blink_released: true,
            ..Default::default()
        },
    );
    assert!(scratch.kinematics.pos.x > start.x + 20.0);
    assert_eq!(events.blinks.len(), 1);
    assert!(!events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::Blink));
}

#[test]
fn held_blink_enters_precision_aiming() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    for _ in 0..20 {
        let blink_pressed = !scratch.blink.hold_active;
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                blink_aim_step: crate::WorldVec2(Vec2::new(1.0, 0.0)),
                blink_held: true,
                blink_pressed,
                ..Default::default()
            },
        );
    }
    assert!(scratch.blink.aiming);
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            axes: crate::LocalAxes::new(1.0, 0.0),
            blink_released: true,
            ..Default::default()
        },
    );
    assert_eq!(events.blinks.len(), 1);
    assert!(events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::PrecisionBlink));
}

#[test]
fn repeated_blinks_clamp_downward_velocity_each_time() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos = Vec2::new(420.0, 620.0);

    for _ in 0..2 {
        scratch.kinematics.vel = Vec2::new(25.0, 900.0);
        scratch.blink.cooldown = 0.0;
        scratch.blink.hold_active = true;
        scratch.blink.aiming = false;
        let events = update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                axes: crate::LocalAxes::new(1.0, 0.0),
                blink_released: true,
                ..Default::default()
            },
            1.0 / 60.0,
            TEST_TUNING,
        );
        assert_eq!(events.blinks.len(), 1);
        assert!(
            scratch.kinematics.vel.y
                <= TEST_TUNING.blink_max_downward_speed + TEST_TUNING.gravity / 60.0 + 1.0,
            "blink should not preserve a large downward fall speed; got {}",
            scratch.kinematics.vel.y
        );
        assert!(scratch.blink.grace_timer > 0.0);
    }
}

#[test]
fn post_blink_grace_suspends_gravity_for_tiny_window() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos = Vec2::new(420.0, 620.0);
    scratch.kinematics.vel = Vec2::new(0.0, 900.0);
    scratch.blink.hold_active = true;
    let _events = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axes: crate::LocalAxes::new(1.0, 0.0),
            blink_released: true,
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    let after_blink_vy = scratch.kinematics.vel.y;
    let _events = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 240.0,
        TEST_TUNING,
    );
    assert!(
        scratch.kinematics.vel.y <= after_blink_vy + 0.1,
        "gravity should be suspended during the short post-blink grace window"
    );
}

#[test]
fn blink_walls_can_be_passed_by_upgrade_without_allowing_solid_walls() {
    let mut world = test_world();
    world.blocks.clear();
    world.blocks.push(crate::world::Block::blink_wall(
        "test soft blink membrane",
        Vec2::new(220.0, 0.0),
        Vec2::new(22.0, 300.0),
        BlinkWallTier::Soft,
    ));

    let mut blocked_abilities = AbilitySet::basic();
    blocked_abilities.blink = true;
    let blocked = scratch_with(blocked_abilities, Vec2::new(140.0, 140.0));
    let blocked_to = blink_destination_to_point_clusters(
        &world,
        &blocked.kinematics,
        &blocked.abilities,
        Vec2::new(340.0, 140.0),
    );
    assert!(blocked_to.x < 220.0);

    let mut pass_abilities = blocked_abilities;
    pass_abilities.blink_through_soft_walls = true;
    let pass = scratch_with(pass_abilities, Vec2::new(140.0, 140.0));
    let passed_to = blink_destination_to_point_clusters(
        &world,
        &pass.kinematics,
        &pass.abilities,
        Vec2::new(340.0, 140.0),
    );
    assert!(passed_to.x > 300.0);
}
