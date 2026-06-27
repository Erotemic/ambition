//! Actor stimulus → aggression updates.
//!
//! Damage says "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation (flip hostile), escalation (already hostile),
//! or nothing (passive). Every actor is the SAME cluster now, so one system over
//! one query handles both the peaceful→hostile flip and the
//! already-hostile re-derive — in place, no cluster swap, no entity churn.

use bevy::prelude::*;

use super::{
    sync_actor_components_from_cluster, ActorAggression, BodyCombat, ActorCooldowns,
    ActorDisposition, ActorIdentity, ActorIntent, ActorInteraction, AggressionMode,
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
            &mut BodyCombat,
            &mut ActorIntent,
            &mut ActorCooldowns,
            super::actor_clusters::ActorClusterQueryData,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for stimulus in stimuli.read().copied() {
        // A `Challenged` stimulus is the player's explicit consent to fight, so
        // it provokes unconditionally; `DamagedBy` defers to the actor's
        // aggression policy (passive actors ignore it, retaliators need to cross
        // their strike threshold). Both funnel into the SAME in-place flip.
        let (actor, source, challenged) = match stimulus {
            ActorStimulus::DamagedBy { actor, source, .. } => (actor, source, false),
            ActorStimulus::Challenged { actor, challenger } => (actor, challenger, true),
        };
        let Ok((
            entity,
            mut aggression,
            combat_kit,
            held_item,
            interaction,
            mut identity,
            mut disposition,
            mut combat,
            mut intent,
            mut cooldowns,
            mut cq,
        )) = actors.get_mut(actor)
        else {
            continue;
        };

        // The challenge bypasses the passivity / threshold gates entirely.
        if !challenged {
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
        } else {
            aggression.target = source.or(aggression.target);
        }
        aggression.mode = AggressionMode::HostileToPlayer;

        let dialogue_id = interaction.and_then(|i| match &i.interactable.kind {
            ambition_interaction::InteractionKind::Npc { dialogue_id, .. } => {
                dialogue_id.as_deref()
            }
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
            // Chase immediately when challenged (the duel is on), or when a
            // damage source is known.
            challenged || source.is_some(),
        );
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::NPC_HOSTILE_STRIKE_THRESHOLD;
    use crate::features::{CenteredAabb, FeatureId, FeatureSimEntity};
    use ambition_engine_core::{self as ae, AabbExt};
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
        let (identity, disposition, combat, intent, cooldowns) =
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

    #[test]
    fn a_challenge_flips_a_peaceful_npc_hostile_with_zero_strikes() {
        // The dialogue-gated combat trigger: an explicit `Challenged`
        // stimulus provokes the actor unconditionally — no strikes, no
        // threshold — because picking "challenge" IS consent to fight. This
        // is the gate the Perfect Cell-ular Automaton encounter rides on.
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_actor_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, 0);
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: npc,
            challenger: None,
        });
        app.update();
        assert_eq!(
            *app.world().get::<ActorDisposition>(npc).unwrap(),
            ActorDisposition::Hostile,
            "a challenged NPC must flip hostile even with zero strikes"
        );
        // The flip swaps in a hostile combat brain (the generic provoked NPC
        // resolves to the `combatant` Smash brawler). Pin that it's now a
        // reactive fighter, not the peaceful stand-still brain.
        let brain = app
            .world()
            .get::<ambition_characters::brain::Brain>(npc)
            .expect("provoke inserts a Brain");
        assert!(
            brain.is_hostile(),
            "the post-challenge brain should be hostile, got {}",
            brain.label()
        );
    }

    /// Regression: a SECOND stimulus on an already-hostile actor must NOT rebuild
    /// its brain. Re-deriving the brain on every stimulus zeroed all of its
    /// `SmashState` cadences (ranged / dash / blink / footsies timers, mode-dwell
    /// hysteresis) each hit — which is what turned the Perfect Cell-ular Automaton
    /// into a per-tick glider spammer that never got to duel. The live brain (and
    /// its accumulated state) must persist across repeat stimuli.
    #[test]
    fn a_repeat_stimulus_preserves_an_already_hostile_brain_state() {
        use ambition_characters::brain::{Brain, StateMachineCfg};
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_actor_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, 0);
        // First stimulus: the peaceful→hostile flip builds the (combatant Smash)
        // brain exactly once.
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: npc,
            challenger: None,
        });
        app.update();
        // Advance a cadence on the LIVE brain, as a mid-duel shot would.
        const SENTINEL: f32 = 0.9;
        {
            let mut brain = app
                .world_mut()
                .get_mut::<Brain>(npc)
                .expect("the flip inserts a Brain");
            let Brain::StateMachine(StateMachineCfg::Smash { state, .. }) = &mut *brain else {
                panic!("the provoked combatant should be a Smash brain");
            };
            state.dash_cooldown_remaining = SENTINEL;
            state.mode_dwell_s = SENTINEL;
        }
        // A second stimulus on the now-hostile actor must leave the brain intact.
        app.world_mut().write_message(ActorStimulus::DamagedBy {
            actor: npc,
            source: None,
            damage: 1,
        });
        app.update();
        let brain = app.world().get::<Brain>(npc).unwrap();
        let Brain::StateMachine(StateMachineCfg::Smash { state, .. }) = brain else {
            panic!("the brain should still be a Smash brain");
        };
        assert_eq!(
            state.dash_cooldown_remaining, SENTINEL,
            "a repeat stimulus must not reset the brain's dash cadence (no brain rebuild)"
        );
        assert_eq!(
            state.mode_dwell_s, SENTINEL,
            "a repeat stimulus must not reset mode-dwell hysteresis"
        );
    }

    #[test]
    fn a_floating_npc_grounds_when_provoked_into_a_grounded_archetype() {
        // The Perfect Cell-ular Automaton path: a peaceful *Floating* NPC
        // (gravity_scale 0 at spawn) that challenges into a grounded Smash
        // archetype must re-sync gravity_scale to 1.0 — otherwise the aerial
        // integrator reads `velocity_target` (which the grounded brain never
        // sets) and the actor freezes mid-air. Pins the provoke gravity sync.
        use crate::features::enemies::ActorSurfaceState;
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_actor_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, 0);
        // Force the spawned NPC to float, as a catalog `Floating` body would.
        app.world_mut()
            .get_mut::<ActorSurfaceState>(npc)
            .unwrap()
            .gravity_scale = 0.0;
        app.world_mut().write_message(ActorStimulus::Challenged {
            actor: npc,
            challenger: None,
        });
        app.update();
        let g = app
            .world()
            .get::<ActorSurfaceState>(npc)
            .unwrap()
            .gravity_scale;
        assert_eq!(
            g, 1.0,
            "a floating NPC provoked into a grounded archetype must drop to the ground"
        );
    }

    #[test]
    fn an_un_challenged_passive_npc_ignores_damage() {
        // Symmetric negative: without the explicit challenge, a passive
        // actor stays peaceful when merely damaged — only the challenge (or
        // crossing the retaliation threshold) arms the fight.
        let mut app = App::new();
        app.add_message::<ActorStimulus>();
        app.add_systems(Update, apply_actor_stimuli);
        let npc = spawn_npc_with_strikes(&mut app, 0);
        // Force passive so DamagedBy is a no-op.
        app.world_mut()
            .get_mut::<ActorAggression>(npc)
            .unwrap()
            .mode = AggressionMode::Passive;
        run(&mut app, npc);
        assert_eq!(
            *app.world().get::<ActorDisposition>(npc).unwrap(),
            ActorDisposition::Peaceful,
            "a passive, un-challenged NPC stays peaceful under damage"
        );
    }
}
