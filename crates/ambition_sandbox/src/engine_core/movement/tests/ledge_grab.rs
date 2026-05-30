//! Ledge grab latch (blink-wall surface, one-way surface) and the
//! "climb finishes inside one simulation tick" invariant.

use super::super::*;
use super::test_world;
use crate::engine_core::player_clusters::PlayerClusterScratch;
use crate::engine_core::world::{BlinkWallTier, Block};
use crate::engine_core::{AbilitySet, Vec2};

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> PlayerClusterScratch {
    PlayerClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn simulation_latches_ledge_grab_on_blink_wall_surface() {
    let mut world = test_world();
    world.blocks.push(Block::blink_wall(
        "soft blink ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
        BlinkWallTier::Soft,
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(86.0, 110.0));
    scratch.kinematics.vel = Vec2::new(120.0, 20.0);
    scratch.wall.wall_clinging = true;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = -1.0;

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: 1.0,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        scratch.ledge.grab.is_some(),
        "blink wall ledge should latch"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn simulation_latches_ledge_grab_on_one_way_surface_without_wall_collision() {
    let mut world = test_world();
    world.blocks.push(Block::one_way(
        "one-way ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 16.0),
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(86.0, 110.0));
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::new(20.0, 40.0);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: 1.0,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        scratch.ledge.grab.is_some(),
        "pressing toward a one-way edge should allow a pull-up even though one-way platforms do not collide on X"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn attack_press_from_hang_starts_getup_attack_and_fires_slash() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(87.0, 119.0));
    let contact = crate::engine_core::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::engine_core::LedgeGrabState::hanging(contact);
    // Skip past the hang debounce so the input is accepted this tick.
    state.elapsed = crate::engine_core::LEDGE_MIN_CLIMB_DELAY;
    scratch.ledge.grab = Some(state);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            attack_pressed: true,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
        events.operations.contains(&MovementOp::LedgeGetupAttack),
        "attack from hang should emit LedgeGetupAttack"
    );
    assert!(
        events.operations.contains(&MovementOp::Slash),
        "getup attack should also emit a Slash op so the hitbox fires"
    );
    let new_state = scratch.ledge.grab.expect("getup-attack keeps ledge state");
    assert!(new_state.climbing, "state should be in getup transition");
    assert_eq!(
        new_state.getup_kind,
        crate::engine_core::LedgeGetupKind::Attack
    );
    assert!(
        scratch.dodge.roll_timer > 0.0,
        "getup attack grants invuln frames via dodge_roll_timer"
    );
}

#[test]
fn active_ledge_grab_climb_finishes_inside_simulation_tick() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(87.0, 119.0));
    let contact = crate::engine_core::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::engine_core::LedgeGrabState::hanging(contact);
    state.elapsed = crate::engine_core::LEDGE_MIN_CLIMB_DELAY;
    state.climbing = true;
    state.climb_elapsed = crate::engine_core::LEDGE_CLIMB_TIME;
    scratch.ledge.grab = Some(state);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        scratch.ledge.grab.is_none(),
        "completed climb clears ledge state"
    );
    assert_eq!(scratch.kinematics.pos, contact.climb_target);
    assert!(scratch.ground.on_ground);
    assert!(events.operations.contains(&MovementOp::LedgeClimbFinish));
}
