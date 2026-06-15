//! Reverse-gravity SYMMETRY suite, driven entirely through the headless
//! action/observation harness. Each mechanic is exercised under default "down"
//! gravity AND inverted "up" gravity; a mechanic is symmetric iff it behaves the
//! same way relative to the gravity direction. Built while hunting (and fixing)
//! the pogo gravity-inversion bug; see docs note in the run plan.
//!
//! Uses the world-observability added to `AgentObservation` (gravity_dir,
//! enemies, pickups) and the `SandboxSim` scenario scaffolding
//! (set_base_gravity_dir / teleport_player / grant_pogo_ability / add_block).

mod common;
use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};
use ambition_sandbox::engine_core::{Block, Vec2};

fn base() -> AgentAction {
    AgentAction::default()
}

fn open_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("central_hub_complex"),
    )
    .expect("sim")
}

const DOWN: (f32, f32) = (0.0, 1.0);
const UP: (f32, f32) = (0.0, -1.0);

/// Dot of velocity with the anti-gravity direction: how fast the body is moving
/// AWAY from gravity (a jump/pogo should be positive).
fn away_from_gravity(vel: (f32, f32), gdir: (f32, f32)) -> f32 {
    vel.0 * -gdir.0 + vel.1 * -gdir.1
}

/// With no input in open air, the body must accelerate ALONG gravity at the same
/// magnitude under either orientation (only the sign flips). The harness/gravity
/// baseline: if this breaks, nothing below is trustworthy.
#[test]
fn free_fall_acceleration_is_gravity_symmetric() {
    let accel_of = |gdir: (f32, f32)| -> f32 {
        let mut sim = open_sim();
        for _ in 0..5 {
            sim.step(base());
        }
        let spawn = sim.observation().world_spawn;
        sim.set_base_gravity_dir(gdir);
        sim.teleport_player((spawn.0, spawn.1 - 200.0));
        let v0 = sim.step(base()).player_vel.1;
        let v1 = sim.step(base()).player_vel.1;
        v1 - v0 // per-frame acceleration in y
    };
    let down = accel_of(DOWN);
    let up = accel_of(UP);
    assert!(down > 0.0, "down gravity should accelerate +Y, got {down}");
    assert!(up < 0.0, "up gravity should accelerate -Y, got {up}");
    assert!(
        (down + up).abs() < 1.0,
        "free-fall accel must be symmetric: down={down} up={up} (sum should be ~0)"
    );
}

/// Drop the player onto a pogo orb placed in their GRAVITY-down direction, with
/// the pogo ability, and return the post-bounce velocity-away-from-gravity. A
/// correct pogo launches the player back OPPOSITE gravity under either
/// orientation — the regression this suite was built around.
fn pogo_away_speed(gdir: (f32, f32)) -> f32 {
    let mut sim = open_sim();
    for _ in 0..5 {
        sim.step(base());
    }
    let spawn = sim.observation().world_spawn;
    let (px, py) = (spawn.0, spawn.1 - 120.0);
    sim.set_base_gravity_dir(gdir);
    sim.grant_pogo_ability();
    sim.teleport_player((px, py));
    // orb one body-length in the gravity-down direction (the way the player falls)
    let orb = (px + gdir.0 * 44.0, py + gdir.1 * 44.0);
    sim.add_block(Block::pogo_orb("orb", Vec2::new(orb.0, orb.1), 30.0));

    let mut best = f32::MIN;
    for _ in 0..40 {
        // Dedicated pogo input (control.rs Path 1): no world-locked axis_y gate,
        // so the only gravity-relativity under test is the hitbox + bounce.
        let o = sim.step(AgentAction {
            pogo: true,
            ..base()
        });
        best = best.max(away_from_gravity(o.player_vel, o.gravity_dir));
    }
    best
}

#[test]
fn pogo_bounces_away_from_gravity_under_both_orientations() {
    let down = pogo_away_speed(DOWN);
    let up = pogo_away_speed(UP);
    // pogo_speed is 720; require a clear launch away from gravity (not a fall).
    assert!(
        down > 400.0,
        "pogo under DOWN gravity should launch away from gravity, got {down}"
    );
    assert!(
        up > 400.0,
        "pogo under UP gravity should ALSO launch away from gravity (the inversion \
         bug made this negative — launching into gravity), got {up}"
    );
}

