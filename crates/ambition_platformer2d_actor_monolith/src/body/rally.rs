//! Rally: a body that fights beside another faction joins that faction's fight
//! in a dangerous room, and goes back to its own mind when the fight ends.
//!
//! ONE CONCERN, and it is a provocation by the fight rather than by a blow. A
//! peaceful body that [`FactionRelations::fights_beside`] the player, in a room
//! authored `dangerous` (`RoomMetadata::dangerous`), takes as its own a foe
//! that has come to blows with the player (the player struck it, or it swings
//! at the player now): the same in-place flip a provocation makes
//! ([`provoke_actor_in_place`](crate::body::actors::provoke_actor_in_place)),
//! so it fights with its own kit and its character's own fighting mind (its
//! provoked policy), and holds a grudge against that foe.
//!
//! A rally is told apart from a provocation by the aggression MODE: a
//! provocation sets `Hostile`, a rally keeps the body's own
//! `RetaliatesWhenHit`. When the rallied body has no foe left, or its foe
//! fights nobody on its side any more, this system ends the feud and restores
//! its authored mind (`BrainCommand::restore_default`), which rebuilds around its authored
//! home. No state of its own: everything it reads and writes already rewinds.
//!
//! Which rooms are dangerous and which factions ally are the game's content.

use bevy::prelude::*;

use ambition_characters::actor::{ActorFaction, BodyHealth};
use ambition_characters::control::brain_binding::{AutonomousSource, BrainBinding};
use ambition_combat::components::{
    ActorAggression, ActorDisposition, ActorTarget, AggressionMode, CenteredAabb, Grudge,
};
use ambition_combat::targeting::{effective_faction, FactionRelations};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_room_of, FeatureSimEntity, InRoomInstance, LiveBodyId, LiveRoomInstance,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::body::BrainCommand;

