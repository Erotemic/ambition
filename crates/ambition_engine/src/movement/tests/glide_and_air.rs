//! Glide cap, fast-fall double-tap signal, fly mode toggle, pogo orb
//! AABB feedback — anything air-borne that isn't a blink.

use super::super::*;
use super::{step, test_world};
use crate::geometry::AabbExt;
use crate::{AbilitySet, Vec2};

#[test]
fn glide_caps_fall_speed_while_jump_held() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.on_ground = false;
    // Drop the player into free fall well above any contact, with
    // velocity already above the glide cap so the cap clamp is the
    // only thing that can pull it back down.
    player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    player.vel = Vec2::new(0.0, 800.0);

    let events = step(
        &world,
        &mut player,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    let _ = events; // unused

    assert!(
        player.gliding,
        "hold-jump while falling should engage glide"
    );
    assert!(
        player.vel.y <= DEFAULT_TUNING.glide_fall_speed + 1.0,
        "glide cap should clamp fall speed; got {}",
        player.vel.y
    );
    assert!(
        player.vel.y < DEFAULT_TUNING.max_fall_speed * 0.5,
        "glide cap must be markedly below max_fall_speed; got {}",
        player.vel.y
    );
}

#[test]
fn glide_disengages_when_jump_released() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.on_ground = false;
    player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    player.vel = Vec2::new(0.0, 800.0);

    // Frame 1: held → glide engages
    step(
        &world,
        &mut player,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(player.gliding);

    // Frame 2: released → glide disengages, fall speed climbs back
    // toward max_fall_speed (gravity reapplied without the glide cap)
    step(&world, &mut player, InputState::default());
    assert!(!player.gliding);
}

#[test]
fn glide_requires_ability_flag() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.glide = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = false;
    player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    player.vel = Vec2::new(0.0, 800.0);

    step(
        &world,
        &mut player,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(
        !player.gliding,
        "glide should not engage when the ability flag is off"
    );
}

/// Multi-frame glide: hold-jump for 60 frames (1 second at
/// 60fps) — the player must keep gliding the whole time, with
/// vel.y staying near `glide_fall_speed` and the body not falling
/// out of the world. Catches a regression where `gliding` flips
/// off mid-flight (e.g. an off-by-one in the predicate or a
/// state mutation that clears the flag).
#[test]
fn glide_sustains_across_many_frames() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.on_ground = false;
    player.pos = Vec2::new(world.spawn.x, world.spawn.y - 800.0);
    player.vel = Vec2::new(0.0, 0.0);

    let dt = 1.0 / 60.0;
    for frame in 0..60 {
        step(
            &world,
            &mut player,
            InputState {
                jump_held: true,
                control_dt: dt,
                ..Default::default()
            },
        );
        if player.on_ground {
            break;
        }
        // After the first ~5 frames gravity has bumped vel.y past
        // the glide cap so the cap is actively clamping. Don't
        // assert on the very first frames where vel.y < cap.
        if frame >= 6 {
            assert!(
                player.gliding,
                "frame {frame}: gliding flipped off (vel=({},{}) on_ground={})",
                player.vel.x, player.vel.y, player.on_ground,
            );
            assert!(
                player.vel.y <= DEFAULT_TUNING.glide_fall_speed + 5.0,
                "frame {frame}: vel.y exceeded glide cap ({} > {})",
                player.vel.y,
                DEFAULT_TUNING.glide_fall_speed,
            );
        }
    }
}

#[test]
fn fast_fall_requires_double_tap_signal() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.on_ground = false;
    player.vel.y = 0.0;

    // Holding down is still useful for pogo / downward attack intent, but
    // should not automatically trigger fast-fall.
    step(
        &world,
        &mut player,
        InputState {
            axis_y: 1.0,
            ..Default::default()
        },
    );
    assert!(!player.fast_falling);

    // The presentation layer recognizes double-tap-down and sends this
    // explicit event to the engine.
    step(
        &world,
        &mut player,
        InputState {
            axis_y: 1.0,
            fast_fall_pressed: true,
            ..Default::default()
        },
    );
    assert!(player.fast_falling);
}

#[test]
fn fly_toggle_switches_mode_and_counters_gravity() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    assert!(!player.fly_enabled);
    let events = step(
        &world,
        &mut player,
        InputState {
            fly_toggle_pressed: true,
            ..Default::default()
        },
    );
    assert!(player.fly_enabled);
    assert!(events.operations.contains(&MovementOp::FlyToggle));
    player.on_ground = false;
    player.vel = Vec2::ZERO;
    step(
        &world,
        &mut player,
        InputState {
            axis_y: -1.0,
            ..Default::default()
        },
    );
    assert!(
        player.vel.y < 0.0,
        "flying upward input should accelerate upward"
    );
}

/// A successful pogo bounce records the orb's AABB on `FrameEvents`,
/// so the sandbox can route damage to a matching breakable pogo orb.
#[test]
fn pogo_bounce_records_orb_aabb_on_frame_events() {
    let mut world = test_world();
    let orb_center = Vec2::new(700.0, 600.0);
    world
        .blocks
        .push(crate::world::Block::pogo_orb("orb", orb_center, 18.0));

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Place the player just above the orb so a downward pogo press hits it.
    player.pos = Vec2::new(orb_center.x, orb_center.y - 24.0);
    player.vel = Vec2::ZERO;
    player.on_ground = false;

    let events = update_player_control_with_tuning(
        &world,
        &mut player,
        InputState {
            pogo_pressed: true,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        events.operations.contains(&MovementOp::Pogo),
        "expected MovementOp::Pogo to fire, got {:?}",
        events.operations
    );
    assert_eq!(events.pogo_hits.len(), 1, "{:?}", events.pogo_hits);
    let hit = events.pogo_hits[0];
    let dx = (hit.center().x - orb_center.x).abs();
    let dy = (hit.center().y - orb_center.y).abs();
    assert!(
        dx < 1.0 && dy < 1.0,
        "pogo_hit center {:?} != orb {:?}",
        hit.center(),
        orb_center
    );
}
