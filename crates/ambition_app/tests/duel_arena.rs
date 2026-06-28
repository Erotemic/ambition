//! Real-ECS headless DUEL harness — start the game in the authored `duel_arena`
//! room and watch the sim play out, exactly as if the player had walked through
//! the basement door into it.
//!
//! This is the "start a room and see how the sim plays out" test: it builds the
//! full `SandboxSim` app with `start_room = "duel_arena"`, so the room's normal
//! load path (`spawn_room_feature_entities` → `features::stage_room_duel`) auto-
//! stages the fight — a Perfect Cell-ular Automaton (`Enemy`) vs a robot copy of
//! the player (`Boss`), on different factions so the physical-damage rule lets
//! them hurt each other, hostile to each other but not to the observing player.
//! No trigger, no manual staging: the fight is already underway the instant the
//! room exists. Then we step the real game loop and assert a real fight — both
//! fighters move under their brains and exchange real damage.
//!
//! Unlike the brain-contract proxy arena (`smash::arena`, own kinematics), this
//! drives the real movement pipeline + collision + targeting + damage through the
//! actual authored room, so green means the two brains fight *in the game*.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions, TimestepMode};
use ambition_engine_core as ae;
use ambition_gameplay_core::actor::{BodyHealth, BodyKinematics};
use ambition_gameplay_core::features::{FeatureId, DUEL_PCA_ID, DUEL_ROBOT_ID};
use bevy::prelude::World;

/// `(pos, hp)` of the spawned fighter with this feature id, if it exists.
fn fighter_state(world: &mut World, id: &str) -> Option<(ae::Vec2, i32)> {
    let mut q = world.query::<(&FeatureId, &BodyKinematics, &BodyHealth)>();
    q.iter(world)
        .find(|(fid, _, _)| fid.as_str() == id)
        .map(|(_, kin, hp)| (kin.pos, hp.current()))
}

/// Start the game in the authored duel arena; the two AI fighters auto-stage and
/// duel through the real sim while the player watches. Pins that walking into the
/// room yields a real brain-vs-brain fight — both fighters stay live, MOVE (not
/// frozen), and exchange real damage (at least one loses HP).
#[test]
fn duel_arena_room_auto_stages_a_real_fight() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("duel_arena"),
    )
    .expect("sandbox sim builds in the duel arena");

    // A couple of frames for the room-load spawn requests to materialize.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    let pca0 = fighter_state(sim.world_mut(), DUEL_PCA_ID).expect("PCA auto-spawned on room load");
    let robot0 =
        fighter_state(sim.world_mut(), DUEL_ROBOT_ID).expect("robot auto-spawned on room load");

    // Run the duel for ~12s of sim time.
    for _ in 0..720 {
        sim.step(AgentAction::default());
    }

    let pca1 = fighter_state(sim.world_mut(), DUEL_PCA_ID).expect("PCA still present");
    let robot1 = fighter_state(sim.world_mut(), DUEL_ROBOT_ID).expect("robot still present");

    // Both fighters MOVED from spawn — neither is frozen (the brain drives the
    // real movement pipeline).
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
