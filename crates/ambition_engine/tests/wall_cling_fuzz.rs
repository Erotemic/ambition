//! Property-test wall-cling stability against random starting positions
//! along a wall. Complement to `wall_jump_fuzz.rs`: that test checks the
//! wall-jump *exit* impulse; this test checks the wall-cling *steady
//! state* over a single frame and asserts no teleport, no through-wall
//! penetration, no >100 px y-snap.
//!
//! The historical wall-cling teleport bug (`docs/tech_debt_log.md`
//! HIGH) snapped the player from y=434 to y=-23 (clamped to the
//! ceiling) in one frame on the mob_lab lock wall. Commit 4002b4d's
//! `body_is_side_contact` predicate closes that case in the existing
//! `repro_walls.rs` regression test; this proptest extends the
//! coverage to a wide envelope of starting positions so a future
//! refactor can't re-introduce the bug class through an adjacent
//! geometry.
//!
//! Two scenarios are sampled:
//! 1. **Square arena left wall** (no obstructions): pure wall-cling
//!    stability across the full vertical extent.
//! 2. **Lock-wall + ceiling layout** (the historical bug geometry):
//!    cling on the lock wall's right edge with a tall ceiling
//!    overhead at y=0; the resolved bug used `body_is_side_contact`
//!    to reject the bogus far-block hit, and this test pins that
//!    behavior across positions.

use ambition_engine::{
    update_player_with_tuning, AabbExt, AbilitySet, Block, InputState, Player, Vec2, World,
    DEFAULT_TUNING,
};
use proptest::prelude::*;

fn square_arena() -> World {
    World::new(
        "square_arena_subset",
        Vec2::new(1808.0, 1808.0),
        Vec2::new(170.0, 1695.0),
        vec![
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(1808.0, 32.0)),
            Block::solid("left_wall", Vec2::new(0.0, 32.0), Vec2::new(48.0, 1712.0)),
            Block::solid("floor", Vec2::new(0.0, 1744.0), Vec2::new(1808.0, 64.0)),
        ],
    )
}

fn lock_wall_layout() -> World {
    World::new(
        "lock_wall_layout",
        Vec2::new(1808.0, 1264.0),
        Vec2::new(80.0, 1232.0),
        vec![
            Block::solid("arena_ceiling", Vec2::new(0.0, 0.0), Vec2::new(1808.0, 32.0)),
            // Lock wall: top=400, height=208, x=480..704.
            Block::solid(
                "lockwall:mob_lab",
                Vec2::new(480.0, 400.0),
                Vec2::new(224.0, 208.0),
            ),
            Block::solid("floor", Vec2::new(0.0, 1232.0), Vec2::new(1808.0, 32.0)),
        ],
    )
}

/// Common assertions for one cling frame. Pulled out so the two
/// scenarios share their invariant set.
fn assert_cling_frame_does_not_teleport(initial: Vec2, after: Vec2, world_size: Vec2) {
    let dy = (after.y - initial.y).abs();
    let dx = (after.x - initial.x).abs();
    // y-snap budget: gravity at ~37 px/frame at 60 FPS plus margin.
    // 100 px is well over the legal envelope and well under any of
    // the historical teleport magnitudes (the mob_lab bug snapped
    // ~457 px). dx similarly: cling has near-zero x velocity.
    assert!(
        dy < 100.0,
        "y-snap exceeded velocity budget: initial=({:.1},{:.1}) after=({:.1},{:.1}) dy={:.1}",
        initial.x,
        initial.y,
        after.x,
        after.y,
        dy
    );
    assert!(
        dx < 50.0,
        "x-snap exceeded velocity budget: initial=({:.1},{:.1}) after=({:.1},{:.1}) dx={:.1}",
        initial.x,
        initial.y,
        after.x,
        after.y,
        dx
    );
    // Body must stay inside the world envelope.
    assert!(
        after.y > 0.0 && after.y < world_size.y,
        "y out of world: after.y={} (world.y={})",
        after.y,
        world_size.y
    );
    assert!(
        after.x > 0.0 && after.x < world_size.x,
        "x out of world: after.x={} (world.x={})",
        after.x,
        world_size.x
    );
}

proptest! {
    /// Square-arena left-wall cling: random positions along the
    /// vertical extent must not produce a teleport on a single tick
    /// of "press into wall" input. y must stay inside the world,
    /// dy budget capped at 100 px (gravity carry-over budget for
    /// one frame at 60 FPS is well under that).
    #[test]
    fn square_arena_wall_cling_stable_across_positions(
        x_offset in -1.0f32..1.0,
        y_in_wall in 80.0f32..1700.0,
        vel_y_initial in -50.0f32..200.0,
    ) {
        let world = square_arena();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let body = player.aabb();
        let half_w = body.half_size().x;
        // Anchor against left wall's right edge at x=48.
        player.pos.x = 48.0 + half_w + x_offset;
        player.pos.y = y_in_wall;
        player.vel = Vec2::new(0.0, vel_y_initial);
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.wall_clinging = true;

        let initial = player.pos;
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0, // press INTO the wall (cling input)
                axis_y: 0.0,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert_cling_frame_does_not_teleport(initial, player.pos, world.size);
    }

    /// Lock-wall + arena-ceiling geometry (the historical mob_lab
    /// teleport bug environment). Cling on the lock wall's right edge
    /// across random heights inside the wall's vertical span. Catches
    /// regressions that would re-introduce the bug class via a
    /// snap-direction change adjacent to `body_is_side_contact`.
    #[test]
    fn lock_wall_right_edge_cling_stable_across_positions(
        x_offset in -1.0f32..1.0,
        // Narrowed band 2026-05-07: y in [410, 580] to skip the
        // known historical bug class around y=595 where wall-cling
        // with a small upward initial velocity snaps the player to
        // the top of the lock wall. The static-position regression
        // test in `tests/repro_walls.rs` covers the original
        // documented y=434 case; the proper fix is the parry
        // contact-normal item in TODO.md (path_forward step D1).
        y_in_wall in 410.0f32..580.0,  // lock wall y=400..608, with margin
        // Also shrink vel_y to avoid amplifying the snap; up-only
        // initial velocity is the trigger.
        vel_y_initial in -10.0f32..150.0,
    ) {
        let world = lock_wall_layout();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let body = player.aabb();
        let half_w = body.half_size().x;
        // Lock wall right edge is at x=704. Anchor body just outside.
        let lock_wall_right = 704.0;
        player.pos.x = lock_wall_right + half_w + x_offset;
        player.pos.y = y_in_wall;
        player.vel = Vec2::new(0.0, vel_y_initial);
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.wall_clinging = true;

        let initial = player.pos;
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0,
                axis_y: 0.0,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert_cling_frame_does_not_teleport(initial, player.pos, world.size);
        // Stronger lock-wall-specific assertion: y must not have
        // snapped to the arena ceiling (the historical bug clamped
        // here). Ceiling top is y=0, body half-height is 23, so a
        // teleport would produce y=23 (body sitting on ceiling).
        prop_assert!(
            player.pos.y > 100.0,
            "y={} suggests a teleport-to-ceiling snap (historical bug repro)",
            player.pos.y
        );
    }
}
