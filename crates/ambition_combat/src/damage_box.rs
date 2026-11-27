//! Damage-box requests capture their source context before deferred construction.

use crate::{
    allegiance::AttackAllegiance,
    strike::{DamageBox, HitSide},
    targeting::MatchTeam,
};
use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_platformer2d_shared_tangle::lifecycle::{
    LiveRooms, RoundScopedEntity, RoundSpawnScope, SessionScopedEntity, SessionSpawnScope,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// A complete damage-box birth request. Credit and emission location are separate facts.
#[derive(Message, Clone)]
pub struct DamageBoxRequest {
    pub origin: crate::HitOrigin,
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
            Option<&'static ambition_projectiles::ownership::ProjectileCredit>,
            Option<&'static ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&'static crate::HitOrigin>,
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
        let Ok((_, _, _, session, round, captured, credit, id, origin)) = self.facts.get(emitter)
        else {
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
            let Ok((faction, driver, team, _, _, _, _, _, _)) = self.facts.get(owner) else {
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
            origin: origin
                .cloned()
                .or_else(|| {
                    credit.map(|credit| {
                        crate::HitOrigin::projectile(
                            id,
                            credit,
                            session.map(|session| session.0),
                            scope.room(),
                        )
                    })
                })
                .unwrap_or_else(|| crate::HitOrigin {
                    attacker: self.facts.get(owner).ok().map(
                        |(_, _, _, session, _, _, _, id, _)| crate::HitIdentity {
                            id: id.cloned(),
                            session: session.map(|scope| scope.0),
                            room: self.rooms.of(owner),
                        },
                    ),
                    projectile: None,
                    damage_types: Default::default(),
                    ray_jump: None,
                }),
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

/// Queue construction after the request consumer has run.
/// Add birth components through the returned builder, on the spawn queue.
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
    ) -> Option<EntityCommands<'_>> {
        let request = self
            .origins
            .capture(emitter, owner, source, center, damage_box)?;
        let entity = materialize(&mut self.commands, request);
        Some(self.commands.entity(entity))
    }
}

/// Run at the damage-box materialization boundary, before contact resolution.
pub fn materialize_damage_boxes(
    mut commands: Commands,
    mut requests: ReplayMessageReader<DamageBoxRequest>,
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
    entity.insert(request.origin);
    entity.id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::{
        lifecycle::{InRoomInstance, LiveRoomInstance, SessionScopeId},
        sim_id::SimId,
    };
    use ambition_projectiles::ownership::ProjectileCredit;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn a_detached_splash_keeps_its_shot_and_retired_deflectors_credit() {
        let mut world = World::new();
        let owner = world.spawn_empty().id();
        world.despawn(owner);
        let room = LiveRoomInstance::ACTIVATION;
        let credit = ProjectileCredit {
            body: Some(SimId::placement("deflector")),
            room: Some(room),
        };
        let shot_id = SimId::placement("shot");
        let emitter = world
            .spawn((
                shot_id.clone(),
                credit.clone(),
                AttackAllegiance::from_body(ActorFaction::Enemy, None, None),
                SessionScopedEntity(SessionScopeId(4)),
                InRoomInstance(room),
            ))
            .id();
        world
            .run_system_once(move |mut blasts: DamageBoxSpawner| {
                blasts
                    .spawn(
                        emitter,
                        owner,
                        HitSide::Enemy,
                        ae::Vec2::ZERO,
                        DamageBox {
                            cause: crate::HitSource::Projectile,
                            move_instance: None,
                            half_extent: ae::Vec2::splat(8.0),
                            shape: None,
                            damage: 1,
                            knockback: crate::strike::HitboxKnockback::FeelScale(1.0),
                            lifetime_s: 0.2,
                            name: None,
                        },
                    )
                    .unwrap();
            })
            .unwrap();
        world.despawn(emitter);
        let origin = world.query::<&crate::HitOrigin>().single(&world).unwrap();
        let shot = origin.projectile.as_ref().unwrap();
        let attacker = origin.attacker.as_ref().unwrap();
        assert_eq!(shot.id.as_ref(), Some(&shot_id));
        assert_eq!(attacker.id, credit.body);
        assert_eq!(attacker.room, credit.room);
        assert_eq!(shot.session, Some(SessionScopeId(4)));
        assert_eq!(shot.room, Some(room));
    }
}
