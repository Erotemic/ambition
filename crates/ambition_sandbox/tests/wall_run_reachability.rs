//! Verifies the `wall_run` showcase room actually exercises the flagship
//! wall-walking (vector gravity): a player who walks right into the rightward
//! `GravityZone` should be pulled onto the right wall, not just stop at the zone
//! boundary or fall down. This is the room-level companion to the engine's
//! `wall_walking_grounds_walks_and_jumps_off_a_side_wall` unit test — it confirms
//! the LDtk-authored zone + geometry wire up to the mechanic in a real room.

use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

/// "Hold right" — the only input this test needs. Built fresh each tick so we
/// don't rely on `AgentAction` being `Copy`/`Clone`.
fn hold_right() -> AgentAction {
    AgentAction {
        move_x: 1.0,
        move_y: 0.0,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

#[test]
fn wall_run_field_pulls_the_player_onto_the_right_wall() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("wall_run");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new");

    // Spawns in the left, normal-gravity strip (x≈80), well left of the field
    // (which starts at x=260) and the right wall (x=624).
    let spawn = sim.observation().player_pos;
    assert!(
        spawn.0 < 200.0,
        "player should spawn in the left strip, got x={}",
        spawn.0
    );

    // Walk right: cross into the rightward GravityZone (x>260), whose gravity then
    // carries the player onto the right wall (x≈624).
    for _ in 0..90 {
        sim.step(hold_right());
    }

    let (px, py) = sim.observation().player_pos;
    assert!(
        px > 540.0,
        "the rightward field should carry the player onto the right wall \
         (got x={px}, y={py}); if vector gravity weren't wired for this room the \
         player would stop near the field boundary instead",
    );
    // And it didn't fling them out of the room.
    assert!(
        px < 640.0 && (16.0..752.0).contains(&py),
        "player stays inside the room (x={px}, y={py})",
    );
}
