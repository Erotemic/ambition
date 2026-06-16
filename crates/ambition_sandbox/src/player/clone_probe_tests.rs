//! Probe: can a BRAIN drive a full player body through the player's own
//! movement integration? (Jon's "an NPC that is a copy of the player".)
//!
//! This is the headless proof of the universal-brain seam for the PLAYER path
//! specifically. We assemble the 18 player movement clusters
//! ([`PlayerClusterScratch`]) with the full ability set, attach a
//! [`StateMachineCfg::PlayerDemo`] brain (which emits the player's own
//! run/jump/dash/fly verbs on the shared `ActorControlFrame`), and drive it
//! through the exact same `update_player_clusters` integration the human player
//! uses — with ZERO player-specific code path. We then assert the emergent
//! result: the brain made the body run, leave the ground, and fly.
//!
//! What this proves: the Brain -> ActorControlFrame -> InputState ->
//! movement-integration pipeline is genuinely actor-generic; a state-machine
//! brain drives a player body identically to a human. (The remaining gap is not
//! the seam but the SYSTEMS: every live player sim system in `sim_systems.rs` is
//! `single_mut()`-keyed to the one `PlayerEntity` — see the player-clone probe
//! in `docs/journals/content-authoring-pain-points.md`.)

use crate::actor::control::ActorControlFrame;
use crate::brain::state_machine::PlayerDemoCfg;
use crate::brain::{Brain, BrainSnapshot, StateMachineCfg};
use crate::engine_core as ae;
use ae::movement::{update_player_clusters, InputState};
use ae::world::{Block, World};
use ae::Vec2;

/// Mirror of `engine_input_from_actor_control` (ambition_app): for the PLAYER
/// path, `desired_vel` is the normalized stick axis, fed straight into the
/// movement axis. No hitstun in this probe.
fn input_from_frame(f: &ActorControlFrame) -> InputState {
    InputState {
        axis_x: f.desired_vel.x,
        axis_y: f.desired_vel.y,
        jump_pressed: f.jump_pressed,
        jump_held: f.jump_held,
        jump_released: f.jump_released,
        dash_pressed: f.dash_pressed,
        fly_toggle_pressed: f.fly_toggle_pressed,
        fast_fall_pressed: f.fast_fall_pressed,
        attack_pressed: f.melee_pressed,
        pogo_pressed: f.pogo_pressed,
        interact_pressed: f.interact_pressed,
        shield_held: f.shield_held,
        control_dt: 1.0 / 60.0,
        ..InputState::default()
    }
}

#[test]
fn a_brain_drives_a_full_player_body_through_the_player_movement() {
    // A long floor at y=500; the player body starts just above it.
    let floor = Block::solid("floor", Vec2::new(-2000.0, 500.0), Vec2::new(8000.0, 60.0));
    let world = World::new(
        "player_clone_probe",
        Vec2::new(8000.0, 2000.0),
        Vec2::new(0.0, 460.0),
        vec![floor],
    );

    let mut scratch =
        ae::PlayerClusterScratch::new_with_abilities(Vec2::new(0.0, 460.0), ae::AbilitySet::sandbox_all());

    let dt = 1.0 / 60.0;

    // Let it settle onto the floor under gravity (no input).
    for _ in 0..40 {
        let mut clusters = scratch.as_mut();
        update_player_clusters(&world, &mut clusters, InputState::default(), dt);
    }
    let grounded_y = scratch.kinematics.pos.y;
    let start_x = scratch.kinematics.pos.x;
    assert!(
        scratch.ground.on_ground,
        "the player body settles onto the floor before the demo (y={grounded_y})",
    );

    // Now hand control to the BRAIN. Drive several full verb cycles
    // (Run -> Jump -> Dash -> Fly, ~1s each).
    let mut brain = Brain::StateMachine(StateMachineCfg::PlayerDemo {
        cfg: PlayerDemoCfg::default(),
        state: Default::default(),
    });

    let mut left_ground = false;
    let mut min_y = grounded_y; // smaller y == higher (engine +y is down)
    let mut max_speed_x: f32 = 0.0;
    let mut flew_high = false;

    for i in 0..(60 * 6) {
        let now = i as f32 * dt;
        let mut snapshot = BrainSnapshot::idle();
        snapshot.actor_pos = scratch.kinematics.pos;
        snapshot.actor_vel = scratch.kinematics.vel;
        snapshot.actor_facing = scratch.kinematics.facing;
        snapshot.actor_on_ground = scratch.ground.on_ground;
        snapshot.alive = true;
        snapshot.sim_time = now;
        snapshot.dt = dt;

        let mut frame = ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);

        let input = input_from_frame(&frame);
        let mut clusters = scratch.as_mut();
        update_player_clusters(&world, &mut clusters, input, dt);

        if !scratch.ground.on_ground {
            left_ground = true;
        }
        min_y = min_y.min(scratch.kinematics.pos.y);
        max_speed_x = max_speed_x.max(scratch.kinematics.vel.x.abs());
        if scratch.kinematics.pos.y < grounded_y - 140.0 {
            flew_high = true;
        }
    }

    assert!(
        left_ground,
        "the brain's jump/fly verbs took the player body off the ground",
    );
    assert!(
        min_y < grounded_y - 24.0,
        "the body rose off the floor under brain control (min_y={min_y}, floor={grounded_y})",
    );
    assert!(
        max_speed_x > 200.0,
        "the brain drove a fast horizontal move via run/dash (max_speed_x={max_speed_x})",
    );
    assert!(
        flew_high,
        "the brain's fly toggle carried the body well above the floor",
    );
    assert!(
        scratch.kinematics.pos.x > start_x + 100.0,
        "the body traveled right under brain control (dx={})",
        scratch.kinematics.pos.x - start_x,
    );
}
