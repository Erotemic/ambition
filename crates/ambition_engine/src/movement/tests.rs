use super::*;
use crate::geometry::AabbExt;
use crate::world::{BlinkWallTier, Block};
use crate::{Aabb, AbilitySet, Vec2};

fn step(world: &World, player: &mut Player, input: InputState) -> FrameEvents {
    update_player_with_tuning(world, player, input, 1.0 / 60.0, DEFAULT_TUNING)
}

fn test_world() -> World {
    let w = 1600.0;
    let h = 900.0;
    World {
        name: "movement test world".to_string(),
        size: Vec2::new(w, h),
        spawn: Vec2::new(210.0, h - 95.0),
        blocks: vec![
            Block::solid("floor", Vec2::new(0.0, h - 48.0), Vec2::new(w, 48.0)),
            Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(36.0, h)),
            Block::solid("right wall", Vec2::new(w - 36.0, 0.0), Vec2::new(36.0, h)),
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(w, 24.0)),
        ],
        objects: Vec::new(),
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    }
}

#[test]
fn tiny_dt_preserves_bullet_time_scale() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.vel = Vec2::ZERO;
    let _ = update_player_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    let normal_fall_speed = player.vel.y;

    let mut slow_player = Player::new(world.spawn);
    slow_player.on_ground = false;
    slow_player.coyote_timer = 0.0;
    slow_player.vel = Vec2::ZERO;
    let _ = update_player_with_tuning(
        &world,
        &mut slow_player,
        InputState::default(),
        (1.0 / 60.0) * 0.001,
        DEFAULT_TUNING,
    );

    assert!(slow_player.vel.y > 0.0);
    assert!(
        slow_player.vel.y < normal_fall_speed * 0.01,
        "tiny dt should not be clamped up to normal-ish gravity"
    );
}

#[test]
fn control_clock_can_aim_blink_while_sim_clock_is_nearly_frozen() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.vel = Vec2::ZERO;

    // Real-time control crosses the precision-blink threshold.
    for i in 0..8 {
        let _ = update_player_control_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_pressed: i == 0,
                blink_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
    }
    assert!(
        player.blink_aiming,
        "control time should enter precision aim quickly"
    );

    // Game-time simulation is almost frozen, so gravity should barely change.
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        (1.0 / 60.0) * 0.000035,
        DEFAULT_TUNING,
    );
    assert!(
        player.vel.y < 0.01,
        "player gravity must use scaled game time while control remains real-time; got {}",
        player.vel.y
    );
}

#[test]
fn held_blink_arms_when_cooldown_clears_without_new_press() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.blink_cooldown = 0.02;

    // Pressing slightly early should not arm yet.
    let _ = update_player_control_with_tuning(
        &world,
        &mut player,
        InputState {
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(!player.blink_hold_active);

    // Cooldown clears in simulation time.
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        0.03,
        DEFAULT_TUNING,
    );
    assert_eq!(player.blink_cooldown, 0.0);

    // The user is still holding the button, so control time can arm blink
    // without requiring another just-pressed edge.
    let _ = update_player_control_with_tuning(
        &world,
        &mut player,
        InputState {
            blink_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(player.blink_hold_active);
}

#[test]
fn double_jump_ability_controls_air_jump() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.double_jump = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.air_jumps_available = 0;
    let events = step(
        &world,
        &mut player,
        InputState {
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert!(!events.operations.contains(&MovementOp::DoubleJump));

    abilities.double_jump = true;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = false;
    player.coyote_timer = 0.0;
    player.air_jumps_available = 1;
    let events = step(
        &world,
        &mut player,
        InputState {
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert!(events.operations.contains(&MovementOp::DoubleJump));
}

#[test]
fn double_dash_ability_controls_dash_charges() {
    let world = test_world();
    let mut single_dash = AbilitySet::sandbox_all();
    single_dash.double_dash = false;
    let player = Player::new_with_abilities(world.spawn, single_dash);
    assert_eq!(player.dash_charges_available, 1);

    let player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    assert_eq!(player.dash_charges_available, 2);
}

#[test]
fn wall_climb_requires_wall_cling() {
    let mut abilities = AbilitySet::sandbox_all();
    abilities.wall_cling = false;
    assert!(abilities
        .compatibility_warnings()
        .iter()
        .any(|w| w.contains("wall_climb")));
}

#[test]
fn blink_ability_gates_teleport() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.blink = false;
    abilities.precision_blink = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    let start = player.pos;
    let input = InputState {
        axis_x: 1.0,
        blink_pressed: true,
        blink_held: true,
        ..Default::default()
    };
    let _ =
        update_player_control_with_tuning(&world, &mut player, input, 1.0 / 60.0, DEFAULT_TUNING);
    let input = InputState {
        axis_x: 1.0,
        blink_released: true,
        ..Default::default()
    };
    let events =
        update_player_control_with_tuning(&world, &mut player, input, 1.0 / 60.0, DEFAULT_TUNING);
    assert_eq!(player.pos, start);
    assert!(events.blinks.is_empty());
}

#[test]
fn quick_blink_moves_on_release() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let start = player.pos;
    step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        },
    );
    let events = step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
    );
    assert!(player.pos.x > start.x + 20.0);
    assert_eq!(events.blinks.len(), 1);
    assert!(!events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::Blink));
}

