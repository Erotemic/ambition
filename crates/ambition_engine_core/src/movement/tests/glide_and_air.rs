//! Glide cap, fast-fall double-tap signal, fly mode toggle, pogo orb
//! AABB feedback — anything air-borne that isn't a blink.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::{AbilitySet, Vec2};

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn flipped_gravity_makes_the_player_fall_up_and_stand_on_the_ceiling() {
    use crate::world::{Block, World};
    let w = 800.0;
    let h = 600.0;
    let world = World {
        name: "gravity flip world".to_string(),
        size: Vec2::new(w, h),
        spawn: Vec2::new(400.0, 300.0),
        blocks: vec![
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(w, 40.0)),
            Block::solid("floor", Vec2::new(0.0, h - 40.0), Vec2::new(w, 40.0)),
        ],
        climbable_regions: Vec::new(),
        water_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 300.0));
    scratch.ground.on_ground = false;
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = Vec2::new(0.0, -1.0); // up — drives the integrator
    tuning.gravity_sign = -1.0; // legacy mirror, still read by the sweeps (pre-P4)

    // Let it fall UP for a while.
    for _ in 0..240 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.kinematics.pos.y < 300.0,
        "flipped gravity should pull the player UP, got y={}",
        scratch.kinematics.pos.y
    );
    assert!(
        scratch.ground.on_ground,
        "the player should land on (stand under) the ceiling with flipped gravity"
    );
}

#[test]
fn sideways_gravity_pulls_the_player_along_x() {
    // Vector-gravity foundation (wall-walking P2/P3): with gravity pointing RIGHT
    // the player accelerates +x and barely moves in y -- the same integrator that
    // handles down/up now handles sideways. (Standing ON the wall needs the P4
    // ground-sweep generalization; this pins the acceleration model.)
    use crate::world::{Block, World};
    let world = World {
        name: "sideways gravity".to_string(),
        size: Vec2::new(2000.0, 600.0),
        spawn: Vec2::new(200.0, 300.0),
        blocks: vec![Block::solid(
            "right wall",
            Vec2::new(1900.0, 0.0),
            Vec2::new(100.0, 600.0),
        )],
        climbable_regions: Vec::new(),
        water_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(200.0, 300.0));
    scratch.ground.on_ground = false;
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = Vec2::new(1.0, 0.0); // gravity points RIGHT
    tuning.gravity_sign = 1.0;
    let y0 = scratch.kinematics.pos.y;
    for _ in 0..30 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.kinematics.vel.x > 100.0,
        "rightward gravity should build +x velocity, got {}",
        scratch.kinematics.vel.x
    );
    assert!(
        scratch.kinematics.pos.x > 220.0,
        "should have fallen right, got x={}",
        scratch.kinematics.pos.x
    );
    assert!(
        (scratch.kinematics.pos.y - y0).abs() < 5.0,
        "should barely move in y, got dy={}",
        scratch.kinematics.pos.y - y0
    );
}

#[test]
fn wall_walking_grounds_walks_and_jumps_off_a_side_wall() {
    // P4 — the flagship wall-walking slice: under RIGHTWARD gravity the player
    // falls onto the right wall, is grounded ON it, walks ALONG it (axis_x -> the
    // vertical move axis), and jumps OFF it (-x, opposite gravity).
    use crate::world::{Block, World};
    let world = World {
        name: "wall walk".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(400.0, 300.0),
        blocks: vec![
            Block::solid("right wall", Vec2::new(760.0, 0.0), Vec2::new(40.0, 600.0)),
            Block::solid("floor", Vec2::new(0.0, 560.0), Vec2::new(800.0, 40.0)),
        ],
        climbable_regions: Vec::new(),
        water_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 300.0));
    scratch.ground.on_ground = false;
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = Vec2::new(1.0, 0.0); // gravity RIGHT
    tuning.gravity_sign = 1.0;

    for _ in 0..120 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.kinematics.pos.x > 700.0,
        "should fall onto the right wall, got x={}",
        scratch.kinematics.pos.x
    );
    assert!(scratch.ground.on_ground, "should be grounded ON the wall");

    // Walk along the wall: axis_x drives the vertical move axis.
    let y0 = scratch.kinematics.pos.y;
    for _ in 0..30 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_x: 1.0,
                ..Default::default()
            },
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        (scratch.kinematics.pos.y - y0).abs() > 20.0,
        "axis_x should walk the player ALONG the wall (in Y), dy={}",
        scratch.kinematics.pos.y - y0
    );
    assert!(
        scratch.kinematics.pos.x > 700.0,
        "still pinned to the wall, x={}",
        scratch.kinematics.pos.x
    );

    // Jump OFF the wall: launches in -x (opposite gravity).
    let x0 = scratch.kinematics.pos.x;
    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    );
    for _ in 0..8 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                jump_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.kinematics.pos.x < x0 - 5.0,
        "jump should push OFF the wall (-x), x {} -> {}",
        x0,
        scratch.kinematics.pos.x
    );
}

