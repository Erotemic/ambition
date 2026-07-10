//! Phase C / C2 — the PROTAGONIST'S OWN KIT fights the player (invariant I7).
//!
//! The `player_robot` archetype carries the full player kit as body-enforced
//! capabilities (blink/fly/shield/dash + melee + the signature Hadouken ranged),
//! driven by the unified Smash brain. Prior work pinned that at the SPEC level
//! (`player_robot_archetype_carries_the_full_player_kit`) and as a duel NPC vs
//! the PCA. This pins the acceptance demo: dropped as a hostile combatant beside
//! the human, the player-robot ENGAGES and fights the player with that kit
//! through the ONE actor path (`update_ecs_actors`) — the player facing their
//! own character, under the strong brain. (Post-duel-reframe, combatant role is
//! faction DATA, not a special "boss" type; a hostile Enemy-faction player_robot
//! is the player-faces-its-own-kit scenario.)

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition::actors::combat::components::{ActorDisposition, ActorTarget};
use ambition::actors::features::FeatureId;
use ambition::actors::player::BodyMelee;
use ambition::characters::actor::BodyHealth;
use ambition::characters::brain::ActorControl;
use ambition::engine_core as ae;
use ambition::entity_catalog::placements::CharacterBrain;
use ambition::projectiles::enemy::EnemyProjectile;
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use bevy::prelude::World;

const ROBOT_ID: &str = "player_robot_boss";

fn player(world: &mut World) -> (ae::Vec2, i32) {
    let mut q = world.query_filtered::<(&BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, hp) = q.single(world).expect("primary player");
    (kin.pos, hp.current())
}

#[derive(Default, Debug)]
struct Tally {
    present: usize,
    hostile: usize,
    target_some: usize,
    melee_swing_frames: usize,
    projectile_frames: usize,
    min_dist: f32,
}

fn observe(world: &mut World, player_pos: ae::Vec2, t: &mut Tally) {
    // The player-robot's own engagement state.
    let mut q = world.query::<(
        &FeatureId,
        &BodyKinematics,
        &ActorDisposition,
        &ActorTarget,
        &BodyMelee,
    )>();
    if let Some((_, kin, disp, target, melee)) =
        q.iter(world).find(|(f, ..)| f.as_str() == ROBOT_ID)
    {
        t.present += 1;
        if disp.is_hostile() {
            t.hostile += 1;
        }
        if target.entity.is_some() {
            t.target_some += 1;
        }
        if melee.is_swinging() {
            t.melee_swing_frames += 1;
        }
        let d = (kin.pos - player_pos).length();
        if t.present == 1 || d < t.min_dist {
            t.min_dist = d;
        }
    }
    // Its signature ranged kit in flight (any actor-fired bolt this frame).
    let mut pq = world.query::<&EnemyProjectile>();
    if pq.iter(world).next().is_some() {
        t.projectile_frames += 1;
    }
}

#[test]
fn the_player_robot_fights_the_player_with_its_own_full_kit() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // Drop the protagonist's own body a medium stride away — far enough to open
    // with the signature Hadouken, close enough to then melee the human.
    let (p, start_hp) = player(sim.world_mut());
    sim.spawn_enemy_at(
        ROBOT_ID,
        "Player",
        (p.x + 200.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("player_robot".to_string()),
    );

    let mut t = Tally::default();
    for _ in 0..300 {
        sim.step(AgentAction::default());
        let (pp, _) = player(sim.world_mut());
        observe(sim.world_mut(), pp, &mut t);
    }
    let (_, end_hp) = player(sim.world_mut());

    println!("player-robot fight tally: {t:#?} | player hp {start_hp} -> {end_hp}");
    // Make sure the actor matters & has a control frame (it's a real fighter).
    assert!(
        sim.world_mut()
            .query::<(&FeatureId, &ActorControl)>()
            .iter(sim.world_mut())
            .any(|(f, _)| f.as_str() == ROBOT_ID),
        "the player-robot is a real ActorControl-driven body"
    );
    assert!(t.present > 100, "the player-robot persists: {t:#?}");
    assert!(
        t.hostile == t.present,
        "a hostile_to_player player-robot stays hostile: {t:#?}"
    );
    assert!(
        t.target_some > 0,
        "the player-robot acquires the player as its target: {t:#?}"
    );
    // It actually fights with its kit: at least one melee swing AND it lands
    // damage on the human (the protagonist's kit reaches the player through the
    // shared damage path).
    assert!(
        t.melee_swing_frames > 0,
        "the player-robot swings its melee at the player: {t:#?}"
    );
    assert!(
        end_hp < start_hp,
        "the player-robot's kit damages the player: hp {start_hp} -> {end_hp}, {t:#?}"
    );
    // And it uses its signature ranged kit too (full kit, not just melee).
    assert!(
        t.projectile_frames > 0,
        "the player-robot fires its signature Hadouken ranged kit: {t:#?}"
    );
}