#[test]
fn held_blink_enters_precision_aiming() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    for _ in 0..20 {
        let blink_pressed = !player.blink_hold_active;
        step(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_held: true,
                blink_pressed,
                ..Default::default()
            },
        );
    }
    assert!(player.blink_aiming);
    let events = step(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
    );
    assert_eq!(events.blinks.len(), 1);
    assert!(events.blinks[0].precision);
    assert!(events.operations.contains(&MovementOp::PrecisionBlink));
}

#[test]
fn one_way_platform_requires_down_plus_jump_to_drop_through() {
    let mut world = test_world();
    // One-way platform suspended above the floor. Player will land on it
    // from above and we expect plain "down" alone to keep them resting.
    let plat_top_y = 600.0;
    world.blocks.push(Block::one_way(
        "drop test platform",
        Vec2::new(360.0, plat_top_y),
        Vec2::new(180.0, 12.0),
    ));

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.pos = Vec2::new(450.0, plat_top_y - player.size.y * 0.5);
    player.vel = Vec2::ZERO;
    player.on_ground = false;

    // Settle onto the platform.
    for _ in 0..6 {
        step(&world, &mut player, InputState::default());
    }
    assert!(player.on_ground, "player should land on the one-way");
    let resting_y = player.pos.y;

    // Holding down alone must NOT drop through anymore.
    for _ in 0..6 {
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        (player.pos.y - resting_y).abs() < 1.0,
        "down-alone must not drop through one-way (moved {} px)",
        player.pos.y - resting_y
    );

    // Down + jump (with the explicit drop_through_pressed gesture) drops.
    // Critically the gesture only fires for one frame: the presentation
    // layer recomputes drop_through_pressed each frame from
    // `axis_y > 0.35 && jump_pressed`, and `jump_pressed` is just-pressed,
    // so subsequent frames see drop_through_pressed=false. The engine must
    // latch the drop-through internally for long enough to clear the
    // landing-tolerance band.
    step(
        &world,
        &mut player,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            drop_through_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..10 {
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                // jump_pressed and drop_through_pressed are NOT held: this
                // is exactly the input shape the sandbox produces after
                // the initial press.
                ..Default::default()
            },
        );
    }
    assert!(
        player.pos.y > resting_y + 12.0,
        "down+jump should drop the player below the one-way (delta {})",
        player.pos.y - resting_y
    );
}

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
fn repeated_blinks_clamp_downward_velocity_each_time() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.pos = Vec2::new(420.0, 620.0);

    for _ in 0..2 {
        player.vel = Vec2::new(25.0, 900.0);
        player.blink_cooldown = 0.0;
        player.blink_hold_active = true;
        player.blink_aiming = false;
        let events = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_released: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert_eq!(events.blinks.len(), 1);
        assert!(
            player.vel.y
                <= DEFAULT_TUNING.blink_max_downward_speed + DEFAULT_TUNING.gravity / 60.0 + 1.0,
            "blink should not preserve a large downward fall speed; got {}",
            player.vel.y
        );
        assert!(player.blink_grace_timer > 0.0);
    }
}

