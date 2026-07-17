// Live, in-app proof that a brain-driven player CLONE moves through the same
// player movement systems as the human — the payoff of the universal-brain
// refactor. Only built with the RL stepping API.
#![cfg(feature = "rl_sim")]
//! Spawns a `PlayerClone` (a non-player body carrying the full player movement
//! clusters + a `PlayerDemo` brain) into the LIVE sandbox app via the
//! `SpawnPlayerCloneRequest` resource, steps the real schedule, and asserts the
//! clone runs, leaves the ground, and rises — driven entirely by its brain, with
//! no human input. Complements the engine-level proof in
//! `ambition::actors::avatar::clone_probe_tests`.

use crate::common::base;

use ambition::actors::actor::BodyGroundState;
use ambition::actors::actor::BodyKinematics;
use ambition_app::app::{PlayerClone, SpawnPlayerCloneRequest};
use bevy::prelude::{With, World};

/// (x, y, on_ground) of the single player clone, if it exists.
fn clone_state(world: &mut World) -> Option<(f32, f32, bool)> {
    let mut q = world.query_filtered::<(&BodyKinematics, &BodyGroundState), With<PlayerClone>>();
    q.iter(world)
        .next()
        .map(|(kin, ground)| (kin.pos.x, kin.pos.y, ground.on_ground))
}

#[test]
fn brain_driven_player_clone_runs_and_leaves_the_ground_in_the_live_app() {
    let mut sim = crate::common::fixed_60hz_sim();
    // Let the player settle into the room.
    sim.step_n(base(), 30);

    // Ask the app to spawn a brain-driven clone next frame (the K-hotkey path,
    // poked directly so the test needs no synthetic key event).
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    // Spawn, then let it fall + settle onto the floor before timing the demo.
    sim.step_n(base(), 50);

    let (start_x, start_y, _) = clone_state(sim.world_mut()).expect("the clone spawned");

    let mut min_y = start_y; // engine +y is DOWN, so smaller y == higher
    let mut max_x = start_x;
    let mut left_ground = false;
    // ~6s — several full Run -> Jump -> Dash -> Fly cycles.
    for _ in 0..(60 * 6) {
        sim.step_n(base(), 1);
        if let Some((x, y, on_ground)) = clone_state(sim.world_mut()) {
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            if !on_ground {
                left_ground = true;
            }
        }
    }

    assert!(
        left_ground,
        "the clone's brain (jump/fly) took it off the ground in-app",
    );
    assert!(
        min_y < start_y - 24.0,
        "the clone rose off the floor under brain control (min_y={min_y}, start_y={start_y})",
    );
    assert!(
        max_x > start_x + 60.0,
        "the clone ran horizontally under brain control (dx={})",
        max_x - start_x,
    );
}
