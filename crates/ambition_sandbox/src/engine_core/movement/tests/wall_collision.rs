//! One-way platforms, wall-jump catapult guards, wall-cling y-sweep
//! teleport guards, top-corner landing, and the `body_is_side_contact`
//! predicate that the y-sweep / vertical resolver share.

use super::super::*;
use super::{step, test_world};
use crate::engine_core::geometry::AabbExt;
use crate::engine_core::world::Block;
use crate::engine_core::{Aabb, AbilitySet, Vec2, World};

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

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.pos = Vec2::new(450.0, plat_top_y - player.size.y * 0.5);
    player.vel = Vec2::ZERO;
    player.on_ground = false;

    // Settle onto the platform.
    for _ in 0..6 {
        step(&world, &mut player, InputState::default());
    }
    assert!(player.on_ground, "player should land on the one-way");
    let resting_y = player.pos.y;

    // Holding down alone must NOT drop through anymore.
    for _ in 0..6 {
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        (player.pos.y - resting_y).abs() < 1.0,
        "down-alone must not drop through one-way (moved {} px)",
        player.pos.y - resting_y
    );

    // Down + jump (with the explicit drop_through_pressed gesture) drops.
    // Critically the gesture only fires for one frame: the presentation
    // layer recomputes drop_through_pressed each frame from
    // `axis_y > 0.35 && jump_pressed`, and `jump_pressed` is just-pressed,
    // so subsequent frames see drop_through_pressed=false. The engine must
    // latch the drop-through internally for long enough to clear the
    // landing-tolerance band.
    step(
        &world,
        &mut player,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            drop_through_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..10 {
        step(
            &world,
            &mut player,
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
        player.pos.y > resting_y + 12.0,
        "down+jump should drop the player below the one-way (delta {})",
        player.pos.y - resting_y
    );
}

/// Wall-jumping off the left wall while the player's body slightly
/// overlaps a wide horizontal block (floor/ceiling) must not catapult
/// the player out the opposite side of the room.
///
/// Reproduction in the square_arena: player is wall-clinging the left
/// wall low enough that their feet still poke into the floor block.
/// `resolve_axis(Axis::X)` saw the residual floor overlap and tried to
/// resolve it *horizontally* — the floor block spans the whole room,
/// so its left edge is at x=0, which produced a single-frame push
/// equal to the negative of the player's right edge (~58 pixels left)
/// and dumped the player at negative x.
#[test]
fn wall_jump_does_not_catapult_through_left_wall() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());

    // Park the player against the left wall with a tiny overlap into the
    // floor (1 pixel deep) — the kind of residual penetration the engine
    // tolerates between sweeps.
    let body = player.aabb();
    let left_wall_right = 36.0;
    let floor_top = world.size.y - 48.0;
    player.pos.x = left_wall_right + body.half_size().x; // touching wall on its right edge
    player.pos.y = floor_top - body.half_size().y + 1.0; // bottom 1 px below floor top
    player.vel = Vec2::ZERO;
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.coyote_timer = 0.0;

    let initial_x = player.pos.x;
    let _ = update_player_with_tuning(
        &world,
        &mut player,
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

    // After one wall-jump frame the player should be drifting *right*
    // (away from the wall) or at worst still touching it — never past
    // the wall's right edge in the negative-x direction by tens of
    // pixels.
    assert!(
            player.pos.x >= initial_x - 1.0,
            "wall jump pushed player to x={} from x={} — expected to stay near or right of starting position",
            player.pos.x,
            initial_x,
        );
    assert!(
        player.pos.x - body.half_size().x >= 0.0,
        "wall jump punched the player through the left wall (body left = {})",
        player.pos.x - body.half_size().x,
    );
}

/// Closer match to the actual reported bug: the player has a tiny
/// residual penetration into the left wall (sub-pixel rounding from
/// the previous frame's snap) and is moving away from it on
/// wall-jump. The horizontal sweep finds the wall at ToI=0; the snap
/// uses delta direction (+x → "block is to my right") and pushes the
/// player through the wall by `wall.left() - body.right() = -63`.
#[test]
fn wall_jump_does_not_catapult_player_off_wall_overlap() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let body = player.aabb();
    let left_wall_right = 36.0;
    // Body penetrates wall by 1 px on the x-axis, mid-height of the
    // room (no floor/ceiling overlap to confuse the issue).
    player.pos.x = left_wall_right + body.half_size().x - 1.0;
    player.pos.y = world.size.y * 0.5;
    player.vel = Vec2::new(500.0, -650.0); // wall-jump initial velocities
    player.on_ground = false;
    player.on_wall = false;
    player.wall_normal_x = 0.0;

    let initial_x = player.pos.x;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // After one frame the player should be sitting at body.left ≈
    // wall.right (the wall snap aligns them), or at most a few pixels
    // to the right (motion delta). They must not be teleported by
    // tens of pixels in any direction.
    let dx = (player.pos.x - initial_x).abs();
    assert!(
        dx < 30.0,
        "wall overlap caused horizontal teleport: dx={dx}, pos.x went from {initial_x} to {}",
        player.pos.x,
    );
    assert!(
        player.pos.x - body.half_size().x >= 0.0 - 0.5,
        "player was punched through the left wall: body left = {}",
        player.pos.x - body.half_size().x,
    );
}

