use super::*;

/// The shared teleport rule (player blink and any actor body): full distance
/// over open space, stopped a body-half short of a wall, never embedding.
#[test]
fn blink_target_travels_full_distance_then_clamps_at_a_wall() {
    let half = ae::Vec2::new(12.0, 20.0);
    // Open world (no blocks): blink the full distance to the right.
    let empty = ae::World::new("t", ae::Vec2::new(2000.0, 600.0), ae::Vec2::ZERO, vec![]);
    let from = ae::Vec2::new(0.0, 0.0);
    let open = blink_target(&empty, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        (open.x - 150.0).abs() < 1e-3,
        "open blink travels full distance: {open:?}"
    );

    // A wall whose left face is at x=100 (Block::solid takes the MIN corner):
    // the body stops a half-width (+margin) short of it, never crossing in.
    let walled = ae::World::new(
        "t",
        ae::Vec2::new(2000.0, 600.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, -300.0),
            ae::Vec2::new(120.0, 600.0),
        )],
    );
    let clamped = blink_target(&walled, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        clamped.x + half.x <= 100.0 + 1e-3,
        "clamped blink must not cross the wall's left face at x=100: right edge={}",
        clamped.x + half.x
    );
    assert!(
        clamped.x > 0.0,
        "but it should still carry toward the wall: {clamped:?}"
    );
}
