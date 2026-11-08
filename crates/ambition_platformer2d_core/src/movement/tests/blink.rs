//! Blink mechanics: press / hold / release, precision aim, soft-wall
//! pass, post-blink gravity grace, downward-velocity clamp.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::world::BlinkWallTier;
use crate::{AabbExt, AbilitySet, Vec2};

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
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            ),
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(!scratch.axis().blink_hold_active);

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
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: false,
                    held: true,
                    released: false,
                },
            ),
            ..Default::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(scratch.axis().blink_hold_active);
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
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Blink,
            crate::Edge {
                pressed: true,
                held: true,
                released: false,
            },
        ),
        axes: crate::LocalAxes::new(1.0, 0.0),
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
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Blink,
            crate::Edge {
                pressed: false,
                held: false,
                released: true,
            },
        ),
        axes: crate::LocalAxes::new(1.0, 0.0),
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
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            ),
            axes: crate::LocalAxes::new(1.0, 0.0),
            ..Default::default()
        },
    );
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: false,
                    held: false,
                    released: true,
                },
            ),
            blink_quick_dir: crate::WorldVec2(Vec2::new(1.0, 0.0)),
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
        let blink_pressed = !scratch.axis().blink_hold_active;
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                movement: crate::ActionEdges::EMPTY.with(
                    crate::MovementAction::Blink,
                    crate::Edge {
                        pressed: blink_pressed,
                        held: true,
                        released: false,
                    },
                ),
                blink_aim_step: crate::WorldVec2(Vec2::new(1.0, 0.0)),
                ..Default::default()
            },
        );
    }
    assert!(scratch.axis().blink_aiming);
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: false,
                    held: false,
                    released: true,
                },
            ),
            axes: crate::LocalAxes::new(1.0, 0.0),
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
        scratch.axis_mut().blink_hold_active = true;
        scratch.axis_mut().blink_aiming = false;
        let events = update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                movement: crate::ActionEdges::EMPTY.with(
                    crate::MovementAction::Blink,
                    crate::Edge {
                        pressed: false,
                        held: false,
                        released: true,
                    },
                ),
                axes: crate::LocalAxes::new(1.0, 0.0),
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
        assert!(scratch.axis().blink_grace_timer > 0.0);
    }
}

#[test]
fn post_blink_grace_suspends_gravity_for_tiny_window() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos = Vec2::new(420.0, 620.0);
    scratch.kinematics.vel = Vec2::new(0.0, 900.0);
    scratch.axis_mut().blink_hold_active = true;
    let _events = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Blink,
                crate::Edge {
                    pressed: false,
                    held: false,
                    released: true,
                },
            ),
            axes: crate::LocalAxes::new(1.0, 0.0),
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

#[test]
fn god_blink_accepts_destinations_inside_walls_and_outside_the_world() {
    use crate::world::{Block, BlockKind};
    let mut world = test_world();
    world.blocks.clear();
    let from = Vec2::new(140.0, 140.0);
    let target = Vec2::new(340.0, 140.0);
    for kind in [BlockKind::Solid, BlockKind::Barrier,
        BlockKind::BlinkWall { tier: BlinkWallTier::Soft },
        BlockKind::BlinkWall { tier: BlinkWallTier::Hard }] {
        let mut wall = Block::solid("locked passage", Vec2::new(220.0, 0.0), Vec2::new(22.0, 300.0));
        wall.kind = kind;
        world.blocks.clear();
        world.blocks.push(wall);
        let normal = scratch_with(AbilitySet::basic(), from);
        let god = scratch_with(crate::AbilityGrant::GodBlink.to_set(), from);
        let normal_to = blink_destination_to_point_clusters(&world, &normal.kinematics, &normal.abilities, target);
        assert!(normal_to.x < 220.0, "normal blink passed {kind:?}");
        let to = blink_destination_to_point_clusters(&world, &god.kinematics, &god.abilities, target);
        assert_eq!(to, target, "God Blink stopped at {kind:?}");
        let inside_wall = blink_destination_to_point_clusters(&world, &god.kinematics, &god.abilities, Vec2::new(230.0, 140.0));
        assert_eq!(inside_wall, Vec2::new(230.0, 140.0));
        assert!(crate::geometry::Aabb::new(inside_wall, god.kinematics.size() * 0.5).strict_intersects(world.blocks[0].aabb));
        let outside = blink_destination_to_point_clusters(&world, &god.kinematics, &god.abilities, Vec2::new(5000.0, -5000.0));
        assert_eq!(outside, Vec2::new(5000.0, -5000.0));
    }
    assert!(!AbilitySet::sandbox_all().unrestricted_blink);
}

#[test]
fn a_blink_with_no_free_destination_emits_no_committed_effect() {
    let mut world = test_world();
    world.blocks.clear();
    let mut scratch = scratch_at(Vec2::new(140.0, 140.0));
    world.blocks.push(crate::world::Block::solid(
        "flush wall", scratch.kinematics.pos + Vec2::new(scratch.kinematics.size().x * 0.5, -100.0),
        Vec2::new(200.0, 300.0)));
    scratch.axis_mut().blink_hold_active = true;
    let from = scratch.kinematics.pos;
    let input = InputState {
        movement: crate::ActionEdges::EMPTY.with(crate::MovementAction::Blink,
            crate::Edge { pressed: false, held: false, released: true }),
        blink_quick_dir: crate::WorldVec2(Vec2::new(1.0, 0.0)),
        ..Default::default()
    };
    let events = update_player_control_with_tuning_scratch(&world, &mut scratch, input, 1.0 / 60.0, TEST_TUNING);
    assert_eq!(scratch.kinematics.pos, from);
    assert!(events.blinks.is_empty());
    assert!(!events.operations.contains(&MovementOp::Blink));
    assert!(!scratch.axis().blink_hold_active);
    assert_eq!(scratch.blink.cooldown, 0.0);
}

