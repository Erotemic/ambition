//! Actor stimulus → aggression updates.
//!
//! Damage says "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation (flip hostile), escalation (already hostile),
//! or nothing (passive). Every actor is the SAME cluster now, so one system over
//! one query handles both the peaceful→hostile flip and the
//! already-hostile re-derive — in place, no cluster swap, no entity churn.

use bevy::prelude::*;

use super::{
    sync_actor_components_from_cluster, ActorAggression, ActorCombatState, ActorCooldowns,
    ActorDisposition, ActorHealth, ActorIdentity, ActorIntent, ActorInteraction, AggressionMode,
    CombatKit, FeatureSimEntity, HeldItem,
};
use crate::features::ActorStimulus;

/// Apply actor stimuli to aggression: a non-passive actor that crosses its
/// provocation threshold flips hostile IN PLACE (peaceful NPC → hostile
/// archetype), and an already-hostile actor re-derives its aggressive brain.
pub fn apply_actor_stimuli(
    mut commands: Commands,
    mut stimuli: MessageReader<ActorStimulus>,
    mut actors: Query<
        (
            Entity,
            &mut ActorAggression,
            &CombatKit,
            Option<&HeldItem>,
            Option<&ActorInteraction>,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            super::actor_clusters::ActorClusterQueryData,
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
            interaction,
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

        let should_be_aggressive = match aggression.mode {
            AggressionMode::RetaliatesWhenHit { strike_threshold } => {
                aggression.strikes >= i32::from(strike_threshold)
            }
            AggressionMode::HostileToPlayer => true,
            AggressionMode::Passive => false,
        };
        if !should_be_aggressive {
            continue;
        }
        aggression.mode = AggressionMode::HostileToPlayer;

        let dialogue_id = interaction.and_then(|i| match &i.interactable.kind {
            ambition_interaction::InteractionKind::Npc { dialogue_id, .. } => dialogue_id.as_deref(),
            _ => None,
        });

        let mut em = cq.as_actor_mut();
        super::actors::provoke_actor_in_place(
            &mut commands,
            entity,
            &mut em,
            &mut disposition,
            combat_kit,
            held_item,
            dialogue_id,
            source.is_some(),
        );
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
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
        let interactable = ambition_interaction::Interactable::new(
            "alice",
            "Talk",
            aabb,
            ambition_interaction::InteractionKind::Npc {
                character_id: None,
                dialogue_id: None,
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        );
        // Peaceful actor = the unified enemy cluster with peaceful tuning.
        let (seed, _render) = super::super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
            "alice",
            "Alice",
            aabb,
            &interactable,
            &[],
        );
        let (identity, disposition, health, combat, intent, cooldowns) =
            super::super::actors::actor_component_snapshot(&seed, ActorDisposition::Peaceful);
        // Provoke accumulator lives on `ActorAggression` now.
        let aggression = ActorAggression {
            mode: AggressionMode::RetaliatesWhenHit {
                strike_threshold: NPC_HOSTILE_STRIKE_THRESHOLD as u8,
            },
            target: None,
            strikes,
        };
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("alice"),
                CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
                aggression,
                CombatKit::default(),
                seed.into_components(),
                ActorInteraction {
                    interactable,
                    talk_radius: crate::features::NPC_TALK_RADIUS,
                },
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
        app.add_systems(Update, apply_actor_stimuli);
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
        app.add_systems(Update, apply_actor_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, NPC_HOSTILE_STRIKE_THRESHOLD - 1);
        run(&mut app, npc);
        assert_eq!(
            *app.world().get::<ActorDisposition>(npc).unwrap(),
            ActorDisposition::Peaceful,
            "an NPC below the strike threshold should stay peaceful"
        );
    }
}