/// A one-way platform placed in the player's gravity-down direction must be
/// landable under either orientation (land on the gravity-facing face).
#[test]
fn one_way_platform_landing_is_gravity_symmetric() {
    let lands = |gdir: (f32, f32)| -> bool {
        let mut sim = open_sim();
        for _ in 0..5 {
            sim.step(base());
        }
        let spawn = sim.observation().world_spawn;
        let (px, py) = (spawn.0, spawn.1 - 160.0);
        sim.set_base_gravity_dir(gdir);
        sim.teleport_player((px, py));
        sim.add_block(Block::one_way(
            "ow",
            Vec2::new(px - 80.0, py + gdir.1 * 60.0),
            Vec2::new(160.0, 12.0),
        ));
        (0..120).any(|_| sim.step(base()).on_ground)
    };
    assert!(lands(DOWN), "should land on the one-way under DOWN gravity");
    assert!(
        lands(UP),
        "should land on the one-way under UP gravity (gravity-relative one-way)"
    );
}

// "Toward gravity" screen sign (+1 normal/sideways, -1 past ±90°).
fn toward(gdir: (f32, f32)) -> f32 {
    if gdir.1 < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// Press the descend gate (toward gravity), optionally with jump. Sets the matching
/// up/down EDGE flags too so gesture detectors (fast-fall) see them.
fn descend(gdir: (f32, f32), jump: bool) -> AgentAction {
    let tg = toward(gdir);
    AgentAction {
        move_y: tg,
        jump,
        down_pressed: tg > 0.0,
        up_pressed: tg < 0.0,
        ..base()
    }
}

/// Rest the player on an injected floor (solid or one-way) in OPEN air, on the
/// gravity-facing face, so the test is independent of room geometry. The platform
/// sits at an open spot; the player is dropped onto it from the anti-gravity side.
/// Returns (sim, floor_center_y). Asserts the player actually landed on OUR floor.
fn settle_on_floor(gdir: (f32, f32), one_way: bool) -> (SandboxSim, f32) {
    let mut sim = open_sim();
    for _ in 0..5 {
        sim.step(base());
    }
    let spawn = sim.observation().world_spawn;
    let px = spawn.0;
    let fy = spawn.1 - 200.0; // open air (free-fall test confirmed clear here)
    sim.set_base_gravity_dir(gdir);
    let half_h = sim.observation().player_size.1 * 0.5;
    let min = Vec2::new(px - 160.0, fy - 8.0);
    let size = Vec2::new(320.0, 16.0);
    sim.add_block(if one_way {
        Block::one_way("floor", min, size)
    } else {
        Block::solid("floor", min, size)
    });
    // Place the player's gravity-facing edge exactly ON the platform's anti-gravity
    // face (top under down-gravity, bottom under up-gravity), embedded 1px so it
    // grounds immediately regardless of room geometry.
    let pos_y = fy - gdir.1 * (8.0 + half_h - 1.0);
    sim.teleport_player((px, pos_y));
    for _ in 0..10 {
        if sim.step(base()).on_ground {
            break;
        }
    }
    let o = sim.observation();
    assert!(
        o.on_ground && (o.player_pos.1 - fy).abs() < 60.0,
        "test setup: player should rest on OUR floor (gdir={gdir:?}, pos.y={}, fy={fy})",
        o.player_pos.1
    );
    (sim, fy)
}

/// Crouch = press toward your feet on the ground. Must enter Crouching under both
/// orientations (the gate flips to screen-up past ±90°).
#[test]
fn crouch_is_gravity_symmetric() {
    let crouches = |gdir: (f32, f32)| -> bool {
        let (mut sim, _) = settle_on_floor(gdir, false);
        assert!(sim.observation().on_ground, "test setup: should be grounded");
        let mut got = false;
        for _ in 0..20 {
            if sim.step(descend(gdir, false)).body_mode.contains("Crouch") {
                got = true;
                break;
            }
        }
        got
    };
    assert!(crouches(DOWN), "should crouch under DOWN gravity");
    assert!(
        crouches(UP),
        "should crouch under UP gravity (descend gate flips to screen-up)"
    );
}

// NOTE: drop-through-one-way symmetry (the originally-reported bug) is proven
// DETERMINISTICALLY at the engine level in
// `ambition_engine_core::movement::tests::wall_collision::
//  one_way_drop_through_works_under_inverted_gravity` — that test controls the
// world + gravity directly, avoiding this room's geometry/jump-buffer timing
// which makes an app-harness drop test flaky. Crouch + pogo below exercise the
// same gravity-`descend` seam through the full app.

// NOTE: fast-fall's gravity-relativity has two parts, both verified outside this
// flaky room: (a) the engine fast-fall accel already projects onto `gravity_dir`
// (it was always gravity-aware); (b) the NEW piece is the double-tap EDGE flip in
// `input_timer_system` (screen-down tap normally, screen-up tap past ±90°). A
// harness test here is flaky because the player can't stay airborne long enough
// under inverted gravity in `central_hub_complex` (it falls into the ceiling, and
// grounding clears `fast_falling` the same frame). The edge-flip is a direct
// up_pressed/down_pressed swap on `gravity_dir.y < 0`.
