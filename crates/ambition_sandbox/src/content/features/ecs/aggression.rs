//! Actor stimulus → aggression updates.
//!
//! Damage should say "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation, fleeing, ignoring the hit, or some future
//! faction relationship change. This module is the first slice of that ECS seam:
//! cove NPCs still convert through the legacy `ActorRuntime::Hostile` runtime, but
//! the decision is now driven by `ActorAggression + CombatKit + HeldItem` instead
//! of being embedded directly in the damage application loop.

use bevy::prelude::*;

use super::{
    actor_component_snapshot, sync_actor_components_from_runtime, ActorAggression, ActorCombatState,
    ActorCooldowns, ActorDisposition, ActorHealth, ActorIdentity, ActorIntent, ActorRuntime,
    AggressionMode, CombatKit, FeatureSimEntity, HeldItem,
};
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
        ),
        With<FeatureSimEntity>,
    >,
) {
    for stimulus in stimuli.read().copied() {
        let ActorStimulus::DamagedBy { actor, source, damage: _ } = stimulus;
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
        )) = actors.get_mut(actor) else {
            continue;
        };

        if matches!(aggression.mode, AggressionMode::Passive) {
            continue;
        }
        aggression.target = source.or(aggression.target);

        let should_be_aggressive = match (&*runtime, aggression.mode) {
            (ActorRuntime::Peaceful(npc), AggressionMode::RetaliatesWhenHit { strike_threshold }) => {
                npc.strikes >= i32::from(strike_threshold)
            }
            (_, AggressionMode::HostileToPlayer) => true,
            (ActorRuntime::Hostile(_), AggressionMode::RetaliatesWhenHit { .. }) => true,
            _ => false,
        };
        if !should_be_aggressive {
            continue;
        }
        aggression.mode = AggressionMode::HostileToPlayer;

        if let ActorRuntime::Peaceful(npc) = &*runtime {
            let mut hostile = ActorRuntime::hostile_from_npc(npc);
            if source.is_some() {
                hostile.ai_mode = crate::character_ai::CharacterAiMode::Chase;
            }
            let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
                &hostile,
                combat_kit,
                held_item,
            );
            *runtime = ActorRuntime::Hostile(hostile);
            commands.entity(entity).insert((brain, action_set));
        } else if let ActorRuntime::Hostile(enemy) = &*runtime {
            let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
                enemy,
                combat_kit,
                held_item,
            );
            commands.entity(entity).insert((brain, action_set));
        }

        sync_actor_components_from_runtime(
            &runtime,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}
