//! Attack clashes settle before projectile damage in the composed runtime.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::actor::{ActorFaction, BodyHealth};
use ambition_platformer2d::combat::clank::AttacksClanked;
use ambition_platformer2d::combat::rules::CombatRules;
use ambition_platformer2d::combat::scoped_rules::{DeclaredRules, RulesScope};
use ambition_platformer2d::engine_core::{BodyKinematics, Vec2};
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::{
    LiveProjectile, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart,
};
use bevy::prelude::*;

fn a_pair_at_the_victim(window: f32) -> (i32, usize, usize) {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(
        "switch_lab",
    ))
    .expect("the quiet room builds");
    sim.step_n(AgentAction::default(), 30);
    let mut rules = ambition_app::app::versus::versus_combat_rules();
    rules.clank_damage_window = window;
    sim.world_mut()
        .resource_mut::<DeclaredRules<CombatRules>>()
        .declare(RulesScope::UntaggedRooms, rules);
    sim.step(AgentAction::default());
    let victim = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
    };
    let at = sim.world().get::<BodyKinematics>(victim).unwrap().pos;
    sim.spawn_enemy_character_at(
        "clash_firer",
        "Clash firer",
        (at.x + 200.0, at.y),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "perfect_cellular_automaton",
    );
    let enemy = {
        let world = sim.world_mut();
        world
            .query::<(
                Entity,
                &ambition_platformer2d::combat::components::FeatureId,
            )>()
            .iter(world)
            .find(|(_, id)| id.as_str() == "clash_firer")
            .unwrap()
            .0
    };
    sim.world_mut().entity_mut(enemy).insert((
        ActorFaction::Enemy,
        ambition_platformer2d::characters::brain::Brain::stand_still(),
    ));
    let before = sim
        .world()
        .get::<BodyHealth>(victim)
        .unwrap()
        .damage_taken();
    for owner in [victim, enemy] {
        sim.world_mut().write_message(ProjectileSpawnRequest::open(
            owner,
            ProjectileSpawn {
                origin: at,
                dir: Vec2::new(1.0, 0.0),
                speed: 0.0,
                damage: 3,
                max_lifetime: 1.0,
                half_extent: Vec2::new(8.0, 8.0),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                boomerang_return_s: None,
                splash_half_extent: 0.0,
            },
            ProjectileStart::StepThisTick,
        ));
    }
    sim.step(AgentAction::default());
    let clanks = sim
        .world()
        .resource::<Messages<AttacksClanked>>()
        .iter_current_update_messages()
        .count();
    // Projectile contact stages a player hit after this tick's damage drain.
    // The next tick applies that staged hit to health.
    sim.step(AgentAction::default());
    let damage = sim
        .world()
        .get::<BodyHealth>(victim)
        .unwrap()
        .damage_taken()
        - before;
    let world = sim.world_mut();
    let shots = world
        .query_filtered::<Entity, With<LiveProjectile>>()
        .iter(world)
        .count();
    (damage, shots, clanks)
}

#[test]
fn opposed_shots_clash_before_either_can_damage_a_body() {
    let (damage, shots, clanks) = a_pair_at_the_victim(0.0);
    let enabled = a_pair_at_the_victim(9.0);
    assert!(
        damage > 0,
        "without a clash, the hostile shot must reach the victim"
    );
    assert_eq!((shots, clanks), (1, 0));
    assert_eq!(
        enabled,
        (0, 0, 1),
        "the clash must spend both shots before either damage path runs"
    );
}
