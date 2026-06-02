//! Actor stimulus → aggression updates.
//!
//! Damage should say "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation, fleeing, ignoring the hit, or some future
//! faction relationship change. This module is the first slice of that ECS seam:
//! cove NPCs still borrow the legacy enemy-shaped combat runtime when they
//! retaliate, but the decision is now driven by `ActorAggression + CombatKit +
//! HeldItem` instead of being embedded directly in the damage application loop.

use bevy::prelude::*;

use super::{
    enemy_runtime_for_npc_combat, make_entity_enemy, sync_actor_components_from_enemy,
    ActorAggression, ActorCombatState, ActorCooldowns, ActorDisposition, ActorHealth, ActorIdentity,
    ActorIntent, ActorRuntime, AggressionMode, CombatKit, FeatureSimEntity, HeldItem,
};
use super::super::enemies::EnemyRuntime;
use crate::features::ActorStimulus;

/// Apply actor stimuli to aggression state and, when an actor becomes
/// aggressive, derive its active brain/action set from the actor's durable
/// combat kit.
pub fn apply_actor_stimuli(
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
            Option<super::enemy_clusters::EnemyClusterQueryData>,
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
            mut clusters,
        )) = actors.get_mut(actor)
        else {
            continue;
        };

        if matches!(aggression.mode, AggressionMode::Passive) {
            continue;
        }
        aggression.target = source.or(aggression.target);

        let should_be_aggressive = match (&*runtime, aggression.mode) {
            (ActorRuntime::Npc(npc), AggressionMode::RetaliatesWhenHit { strike_threshold }) => {
                npc.strikes >= i32::from(strike_threshold)
            }
            (_, AggressionMode::HostileToPlayer) => true,
            (ActorRuntime::Enemy, AggressionMode::RetaliatesWhenHit { .. }) => true,
            _ => false,
        };
        if !should_be_aggressive {
            continue;
        }
        aggression.mode = AggressionMode::HostileToPlayer;

        if let ActorRuntime::Npc(npc) = &*runtime {
            let mut hostile = enemy_runtime_for_npc_combat(npc);
            if source.is_some() {
                hostile.ai_mode = crate::character_ai::CharacterAiMode::Chase;
            }
            let (brain, action_set) =
                super::brain_builders::aggressive_brain_and_action_set_for_enemy(
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
        } else if let Some(cq) = clusters.as_mut() {
            // Already an enemy: re-derive the aggressive brain from the
            // cluster config (reconstruct a throwaway EnemyRuntime — the
            // brain builders only read id + archetype).
            let mut em = cq.as_enemy_mut();
            let proxy = EnemyRuntime::new(
                em.config.id.clone(),
                em.config.name.clone(),
                em.aabb(),
                em.config.brain.clone(),
                &[],
            );
            let (brain, action_set) =
                super::brain_builders::aggressive_brain_and_action_set_for_enemy(
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
}
