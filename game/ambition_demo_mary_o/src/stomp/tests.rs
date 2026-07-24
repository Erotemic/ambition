//! Geometry tests for the shared top/side contact rule.

use super::*;

/// A snake-sized body: 28 x 32 centered at (400, 300) — head at y = 284.
fn body() -> ae::Aabb {
    ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(14.0, 16.0))
}

/// A player: 30 x 48, so its feet are 24 below its center.
fn player_at(x: f32, y: f32) -> ae::Aabb {
    ae::Aabb::new(ae::Vec2::new(x, y), ae::Vec2::new(15.0, 24.0))
}

/// The classification every player-facing enemy branch reads.
#[test]
fn the_touch_classification_separates_a_stomp_from_a_side_hit() {
    let falling = ae::Vec2::new(0.0, 200.0);
    let at_rest = ae::Vec2::ZERO;

    // Feet (y + 24) right on the head at 284 → a stomp, falling OR resting.
    assert_eq!(
        player_touch(body(), player_at(400.0, 260.0), falling),
        Some(PlayerTouch::Top)
    );
    // Clear of it entirely: no touch.
    assert_eq!(player_touch(body(), player_at(600.0, 300.0), at_rest), None);
    // Beside it on the same ground: a side touch, whatever the velocity.
    assert_eq!(
        player_touch(body(), player_at(410.0, 300.0), at_rest),
        Some(PlayerTouch::Side)
    );
}

/// **THE bug, as geometry.** A player who has come to REST on a body is on top of
/// it. Both enemy rules used to demand a falling player (`vel.y > 0`), so standing
/// still on a body read as a SIDE contact — and a body you are standing on that
/// thinks you are beside it will hurt you for as long as you stand there.
#[test]
fn a_player_at_rest_on_a_head_is_stomping_it_not_touching_its_side() {
    assert_eq!(
        player_touch(body(), player_at(400.0, 260.0), ae::Vec2::ZERO),
        Some(PlayerTouch::Top),
        "standing on a body is a stomp — it must never be a side contact"
    );
}

/// Rising into the head from below is NOT a stomp: jumping up into an enemy hurts,
/// it does not squash it. Only the direction differs from the stomp case.
#[test]
fn rising_into_a_head_from_below_is_a_side_hit_not_a_stomp() {
    // Overlapping for real (feet at 289, past the head at 284), since
    // edge-touching boxes do not overlap.
    assert_eq!(
        player_touch(body(), player_at(400.0, 265.0), ae::Vec2::new(0.0, -200.0)),
        Some(PlayerTouch::Side)
    );
    assert_eq!(
        player_touch(body(), player_at(400.0, 265.0), ae::Vec2::new(0.0, 200.0)),
        Some(PlayerTouch::Top),
        "the same geometry falling is still a stomp"
    );
}
