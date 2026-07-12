//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod safe_pos_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;
use ambition_engine_core::Block;

fn dummy_world() -> ae::World {
    ae::World::new(
        "test",
        ae::Vec2::new(1800.0, 1800.0),
        ae::Vec2::new(170.0, 1695.0),
        vec![Block::solid(
            "left wall",
            ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(36.0, 1800.0),
        )],
    )
}

fn player_at(
    world: &ae::World,
    pos: ae::Vec2,
) -> (ae::BodyClusterScratch, crate::avatar::PlayerSafetyState) {
    let mut scratch =
        crate::avatar::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
    ae::refresh_movement_resources_clusters(
        &scratch.abilities,
        &mut scratch.dash,
        &mut scratch.jump,
        ae::DEFAULT_TUNING.air_jumps,
    );
    scratch.kinematics.pos = pos;
    scratch.ground.on_ground = true;
    // Force a known starting "safe pos" we can detect changes from.
    let safety = crate::avatar::PlayerSafetyState::new(ae::Vec2::new(170.0, 1695.0));
    (scratch, safety)
}

/// The OOB y=-23 position (above the world envelope) must NOT be
/// recorded as safe even though `on_ground` is true. This is the
/// invariant the wall-cling teleport bug violated for two consecutive
/// reproductions before the fix.
#[test]
fn rejects_position_above_world_envelope() {
    let world = dummy_world();
    let (player, mut safety) = player_at(&world, ae::Vec2::new(62.0, -23.0));
    let initial = safety.last_safe_pos;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        SafePositionContext::ideal(),
    );
    assert_eq!(
        safety.last_safe_pos, initial,
        "above-world position must not become last_safe_pos"
    );
}

/// The position update should fire when the player is grounded inside
/// the world envelope and not overlapping a Solid.
#[test]
fn accepts_legitimate_grounded_position() {
    let world = dummy_world();
    let (player, mut safety) = player_at(&world, ae::Vec2::new(200.0, 900.0));
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        SafePositionContext::ideal(),
    );
    assert_eq!(
        safety.last_safe_pos,
        ae::Vec2::new(200.0, 900.0),
        "a legal grounded position should be remembered"
    );
}

/// Even if the player is grounded somewhere legitimate, an in-flight
/// reset / damage / hitstun / blink / room transition must veto the
/// write so the safe pos doesn't drift while the player is being
/// teleported.
#[test]
fn vetoes_write_during_damage_or_reset() {
    let world = dummy_world();
    let (player, mut safety) = player_at(&world, ae::Vec2::new(200.0, 900.0));
    let initial = safety.last_safe_pos;

    let mut ctx = SafePositionContext::ideal();
    ctx.damaged_this_frame = true;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        ctx,
    );
    assert_eq!(safety.last_safe_pos, initial);

    let mut ctx = SafePositionContext::ideal();
    ctx.feature_requested_reset = true;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        ctx,
    );
    assert_eq!(safety.last_safe_pos, initial);

    let mut ctx = SafePositionContext::ideal();
    ctx.in_hitstun = true;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        ctx,
    );
    assert_eq!(safety.last_safe_pos, initial);

    let mut ctx = SafePositionContext::ideal();
    ctx.blink_grace_active = true;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        ctx,
    );
    assert_eq!(safety.last_safe_pos, initial);

    let mut ctx = SafePositionContext::ideal();
    ctx.room_transitioning = true;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        ctx,
    );
    assert_eq!(safety.last_safe_pos, initial);
}

/// A position INSIDE a Solid block is not safe even if `on_ground`
/// is true. Mirror of `classify_player_safety`'s InsideSolid case.
#[test]
fn rejects_position_inside_solid() {
    let world = dummy_world();
    // The left wall's right edge is at x=36; place the player center
    // at x=18 with half-width 14, body covers x=4..32 — fully inside
    // the wall.
    let (player, mut safety) = player_at(&world, ae::Vec2::new(18.0, 900.0));
    let initial = safety.last_safe_pos;
    remember_safe_player_position_from_kinematics(
        &mut safety,
        player.kinematics.pos,
        player.kinematics.vel,
        player.kinematics.aabb(),
        player.ground.on_ground,
        &world,
        SafePositionContext::ideal(),
    );
    assert_eq!(safety.last_safe_pos, initial);
}

#[test]
fn register_down_tap_returns_true_on_double_tap_within_window() {
    let mut interaction = crate::control::SlotGestures::default();
    // First tap: returns false, opens the window.
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
    // Tap again before window expires: returns true.
    assert!(interaction.register_down_tap(true, 0.05, 0.25));
}

#[test]
fn register_down_tap_window_closes_on_idle_frames() {
    let mut interaction = crate::control::SlotGestures::default();
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
    // Many idle frames — tap timer drains.
    for _ in 0..20 {
        let _ = interaction.register_down_tap(false, 0.05, 0.25);
    }
    // Next tap is treated as the FIRST tap (window had closed).
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
}

#[test]
fn register_up_tap_mirrors_down_tap_semantics() {
    let mut interaction = crate::control::SlotGestures::default();
    assert!(!interaction.register_up_tap(true, 0.0, 0.30));
    assert!(interaction.register_up_tap(true, 0.05, 0.30));
}

#[test]
fn buffered_interact_holds_for_window_seconds() {
    let mut interaction = crate::control::SlotGestures::default();
    // Press once → buffer holds for `window` seconds.
    assert!(interaction.buffered_interact(true, 0.0, 0.12));
    // Subsequent frames within the window also report true.
    assert!(interaction.buffered_interact(false, 0.05, 0.12));
    assert!(interaction.buffered_interact(false, 0.05, 0.12));
    // After the window passes, the buffer drains.
    assert!(!interaction.buffered_interact(false, 0.20, 0.12));
}

#[test]
fn clear_interact_buffer_drops_buffer_immediately() {
    let mut interaction = crate::control::SlotGestures::default();
    let _ = interaction.buffered_interact(true, 0.0, 1.0);
    // Without the clear, next frame would still report true.
    interaction.clear();
    assert!(!interaction.buffered_interact(false, 0.001, 1.0));
}