#[test]
fn post_blink_grace_suspends_gravity_for_tiny_window() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.pos = Vec2::new(420.0, 620.0);
    player.vel = Vec2::new(0.0, 900.0);
    player.blink_hold_active = true;
    let _events = update_player_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    let after_blink_vy = player.vel.y;
    let _events = update_player_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 240.0,
        DEFAULT_TUNING,
    );
    assert!(
        player.vel.y <= after_blink_vy + 0.1,
        "gravity should be suspended during the short post-blink grace window"
    );
}

#[test]
fn blink_walls_can_be_passed_by_upgrade_without_allowing_solid_walls() {
    let mut world = test_world();
    world.blocks.clear();
    world.blocks.push(Block::blink_wall(
        "test soft blink membrane",
        Vec2::new(220.0, 0.0),
        Vec2::new(22.0, 300.0),
        BlinkWallTier::Soft,
    ));

    let mut blocked_abilities = AbilitySet::basic();
    blocked_abilities.blink = true;
    let blocked_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), blocked_abilities);
    let blocked = blink_destination_to_point(&world, &blocked_player, Vec2::new(340.0, 140.0));
    assert!(blocked.x < 220.0);

    let mut pass_abilities = blocked_abilities;
    pass_abilities.blink_through_soft_walls = true;
    let pass_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), pass_abilities);
    let passed = blink_destination_to_point(&world, &pass_player, Vec2::new(340.0, 140.0));
    assert!(passed.x > 300.0);
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
    world.blocks.push(Block::pogo_orb("orb", orb_center, 18.0));

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

/// Wall-jumping off the left wall while the player's body slightly
/// overlaps a wide horizontal block (floor/ceiling) must not catapult
/// the player out the opposite side of the room.
///
/// Reproduction in the square_arena: player is wall-clinging the left
/// wall low enough that their feet still poke into the floor block.
/// `resolve_axis(Axis::X)` saw the residual floor overlap and tried to
/// resolve it *horizontally* — the floor block spans the whole room,
/// so its left edge is at x=0, which produced a single-frame push
/// equal to the negative of the player's right edge (~58 pixels left)
/// and dumped the player at negative x.
#[test]
fn wall_jump_does_not_catapult_through_left_wall() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());

    // Park the player against the left wall with a tiny overlap into the
    // floor (1 pixel deep) — the kind of residual penetration the engine
    // tolerates between sweeps.
    let body = player.aabb();
    let left_wall_right = 36.0;
    let floor_top = world.size.y - 48.0;
    player.pos.x = left_wall_right + body.half_size().x; // touching wall on its right edge
    player.pos.y = floor_top - body.half_size().y + 1.0; // bottom 1 px below floor top
    player.vel = Vec2::ZERO;
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.coyote_timer = 0.0;

    let initial_x = player.pos.x;
    let _ = update_player_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0,
            axis_y: 0.0,
            jump_pressed: true,
            jump_held: true,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // After one wall-jump frame the player should be drifting *right*
    // (away from the wall) or at worst still touching it — never past
    // the wall's right edge in the negative-x direction by tens of
    // pixels.
    assert!(
            player.pos.x >= initial_x - 1.0,
            "wall jump pushed player to x={} from x={} — expected to stay near or right of starting position",
            player.pos.x,
            initial_x,
        );
    assert!(
        player.pos.x - body.half_size().x >= 0.0,
        "wall jump punched the player through the left wall (body left = {})",
        player.pos.x - body.half_size().x,
    );
}

/// Closer match to the actual reported bug: the player has a tiny
/// residual penetration into the left wall (sub-pixel rounding from
/// the previous frame's snap) and is moving away from it on
/// wall-jump. The horizontal sweep finds the wall at ToI=0; the snap
/// uses delta direction (+x → "block is to my right") and pushes the
/// player through the wall by `wall.left() - body.right() = -63`.
#[test]
fn wall_jump_does_not_catapult_player_off_wall_overlap() {
    let world = test_world();
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let body = player.aabb();
    let left_wall_right = 36.0;
    // Body penetrates wall by 1 px on the x-axis, mid-height of the
    // room (no floor/ceiling overlap to confuse the issue).
    player.pos.x = left_wall_right + body.half_size().x - 1.0;
    player.pos.y = world.size.y * 0.5;
    player.vel = Vec2::new(500.0, -650.0); // wall-jump initial velocities
    player.on_ground = false;
    player.on_wall = false;
    player.wall_normal_x = 0.0;

    let initial_x = player.pos.x;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // After one frame the player should be sitting at body.left ≈
    // wall.right (the wall snap aligns them), or at most a few pixels
    // to the right (motion delta). They must not be teleported by
    // tens of pixels in any direction.
    let dx = (player.pos.x - initial_x).abs();
    assert!(
        dx < 30.0,
        "wall overlap caused horizontal teleport: dx={dx}, pos.x went from {initial_x} to {}",
        player.pos.x,
    );
    assert!(
        player.pos.x - body.half_size().x >= 0.0 - 0.5,
        "player was punched through the left wall: body left = {}",
        player.pos.x - body.half_size().x,
    );
}

