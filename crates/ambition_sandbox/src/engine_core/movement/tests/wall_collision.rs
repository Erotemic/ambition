//! One-way platforms, wall-jump catapult guards, wall-cling y-sweep
//! teleport guards, top-corner landing, and the `body_is_side_contact`
//! predicate that the y-sweep / vertical resolver share.

use super::super::*;
use super::{step_scratch, test_world};
use crate::engine_core::geometry::AabbExt;
use crate::engine_core::player_clusters::PlayerClusterScratch;
use crate::engine_core::world::Block;
use crate::engine_core::{Aabb, AbilitySet, Vec2, World};

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> PlayerClusterScratch {
    PlayerClusterScratch::from_player(&Player::new_with_abilities(spawn, abilities))
}

#[test]
fn one_way_platform_requires_down_plus_jump_to_drop_through() {
    let mut world = test_world();
    // One-way platform suspended above the floor. Player will land on it
    // from above and we expect plain "down" alone to keep them resting.
    let plat_top_y = 600.0;
    world.blocks.push(Block::one_way(
        "drop test platform",
        Vec2::new(360.0, plat_top_y),
        Vec2::new(180.0, 12.0),
    ));

    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.pos = Vec2::new(450.0, plat_top_y - scratch.kinematics.size.y * 0.5);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    // Settle onto the platform.
    for _ in 0..6 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "player should land on the one-way");
    let resting_y = scratch.kinematics.pos.y;

    // Holding down alone must NOT drop through anymore.
    for _ in 0..6 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        (scratch.kinematics.pos.y - resting_y).abs() < 1.0,
        "down-alone must not drop through one-way (moved {} px)",
        scratch.kinematics.pos.y - resting_y
    );

    // Down + jump (with the explicit drop_through_pressed gesture) drops.
    // Critically the gesture only fires for one frame: the presentation
    // layer recomputes drop_through_pressed each frame from
    // `axis_y > 0.35 && jump_pressed`, and `jump_pressed` is just-pressed,
    // so subsequent frames see drop_through_pressed=false. The engine must
    // latch the drop-through internally for long enough to clear the
    // landing-tolerance band.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            drop_through_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..10 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                // jump_pressed and drop_through_pressed are NOT held: this
                // is exactly the input shape the sandbox produces after
                // the initial press.
                ..Default::default()
            },
        );
    }
    assert!(
        scratch.kinematics.pos.y > resting_y + 12.0,
        "down+jump should drop the player below the one-way (delta {})",
        scratch.kinematics.pos.y - resting_y
    );
}

/// Wall-jumping off the left wall while the player's body slightly
/// overlaps a wide horizontal block (floor/ceiling) must not catapult
/// the player out the opposite side of the room.
#[test]
fn wall_jump_does_not_catapult_through_left_wall() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);

    // Park the player against the left wall with a tiny overlap into the
    // floor (1 pixel deep) — the kind of residual penetration the engine
    // tolerates between sweeps.
    let body = scratch.kinematics.aabb();
    let left_wall_right = 36.0;
    let floor_top = world.size.y - 48.0;
    scratch.kinematics.pos.x = left_wall_right + body.half_size().x; // touching wall on its right edge
    scratch.kinematics.pos.y = floor_top - body.half_size().y + 1.0; // bottom 1 px below floor top
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = 1.0;
    scratch.ground.coyote_timer = 0.0;

    let initial_x = scratch.kinematics.pos.x;
    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            axis_y: 0.0,
            jump_pressed: true,
            jump_held: true,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
            scratch.kinematics.pos.x >= initial_x - 1.0,
            "wall jump pushed player to x={} from x={} — expected to stay near or right of starting position",
            scratch.kinematics.pos.x,
            initial_x,
        );
    assert!(
        scratch.kinematics.pos.x - body.half_size().x >= 0.0,
        "wall jump punched the player through the left wall (body left = {})",
        scratch.kinematics.pos.x - body.half_size().x,
    );
}

/// Closer match to the actual reported bug: the player has a tiny
/// residual penetration into the left wall (sub-pixel rounding from
/// the previous frame's snap) and is moving away from it on
/// wall-jump.
#[test]
fn wall_jump_does_not_catapult_player_off_wall_overlap() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let body = scratch.kinematics.aabb();
    let left_wall_right = 36.0;
    scratch.kinematics.pos.x = left_wall_right + body.half_size().x - 1.0;
    scratch.kinematics.pos.y = world.size.y * 0.5;
    scratch.kinematics.vel = Vec2::new(500.0, -650.0); // wall-jump initial velocities
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = false;
    scratch.wall.wall_normal_x = 0.0;

    let initial_x = scratch.kinematics.pos.x;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let dx = (scratch.kinematics.pos.x - initial_x).abs();
    assert!(
        dx < 30.0,
        "wall overlap caused horizontal teleport: dx={dx}, pos.x went from {initial_x} to {}",
        scratch.kinematics.pos.x,
    );
    assert!(
        scratch.kinematics.pos.x - body.half_size().x >= 0.0 - 0.5,
        "player was punched through the left wall: body left = {}",
        scratch.kinematics.pos.x - body.half_size().x,
    );
}