/// Regression: reproduces the wall-cling → Grounded teleport captured
/// in `debug_traces/ambition_trace_1777903935-558508824-000000_*.json`.
/// The player wall-clings on a tall left-side wall (top at world y=0,
/// bottom at world's floor) and slides downward at `wall_slide_speed`.
/// Before the fix, the y-axis sweep would return `time_of_impact = 0`
/// on the wall (the body was edge-touching / fractionally penetrating
/// it), then unconditionally snap the body's bottom to the wall's TOP
/// edge — teleporting the player ~1700 px upward to
/// `y = 0 - half_height = -23`.
///
/// The fix filters dominantly-horizontal overlaps out of the y-sweep
/// and adds the symmetric guard to `resolve_vertical`. After the fix
/// the player either stays roughly where they were (continuing the
/// wall slide) or moves by at most one frame's worth of velocity.
#[test]
fn wall_cling_does_not_teleport_to_wall_top_on_y_sweep() {
    let world = test_world();
    // Wall-cling pose: edge-touching left wall (wall.right = 36),
    // mid-room vertically, with wall_slide_speed downward.
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let half = player.size * 0.5;
    let wall_right = 36.0;
    // 0.05 px penetration into the wall — within the kind of float
    // fuzz that survives between the x-sweep and the y-sweep.
    player.pos.x = wall_right + half.x - 0.05;
    player.pos.y = world.size.y * 0.5; // ~450, well inside the room
    player.vel = Vec2::new(0.0, DEFAULT_TUNING.wall_slide_speed);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;

    let initial_y = player.pos.y;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0, // pressing into the wall
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // Hard invariant: after one sim step the y position must still be
    // inside the world envelope, and the y delta must be bounded by
    // the velocity-budget plus a small slop. The pre-fix behavior
    // teleported to y ≈ -23 (about 470 px above start); the post-fix
    // behavior should be |dy| < 50 px.
    assert!(
            player.pos.y >= 0.0 && player.pos.y <= world.size.y,
            "wall-cling y-sweep teleported player out of the world envelope: pos.y = {} (world.size.y = {})",
            player.pos.y,
            world.size.y,
        );
    let dy = (player.pos.y - initial_y).abs();
    assert!(
            dy < 50.0,
            "wall-cling y-sweep moved player by {dy} px in one frame; expected at most a few pixels of slide",
        );
    // The player must not have transitioned to Grounded against a
    // surface that doesn't exist at this y. The bug snapped the body
    // bottom to the wall's TOP (y=0) and set on_ground=true.
    assert!(
        !player.on_ground,
        "wall-cling y-sweep falsely set on_ground; player was supposedly grounded at y={}",
        player.pos.y,
    );
}

/// Regression: player wall-clinging on a tall column whose top
/// is far above the player must NOT teleport upward when their
/// body partially overlaps the column on its bottom edge.
///
/// Concrete repro from the May 2026 mob_lab F8 trace: player at
/// (718, 419), body=(704, 396, 732, 442), wall-clinging on the
/// right face of a column at (704, 16, 720, 400). The body's
/// top corner (y=396) sticks 4 px above column.bottom (y=400),
/// so body and column strictly overlap in both axes. The y-sweep
/// found a TOI=0 hit on the column with delta.y ≈ 0.1 (tiny,
/// gravity-decelerated downward motion), and the falling-branch
/// snapped body.bottom to column.top (y=16) — teleporting the
/// player from y=419 to y=-7 (above the world's top edge).
///
/// Two guards prevent this:
/// 1. y-sweep predicate rejects blocks `start_body` already
///    strictly intersects (entrenched penetrations belong to
///    the x-resolver, not the y-sweep).
/// 2. The landing-from-above branch additionally requires
///    `prev_bottom <= block.top + tol`, mirroring the OneWay
///    landing test, so a downward-but-tiny delta near a far-away
///    block can't fire the snap.
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
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Reproduce the exact pre-OOB state from the trace.
    player.pos = Vec2::new(718.0, 419.0);
    player.vel = Vec2::new(0.0, 15.0); // gravity-decelerated tiny downward
    player.on_ground = false;
    player.on_wall = true;
    player.wall_clinging = true;
    player.wall_normal_x = -1.0;

    let start_y = player.pos.y;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            control_dt: 1.0 / 60.0,
            axis_x: -1.0, // pressing toward wall
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // The player must NOT have been catapulted across the room.
    // A normal frame's motion is single-digit pixels; anything more
    // than ~50 px is the bug.
    let dy = (player.pos.y - start_y).abs();
    assert!(
        dy < 50.0,
        "y-sweep teleported player by {} px; expected ~tiny gravity-driven motion (start_y={}, end_y={})",
        dy, start_y, player.pos.y,
    );
    // Sanity: still inside the world.
    assert!(
        player.pos.y > 0.0 && player.pos.y < world.size.y,
        "player ended OOB at y={}",
        player.pos.y,
    );
}