/// Regression: reproduces the wall-cling → Grounded teleport captured
/// in `debug_traces/ambition_trace_1777903935-558508824-000000_*.json`.
/// The player wall-clings on a tall left-side wall (top at world y=0,
/// bottom at world's floor) and slides downward at `wall_slide_speed`.
/// Before the fix, the y-axis sweep would return `time_of_impact = 0`
/// on the wall (the body was edge-touching / fractionally penetrating
/// it), then unconditionally snap the body's bottom to the wall's TOP
/// edge — teleporting the player ~1700 px upward to
/// `y = 0 - half_height = -23`.
///
/// The fix filters dominantly-horizontal overlaps out of the y-sweep
/// and adds the symmetric guard to `resolve_vertical`. After the fix
/// the player either stays roughly where they were (continuing the
/// wall slide) or moves by at most one frame's worth of velocity.
#[test]
fn wall_cling_does_not_teleport_to_wall_top_on_y_sweep() {
    let world = test_world();
    // Wall-cling pose: edge-touching left wall (wall.right = 36),
    // mid-room vertically, with wall_slide_speed downward.
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    let half = player.size * 0.5;
    let wall_right = 36.0;
    // 0.05 px penetration into the wall — within the kind of float
    // fuzz that survives between the x-sweep and the y-sweep.
    player.pos.x = wall_right + half.x - 0.05;
    player.pos.y = world.size.y * 0.5; // ~450, well inside the room
    player.vel = Vec2::new(0.0, DEFAULT_TUNING.wall_slide_speed);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;

    let initial_y = player.pos.y;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0, // pressing into the wall
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // Hard invariant: after one sim step the y position must still be
    // inside the world envelope, and the y delta must be bounded by
    // the velocity-budget plus a small slop. The pre-fix behavior
    // teleported to y ≈ -23 (about 470 px above start); the post-fix
    // behavior should be |dy| < 50 px.
    assert!(
            player.pos.y >= 0.0 && player.pos.y <= world.size.y,
            "wall-cling y-sweep teleported player out of the world envelope: pos.y = {} (world.size.y = {})",
            player.pos.y,
            world.size.y,
        );
    let dy = (player.pos.y - initial_y).abs();
    assert!(
            dy < 50.0,
            "wall-cling y-sweep moved player by {dy} px in one frame; expected at most a few pixels of slide",
        );
    // The player must not have transitioned to Grounded against a
    // surface that doesn't exist at this y. The bug snapped the body
    // bottom to the wall's TOP (y=0) and set on_ground=true.
    assert!(
        !player.on_ground,
        "wall-cling y-sweep falsely set on_ground; player was supposedly grounded at y={}",
        player.pos.y,
    );
}