#[test]
fn god_blink_can_land_inside_a_solid_outline() {
    let mut world = test_world();
    world.blocks.clear();
    world.chains.push(crate::world::SurfaceChain::solid("wall outline", vec![
        Vec2::new(220.0, 0.0), Vec2::new(260.0, 0.0),
        Vec2::new(260.0, 300.0), Vec2::new(220.0, 300.0),
    ]));
    let god = scratch_with(crate::AbilityGrant::GodBlink.to_set(), Vec2::new(140.0, 140.0));
    let to = blink_destination_to_point_clusters(&world, &god.kinematics, &god.abilities, Vec2::new(240.0, 140.0));
    assert_eq!(to, Vec2::new(240.0, 140.0));
    assert!(crate::solid_lines::put_out_of_solid_lines(&world, crate::Aabb::new(to, god.kinematics.size() * 0.5)).is_some());
    let beyond = blink_destination_to_point_clusters(&world, &god.kinematics, &god.abilities, Vec2::new(340.0, 140.0));
    assert_eq!(beyond, Vec2::new(340.0, 140.0));
}

#[test]
fn god_blink_cursor_can_move_beyond_normal_range() {
    let world = test_world();
    for unrestricted in [false, true] {
        let mut set = AbilitySet::sandbox_all();
        set.unrestricted_blink = unrestricted;
        let mut body = scratch_with(set, world.spawn);
        body.axis_mut().blink_hold_active = true;
        body.axis_mut().blink_aiming = true;
        body.axis_mut().blink_aim_offset = Vec2::new(TEST_TUNING.precision_blink_distance, 0.0);
        let input = InputState {
            movement: crate::ActionEdges::EMPTY.with(crate::MovementAction::Blink,
                crate::Edge { held: true, ..Default::default() }),
            blink_aim_step: crate::WorldVec2(Vec2::new(1.0, 0.0)),
            ..Default::default()
        };
        for _ in 0..120 {
            update_player_control_with_tuning_scratch(&world, &mut body, input, 1.0 / 60.0, TEST_TUNING);
        }
        let range = body.axis().blink_aim_offset.length();
        if unrestricted { assert!(range > TEST_TUNING.precision_blink_distance * 2.0); }
        else { assert!(range <= TEST_TUNING.precision_blink_distance + 0.001); }
    }
}

#[test]
fn leaving_god_blink_restores_the_range_before_a_stationary_hold_or_release() {
    let mut world = test_world();
    world.blocks.clear();
    for released in [false, true] {
        let mut body = scratch_at(Vec2::new(140.0, 140.0));
        body.axis_mut().blink_hold_active = true;
        body.axis_mut().blink_aiming = true;
        body.axis_mut().blink_hold_timer = 10.0;
        body.axis_mut().blink_aim_offset = Vec2::new(TEST_TUNING.precision_blink_distance * 5.0, 0.0);
        let from = body.kinematics.pos;
        let event = update_player_control_with_tuning_scratch(&world, &mut body, InputState {
            movement: crate::ActionEdges::EMPTY.with(crate::MovementAction::Blink,
                crate::Edge { held: !released, released, ..Default::default() }),
            ..Default::default()
        }, 1.0 / 60.0, TEST_TUNING);
        assert!(body.axis().blink_aim_offset.length() <= TEST_TUNING.precision_blink_distance + 0.001);
        if released {
            assert_eq!(event.blinks.len(), 1);
            assert!((body.kinematics.pos - from).length() <= TEST_TUNING.precision_blink_distance + 0.001);
        }
    }
}

#[test]
fn god_blink_does_not_freeze_one_way_drops_flight_or_collision_recovery() {
    use crate::world::{Block, BlockKind};
    for scenario in 0..3 {
        let mut world = test_world();
        world.blocks.clear();
        let mut block = Block::solid("surface", Vec2::new(220.0, 200.0), Vec2::new(180.0, 16.0));
        if scenario != 2 { block.kind = BlockKind::OneWay; }
        world.blocks.push(block);
        let run = |unrestricted| {
            let mut set = AbilitySet::sandbox_all();
            set.unrestricted_blink = unrestricted;
            let mut body = scratch_with(set, Vec2::new(260.0, 200.0));
            if scenario == 0 {
                body.kinematics.pos.y = 200.0 - body.kinematics.size().y * 0.5;
                body.ground.on_ground = true;
                body.axis_mut().drop_through_timer = 1.0;
            }
            if scenario == 1 { body.flight.fly_enabled = true; }
            let mut seen = vec![(body.kinematics.pos, body.kinematics.vel, body.ground.on_ground)];
            for _ in 0..40 {
                step_scratch(&world, &mut body, InputState {
                    axes: crate::LocalAxes::new(if scenario == 1 { 1.0 } else { 0.0 }, 1.0),
                    ..Default::default()
                });
                seen.push((body.kinematics.pos, body.kinematics.vel, body.ground.on_ground));
            }
            seen
        };
        let normal = run(false);
        let god = run(true);
        assert_eq!(god, normal, "God Blink changed ordinary movement in scenario {scenario}");
        assert!(normal.windows(2).any(|frames| frames[0].0 != frames[1].0), "the control must move");
    }
}
