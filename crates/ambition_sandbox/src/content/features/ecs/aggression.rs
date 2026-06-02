//! Actor stimulus → aggression updates.
//!
//! Damage should say "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation, fleeing, ignoring the hit, or some future
//! faction relationship change. This module is the first slice of that ECS seam:
//! cove NPCs still borrow the legacy enemy-shaped combat runtime when they
//! retaliate, but the decision is now driven by `ActorAggression + CombatKit +
//! HeldItem` instead of being embedded directly in the damage application loop.

use bevy::prelude::*;

use super::super::enemies::EnemyRuntime;
use super::{
    enemy_runtime_for_npc_combat, make_entity_enemy, sync_actor_components_from_enemy,
    ActorAggression, ActorCombatState, ActorCooldowns, ActorDisposition, ActorHealth,
    ActorIdentity, ActorIntent, ActorRuntime, AggressionMode, CombatKit, FeatureSimEntity,
    HeldItem,
};
use crate::features::ActorStimulus;

/// Apply actor stimuli to NPC aggression: a peaceful NPC that has been
/// provoked past its strike threshold flips to a hostile enemy in
/// place. Split from the enemy path because the NPC cluster query and
/// enemy cluster query both borrow the shared kinematics/surface/motion
/// components mutably and cannot coexist in one query.
pub fn apply_npc_stimuli(
    mut commands: Commands,
    mut stimuli: MessageReader<ActorStimulus>,
    mut actors: Query<
        (
            Entity,
            &mut ActorRuntime,
            &mut ActorAggression,
            &CombatKit,
            Option<&HeldItem>,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            super::npc_clusters::NpcClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for stimulus in stimuli.read().copied() {
        let ActorStimulus::DamagedBy {
            actor,
            source,
            damage: _,
        } = stimulus;
        let Ok((
            entity,
            mut runtime,
            mut aggression,
            combat_kit,
            held_item,
            mut identity,
            mut disposition,
            mut health,
            mut combat,
            mut intent,
            mut cooldowns,
            npc,
        )) = actors.get_mut(actor)
        else {
            continue;
        };

        if matches!(aggression.mode, AggressionMode::Passive) {
            continue;
        }
        aggression.target = source.or(aggression.target);

        let should_be_aggressive = match aggression.mode {
            AggressionMode::RetaliatesWhenHit { strike_threshold } => {
                npc.status.strikes >= i32::from(strike_threshold)
            }
            AggressionMode::HostileToPlayer => true,
            AggressionMode::Passive => false,
        };
        if !should_be_aggressive {
            continue;
        }
        aggression.mode = AggressionMode::HostileToPlayer;

        let mut hostile = enemy_runtime_for_npc_combat(&npc.config, &npc.kin, &npc.surface);
        if source.is_some() {
            hostile.ai_mode = crate::character_ai::CharacterAiMode::Chase;
        }
        let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
            &hostile, combat_kit, held_item,
        );
        make_entity_enemy(
            &mut commands,
            entity,
            &mut runtime,
            &hostile,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
        commands.entity(entity).insert((brain, action_set));
    }
}

/// Apply actor stimuli to enemy aggression: an already-hostile enemy
/// re-derives its aggressive brain/action set when newly provoked.
pub fn apply_actor_stimuli(
    mut commands: Commands,
    mut stimuli: MessageReader<ActorStimulus>,
    mut actors: Query<
        (
            Entity,
            &mut ActorAggression,
            &CombatKit,
            Option<&HeldItem>,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            super::enemy_clusters::EnemyClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for stimulus in stimuli.read().copied() {
        let ActorStimulus::DamagedBy {
            actor,
            source,
            damage: _,
        } = stimulus;
        let Ok((
            entity,
            mut aggression,
            combat_kit,
            held_item,
            mut identity,
            mut disposition,
            mut health,
            mut combat,
            mut intent,
            mut cooldowns,
            mut cq,
        )) = actors.get_mut(actor)
        else {
            continue;
        };

        if matches!(aggression.mode, AggressionMode::Passive) {
            continue;
        }
        aggression.target = source.or(aggression.target);
        // Every non-passive enemy escalates to hostile on a hit.
        aggression.mode = AggressionMode::HostileToPlayer;

        // Re-derive the aggressive brain from the cluster config
        // (reconstruct a throwaway EnemyRuntime — the brain builders
        // only read id + archetype).
        let em = cq.as_enemy_mut();
        let proxy = EnemyRuntime::new(
            em.config.id.clone(),
            em.config.name.clone(),
            em.aabb(),
            em.config.brain.clone(),
            &[],
        );
        let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
            &proxy, combat_kit, held_item,
        );
        commands.entity(entity).insert((brain, action_set));
        sync_actor_components_from_enemy(
            &em,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}
