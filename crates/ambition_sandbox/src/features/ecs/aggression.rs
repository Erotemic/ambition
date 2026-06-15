//! Actor stimulus → aggression updates.
//!
//! Damage should say "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation, fleeing, ignoring the hit, or some future
//! faction relationship change. Cove NPCs convert to enemy component clusters
//! when provoked; the decision is driven by `ActorAggression + CombatKit +
//! HeldItem`, not by the damage application loop.

use bevy::prelude::*;

use super::{
    sync_actor_components_from_enemy, ActorAggression, ActorCombatState, ActorCooldowns,
    ActorDisposition, ActorHealth, ActorIdentity, ActorIntent, ActorRuntime, AggressionMode,
    CombatKit, FeatureSimEntity, HeldItem,
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

        let conversion = super::actors::HostileNpcConversionPlan::from_npc(
            &npc.config,
            &npc.kin,
            &npc.surface,
            combat_kit,
            held_item,
        );
        let conversion = if source.is_some() {
            conversion.with_chase()
        } else {
            conversion
        };
        conversion.apply(
            &mut commands,
            entity,
            &mut runtime,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
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

        // Re-derive the aggressive brain from the live enemy config.
        let em = cq.as_enemy_mut();
        let (brain, action_set) = super::brain_builders::aggressive_brain_and_action_set_for_enemy(
            em.config, combat_kit, held_item,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_core::{self as ae, AabbExt};
    use crate::features::NPC_HOSTILE_STRIKE_THRESHOLD;
    use crate::features::{CenteredAabb, FeatureId, FeatureSimEntity};
    use bevy::prelude::{App, Update};

    fn spawn_npc_with_strikes(app: &mut App, strikes: i32) -> bevy::prelude::Entity {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
        let interactable = crate::interaction::Interactable::new(
            "alice",
            "Talk",
            aabb,
            crate::interaction::InteractionKind::Npc {
                dialogue_id: None,
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        );
        let npc = super::super::npc_clusters::NpcClusterScratch::new_with_paths(
            "alice",
            "Alice",
            aabb,
            interactable,
            &[],
        );
        let mut bundle = npc.into_components();
        bundle.4.strikes = strikes; // NpcStatus.strikes
        let (identity, disposition, health, combat, intent, cooldowns) =
            super::super::actors::npc_component_snapshot(&bundle.3, &bundle.4);
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("alice"),
                CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
                ActorRuntime::Npc,
                ActorAggression::retaliates_when_hit(
                    crate::features::NPC_HOSTILE_STRIKE_THRESHOLD as u8,
                ),
                CombatKit::default(),
                bundle,
                identity,
                disposition,
                health,
                combat,
                intent,
                cooldowns,
            ))
            .id()
    }

    fn run(app: &mut App, actor: bevy::prelude::Entity) {
        app.world_mut().write_message(ActorStimulus::DamagedBy {
            actor,
            source: None,
            damage: 1,
        });
        app.update();
    }

    #[test]
    fn npc_flips_hostile_once_strikes_reach_the_threshold() {
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_npc_stimuli);
        // Already at the strike threshold (the damage system increments
        // strikes; this stimulus is the provocation that re-evaluates).
        let npc = spawn_npc_with_strikes(&mut app, NPC_HOSTILE_STRIKE_THRESHOLD);
        run(&mut app, npc);
        assert_eq!(
            *app.world().get::<ActorDisposition>(npc).unwrap(),
            ActorDisposition::Hostile,
            "an NPC at the strike threshold should flip hostile when provoked"
        );
    }

    #[test]
    fn npc_below_the_threshold_stays_peaceful() {
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_npc_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, NPC_HOSTILE_STRIKE_THRESHOLD - 1);
        run(&mut app, npc);
        assert_eq!(
            *app.world().get::<ActorDisposition>(npc).unwrap(),
            ActorDisposition::Peaceful,
            "an NPC below the strike threshold should stay peaceful"
        );
    }
}