/// Regression: player wall-clinging on a tall column whose top
/// is far above the player must NOT teleport upward when their
/// body partially overlaps the column on its bottom edge.
///
/// Concrete repro from the May 2026 mob_lab F8 trace: player at
/// (718, 419), body=(704, 396, 732, 442), wall-clinging on the
/// right face of a column at (704, 16, 720, 400). The body's
/// top corner (y=396) sticks 4 px above column.bottom (y=400),
/// so body and column strictly overlap in both axes. The y-sweep
/// found a TOI=0 hit on the column with delta.y ≈ 0.1 (tiny,
/// gravity-decelerated downward motion), and the falling-branch
/// snapped body.bottom to column.top (y=16) — teleporting the
/// player from y=419 to y=-7 (above the world's top edge).
///
/// Two guards prevent this:
/// 1. y-sweep predicate rejects blocks `start_body` already
///    strictly intersects (entrenched penetrations belong to
///    the x-resolver, not the y-sweep).
/// 2. The landing-from-above branch additionally requires
///    `prev_bottom <= block.top + tol`, mirroring the OneWay
///    landing test, so a downward-but-tiny delta near a far-away
///    block can't fire the snap.
#[test]
fn partial_wall_cling_overlap_does_not_teleport_upward() {
    let world = World {
        name: "column".into(),
        size: Vec2::new(1600.0, 768.0),
        spawn: Vec2::new(50.0, 50.0),
        // Column matching the trace: x=[704, 720], y=[16, 400].
        // Center=(712, 208), size=(16, 384).
        blocks: vec![Block::solid(
            "column",
            Vec2::new(712.0, 208.0),
            Vec2::new(16.0, 384.0),
        )],
        objects: Vec::new(),
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Reproduce the exact pre-OOB state from the trace.
    player.pos = Vec2::new(718.0, 419.0);
    player.vel = Vec2::new(0.0, 15.0); // gravity-decelerated tiny downward
    player.on_ground = false;
    player.on_wall = true;
    player.wall_clinging = true;
    player.wall_normal_x = -1.0;

    let start_y = player.pos.y;
    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            control_dt: 1.0 / 60.0,
            axis_x: -1.0, // pressing toward wall
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    // The player must NOT have been catapulted across the room.
    // A normal frame's motion is single-digit pixels; anything more
    // than ~50 px is the bug.
    let dy = (player.pos.y - start_y).abs();
    assert!(
        dy < 50.0,
        "y-sweep teleported player by {} px; expected ~tiny gravity-driven motion (start_y={}, end_y={})",
        dy, start_y, player.pos.y,
    );
    // Sanity: still inside the world.
    assert!(
        player.pos.y > 0.0 && player.pos.y < world.size.y,
        "player ended OOB at y={}",
        player.pos.y,
    );
}

/// Guards against `body_is_side_contact` being too broad. Player
/// descending onto the *top corner* of a tall solid (a pillar) with
/// slight x overlap should still resolve as a normal landing —
/// `on_ground = true`, `pos.y` snaps so `body.bottom = pillar.top`.
/// If this test ever starts failing, the side-contact filter has
/// expanded into legitimate vertical-landing geometry.
#[test]
fn descending_onto_top_corner_of_tall_block_lands_normally() {
    // World with a tall pillar centered horizontally.
    let world = World {
        name: "pillar".into(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(50.0, 50.0),
        blocks: vec![Block::solid(
            "pillar",
            Vec2::new(380.0, 200.0),
            Vec2::new(40.0, 400.0),
        )],
        objects: Vec::new(),
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    // Pillar AABB: (380, 200) → (420, 600). Top = 200, bottom = 600.
    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Position player so body slightly overlaps the pillar on x and is
    // about to land on its top: body x range covers ~[380-14+5, 380+5+14)
    // = [371, 405) with player half-width 14. With pos.x = 391,
    // body.left = 377 < pillar.left = 380, body.right = 405 > 380 →
    // x overlap of 25 px. body.top is well above pillar.top, body.bottom
    // is just above pillar.top.
    player.pos = Vec2::new(391.0, 200.0 - 23.0 - 0.5);
    // Falling straight down at a typical mid-arc speed.
    player.vel = Vec2::new(0.0, 200.0);
    player.on_ground = false;
    player.on_wall = false;

    let _ = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let body = player.aabb();
    assert!(
        player.on_ground,
        "descending onto pillar top should land (on_ground = true); got pos={:?}",
        player.pos
    );
    // body.bottom should be at or extremely near the pillar's top.
    assert!(
        (body.bottom() - 200.0).abs() < 1.0,
        "body.bottom should snap to pillar.top = 200; got {} (pos.y = {})",
        body.bottom(),
        player.pos.y,
    );
}

/// Direct unit test of `body_is_side_contact`. Both `sweep_player_y`
/// and `resolve_vertical` consult it to avoid the wall-cling teleport
/// class. The first revision used `overlap_x > 0` and missed the
/// exact-edge-touching case captured in
/// `debug_traces/ambition_trace_1777905256-*.json`; the predicate
/// now keys on the body's y-range being nested inside the block's
/// y-range, which catches edge-touching and penetrating side
/// contacts uniformly.
#[test]
fn body_is_side_contact_classifies_walls_vs_floors() {
    // Player about to land on a wide floor: body.top < floor.top,
    // so body's y-range is NOT nested inside floor's y-range. Not
    // a side contact.
    let body = Aabb::new(Vec2::new(50.0, 100.0), Vec2::new(14.0, 23.0));
    let floor = Aabb::new(Vec2::new(80.0, 125.0), Vec2::new(60.0, 6.0));
    assert!(
        !body_is_side_contact(body, floor),
        "player about to land on a wide floor must NOT be classified as a side contact"
    );

    // Tall left wall, body fully alongside it (body's y-range is
    // strictly inside the wall's y-range). Edge-touching on x.
    // Side contact regardless of x-overlap.
    let wall = Aabb::new(Vec2::new(18.0, 450.0), Vec2::new(18.0, 450.0));
    let body_alongside_edge = Aabb::new(Vec2::new(36.0 + 14.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_alongside_edge, wall),
        "body alongside a tall wall (edge-touching on x) must be a side contact"
    );

    // Same wall, body penetrating by 1 px on x. Still alongside on y.
    let body_inside_wall = Aabb::new(Vec2::new(36.0 + 14.0 - 1.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_inside_wall, wall),
        "body penetrating a tall wall on x is still a side contact"
    );

    // Player landing on the top corner of a tall block (small x
    // overlap, body.bottom near block.top, body.top above block.top).
    // The body's y-range is NOT nested inside the block's y-range
    // (body.top < block.top), so this is a real vertical contact —
    // NOT a side contact. Guards against the predicate becoming too
    // broad.
    let pillar = Aabb::new(Vec2::new(900.0, 800.0), Vec2::new(40.0, 200.0));
    let body_landing_on_pillar = Aabb::new(
        Vec2::new(900.0 - 40.0 + 5.0, 600.0 - 23.0 + 1.0),
        Vec2::new(14.0, 23.0),
    );
    assert!(
            !body_is_side_contact(body_landing_on_pillar, pillar),
            "descending onto the top edge of a tall block (slight x overlap, body.top above block.top) must NOT be classified as a side contact"
        );

    // Player jumping up into a thick ceiling block (body.bottom
    // crossing block.bottom from below). body.bottom > block.bottom
    // → not nested → real vertical contact.
    let ceiling = Aabb::new(Vec2::new(900.0, 200.0), Vec2::new(400.0, 100.0));
    let body_under_ceiling = Aabb::new(Vec2::new(900.0, 300.0 + 23.0 - 1.0), Vec2::new(14.0, 23.0));
    assert!(
            !body_is_side_contact(body_under_ceiling, ceiling),
            "rising into a thick ceiling (body.bottom poking past block.bottom) must NOT be classified as a side contact"
        );
}

#[test]
fn climbable_contact_is_populated_when_player_intersects_ladder() {
    // Mirror of the water_contact integration test: when the
    // player AABB overlaps a ClimbableRegion in the world, the
    // engine should cache the contact on the player struct so
    // sandbox-side gameplay systems and the RL adapter read a
    // consistent answer for the frame.
    use crate::world::{ClimbableKind, ClimbableRegion, ClimbableSpec};
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
    use crate::world::{ClimbableKind, ClimbableRegion, ClimbableSpec};
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
    use crate::world::{Block, ClimbableKind, ClimbableRegion, ClimbableSpec};
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
    use crate::world::{Block, ClimbableKind, ClimbableRegion, ClimbableSpec};
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
    use crate::world::{Block, ClimbableKind, ClimbableRegion, ClimbableSpec};
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
    use crate::world::{ClimbableKind, ClimbableRegion, ClimbableSpec};
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

#[test]
fn simulation_latches_ledge_grab_on_blink_wall_surface() {
    let mut world = test_world();
    world.blocks.push(Block::blink_wall(
        "soft blink ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
        BlinkWallTier::Soft,
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(86.0, 110.0), abilities);
    player.vel = Vec2::new(120.0, 20.0);
    player.wall_clinging = true;
    player.on_wall = true;
    player.wall_normal_x = -1.0;

    let events = update_player_simulation_with_tuning(
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
    assert!(player.ledge_grab.is_some(), "blink wall ledge should latch");
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn simulation_latches_ledge_grab_on_one_way_surface_without_wall_collision() {
    let mut world = test_world();
    world.blocks.push(Block::one_way(
        "one-way ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 16.0),
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(86.0, 110.0), abilities);
    player.on_ground = false;
    player.vel = Vec2::new(20.0, 40.0);

    let events = update_player_simulation_with_tuning(
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
    assert!(
        player.ledge_grab.is_some(),
        "pressing toward a one-way edge should allow a pull-up even though one-way platforms do not collide on X"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn active_ledge_grab_climb_finishes_inside_simulation_tick() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut player = Player::new_with_abilities(Vec2::new(87.0, 119.0), abilities);
    let contact = crate::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::LedgeGrabState::hanging(contact);
    state.elapsed = crate::LEDGE_MIN_CLIMB_DELAY;
    state.climbing = true;
    state.climb_elapsed = crate::LEDGE_CLIMB_TIME;
    player.ledge_grab = Some(state);

    let events = update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState::default(),
        1.0 / 60.0,
        DEFAULT_TUNING,
    );
    assert!(
        player.ledge_grab.is_none(),
        "completed climb clears ledge state"
    );
    assert_eq!(player.pos, contact.climb_target);
    assert!(player.on_ground);
    assert!(events.operations.contains(&MovementOp::LedgeClimbFinish));
}

#[test]
fn dodge_roll_triggers_on_ground_with_ability() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.0;
    assert!(player.abilities.dodge, "sandbox_all enables dodge");
    let events = step(
        &world,
        &mut player,
        InputState { dash_pressed: true, ..Default::default() },
    );
    assert!(
        events.operations.contains(&MovementOp::DodgeRoll),
        "dash on ground with dodge ability should trigger DodgeRoll"
    );
    assert!(player.dodge_roll_timer > 0.0, "dodge_roll_timer should be set");
    assert!(player.vel.x.abs() > 100.0, "should have lateral velocity from dodge");
}

#[test]
fn dodge_roll_blocked_by_cooldown() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.3;
    let events = step(
        &world,
        &mut player,
        InputState { dash_pressed: true, ..Default::default() },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should be blocked when on cooldown"
    );
}

#[test]
fn dodge_roll_disabled_when_ability_off() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.dodge = false;
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = true;
    player.dodge_roll_cooldown = 0.0;
    let events = step(
        &world,
        &mut player,
        InputState { dash_pressed: true, ..Default::default() },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should not trigger when ability is disabled"
    );
}

#[test]
fn shield_activates_when_held_with_ability() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    let events = step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(player.shield_active, "shield should be active while held");
    assert!(
        player.parry_window_timer > 0.0,
        "parry window should start on first activation"
    );
    assert!(
        events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp op should be recorded"
    );
}

#[test]
fn shield_deactivates_when_released() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(player.shield_active);
    step(
        &world,
        &mut player,
        InputState { shield_held: false, ..Default::default() },
    );
    assert!(!player.shield_active, "shield should drop when button released");
}

#[test]
fn shield_blocked_during_dash() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    player.dash_timer = 0.10; // force active dash
    step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(!player.shield_active, "shield cannot be raised during a dash");
}

#[test]
fn shield_gives_fresh_parry_on_each_activation() {
    let world = test_world();
    let mut player = Player::new(world.spawn);
    player.on_ground = true;
    player.abilities.shield = true;
    step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(player.parry_window_timer > 0.0);
    // Expire the parry window and drop shield.
    player.parry_window_timer = 0.0;
    step(
        &world,
        &mut player,
        InputState { shield_held: false, ..Default::default() },
    );
    // Re-raise: fresh parry window.
    step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(
        player.parry_window_timer > 0.0,
        "raising shield again should reset the parry window"
    );
}

#[test]
fn shield_disabled_when_ability_off() {
    let world = test_world();
    let abilities = AbilitySet::basic(); // basic() has shield: false
    let mut player = Player::new_with_abilities(world.spawn, abilities);
    player.on_ground = true;
    let events = step(
        &world,
        &mut player,
        InputState { shield_held: true, ..Default::default() },
    );
    assert!(!player.shield_active, "shield should not activate without the ability");
    assert!(
        !events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp should not fire without the ability"
    );
}
