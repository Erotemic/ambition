//! Real-ECS headless DUEL harness — two brain-driven fighters dueling through the
//! EXACT game sim (the full `SandboxSim` app), the player a neutral observer.
//!
//! This is the "behaves exactly like the game, but headless" test for the advanced
//! fighter brain (the smash brain that drives PCA + the robot). It stages the duel
//! through the SAME reusable helpers the in-game `<<duel>>` dialog command uses
//! (`apply_duel_relations` + `duel_spawn_requests`), so the test exercises the real
//! staging path, not a bespoke setup: PCA (`Enemy`) vs robot (`Boss`) on different
//! factions (so the physical damage rule lets them hurt each other), hostile to each
//! other but not to the player. Then it steps the full game loop and asserts a real
//! fight — both fighters move under their brains and exchange real damage.
//!
//! Unlike the brain-contract proxy arena (`smash::arena`, own kinematics), this
//! drives the real movement pipeline + collision + targeting + damage, so green
//! means the two brains fight *in the engine*.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_engine_core as ae;
use ambition_gameplay_core::actor::{BodyHealth, BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::features::{
    apply_duel_relations, duel_spawn_requests, FactionRelations, FeatureId, DUEL_PCA_ID,
    DUEL_ROBOT_ID,
};
use bevy::prelude::World;

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player exists").pos
}

/// `(pos, hp)` of the spawned fighter with this feature id, if it exists.
fn fighter_state(world: &mut World, id: &str) -> Option<(ae::Vec2, i32)> {
    let mut q = world.query::<(&FeatureId, &BodyKinematics, &BodyHealth)>();
    q.iter(world)
        .find(|(fid, _, _)| fid.as_str() == id)
        .map(|(_, kin, hp)| (kin.pos, hp.current()))
}

/// Two AI fighters (PCA + the robot) duel through the real sim; the player watches.
/// Pins that the engine can stage a brain-vs-brain fight — both fighters stay live,
/// MOVE (not frozen), and exchange real damage (at least one loses HP).
#[test]
fn pca_vs_robot_duel_is_a_real_fight_in_the_engine() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // Stage the duel off to one side, away from the player observer, through the SAME
    // helpers the `<<duel>>` command uses.
    let center = player_pos(sim.world_mut()) + ae::Vec2::new(-300.0, 0.0);
    {
        let world = sim.world_mut();
        let mut relations = world.get_resource_or_insert_with(FactionRelations::default);
        apply_duel_relations(&mut relations);
    }
    for req in duel_spawn_requests(center) {
        sim.world_mut().write_message(req);
    }

    // A couple of frames for the spawn requests to materialize.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    let pca0 = fighter_state(sim.world_mut(), DUEL_PCA_ID).expect("PCA spawned");
    let robot0 = fighter_state(sim.world_mut(), DUEL_ROBOT_ID).expect("robot spawned");

    // Run the duel for ~12s of sim time.
    for _ in 0..720 {
        sim.step(AgentAction::default());
    }

    let pca1 = fighter_state(sim.world_mut(), DUEL_PCA_ID).expect("PCA still present");
    let robot1 = fighter_state(sim.world_mut(), DUEL_ROBOT_ID).expect("robot still present");

    // Both fighters MOVED from spawn — neither is frozen (the brain drives the real
    // movement pipeline).
    let pca_moved = (pca1.0 - pca0.0).length();
    let robot_moved = (robot1.0 - robot0.0).length();
    assert!(
        pca_moved > 8.0,
        "PCA should move under its brain (moved {pca_moved:.1}px)"
    );
    assert!(
        robot_moved > 8.0,
        "robot should move under its brain (moved {robot_moved:.1}px)"
    );

    // Real damage exchanged: at least one fighter lost HP — the duel actually
    // resolves combat through the engine.
    let pca_damaged = pca1.1 < pca0.1;
    let robot_damaged = robot1.1 < robot0.1;
    assert!(
        pca_damaged || robot_damaged,
        "the duel should exchange real damage (PCA {} -> {}, robot {} -> {})",
        pca0.1,
        pca1.1,
        robot0.1,
        robot1.1
    );
}