#[test]
fn one_way_platform_works_under_flipped_gravity() {
    // #55: under UP gravity the player falls up and lands on the one-way's BOTTOM
    // face (solid from the side you fall from, passable from the other).
    use crate::world::{Block, World};
    let world = World {
        name: "flip one-way".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(400.0, 400.0),
        blocks: vec![
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(800.0, 40.0)),
            Block::one_way("oneway", Vec2::new(300.0, 200.0), Vec2::new(200.0, 12.0)),
        ],
        climbable_regions: Vec::new(),
        water_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 400.0));
    scratch.ground.on_ground = false;
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = Vec2::new(0.0, -1.0); // UP
    tuning.gravity_sign = -1.0;
    for _ in 0..120 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    // Rests under the one-way's bottom face (y=212): player top ~= 212.
    let top = scratch.kinematics.pos.y - scratch.kinematics.size.y * 0.5;
    assert!(
        (top - 212.0).abs() < 6.0,
        "player should rest under the one-way's bottom (y=212), got top={top}"
    );
    assert!(
        scratch.ground.on_ground,
        "should be grounded on the one-way's bottom under up gravity"
    );
}

#[test]
fn glide_caps_fall_speed_while_jump_held() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = false;
    // Drop the player into free fall well above any contact, with
    // velocity already above the glide cap so the cap clamp is the
    // only thing that can pull it back down.
    scratch.kinematics.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    scratch.kinematics.vel = Vec2::new(0.0, 800.0);

    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    let _ = events; // unused

    assert!(
        scratch.flight.gliding,
        "hold-jump while falling should engage glide"
    );
    assert!(
        scratch.kinematics.vel.y <= DEFAULT_TUNING.glide_fall_speed + 1.0,
        "glide cap should clamp fall speed; got {}",
        scratch.kinematics.vel.y
    );
    assert!(
        scratch.kinematics.vel.y < DEFAULT_TUNING.max_fall_speed * 0.5,
        "glide cap must be markedly below max_fall_speed; got {}",
        scratch.kinematics.vel.y
    );
}

#[test]
fn glide_disengages_when_jump_released() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = false;
    scratch.kinematics.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    scratch.kinematics.vel = Vec2::new(0.0, 800.0);

    // Frame 1: held → glide engages
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.flight.gliding);

    // Frame 2: released → glide disengages, fall speed climbs back
    // toward max_fall_speed (gravity reapplied without the glide cap)
    step_scratch(&world, &mut scratch, InputState::default());
    assert!(!scratch.flight.gliding);
}

#[test]
fn glide_requires_ability_flag() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.glide = false;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = false;
    scratch.kinematics.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
    scratch.kinematics.vel = Vec2::new(0.0, 800.0);

    step_scratch(
        &world,
        &mut scratch,
        InputState {
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(
        !scratch.flight.gliding,
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
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = false;
    scratch.kinematics.pos = Vec2::new(world.spawn.x, world.spawn.y - 800.0);
    scratch.kinematics.vel = Vec2::new(0.0, 0.0);

    let dt = 1.0 / 60.0;
    for frame in 0..60 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                jump_held: true,
                control_dt: dt,
                ..Default::default()
            },
        );
        if scratch.ground.on_ground {
            break;
        }
        // After the first ~5 frames gravity has bumped vel.y past
        // the glide cap so the cap is actively clamping. Don't
        // assert on the very first frames where vel.y < cap.
        if frame >= 6 {
            assert!(
                scratch.flight.gliding,
                "frame {frame}: gliding flipped off (vel=({},{}) on_ground={})",
                scratch.kinematics.vel.x, scratch.kinematics.vel.y, scratch.ground.on_ground,
            );
            assert!(
                scratch.kinematics.vel.y <= DEFAULT_TUNING.glide_fall_speed + 5.0,
                "frame {frame}: vel.y exceeded glide cap ({} > {})",
                scratch.kinematics.vel.y,
                DEFAULT_TUNING.glide_fall_speed,
            );
        }
    }
}

#[test]
fn fast_fall_requires_double_tap_signal() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = false;
    scratch.kinematics.vel.y = 0.0;

    // Holding down is still useful for pogo / downward attack intent, but
    // should not automatically trigger fast-fall.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            ..Default::default()
        },
    );
    assert!(!scratch.flight.fast_falling);

    // The presentation layer recognizes double-tap-down and sends this
    // explicit event to the engine.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            fast_fall_pressed: true,
            ..Default::default()
        },
    );
    assert!(scratch.flight.fast_falling);
}

#[test]
fn fly_toggle_switches_mode_and_counters_gravity() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    assert!(!scratch.flight.fly_enabled);
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            fly_toggle_pressed: true,
            ..Default::default()
        },
    );
    assert!(scratch.flight.fly_enabled);
    assert!(events.operations.contains(&MovementOp::FlyToggle));
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::ZERO;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: -1.0,
            ..Default::default()
        },
    );
    assert!(
        scratch.kinematics.vel.y < 0.0,
        "flying upward input should accelerate upward"
    );
}

