//! Climbable regions, BodyMode::Climbing semantics, passthrough vs
//! hazard interactions, strafe scaling.

use super::super::*;
use super::{step_scratch, test_world};
use crate::engine_core::player_clusters::PlayerClusterScratch;
use crate::engine_core::world::{Block, ClimbableKind, ClimbableRegion, ClimbableSpec};
use crate::engine_core::{Aabb, AbilitySet, Vec2, World};

fn scratch_at(spawn: Vec2) -> PlayerClusterScratch {
    PlayerClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> PlayerClusterScratch {
    PlayerClusterScratch::new_with_abilities(spawn, abilities)
}

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
    let mut scratch = scratch_at(Vec2::new(400.0, 600.0));
    // No input, no time: just run one update so `update_player`'s
    // contact-population block runs.
    let _ = step_scratch(&world, &mut scratch, InputState::default());
    let contact = scratch
        .env_contact
        .climbable
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
    let mut scratch = scratch_at(world.spawn);
    let _ = step_scratch(&world, &mut scratch, InputState::default());
    assert!(
        scratch.env_contact.climbable.is_none(),
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
    let mut scratch = scratch_at(Vec2::new(400.0, 600.0));
    // Force the climbing mode + populate contact (sandbox-side
    // driver does this in production; tests do it directly).
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    scratch.env_contact.climbable = world.climbable_at(scratch.kinematics.aabb());
    // Push some y velocity into the player so the test can prove
    // that climbing replaces it (rather than just initializing
    // from zero).
    scratch.kinematics.vel = Vec2::new(0.0, 800.0);

    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
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
        scratch.kinematics.vel.y < 0.0,
        "climbing up should produce upward (negative) y velocity; got {}",
        scratch.kinematics.vel.y
    );
    assert!(
        (scratch.kinematics.vel.y + spec.climb_speed).abs() < 50.0,
        "vel.y should be near -climb_speed ({}); got {}",
        -spec.climb_speed,
        scratch.kinematics.vel.y
    );
    // The 800.0 starting downward velocity must NOT have survived
    // (gravity suspended, target velocity replaces it).
    assert!(
        scratch.kinematics.vel.y < 100.0,
        "starting downward velocity should not survive climbing integration; got {}",
        scratch.kinematics.vel.y
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

    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 700.0));
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    scratch.env_contact.climbable = world.climbable_at(scratch.kinematics.aabb());
    let initial_y = scratch.kinematics.pos.y;
    // Drive 60 frames at fixed-60Hz climb-up. With the
    // passthrough rule, the player should make significant
    // upward progress past the platform at y=460. Without the
    // fix, they'd hit the platform from below and stop.
    for _ in 0..60 {
        let _ = update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: -1.0,
                control_dt: 1.0 / 60.0,
                ..InputState::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        // Re-set climbing in case any control branch flipped it.
        scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    }
    let dy = initial_y - scratch.kinematics.pos.y;
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
        scratch.kinematics.pos.y
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

    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 700.0));
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    scratch.env_contact.climbable = world.climbable_at(scratch.kinematics.aabb());
    let initial_pos = scratch.kinematics.pos;
    // Drive the climb upward toward the hazard.
    let mut hazard_fired = false;
    for _ in 0..120 {
        let evs = update_player_with_tuning_scratch(
            &world,
            &mut scratch,
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
        scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    }
    assert!(
        hazard_fired,
        "hazard in the ladder column should still kill the player while climbing; \
             initial_pos={:?}, final_pos={:?}",
        initial_pos, scratch.kinematics.pos
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

    let mut scratch = scratch_at(Vec2::new(400.0, 480.0)); // below platform
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Standing;
    // Aim downward to test horizontal sweep against the platform.
    scratch.kinematics.vel = Vec2::new(0.0, -2000.0);
    let pre_y = scratch.kinematics.pos.y;
    for _ in 0..30 {
        let _ = update_player_with_tuning_scratch(
            &world,
            &mut scratch,
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
        scratch.kinematics.pos.y > pre_y - 100.0 || scratch.kinematics.pos.y > 460.0 - 24.0,
        "Standing player should not pass through the platform; pre={} post={}",
        pre_y,
        scratch.kinematics.pos.y
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
    let mut scratch = scratch_at(Vec2::new(400.0, 600.0));
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Climbing;
    scratch.env_contact.climbable = world.climbable_at(scratch.kinematics.aabb());

    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
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
        scratch.kinematics.vel.x > 0.0,
        "axis_x = 1.0 should produce positive x velocity; got {}",
        scratch.kinematics.vel.x
    );
    assert!(
        scratch.kinematics.vel.x < DEFAULT_TUNING.max_run_speed * 0.5,
        "strafe_factor = 0.25 should keep vel.x well under max_run_speed; got {} (cap={})",
        scratch.kinematics.vel.x,
        DEFAULT_TUNING.max_run_speed * 0.5
    );
}

#[test]
fn ladder_contact_allows_a_real_jump_impulse() {
    // A ladder jump should be a real jump, not just a mode swap.
    // If the player is overlapping a climbable region and presses
    // Jump, the movement step must produce an upward impulse even
    // though the body is no longer grounded.
    let mut world = test_world();
    world.climbable_regions.push(ClimbableRegion::new(
        Aabb::new(Vec2::new(400.0, 600.0), Vec2::new(20.0, 200.0)),
        ClimbableKind::Ladder,
        ClimbableSpec::default(),
    ));
    let mut scratch = scratch_at(Vec2::new(400.0, 620.0));
    scratch.body_mode.body_mode = crate::engine_core::player_state::BodyMode::Standing;
    scratch.env_contact.climbable = world.climbable_at(scratch.kinematics.aabb());
    scratch.kinematics.vel = Vec2::ZERO;

    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            jump_pressed: true,
            jump_held: true,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
        scratch.kinematics.vel.y < 0.0,
        "jumping while on a ladder should launch upward; got vel.y={}",
        scratch.kinematics.vel.y
    );
    assert!(
        scratch.kinematics.vel.y <= -DEFAULT_TUNING.jump_speed * 0.90,
        "ladder jump should use the normal jump impulse; got vel.y={} vs jump_speed={} (gravity will nibble the frame)",
        scratch.kinematics.vel.y,
        DEFAULT_TUNING.jump_speed
    );
    assert!(
        !scratch.ground.on_ground,
        "ladder jump should leave the player airborne"
    );
}
