//! Safe-position updates from body facts.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::Block;
use super::{remember_safe_player_position_from_kinematics, PlayerSafetyState, SafePositionContext};

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
) -> (
    ae::BodyClusterScratch,
    PlayerSafetyState,
) {
    let mut scratch =
        ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(world.spawn, ae::AbilitySet::sandbox_all());
    ae::refresh_movement_resources_clusters(
        &scratch.abilities,
        &mut scratch.dash,
        &mut scratch.jump,
        &mut scratch.dodge,
        ae::DEFAULT_TUNING.air_jumps,
        ae::RecoveryRefresh::Answered,
    );
    scratch.kinematics.pos = pos;
    scratch.ground.on_ground = true;
    // Force a known starting "safe pos" we can detect changes from.
    let safety = PlayerSafetyState::new(
        ae::Vec2::new(170.0, 1695.0),
    );
    (scratch, safety)
}

/// The OOB y=-23 position (above the world envelope) must NOT be recorded as safe even though
/// `on_ground` is true.
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