/// Guards against `body_is_side_contact` being too broad. Player
/// descending onto the *top corner* of a tall solid (a pillar) with
/// slight x overlap should still resolve as a normal landing —
/// `on_ground = true`, `pos.y` snaps so `body.bottom = pillar.top`.
/// If this test ever starts failing, the side-contact filter has
/// expanded into legitimate vertical-landing geometry.
#[test]
fn descending_onto_top_corner_of_tall_block_lands_normally() {
    // World with a tall pillar centered horizontally.
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
    // Pillar AABB: (380, 200) → (420, 600). Top = 200, bottom = 600.
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Position player so body slightly overlaps the pillar on x and is
    // about to land on its top: body x range covers ~[380-14+5, 380+5+14)
    // = [371, 405) with player half-width 14. With pos.x = 391,
    // body.left = 377 < pillar.left = 380, body.right = 405 > 380 →
    // x overlap of 25 px. body.top is well above pillar.top, body.bottom
    // is just above pillar.top.
    player.pos = Vec2::new(391.0, 200.0 - 23.0 - 0.5);
    // Falling straight down at a typical mid-arc speed.
    player.vel = Vec2::new(0.0, 200.0);
    player.on_ground = false;
    player.on_wall = false;

    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let body = player.aabb();
    assert!(
        player.on_ground,
        "descending onto pillar top should land (on_ground = true); got pos={:?}",
        player.pos
    );
    // body.bottom should be at or extremely near the pillar's top.
    assert!(
        (body.bottom() - 200.0).abs() < 1.0,
        "body.bottom should snap to pillar.top = 200; got {} (pos.y = {})",
        body.bottom(),
        player.pos.y,
    );
}

/// Direct unit test of `body_is_side_contact`. Both `sweep_player_y`
/// and `resolve_vertical` consult it to avoid the wall-cling teleport
/// class. The first revision used `overlap_x > 0` and missed the
/// exact-edge-touching case captured in
/// `debug_traces/ambition_trace_1777905256-*.json`; the predicate
/// now keys on the body's y-range being nested inside the block's
/// y-range, which catches edge-touching and penetrating side
/// contacts uniformly.
#[test]
fn body_is_side_contact_classifies_walls_vs_floors() {
    // Player about to land on a wide floor: body.top < floor.top,
    // so body's y-range is NOT nested inside floor's y-range. Not
    // a side contact.
    let body = Aabb::new(Vec2::new(50.0, 100.0), Vec2::new(14.0, 23.0));
    let floor = Aabb::new(Vec2::new(80.0, 125.0), Vec2::new(60.0, 6.0));
    assert!(
        !body_is_side_contact(body, floor),
        "player about to land on a wide floor must NOT be classified as a side contact"
    );

    // Tall left wall, body fully alongside it (body's y-range is
    // strictly inside the wall's y-range). Edge-touching on x.
    // Side contact regardless of x-overlap.
    let wall = Aabb::new(Vec2::new(18.0, 450.0), Vec2::new(18.0, 450.0));
    let body_alongside_edge = Aabb::new(Vec2::new(36.0 + 14.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_alongside_edge, wall),
        "body alongside a tall wall (edge-touching on x) must be a side contact"
    );

    // Same wall, body penetrating by 1 px on x. Still alongside on y.
    let body_inside_wall = Aabb::new(Vec2::new(36.0 + 14.0 - 1.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_inside_wall, wall),
        "body penetrating a tall wall on x is still a side contact"
    );

    // Player landing on the top corner of a tall block (small x
    // overlap, body.bottom near block.top, body.top above block.top).
    // The body's y-range is NOT nested inside the block's y-range
    // (body.top < block.top), so this is a real vertical contact —
    // NOT a side contact. Guards against the predicate becoming too
    // broad.
    let pillar = Aabb::new(Vec2::new(900.0, 800.0), Vec2::new(40.0, 200.0));
    let body_landing_on_pillar = Aabb::new(
        Vec2::new(900.0 - 40.0 + 5.0, 600.0 - 23.0 + 1.0),
        Vec2::new(14.0, 23.0),
    );
    assert!(
            !body_is_side_contact(body_landing_on_pillar, pillar),
            "descending onto the top edge of a tall block (slight x overlap, body.top above block.top) must NOT be classified as a side contact"
        );

    // Player jumping up into a thick ceiling block (body.bottom
    // crossing block.bottom from below). body.bottom > block.bottom
    // → not nested → real vertical contact.
    let ceiling = Aabb::new(Vec2::new(900.0, 200.0), Vec2::new(400.0, 100.0));
    let body_under_ceiling = Aabb::new(Vec2::new(900.0, 300.0 + 23.0 - 1.0), Vec2::new(14.0, 23.0));
    assert!(
            !body_is_side_contact(body_under_ceiling, ceiling),
            "rising into a thick ceiling (body.bottom poking past block.bottom) must NOT be classified as a side contact"
        );
}
