//! Actor stimulus → aggression updates.
//!
//! Damage says "this actor was hit"; the actor's aggression policy decides
//! whether that means retaliation (flip hostile), escalation (already hostile),
//! or nothing (passive). Every actor is the SAME cluster now, so one system over
//! one query handles both the peaceful→hostile flip and the
//! already-hostile re-derive — in place, no cluster swap, no entity churn.

use bevy::prelude::*;

use super::{sync_actor_components_from_cluster, HeldItem};
use ambition_combat::components::{
    ActorAggression, ActorDisposition, ActorIdentity, ActorInteraction, AggressionMode, CombatKit,
};
use ambition_combat::events::ActorStimulus;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

/// Apply actor stimuli to aggression: a non-passive actor that crosses its
/// provocation threshold flips hostile IN PLACE (peaceful NPC → hostile
/// archetype), and an already-hostile actor re-derives its aggressive brain.
pub fn apply_actor_stimuli(
    mut commands: Commands,
    // The prepared cast, so a provoked body can take its own CHARACTER's
    // answer instead of one matched out of its display name.
    prepared: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
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
            super::actor_clusters::ActorClusterQueryData,
            // WHICH CHARACTER THIS BODY IS — the gameplay identity, not
            // the sprite's. See `provoke_actor_in_place`.
            Option<&ambition_characters::actor::WornCharacter>,
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
            _interaction,
            mut identity,
            mut disposition,
            mut cq,
            worn,
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
                // With friendly-fire off, only a real foe's hit lands here, so this never
                // spuriously re-aggros on an ally.
                AggressionMode::Hostile => true,
                AggressionMode::Passive => false,
            };
            if !should_be_aggressive {
                continue;
            }
        } else {
            aggression.target = source.or(aggression.target);
        }
        aggression.mode = AggressionMode::Hostile;
        // Hold a grudge against the attacker (the entity that struck / challenged
        // it). Targeting hunts a grudge entity exactly like a relational faction-foe,
        // so a provoked NPC chases its attacker WITHOUT mutating its `ActorFaction`
        // identity (the old in-place flip to `Enemy`). `None` source (a test, or a
        // hazard with no attacker) leaves the grudge unset → it fights along faction
        // lines only.
        aggression.grudge = source.or(aggression.grudge);

        let mut em = cq.as_actor_mut();
        super::actors::provoke_actor_in_place(
            &mut commands,
            entity,
            &mut em,
            &mut disposition,
            combat_kit,
            held_item,
            worn.map(ambition_characters::actor::WornCharacter::id),
            prepared.as_deref(),
            // Chase immediately when challenged (the duel is on), or when a
            // damage source is known.
            challenged || source.is_some(),
        );
        sync_actor_components_from_cluster(&em, &mut identity);
    }
}

/// Grace period (seconds, gameplay time) between a dialog-gated `<<challenge>>`
/// closing and the actor actually flipping hostile. Lets the player step out of
/// the NPC's body before the fight starts.
pub const CHALLENGE_GRACE_S: f32 = 2.0;

/// An ARMED `<<challenge>>`: the player consented to fight this actor, but the hostile flip is
/// deferred until the dialog box has closed AND `grace` seconds of gameplay have elapsed.
/// Without the delay the actor turned hostile mid-dialog while the player was still reading the
/// box and overlapping its body — and because the victim-side damage system is gated off during
/// dialog, the player's post-hit i-frame never got set, so the actor's body-contact FX streamed
/// every frame with no separation. The grace gives the player a chance to move away. rollback
/// state, and it was not — `<<challenge>>` inserted it from `Update` while
/// [`tick_pending_challenges`] removes it in the sim schedule. It rewinds now, and the insert
/// is a simulation decision like every other.
#[derive(Component, Clone, Copy, Debug)]
pub struct PendingChallenge {
    pub challenger: Option<Entity>,
    pub grace: f32,
}

impl bevy::ecs::entity::MapEntities for PendingChallenge {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(challenger) = self.challenger.as_mut() {
            *challenger = mapper.get_mapped(*challenger);
        }
    }
}

/// A conversation asked for a fight, by stable identity.
///
/// The `SimId` routing is the one [`crate::features::BrainCommand`] already uses, and for the same
/// stated reason — *"so the runtime switch is deterministic and snapshot-safe"*.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ChallengeRequested {
    /// The body being challenged.
    pub target: ambition_platformer2d_shared_tangle::sim_id::SimId,
    /// Who challenged it. `None` when nobody in the world did.
    pub challenger: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

/// Arm a challenge the narrative asked for. (sim)
///
/// ARMS rather than flips: the player is still in the dialog box (and likely
/// overlapping the NPC) when `<<challenge>>` fires, so
/// [`tick_pending_challenges`] emits the actual stimulus only after the box
/// closes and the grace elapses. See [`PendingChallenge`].
pub fn arm_requested_challenges(
    mut requests: MessageReader<ChallengeRequested>,
    bodies: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId)>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let entity_of = |wanted: &ambition_platformer2d_shared_tangle::sim_id::SimId| {
            bodies
                .iter()
                .find(|(_, id)| *id == wanted)
                .map(|(entity, _)| entity)
        };
        let Some(target) = entity_of(&request.target) else {
            warn!(
                target: "crate::features::aggression",
                "challenge for {} names no live body; ignoring", request.target,
            );
            continue;
        };
        commands.entity(target).insert(PendingChallenge {
            challenger: request.challenger.as_ref().and_then(entity_of),
            grace: CHALLENGE_GRACE_S,
        });
    }
}

/// Count down each armed [`PendingChallenge`] and, once its grace elapses, emit the
/// `Challenged` stimulus that `apply_actor_stimuli` turns into the hostile flip.
/// Gated on `gameplay_allowed`, so the grace only ticks in `Playing` — i.e. AFTER
/// the dialog box closes (dialog/cutscene/pause suspend gameplay), exactly matching
/// "flip hostile after the box closes and a few seconds pass".
pub fn tick_pending_challenges(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut pending: Query<(Entity, &mut PendingChallenge)>,
    mut stimuli: MessageWriter<ActorStimulus>,
) {
    let dt = world_time.scaled_dt;
    for (entity, mut pc) in &mut pending {
        pc.grace -= dt;
        if pc.grace <= 0.0 {
            stimuli.write(ActorStimulus::Challenged {
                actor: entity,
                challenger: pc.challenger,
            });
            commands.entity(entity).remove::<PendingChallenge>();
        }
    }
}

#[cfg(test)]
mod tests;
