//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod pogo_policy_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;

#[test]
fn pogo_target_policy_is_authored_pogo_or_rebound_only() {
    let rebound = BlockKind::Rebound {
        impulse: Vec2::ZERO,
    };
    let blink_wall = BlockKind::BlinkWall {
        tier: BlinkWallTier::Soft,
    };

    assert!(BlockKind::PogoOrb.is_pogo_target());
    assert!(rebound.is_pogo_target());
    assert!(!BlockKind::Solid.is_pogo_target());
    assert!(!BlockKind::OneWay.is_pogo_target());
    assert!(!blink_wall.is_pogo_target());
    assert!(!BlockKind::Hazard.is_pogo_target());
}
