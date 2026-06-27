//! Real-ECS headless DUEL harness — two brain-driven fighters dueling through the
//! EXACT game sim (the full `SandboxSim` app), the player a neutral observer.
//!
//! This is the "behaves exactly like the game, but headless" harness for iterating
//! on the advanced fighter brain (the smash brain that drives PCA + the robot).
//! Unlike the brain-contract proxy arena (`ambition_characters::brain::smash::arena`,
//! own kinematics), this drives the REAL movement pipeline, REAL collision, REAL
//! relational targeting, and REAL actor-vs-actor damage — so a green run means the
//! two fighters genuinely fight *in the engine*, not just at the brain-policy level.
//!
//! Setup: the two fighters are DIFFERENT factions (PCA `Enemy`, robot `Boss`) so
//! the physical damage rule lets them hurt each other (same-faction would be
//! friendly-fire-safe). `FactionRelations` makes Enemy↔Boss hostile (they TARGET
//! each other) and clears Player↔Enemy/Boss (the observer isn't targeted). Targeting
//! (who a brain aims at) is the relational concern; damage is physical when it lands —
//! so a stray could still catch the observer, which is fine.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_characters::actor::EnemyBrain;
use ambition_gameplay_core::actor::{BodyHealth, BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::combat::components::ActorFaction;
use ambition_gameplay_core::features::{
    FactionRelations, FeatureId, SpawnActorKind, SpawnActorRequest,
};
use ambition_engine_core as ae;
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

fn spawn_fighter(world: &mut World, id: &str, name: &str, brain_key: &str, pos: ae::Vec2) {
    world.write_message(SpawnActorRequest {
        id: id.to_string(),
        name: name.to_string(),
        pos,
        half_size: ae::Vec2::new(14.0, 23.0),
        kind: SpawnActorKind::Enemy {
            brain: EnemyBrain::Custom(brain_key.to_string()),
        },
    });
}

/// Two AI fighters (PCA + the robot) duel through the real sim; the player watches.
/// Pins that the engine can stage a brain-vs-brain fight at all — both fighters
/// stay live, MOVE (not frozen), and exchange real damage (at least one loses HP).
#[test]
fn pca_vs_robot_duel_is_a_real_fight_in_the_engine() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // Fighters share the player's ground line, off to one side, ~150px apart so
    // they engage quickly; the player stays put as the neutral observer.
    let origin = player_pos(sim.world_mut());
    let pca_spawn = origin + ae::Vec2::new(-260.0, 0.0);
    let robot_spawn = origin + ae::Vec2::new(-110.0, 0.0);

    // Targeting relations: the two fighters (Enemy + Boss) are hostile to EACH
    // OTHER (so they target each other), and NOT to the Player (observer ignored).
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Enemy, false);
    relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Boss, false);
    sim.world_mut().insert_resource(relations);

    spawn_fighter(
        sim.world_mut(),
        "duel_pca",
        "Perfect Cell-ular Automaton",
        "cellular_automaton_fighter",
        pca_spawn,
    );
    spawn_fighter(
        sim.world_mut(),
        "duel_robot",
        "Player Robot",
        "player_robot",
        robot_spawn,
    );

    // A couple of frames for the spawn requests to materialize.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }

    // Put the robot on the Boss faction so the two fighters are DIFFERENT factions
    // and the physical damage rule lets them hurt each other (same-faction allies
    // are friendly-fire-safe). (In an authored room, faction comes from the spawn;
    // here we override post-spawn.)
    {
        let world = sim.world_mut();
        let robot = {
            let mut q = world.query::<(bevy::prelude::Entity, &FeatureId)>();
            q.iter(world)
                .find(|(_, fid)| fid.as_str() == "duel_robot")
                .map(|(e, _)| e)
                .expect("robot spawned")
        };
        world.entity_mut(robot).insert(ActorFaction::Boss);
    }
    let pca0 = fighter_state(sim.world_mut(), "duel_pca").expect("PCA spawned");
    let robot0 = fighter_state(sim.world_mut(), "duel_robot").expect("robot spawned");

    // Run the duel for ~12s of sim time.
    for _ in 0..720 {
        sim.step(AgentAction::default());
    }

    let pca1 = fighter_state(sim.world_mut(), "duel_pca").expect("PCA still present");
    let robot1 = fighter_state(sim.world_mut(), "duel_robot").expect("robot still present");

    // Both fighters MOVED from spawn — neither is frozen (the brain is driving the
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
    // resolves combat through the engine, not just posturing.
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
