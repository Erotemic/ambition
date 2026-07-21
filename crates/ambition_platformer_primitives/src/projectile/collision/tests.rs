//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn world_with_block(kind: ae::BlockKind, center: ae::Vec2, half: ae::Vec2) -> ae::World {
    let mut w = ae::World::new("test", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![]);
    w.blocks.push(ae::Block {
        id: ae::GeoId::anon(),
        name: "wall".into(),
        aabb: ae::Aabb::new(center, half),
        kind,
        velocity: ae::Vec2::ZERO,
        art_color: None,
    });
    w
}

fn straight_projectile(pos: ae::Vec2) -> crate::projectile::ProjectileBody {
    let spec = crate::projectile::ProjectileSpec {
        origin: pos,
        direction: ae::Vec2::new(1.0, 0.0),
        damage: 1,
        speed: 200.0,
        max_lifetime: 1.0,
        half_extent: ae::Vec2::new(6.0, 6.0),
        gravity: 0.0,
        bounces: 0,
        world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
        charge_tier: 0,
    };
    let mut body = crate::projectile::ProjectileBody::from_spec(spec);
    body.game.bounces_remaining = 0; // baseline: no bouncing
    body
}

#[test]
fn enemy_policy_expires_on_solid_contact() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::ExpireOnContact,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn enemy_policy_expires_on_one_way_contact() {
    // The "enemy treats one-way as solid" rule is the whole
    // reason enemy/projectile world-collision can't share the
    // player policy directly. Pin it here.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::ExpireOnContact,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn player_policy_expires_on_solid_when_out_of_bounces() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    // bounces_remaining = 0 (straight shot)
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn player_policy_passes_through_one_way_at_zero_bounces() {
    // Player straight shot (0 bounces) travelling horizontally past a
    // thin one-way platform should NOT be stopped — pins the
    // asymmetry with enemy policy.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Continue));
}

/// A player projectile with bounce budget that lands on the support face of
/// a one-way platform should skip off it exactly like a solid support.
fn falling_player_projectile(pos: ae::Vec2, bounces: u8) -> crate::projectile::ProjectileBody {
    let mut body = straight_projectile(pos);
    body.kin.pos = pos;
    body.kin.vel = ae::Vec2::new(0.0, 80.0); // toward world +Y, normal down-gravity case
    body.game.bounces_remaining = bounces;
    body
}

fn cardinal_gravity_dirs() -> [ae::Vec2; 4] {
    [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ]
}

fn local_to_world(frame: ae::AccelerationFrame, local: ae::Vec2) -> ae::Vec2 {
    frame.to_world(local)
}

fn world_to_local(frame: ae::AccelerationFrame, world: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(world.dot(frame.side), world.dot(frame.down))
}

fn frame_world_with_block(
    gravity_dir: ae::Vec2,
    kind: ae::BlockKind,
    local_center: ae::Vec2,
    local_half: ae::Vec2,
) -> (ae::World, ae::AccelerationFrame) {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let origin = ae::Vec2::new(300.0, 300.0);
    let world_center = origin + local_to_world(frame, local_center);
    let world_half = frame.to_world_half(local_half);
    (world_with_block(kind, world_center, world_half), frame)
}

#[test]
fn player_projectile_bounce_is_frame_equivalent_on_solid_supports() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::Solid,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 50.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 48.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(-15.0, 80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_world_collision(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
        );
        assert!(
            matches!(outcome, WorldHitOutcome::Bounced { .. }),
            "solid support should bounce in frame {gravity_dir:?}: {outcome:?}"
        );
        let local_vel = world_to_local(frame, body.kin.vel);
        assert!((local_vel.x + 15.0).abs() < 1e-3);
        assert!(local_vel.y < 0.0);
    }
}

#[test]
fn player_projectile_bounce_is_frame_equivalent_on_one_way_supports() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::OneWay,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 94.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(20.0, 80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_world_collision(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
        );
        assert!(
            matches!(outcome, WorldHitOutcome::Bounced { .. }),
            "one-way support should bounce in frame {gravity_dir:?}: {outcome:?}"
        );
        let local_vel = world_to_local(frame, body.kin.vel);
        assert!(
            (local_vel.x - 20.0).abs() < 1e-3,
            "side velocity preserved: {local_vel:?}"
        );
        assert!(
            local_vel.y < 0.0,
            "local-down component should reflect away from support: {local_vel:?}"
        );
    }
}

#[test]
fn player_projectile_one_way_passthrough_is_frame_equivalent_from_feet_side() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::OneWay,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 106.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(0.0, -80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_world_collision(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
        );
        assert!(
            matches!(outcome, WorldHitOutcome::Continue),
            "one-way should pass through from feet side in frame {gravity_dir:?}: {outcome:?}"
        );
        assert_eq!(body.game.bounces_remaining, 2);
    }
}

#[test]
fn player_projectile_bounces_off_one_way_top_landing_with_budget() {
    // Thin one-way platform, top at y=96.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    // Falling onto the platform top, overlapping it from above.
    let mut body = falling_player_projectile(ae::Vec2::new(100.0, 94.0), 2);
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Bounced { .. }),
        "bouncing shot should skip off a one-way top like a floor; got {outcome:?}"
    );
    assert_eq!(
        body.game.bounces_remaining, 1,
        "a bounce should be consumed"
    );
    assert!(
        body.kin.vel.y < 0.0,
        "bounce should reflect velocity upward"
    );
}

#[test]
fn player_projectile_passes_through_one_way_from_below() {
    // Same platform, but the projectile rises into it from underneath
    // with full bounce budget — a one-way is non-solid from below, so
    // it must pass through rather than bounce.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 106.0));
    body.kin.vel = ae::Vec2::new(0.0, -80.0); // upward
    body.game.bounces_remaining = 2;
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Continue),
        "a one-way is non-solid from below; got {outcome:?}"
    );
    assert_eq!(
        body.game.bounces_remaining, 2,
        "passthrough must not spend a bounce"
    );
}

#[test]
fn player_projectile_bounces_off_solid_top_landing_with_budget() {
    // Skip-across-floor on a thick solid (parity with the one-way top
    // landing), to pin that both surfaces bounce identically.
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = falling_player_projectile(ae::Vec2::new(100.0, 48.0), 2);
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Bounced { .. }),
        "bouncing shot with budget should skip off a solid floor; got {outcome:?}"
    );
    assert_eq!(body.game.bounces_remaining, 1);
}

#[test]
fn no_contact_returns_continue() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(500.0, 500.0),
        ae::Vec2::new(10.0, 10.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_world_collision(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Continue));
}
