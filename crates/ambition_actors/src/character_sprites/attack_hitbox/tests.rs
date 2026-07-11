//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn collision() -> ae::Vec2 {
    ae::Vec2::new(30.0, 48.0)
}

/// Screen-down gravity (`(0,1)`) — the upright reference frame.
fn down() -> ae::Vec2 {
    ae::Vec2::new(0.0, 1.0)
}

fn player_box(facing: f32) -> ae::Aabb {
    player_attack_hitbox_world(
        "attack_side",
        ae::Vec2::new(0.0, 0.0),
        collision(),
        facing,
        down(),
    )
    .expect("player_robot/attack_side has an authored manifest hitbox")
    .bounds()
}

/// REGRESSION (Jon's gravity report): the manifest attack hitbox is authored
/// in the sprite's screen frame, but the swing happens in the BODY's gravity
/// frame — so the damage box MUST covary with gravity exactly as the slash VFX
/// does (`AttackSpec::into_world_frame`), or the polygon points one way while
/// the VFX points another (the bug: VFX correct, "atk" polygon wrong under
/// every non-down gravity). This pins that covariance: the hitbox offset under
/// gravity `g` is the screen-down offset rotated into `g`'s frame.
#[test]
fn attack_hitbox_covaries_with_gravity_like_the_slash_vfx() {
    let body = ae::Vec2::new(100.0, 100.0);
    let center = |g: ae::Vec2| {
        let b = player_attack_hitbox_world("attack_side", body, collision(), 1.0, g)
            .expect("attack_side authored")
            .bounds();
        (b.min + b.max) * 0.5
    };
    let down_off = center(down()) - body;
    for g in [
        ae::Vec2::new(0.0, -1.0), // screen-up
        ae::Vec2::new(1.0, 0.0),  // screen-right
        ae::Vec2::new(-1.0, 0.0), // screen-left
    ] {
        let off = center(g) - body;
        let expected = ae::AccelerationFrame::new(g).to_world(down_off);
        assert!(
            (off - expected).length() < 1.0,
            "gravity {g:?}: hitbox offset {off:?} should be the down offset \
             {down_off:?} rotated into the gravity frame ({expected:?}) — \
             the box must track gravity like the slash VFX",
        );
    }
}

#[test]
fn player_attack_side_reaches_forward_starts_in_body_and_is_tall() {
    let body_right = collision().x * 0.5; // +15
    let aabb = player_box(1.0);
    // Reaches well forward, PAST the body, to surround the slash effect.
    assert!(
        aabb.max.x > body_right + collision().x,
        "hitbox should reach well forward of the body (max.x {} > {})",
        aabb.max.x,
        body_right + collision().x
    );
    // Starts a bit INSIDE the body (back edge left of the body's right edge),
    // not disjoint in front — the authored hull begins within the player.
    assert!(
        aabb.min.x < body_right,
        "hitbox should start inside the body (min.x {} < {})",
        aabb.min.x,
        body_right
    );
    // At least as tall as the player body.
    let height = aabb.max.y - aabb.min.y;
    assert!(
        height >= collision().y,
        "hitbox should be at least body-height ({height} >= {})",
        collision().y
    );
}

#[test]
fn player_attack_side_mirrors_with_facing() {
    let body_left = -collision().x * 0.5; // -15
    let aabb = player_box(-1.0);
    // Left-facing reaches well forward to the LEFT, past the body.
    assert!(
        aabb.min.x < body_left - collision().x,
        "left-facing hitbox should reach forward on the LEFT (min.x {} < {})",
        aabb.min.x,
        body_left - collision().x
    );
}

#[test]
fn player_attack_side_is_an_authored_convex_blade() {
    // The robot's attack_side authors a poly (blade arc), so the player
    // slash resolves a Convex volume — not a box.
    let vol = player_attack_hitbox_world("attack_side", ae::Vec2::ZERO, collision(), 1.0, down())
        .expect("attack_side authored");
    assert!(
        matches!(vol, ae::CombatVolume::Convex { .. }),
        "expected a Convex blade, got {vol:?}"
    );
}

#[test]
fn actor_attack_hitbox_resolves_an_authored_enemy_blade() {
    // The robot enemy (character_id "robot") authors an `attack_side` hitbox
    // in its sheet, so the actor-neutral path resolves a real box instead of
    // the hardcoded fallback — the unification payoff: an enemy swings the
    // authored blade you see in `debug-hitboxes`, not magic numbers.
    let aabb = actor_attack_hitbox_world(
        "robot",
        "attack_side",
        ae::Vec2::new(0.0, 0.0),
        collision(),
        1.0,
        down(),
    );
    assert!(
        aabb.is_some(),
        "robot/attack_side should resolve an authored manifest hitbox"
    );
}

#[test]
fn actor_attack_hitbox_is_none_for_unknown_character() {
    assert!(actor_attack_hitbox_world(
        "definitely_not_a_character",
        "attack_side",
        ae::Vec2::ZERO,
        collision(),
        1.0,
        down(),
    )
    .is_none());
}

/// The seam-facing resolver resolves the REAL authored player blade for
/// `attack_side` (the assertion the combat-side moveset test delegates
/// here — combat tests the seam with a fixture; the DATA lives with the
/// sprites).
#[test]
fn seam_resolver_resolves_the_authored_player_blade() {
    let volume = authored_attack_volume_resolver(
        None,
        "attack_side",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        1.0,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(volume, Some(ae::CombatVolume::Convex { .. })),
        "the player manifest authors a convex attack_side blade, got {volume:?}"
    );
}
