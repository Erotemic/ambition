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
