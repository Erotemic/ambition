//! Damage-box requests capture their source context before deferred construction.

use crate::{
    allegiance::AttackAllegiance,
    strike::{DamageBox, HitSide},
    targeting::MatchTeam,
};
use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    LiveRooms, RoundScopedEntity, RoundSpawnScope, SessionScopedEntity, SessionSpawnScope,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// A complete damage-box birth request. Credit and emission location are separate facts.
#[derive(Message, Clone)]
pub struct DamageBoxRequest {
    pub owner: Entity,
    pub scope: SessionSpawnScope,
    pub round: RoundSpawnScope,
    pub allegiance: Option<AttackAllegiance>,
    pub source: HitSide,
    pub center: ae::Vec2,
    pub damage_box: DamageBox,
}

#[derive(SystemParam)]
struct DamageBoxOrigins<'w, 's> {
    facts: Query<
        'w,
        's,
        (
            Option<&'static ActorFaction>,
            Option<&'static DrivingParticipant>,
            Option<&'static MatchTeam>,
            Option<&'static SessionScopedEntity>,
            Option<&'static RoundScopedEntity>,
            Option<&'static AttackAllegiance>,
        ),
    >,
    rooms: LiveRooms<'w, 's>,
}

impl DamageBoxOrigins<'_, '_> {
    fn capture(
        &self,
        emitter: Entity,
        owner: Entity,
        source: HitSide,
        center: ae::Vec2,
        damage_box: DamageBox,
    ) -> Option<DamageBoxRequest> {
        let Ok((_, _, _, session, round, captured)) = self.facts.get(emitter) else {
            warn!("damage box refused: its emitter {emitter:?} has left");
            return None;
        };
        let scope = SessionSpawnScope::new(session.map(|session| session.0))
            .in_room(self.rooms.of(emitter));
        let allegiance = if source == HitSide::Environment {
            None
        } else if let Some(captured) = captured {
            Some(captured.clone())
        } else {
            let Ok((faction, driver, team, _, _, _)) = self.facts.get(owner) else {
                warn!("damage box refused: its credit body {owner:?} has left");
                return None;
            };
            Some(AttackAllegiance::from_body(
                faction
                    .copied()
                    .unwrap_or_else(|| crate::actor_faction_from_hit_side(source)),
                team,
                driver,
            ))
        };
        Some(DamageBoxRequest {
            owner,
            scope,
            round: round.map_or(RoundSpawnScope::UNSCOPED, |round| round.0.into()),
            allegiance,
            source,
            center,
            damage_box,
        })
    }
}

/// Emit before the source leaves. Materialization needs no live source lookup.
#[derive(SystemParam)]
pub struct DamageBoxWriter<'w, 's> {
    origins: DamageBoxOrigins<'w, 's>,
    requests: MessageWriter<'w, DamageBoxRequest>,
}

impl DamageBoxWriter<'_, '_> {
    pub fn write(
        &mut self,
        emitter: Entity,
        owner: Entity,
        source: HitSide,
        center: ae::Vec2,
        damage_box: DamageBox,
    ) -> bool {
        let Some(request) = self
            .origins
            .capture(emitter, owner, source, center, damage_box)
        else {
            return false;
        };
        self.requests.write(request);
        true
    }
}

/// Immediate construction for phases that run after the request consumer.
#[derive(SystemParam)]
pub struct DamageBoxSpawner<'w, 's> {
    origins: DamageBoxOrigins<'w, 's>,
    commands: Commands<'w, 's>,
}

impl DamageBoxSpawner<'_, '_> {
    pub fn spawn(
        &mut self,
        emitter: Entity,
        owner: Entity,
        source: HitSide,
        center: ae::Vec2,
        damage_box: DamageBox,
    ) -> Option<Entity> {
        let request = self
            .origins
            .capture(emitter, owner, source, center, damage_box)?;
        Some(materialize(&mut self.commands, request))
    }
}

/// Run at the damage-box materialization boundary, before contact resolution.
pub fn materialize_damage_boxes(
    mut commands: Commands,
    mut requests: MessageReader<DamageBoxRequest>,
) {
    for request in requests.read() {
        materialize(&mut commands, request.clone());
    }
}

fn materialize(commands: &mut Commands, request: DamageBoxRequest) -> Entity {
    let entity = crate::strike::spawn_damage_box(
        commands,
        request.owner,
        request.source,
        request.center,
        request.damage_box,
    );
    let mut entity = commands.entity(entity);
    request.scope.apply_to(&mut entity);
    request.round.apply_to(&mut entity);
    entity.insert(ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity);
    if let Some(allegiance) = request.allegiance {
        entity.insert(allegiance);
    }
    entity.id()
}
