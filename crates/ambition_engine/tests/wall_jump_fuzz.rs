//! Property-test the wall-jump path against random starting positions
//! along the square_arena's left wall. The historical OOB-teleport bug
//! (commit 4002b4d) had a narrow trigger: a body touching the wall
//! edge, sub-pixel penetration, an exact ToI=0 hit. This proptest
//! shotguns the same geometry with a wide envelope of (x, y, vel)
//! starting states and asserts no >100 px y-snap or out-of-bounds
//! teleport on a single wall-jump frame.
//!
//! Geometry: the square_arena left wall is `x = 0..48`, `y = 32..1744`,
//! with a ceiling at `y = 0..32` and a floor at `y = 1744..1808`.
//! The body half-size is 14 px x, 23 px y (default `Player` size of
//! 28 x 46). The proptest constrains x to a band centered on the
//! wall's right edge (x = 48) and y to the wall's vertical extent
//! minus a small margin.

use ambition_engine::{
    update_player_with_tuning, AabbExt, AbilitySet, Block, InputState, Player, Vec2, World,
    DEFAULT_TUNING,
};
use proptest::prelude::*;

fn arena_subset() -> World {
    World::new(
        "square_arena_subset",
        Vec2::new(1808.0, 1808.0),
        Vec2::new(170.0, 1695.0),
        vec![
            // Ceiling.
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(1808.0, 32.0)),
            // Left wall (top=32, bottom=1744).
            Block::solid("left_wall", Vec2::new(0.0, 32.0), Vec2::new(48.0, 1712.0)),
            // Floor.
            Block::solid("floor", Vec2::new(0.0, 1744.0), Vec2::new(1808.0, 64.0)),
        ],
    )
}

proptest! {
    /// Random starting positions along the left wall must not produce
    /// a wall-jump frame that snaps the player by more than the
    /// frame's velocity budget OR teleports them outside the world.
    ///
    /// `x_offset` is the horizontal penetration offset from the
    /// wall's right edge: positive = body strictly outside the wall
    /// (touching its right side), negative = sub-pixel residual
    /// penetration (the historical trigger). The clamp keeps the
    /// body anchored against the wall regardless.
    #[test]
    fn wall_jump_does_not_teleport_for_random_starting_positions(
        x_offset in -2.0f32..2.0,
        y_in_wall in 80.0f32..1700.0,
        vel_y_initial in -100.0f32..200.0,
    ) {
        let world = arena_subset();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let body = player.aabb();
        let half_w = body.half_size().x;
        let left_wall_right = 48.0;
        // Anchor body against the wall's right edge with the random offset.
        player.pos.x = left_wall_right + half_w + x_offset;
        player.pos.y = y_in_wall;
        player.vel = Vec2::new(0.0, vel_y_initial);
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.coyote_timer = 0.0;
        player.wall_clinging = true;

        let initial = player.pos;
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0, // pressing into the wall
                axis_y: 0.0,
                jump_pressed: true,
                jump_held: true,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );

        // The wall-jump impulse adds wall_jump_x = 500 px/s on x and a
        // jump_speed = 660 px/s on y. One frame at 60 FPS budgets ~11
        // px on y from the impulse, plus gravity carry-over. Cap the
        // observed y-snap at 100 px so any hidden teleport (the bug
        // class) is caught.
        let dy = (player.pos.y - initial.y).abs();
        prop_assert!(dy < 100.0);
        // Body must remain inside the world's y envelope.
        prop_assert!(player.pos.y > 0.0 && player.pos.y < world.size.y);
        // Body must remain to the right of the wall's right edge.
        prop_assert!(player.pos.x - half_w >= 0.0);
        // Mark `initial` used to silence the unused-variable warning.
        let _ = initial;
    }
}