/// Regression: reproduces the wall-cling → Grounded teleport.
#[test]
fn wall_cling_does_not_teleport_to_wall_top_on_y_sweep() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let half = scratch.kinematics.size * 0.5;
    let wall_right = 36.0;
    scratch.kinematics.pos.x = wall_right + half.x - 0.05;
    scratch.kinematics.pos.y = world.size.y * 0.5; // ~450, well inside the room
    scratch.kinematics.vel = Vec2::new(0.0, DEFAULT_TUNING.wall_slide_speed);
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = 1.0;
    scratch.wall.wall_clinging = true;

    let initial_y = scratch.kinematics.pos.y;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
            scratch.kinematics.pos.y >= 0.0 && scratch.kinematics.pos.y <= world.size.y,
            "wall-cling y-sweep teleported player out of the world envelope: pos.y = {} (world.size.y = {})",
            scratch.kinematics.pos.y,
            world.size.y,
        );
    let dy = (scratch.kinematics.pos.y - initial_y).abs();
    assert!(
            dy < 50.0,
            "wall-cling y-sweep moved player by {dy} px in one frame; expected at most a few pixels of slide",
        );
    assert!(
        !scratch.ground.on_ground,
        "wall-cling y-sweep falsely set on_ground; player was supposedly grounded at y={}",
        scratch.kinematics.pos.y,
    );
}

/// Regression: player wall-clinging on a tall column whose top
/// is far above the player must NOT teleport upward when their
/// body partially overlaps the column on its bottom edge.
#[test]
fn partial_wall_cling_overlap_does_not_teleport_upward() {
    let world = World {
        name: "column".into(),
        size: Vec2::new(1600.0, 768.0),
        spawn: Vec2::new(50.0, 50.0),
        // Column matching the trace: x=[704, 720], y=[16, 400].
        // Center=(712, 208), size=(16, 384).
        blocks: vec![Block::solid(
            "column",
            Vec2::new(712.0, 208.0),
            Vec2::new(16.0, 384.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    // Reproduce the exact pre-OOB state from the trace.
    scratch.kinematics.pos = Vec2::new(718.0, 419.0);
    scratch.kinematics.vel = Vec2::new(0.0, 15.0); // gravity-decelerated tiny downward
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;

    let start_y = scratch.kinematics.pos.y;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            control_dt: 1.0 / 60.0,
            axis_x: -1.0, // pressing toward wall
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let dy = (scratch.kinematics.pos.y - start_y).abs();
    assert!(
        dy < 50.0,
        "y-sweep teleported player by {} px; expected ~tiny gravity-driven motion (start_y={}, end_y={})",
        dy, start_y, scratch.kinematics.pos.y,
    );
    assert!(
        scratch.kinematics.pos.y > 0.0 && scratch.kinematics.pos.y < world.size.y,
        "player ended OOB at y={}",
        scratch.kinematics.pos.y,
    );
}

/// Descending onto the top corner of a tall solid (a pillar) with
/// slight x overlap should still resolve as a normal landing.
#[test]
fn descending_onto_top_corner_of_tall_block_lands_normally() {
    let world = World {
        name: "pillar".into(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(50.0, 50.0),
        blocks: vec![Block::solid(
            "pillar",
            Vec2::new(380.0, 200.0),
            Vec2::new(40.0, 400.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.pos = Vec2::new(391.0, 200.0 - 23.0 - 0.5);
    scratch.kinematics.vel = Vec2::new(0.0, 200.0);
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = false;

    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let body = scratch.kinematics.aabb();
    assert!(
        scratch.ground.on_ground,
        "descending onto pillar top should land (on_ground = true); got pos={:?}",
        scratch.kinematics.pos
    );
    assert!(
        (body.bottom() - 200.0).abs() < 1.0,
        "body.bottom should snap to pillar.top = 200; got {} (pos.y = {})",
        body.bottom(),
        scratch.kinematics.pos.y,
    );
}

/// Direct unit test of `body_is_side_contact`. Both `sweep_player_y`
/// and `resolve_vertical` consult it to avoid the wall-cling teleport
/// class.
#[test]
fn body_is_side_contact_classifies_walls_vs_floors() {
    // Player about to land on a wide floor.
    let body = Aabb::new(Vec2::new(50.0, 100.0), Vec2::new(14.0, 23.0));
    let floor = Aabb::new(Vec2::new(80.0, 125.0), Vec2::new(60.0, 6.0));
    assert!(
        !body_is_side_contact(body, floor),
        "player about to land on a wide floor must NOT be classified as a side contact"
    );

    // Tall left wall, body fully alongside it.
    let wall = Aabb::new(Vec2::new(18.0, 450.0), Vec2::new(18.0, 450.0));
    let body_alongside_edge = Aabb::new(Vec2::new(36.0 + 14.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_alongside_edge, wall),
        "body alongside a tall wall (edge-touching on x) must be a side contact"
    );

    // Same wall, body penetrating by 1 px on x.
    let body_inside_wall = Aabb::new(Vec2::new(36.0 + 14.0 - 1.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_inside_wall, wall),
        "body penetrating a tall wall on x is still a side contact"
    );

    // Player landing on the top corner of a tall block (small x overlap).
    let pillar = Aabb::new(Vec2::new(900.0, 800.0), Vec2::new(40.0, 200.0));
    let body_landing_on_pillar = Aabb::new(
        Vec2::new(900.0 - 40.0 + 5.0, 600.0 - 23.0 + 1.0),
        Vec2::new(14.0, 23.0),
    );
    assert!(
            !body_is_side_contact(body_landing_on_pillar, pillar),
            "descending onto the top edge of a tall block (slight x overlap, body.top above block.top) must NOT be classified as a side contact"
        );

    // Player jumping up into a thick ceiling block.
    let ceiling = Aabb::new(Vec2::new(900.0, 200.0), Vec2::new(400.0, 100.0));
    let body_under_ceiling = Aabb::new(Vec2::new(900.0, 300.0 + 23.0 - 1.0), Vec2::new(14.0, 23.0));
    assert!(
            !body_is_side_contact(body_under_ceiling, ceiling),
            "rising into a thick ceiling (body.bottom poking past block.bottom) must NOT be classified as a side contact"
        );
}
