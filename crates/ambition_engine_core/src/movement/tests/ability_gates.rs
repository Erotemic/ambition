//! Pure ability-flag sanity: a flag set to `false` must keep the
//! corresponding op out of the FrameEvents / state.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;

fn scratch_with(abilities: AbilitySet, spawn: bevy_math::Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn double_jump_ability_controls_air_jump() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.double_jump = false;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.jump.air_jumps_available = 0;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(!events.operations.contains(&MovementOp::DoubleJump));

    abilities.double_jump = true;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.jump.air_jumps_available = 1;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(events.operations.contains(&MovementOp::DoubleJump));
}

#[test]
fn double_dash_ability_controls_dash_charges() {
    let world = test_world();
    let mut single_dash = AbilitySet::sandbox_all();
    single_dash.double_dash = false;
    let scratch = scratch_with(single_dash, world.spawn);
    assert_eq!(scratch.dash.charges_available, 1);

    let scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    assert_eq!(scratch.dash.charges_available, 2);
}

#[test]
fn wall_climb_requires_wall_cling() {
    let mut abilities = AbilitySet::sandbox_all();
    abilities.wall_cling = false;
    assert!(abilities
        .compatibility_warnings()
        .iter()
        .any(|w| w.contains("wall_climb")));
}