#[test]
fn the_player_rides_a_horizontally_moving_platform() {
    // A floor carrying a rightward per-frame velocity. The player standing on it is
    // carried right by the platform — EMERGENT in the movement sweep (the same rule
    // enemies get from `step_kinematic`), not a player-specific ride path. This is
    // also why the brain-driven clone rides: it runs this exact movement core.
    use crate::world::{Block, World};
    let mut platform = Block::solid("platform", Vec2::new(0.0, 400.0), Vec2::new(400.0, 40.0));
    platform.velocity = Vec2::new(3.0, 0.0); // 3 px/frame right
    let world = World {
        name: "moving platform".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(200.0, 360.0),
        blocks: vec![platform],
        climbable_regions: Vec::new(),
        water_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(200.0, 360.0));
    let tuning = DEFAULT_TUNING;
    for _ in 0..30 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.ground.on_ground,
        "precondition: the player is resting on the platform"
    );
    let x_before = scratch.kinematics.pos.x;
    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );
    assert!(
        (scratch.kinematics.pos.x - (x_before + 3.0)).abs() < 0.05,
        "the player should ride +3px right with the platform, got dx={}",
        scratch.kinematics.pos.x - x_before
    );
}

#[test]
fn an_airborne_fling_above_run_speed_is_preserved_while_holding_into_it() {
    // A portal exit can carry a horizontal speed far above max_run_speed (a
    // fall converted by a floor→wall pair). Airborne, holding INTO the motion
    // must not brake it back to run speed — the run cap is an equilibrium
    // input accelerates UP TO, exactly like the fall cap's `relax`. (Holding
    // AGAINST the fling still brakes at full air control, and landing restores
    // the ordinary grounded approach.)
    let world = test_world();
    // High in the open air, flung hard to the right.
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(200.0, 200.0));
    scratch.ground.on_ground = false;
    let fling = DEFAULT_TUNING.max_run_speed * 4.0;
    scratch.kinematics.vel = Vec2::new(fling, 0.0);
    let hold_right = InputState {
        axis_x: 1.0,
        ..InputState::default()
    };
    for _ in 0..10 {
        step_scratch(&world, &mut scratch, hold_right);
        if scratch.ground.on_ground {
            break;
        }
    }
    assert!(
        scratch.kinematics.vel.x > fling - 1.0,
        "holding into an over-cap fling must not brake it: vx={} (fling was {fling})",
        scratch.kinematics.vel.x
    );

    // Holding AGAINST the fling still brakes (air control is preserved).
    let hold_left = InputState {
        axis_x: -1.0,
        ..InputState::default()
    };
    let vx_before = scratch.kinematics.vel.x;
    step_scratch(&world, &mut scratch, hold_left);
    assert!(
        scratch.kinematics.vel.x < vx_before - 1.0,
        "opposing input must still brake the fling: vx {} -> {}",
        vx_before,
        scratch.kinematics.vel.x
    );
}

#[test]
fn carried_momentum_conserves_flings_while_ordinary_drift_stays_tight() {
    // The middle ground: the CONTROLLER is tight (hands-off drift stops fast)
    // but momentum the WORLD imparted (a portal fling -> `carried_run`) is a
    // floor the stop assist never bleeds below — conserved until input, a
    // wall, or landing consumes it.
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(200.0, 200.0));
    scratch.ground.on_ground = false;
    let fling = DEFAULT_TUNING.max_run_speed * 4.0;
    scratch.kinematics.vel = Vec2::new(fling, 0.0);
    scratch.flight.carried_run = fling; // what the portal adapter sets on transfer
    for _ in 0..10 {
        step_scratch(&world, &mut scratch, InputState::default());
        if scratch.ground.on_ground {
            break;
        }
    }
    assert!(
        scratch.kinematics.vel.x > fling - 1.0,
        "carried momentum is conserved hands-off: vx={} (fling was {fling})",
        scratch.kinematics.vel.x
    );

    // Opposing input brakes at full air control AND eats the carried floor:
    // after braking, releasing the stick does not restore the old speed.
    let hold_left = InputState {
        axis_x: -1.0,
        ..InputState::default()
    };
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, hold_left);
    }
    let braked = scratch.kinematics.vel.x;
    assert!(
        braked < fling * 0.5,
        "opposing input brakes a carried fling: vx={braked}"
    );
    assert!(
        scratch.flight.carried_run <= braked.max(0.0) + 1e-3,
        "the carried floor shrinks with the braked velocity: carried={}, vx={braked}",
        scratch.flight.carried_run
    );

    // WITHOUT carried momentum, hands-off drift stops tight — at any speed.
    // (An un-carried over-cap speed is a controller artifact, not a fling.)
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(200.0, 200.0));
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::new(DEFAULT_TUNING.max_run_speed * 0.8, 0.0);
    scratch.flight.carried_run = 0.0;
    let drift = scratch.kinematics.vel.x;
    step_scratch(&world, &mut scratch, InputState::default());
    assert!(
        scratch.kinematics.vel.x < drift - 1.0,
        "hands-off drift decays via the tight stop assist: vx {} -> {}",
        drift,
        scratch.kinematics.vel.x
    );
}
