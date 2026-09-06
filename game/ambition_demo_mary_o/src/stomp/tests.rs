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

#[test]
fn a_gap_above_the_head_is_not_a_stomp() {
    // Head is y=284. Feet at 283 (one-pixel gap) and 269 (fifteen-pixel gap)
    // are both spatially separate, regardless of downward velocity.
    let falling = ae::Vec2::new(0.0, 200.0);
    assert_eq!(
        player_touch(body(), player_at(400.0, 259.0), falling),
        None,
        "a one-pixel gap is not contact"
    );
    assert_eq!(
        player_touch(body(), player_at(400.0, 245.0), falling),
        None,
        "the stomp tolerance cannot reach through empty space"
    );
}

/// RUNNING INTO A SHORT ENEMY ON FLAT GROUND IS A SIDE HIT, NOT A STOMP.
///
/// hit instead of her."*
///
/// She ran into it, the classifier said `Top`, and the snake shelled instead of hurting her.
/// Every existing case here used a 32-tall body and stayed green.
#[test]
fn running_into_a_short_enemy_on_flat_ground_hits_its_side() {
    const GROUND: f32 = 300.0;
    // 16 tall — exactly the old band, which is what made this misfire.
    let short_enemy = ae::Aabb::new(ae::Vec2::new(400.0, GROUND - 8.0), ae::Vec2::new(14.0, 8.0));
    // Her feet on the same ground, overlapping it, running (no vertical motion).
    let her = ae::Aabb::new(
        ae::Vec2::new(390.0, GROUND - 24.0),
        ae::Vec2::new(15.0, 24.0),
    );
    let running = ae::Vec2::new(180.0, 0.0);

    // The premise: they are actually touching, or the classification is moot.
    assert!(
        player_touch(short_enemy, her, running).is_some(),
        "the fixture does not overlap, so it classifies nothing"
    );
    assert_eq!(
        player_touch(short_enemy, her, running),
        Some(PlayerTouch::Side),
        "running into a short enemy from the side was classified as a stomp, so \
         the enemy takes the hit instead of her"
    );
}

/// The non-vacuity control: the SAME short enemy, stomped from above, is still a
/// stomp. Clamping the band must not make short enemies unstompable.
#[test]
fn the_same_short_enemy_can_still_be_stomped_from_above() {
    const GROUND: f32 = 300.0;
    let short_enemy = ae::Aabb::new(ae::Vec2::new(400.0, GROUND - 8.0), ae::Vec2::new(14.0, 8.0));
    // Feet just into its head, coming down.
    let her = ae::Aabb::new(
        ae::Vec2::new(400.0, GROUND - 16.0 - 24.0 + 2.0),
        ae::Vec2::new(15.0, 24.0),
    );
    let falling = ae::Vec2::new(0.0, 200.0);
    assert_eq!(
        player_touch(short_enemy, her, falling),
        Some(PlayerTouch::Top),
        "a short enemy must still be stompable from above, or the clamp traded \
         one bug for another"
    );
}