/// Rally each idle ally in a dangerous room to the nearest foe of its side
/// that has come to blows with its side, and stand each rallied body down when
/// its fight is over. A foe that only watches the player does not start a
/// rally, so an ally stays talkable at its post until there is a fight.
///
/// A body rallies only when it can fight: it has a kit (a melee or ranged
/// verb, or a move) and its character resolves a provoked policy. A body with
/// neither stays out of the fight, as it does when struck. It notices a foe
/// only inside its fighting mind's `aggro_radius`.
#[allow(clippy::type_complexity)]
pub fn rally_allies_to_the_fight(
    mut commands: Commands,
    prepared: ambition_characters::prepared::SessionCast,
    relations: Option<Res<FactionRelations>>,
    room_specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut brain_commands: MessageWriter<BrainCommand>,
    // Every body that can be a foe or a foe's quarry, players included.
    bodies: Query<(
        Entity,
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&ambition_platformer2d_core::BodyKinematics>,
    )>,
    mut sets: ParamSet<(
    Query<
        (
            Entity,
            &CenteredAabb,
            &ActorFaction,
            Option<&ambition_characters::control::DrivingParticipant>,
            &ActorTarget,
            &BodyHealth,
            Has<ambition_combat::death_rules::OutOfPlay>,
            Option<&ambition_platformer2d_core::DepthPlane>,
            Option<&SimId>,
            Option<&ambition_combat::actor_tuning::ActorPolicy>,
            Option<&ActorAggression>,
            ambition_combat::moveset::MeleeSwingQuery,
        ),
        With<FeatureSimEntity>,
    >,
    Query<
        (
            Entity,
            &SimId,
            (Option<&InRoomInstance>, Option<&LiveRoomInstance>),
            &mut ActorAggression,
            &mut ActorDisposition,
            Option<&ambition_characters::brain::ActionSet>,
            Option<&ambition_combat::moveset::ActorMoveset>,
            &ambition_combat::actor_tuning::ActorConfig,
            &ambition_combat::components::ActorIdentity,
            &ambition_platformer2d_core::BodyAbilities,
            &ambition_characters::actor::WornCharacter,
            Option<&BrainBinding>,
        ),
        (
            With<FeatureSimEntity>,
            // A driven body fights as its driver decides.
            Without<ambition_characters::control::DrivingParticipant>,
            Without<ambition_combat::death_rules::OutOfPlay>,
        ),
    >,
    )>,
) {
    let Some(relations) = relations else {
        return;
    };
    // The foes this tick, in canonical order, so a tie in distance is decided
    // by stable identity and not by `Entity` (which a rewind does not keep).
    let mut candidates: Vec<_> = sets
        .p0()
        .iter()
        .filter(|(_, _, _, _, _, hp, out_of_play, plane, ..)| {
            !ambition_combat::util::body_is_untouchable(Some(*hp), *out_of_play, *plane)
        })
        .map(|(entity, aabb, faction, driver, target, _, _, _, sim_id, policy, aggression, swing)| {
            // Whom this body is fighting NOW: its target, when the target is
            // inside its own notice range. A foe that only points at a player
            // across the room is not in a fight yet.
            let engaged = target.entity.and_then(|quarry| {
                let (_, _, _, kin) = bodies.get(quarry).ok()?;
                let notice = policy?.0.aggro_radius;
                (kin?.pos.distance_squared(aabb.center) <= notice * notice).then_some(quarry)
            });
            // Whom it has come to blows with: the body its grudge names (a
            // body that struck it), or its quarry while it swings at it.
            let blows = aggression
                .and_then(|aggression| aggression.grudge?.body())
                .or(engaged.filter(|_| swing.swing().is_some()));
            (entity, aabb.center, effective_faction(*faction, driver), engaged, sim_id.cloned(), rooms.of(entity), blows)
        })
        .collect();
    candidates.sort_by(|a, b| match (&a.4, &b.4) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });
    let faction_of = |entity: Entity| {
        bodies
            .get(entity)
            .ok()
            .map(|(_, faction, driver, _)| effective_faction(*faction, driver))
    };

    for (
        entity,
        sim_id,
        (stamp, root),
        mut aggression,
        mut disposition,
        repertoire,
        moveset,
        config,
        identity,
        abilities,
        worn,
        binding,
    ) in &mut sets.p1()
    {
        // A rally keeps the body's own retaliation mode; a passive body never
        // fights, and a provoked one (`Hostile`) is provocation's.
        if !matches!(aggression.mode, AggressionMode::RetaliatesWhenHit { .. }) {
            continue;
        }
        let rallied = binding
            .is_some_and(|binding| matches!(binding.source, AutonomousSource::ProvokedProfile { .. }));

        // THE FIGHT IS OVER: targeting found no foe and stood it down, or
        // its foe fights nobody on its side any more. Its feud ends with the
        // fight, and its authored mind comes back.
        let self_faction = faction_of(entity);
        let foe_still_fights = aggression.grudge.and_then(Grudge::body).is_some_and(|foe| {
            candidates.iter().any(|(candidate, _, foe_faction, quarry, ..)| {
                *candidate == foe
                    && quarry.is_some_and(|quarry| {
                        quarry == entity
                            || self_faction
                                .and_then(|me| relations.fights_beside(me, *foe_faction))
                                .is_some_and(|side| faction_of(quarry) == Some(side))
                    })
            })
        });
        if rallied && (disposition.is_peaceful() || !foe_still_fights) {
            aggression.grudge = None;
            aggression.target = None;
            brain_commands.write(BrainCommand::restore_default(LiveBodyId::new(
                sim_id.clone(),
                live_room_of(stamp, root),
            )));
            continue;
        }
        if rallied || !disposition.is_peaceful() {
            continue;
        }

        let dangerous = room_specs
            .as_ref()
            .and_then(|specs| specs.spec_of(entity))
            .is_some_and(|spec| spec.metadata.dangerous);
        if !dangerous {
            continue;
        }
        let has_kit = repertoire.is_some_and(|set| set.melee.is_some() || set.ranged.is_some())
            || moveset.is_some_and(|moveset| !moveset.0.moves.is_empty());
        if !has_kit {
            continue;
        }
        let Some(policy) = prepared.get().and_then(|registry| {
            ambition_platformer2d_actor_spawn::brain_builders::authored_provoked_policy(
                registry,
                worn.id(),
            )
        }) else {
            continue;
        };
        let Some(self_faction) = self_faction else {
            continue;
        };
        let Some(&(_, self_center, ..)) = candidates.iter().find(|candidate| candidate.0 == entity)
        else {
            continue;
        };
        let self_room = rooms.of(entity);
        let reach_sq = policy.0.aggro_radius * policy.0.aggro_radius;

        // The nearest foe of its side, in its room and in its notice range,
        // that is fighting its side now.
        let mut best: Option<(Entity, f32)> = None;
        for (foe, at, foe_faction, _, _, foe_room, blows) in &candidates {
            if *foe == entity || *foe_room != self_room {
                continue;
            }
            let Some(side) = relations.fights_beside(self_faction, *foe_faction) else {
                continue;
            };
            // A FIGHT, not a foe in sight: the foe and the side have come to
            // blows. A foe that only watches the player leaves the ally
            // talkable at its post.
            if blows.and_then(faction_of) != Some(side) {
                continue;
            }
            let d = at.distance_squared(self_center);
            if d > reach_sq || best.is_some_and(|(_, best_d)| d >= best_d) {
                continue;
            }
            best = Some((*foe, d));
        }
        let Some((foe, _)) = best else {
            continue;
        };

        aggression.target = Some(foe);
        aggression.grudge = Some(Grudge::Body(foe));
        crate::body::actors::provoke_actor_in_place(
            &mut commands,
            entity,
            (config, identity, abilities.abilities),
            &mut disposition,
            repertoire,
            Some(policy),
        );
    }
}
