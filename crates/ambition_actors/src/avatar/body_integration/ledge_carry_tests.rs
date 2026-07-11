//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod ledge_carry_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::{ledge_platform_carry, LedgePlatformCarry};
use ambition_engine_core as ae;

fn world_with_right_wall() -> ae::World {
    ae::World::new(
        "ledge_carry_test",
        ae::Vec2::new(400.0, 400.0),
        ae::Vec2::new(20.0, 50.0),
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, 0.0),
            ae::Vec2::new(20.0, 400.0),
        )],
    )
}

fn player() -> ae::Aabb {
    ae::Aabb::new(ae::Vec2::new(80.0, 50.0), ae::Vec2::new(12.0, 20.0))
}

#[test]
fn carry_into_a_wall_knocks_the_player_off() {
    assert_eq!(
        ledge_platform_carry(&world_with_right_wall(), player(), ae::Vec2::new(30.0, 0.0)),
        LedgePlatformCarry::KnockOff,
    );
}

#[test]
fn carry_away_from_walls_rides_normally() {
    let world = world_with_right_wall();
    assert_eq!(
        ledge_platform_carry(&world, player(), ae::Vec2::new(-30.0, 0.0)),
        LedgePlatformCarry::Carry,
    );
    assert_eq!(
        ledge_platform_carry(&world, player(), ae::Vec2::new(5.0, 0.0)),
        LedgePlatformCarry::Carry,
    );
}
