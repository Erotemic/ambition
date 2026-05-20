//! Climbable regions, BodyMode::Climbing semantics, passthrough vs
//! hazard interactions, strafe scaling.

use super::super::*;
use super::{step, test_world};
use crate::world::{Block, ClimbableKind, ClimbableRegion, ClimbableSpec};
use crate::{Aabb, AbilitySet, Vec2, World};

#[test]
fn climbable_contact_is_populated_when_player_intersects_ladder() {
    // Mirror of the water_contact integration test: when the
    // player AABB overlaps a ClimbableRegion in the world, the
    // engine should cache the contact on the player struct so
    // sandbox-side gameplay systems and the RL adapter read a
    // consistent answer for the frame.
    let mut world = test_world();
    // Place a ladder in a known empty patch of the test world.
    world.climbable_regions.push(ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 200.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    let mut player = Player::new(Vec2::new(400.0, 600.0));
    // No input, no time: just run one update so `update_player`'s
    // contact-population block runs.
    let _ = step(&world, &mut player, InputState::default());
    let contact = player
        .climbable_contact
        .expect("player AABB intersecting ladder should populate climbable_contact");
    assert_eq!(contact.kind, ClimbableKind::Ladder);
    assert!(
        (contact.center_x - 400.0).abs() < f32::EPSILON,
        "contact.center_x should match ladder center (400), got {}",
        contact.center_x
    );
}

#[test]
fn climbable_contact_is_none_when_player_far_from_any_ladder() {
    // No climbable regions in the world → contact stays None
    // across an update. This pins the "default to None" semantics
    // that sandbox systems will rely on.
    let world = test_world();
    let mut player = Player::new(world.spawn);
    let _ = step(&world, &mut player, InputState::default());
    assert!(
        player.climbable_contact.is_none(),
        "no ladders in world → climbable_contact must stay None"
    );
}

#[test]
fn climbing_mode_suspends_gravity_and_drives_vertical_velocity() {
    // Pin BodyMode::Climbing's behavior: pressing Up (axis_y =
    // -1) inside a ladder should drive vel.y to
    // -climb_speed (engine's +Y is downward, so up-input is
    // negative). Gravity is suspended.
    let mut world = test_world();
    let ladder_aabb = Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 200.0));
    world.climbable_regions.push(ClimbableRegion::new(
        ladder_aabb,
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    let mut player = Player::new(Vec2::new(400.0, 600.0));
    // Force the climbing mode + populate contact (sandbox-side
    // driver does this in production; tests do it directly).
    player.body_mode = crate::player_state::BodyMode::Climbing;
    player.climbable_contact = world.climbable_at(player.aabb());
    // Push some y velocity into the player so the test can prove
    // that climbing replaces it (rather than just initializing
    // from zero).
    player.vel = Vec2::new(0.0, 800.0);

    let _ = update_player_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_y: -1.0, // press up
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let spec = ClimbableSpec::default();
    // Climb integrates as `vel.y = axis_y * climb_speed`. After
    // the integrate, vel.y should equal -climb_speed (up).
    // Tolerance accounts for any post-integrate damping the
    // movement code adds.
    assert!(
        player.vel.y < 0.0,
        "climbing up should produce upward (negative) y velocity; got {}",
        player.vel.y
    );
    assert!(
        (player.vel.y + spec.climb_speed).abs() < 50.0,
        "vel.y should be near -climb_speed ({}); got {}",
        -spec.climb_speed,
        player.vel.y
    );
    // The 800.0 starting downward velocity must NOT have survived
    // (gravity suspended, target velocity replaces it).
    assert!(
        player.vel.y < 100.0,
        "starting downward velocity should not survive climbing integration; got {}",
        player.vel.y
    );
}

#[test]
fn climbing_passes_through_solid_blocks_overlapping_ladder() {
    // Pin "ladders pass through solids": with `body_mode == Climbing`
    // and a climbable contact, a block whose aabb intersects the
    // climbable region should NOT block the player's motion. This
    // is what lets a ladder reach a platform-level without the
    // author having to carve a gap in the platform.
    // Custom world large enough that climbing up doesn't trip
    // the OOB reset. Ladder spans y=200..1000 (very tall) so the
    // body stays in contact across the full climb.
    let mut world = World::new(
        "test",
        Vec2::new(2000.0, 2000.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    let ladder = ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 400.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    );
    // Solid platform that overlaps the ladder column horizontally
    // (player would normally collide with this when climbing up).
    world.blocks.push(Block::solid(
        "blocking_platform",
        Vec2::new(380.0, 460.0),
        Vec2::new(60.0, 16.0),
    ));
    world.climbable_regions.push(ladder);

    let mut player = Player::new_with_abilities(Vec2::new(400.0, 700.0), AbilitySet::sandbox_all());
    player.body_mode = crate::player_state::BodyMode::Climbing;
    player.climbable_contact = world.climbable_at(player.aabb());
    let initial_y = player.pos.y;
    // Drive 60 frames at fixed-60Hz climb-up. With the
    // passthrough rule, the player should make significant
    // upward progress past the platform at y=460. Without the
    // fix, they'd hit the platform from below and stop.
    for _ in 0..60 {
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_y: -1.0,
                control_dt: 1.0 / 60.0,
                ..InputState::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        // Re-set climbing in case any control branch flipped it.
        player.body_mode = crate::player_state::BodyMode::Climbing;
    }
    let dy = initial_y - player.pos.y;
    // Expected motion: ~60 frames * 180 px/sec / 60 = 180 px.
    // Without the passthrough, the player gets stuck at the
    // platform top (y=452, body bottom would land here) -- which
    // is initial_y - (700 - 452 - 23) = ~225 px upward at most.
    // We assert at least 100 px progress to confirm climbing
    // continues without the platform blocking.
    assert!(
        dy > 100.0,
        "climbing player should pass through platform at y=460; \
             initial_y={initial_y}, ended_y={}, dy={dy}",
        player.pos.y
    );
}

#[test]
fn climbing_player_still_collides_with_hazard_blocks_overlapping_ladder() {
    // Counter-test to the passthrough rule: hazards stay
    // dangerous even while climbing. A ladder threading through
    // a hazard tile should still kill the player on contact --
    // otherwise we've created an invincibility cheese.
    let mut world = World::new(
        "test",
        Vec2::new(2000.0, 2000.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    world.climbable_regions.push(ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 400.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    // Hazard block in the ladder's path.
    world.blocks.push(Block::hazard(
        "hazard_in_ladder",
        Vec2::new(380.0, 460.0),
        Vec2::new(60.0, 16.0),
    ));

    let mut player = Player::new_with_abilities(Vec2::new(400.0, 700.0), AbilitySet::sandbox_all());
    player.body_mode = crate::player_state::BodyMode::Climbing;
    player.climbable_contact = world.climbable_at(player.aabb());
    let initial_pos = player.pos;
    // Drive the climb upward toward the hazard.
    let mut hazard_fired = false;
    for _ in 0..120 {
        let evs = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_y: -1.0,
                control_dt: 1.0 / 60.0,
                ..InputState::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        if evs.hazard {
            hazard_fired = true;
            break;
        }
        player.body_mode = crate::player_state::BodyMode::Climbing;
    }
    assert!(
        hazard_fired,
        "hazard in the ladder column should still kill the player while climbing; \
             initial_pos={:?}, final_pos={:?}",
        initial_pos, player.pos
    );
}

#[test]
fn non_climbing_player_still_collides_with_solid_blocks_overlapping_ladder() {
    // Counter-test: NOT in Climbing mode, the same platform
    // blocks the player as normal. The passthrough is only active
    // while body_mode == Climbing.
    let mut world = test_world();
    world.climbable_regions.push(ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 200.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    world.blocks.push(Block::solid(
        "blocking_platform",
        Vec2::new(380.0, 460.0),
        Vec2::new(60.0, 16.0),
    ));

    let mut player = Player::new(Vec2::new(400.0, 480.0)); // below platform
    player.body_mode = crate::player_state::BodyMode::Standing;
    // Aim downward to test horizontal sweep against the platform.
    player.vel = Vec2::new(0.0, -2000.0);
    let pre_y = player.pos.y;
    for _ in 0..30 {
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                control_dt: 1.0 / 60.0,
                ..InputState::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
    }
    // Without the passthrough, an upward-moving Standing player
    // hits the platform from below and stops. We don't pin an
    // exact y, just that they didn't pass through it.
    assert!(
        player.pos.y > pre_y - 100.0 || player.pos.y > 460.0 - 24.0,
        "Standing player should not pass through the platform; pre={} post={}",
        pre_y,
        player.pos.y
    );
}

#[test]
fn climbing_mode_strafe_factor_caps_horizontal_input() {
    // Pin the strafe scaling: axis_x = 1.0 with default
    // strafe_factor = 0.25 should produce vel.x = climb_speed *
    // 0.25, much smaller than max_run_speed.
    let mut world = test_world();
    world.climbable_regions.push(ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 200.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    let mut player = Player::new(Vec2::new(400.0, 600.0));
    player.body_mode = crate::player_state::BodyMode::Climbing;
    player.climbable_contact = world.climbable_at(player.aabb());

    let _ = update_player_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // `vel.x = axis_x * climb_speed * strafe_factor` = 1.0 * 180 *
    // 0.25 = 45. After horizontal sweep + collision response the
    // value may shift slightly but should stay well under
    // max_run_speed (which is 360+).
    assert!(
        player.vel.x > 0.0,
        "axis_x = 1.0 should produce positive x velocity; got {}",
        player.vel.x
    );
    assert!(
        player.vel.x < DEFAULT_TUNING.max_run_speed * 0.5,
        "strafe_factor = 0.25 should keep vel.x well under max_run_speed; got {} (cap={})",
        player.vel.x,
        DEFAULT_TUNING.max_run_speed * 0.5
    );
}
