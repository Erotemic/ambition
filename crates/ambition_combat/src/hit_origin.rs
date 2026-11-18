//! Capture combat source identity before deferred reception or retirement.

use crate::HitEvent;
use ambition_platformer2d_core::snapshot as wire;
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRoomInstance, LiveRooms, SessionScopeId, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::{ecs::system::SystemParam, prelude::*};

/// A semantic entity name in its native session and live room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitIdentity {
    pub id: Option<SimId>,
    pub session: Option<SessionScopeId>,
    pub room: Option<LiveRoomInstance>,
}

/// Credit and optional shot identity at native hit publication.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct HitOrigin {
    pub attacker: Option<HitIdentity>,
    pub projectile: Option<HitIdentity>,
}

impl HitOrigin {
    pub fn projectile(
        id: Option<&SimId>,
        credit: &ambition_projectiles::ownership::ProjectileCredit,
        session: Option<SessionScopeId>,
        room: Option<LiveRoomInstance>,
    ) -> Self {
        Self {
            attacker: credit.body.as_ref().map(|id| HitIdentity {
                id: Some(id.clone()),
                session,
                room: credit.room,
            }),
            projectile: Some(HitIdentity {
                id: id.cloned(),
                session,
                room,
            }),
        }
    }

    /// Encode semantic facts without native entity handles or session counters.
    pub fn encode_semantics(&self, out: &mut Vec<u8>) {
        for identity in [&self.attacker, &self.projectile] {
            wire::put_bool(out, identity.is_some());
            if let Some(identity) = identity {
                wire::put_opt_str(out, identity.id.as_ref().map(SimId::as_str));
                wire::put_bool(out, identity.session.is_some());
                wire::put_bool(out, identity.room.is_some());
                if let Some(room) = identity.room {
                    wire::put_u32(out, room.ordinal());
                }
            }
        }
    }

    pub fn checksum(&self) -> u64 {
        let mut out = Vec::new();
        self.encode_semantics(&mut out);
        wire::checksum_bytes(&out)
    }
}

/// The native publication path for every hit source and controller.
#[derive(SystemParam)]
pub struct HitWriter<'w, 's> {
    messages: MessageWriter<'w, HitEvent>,
    identities: Query<'w, 's, (Option<&'static SimId>, Option<&'static SessionScopedEntity>)>,
    rooms: LiveRooms<'w, 's>,
}

impl HitWriter<'_, '_> {
    pub fn write(&mut self, mut hit: HitEvent) -> bevy::ecs::message::MessageId<HitEvent> {
        // Captured credit remains authoritative when the native owner link is gone.
        if hit.origin.is_none() {
            hit.origin = Some(HitOrigin {
                attacker: hit.attacker.and_then(|entity| {
                    self.identities
                        .get(entity)
                        .ok()
                        .map(|(id, session)| HitIdentity {
                            id: id.cloned(),
                            session: session.map(|scope| scope.0),
                            room: self.rooms.of(entity),
                        })
                }),
                projectile: None,
            });
        }
        self.messages.write(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn a_melee_hit_retains_source_identity_after_its_attacker_retires() {
        let mut world = World::new();
        world.init_resource::<Messages<HitEvent>>();
        let id = SimId::placement("striker");
        let source = world
            .spawn((id.clone(), SessionScopedEntity(SessionScopeId(4))))
            .id();
        world
            .run_system_once(move |mut hits: HitWriter| {
                hits.write(HitEvent {
                    origin: None,
                    properties: Default::default(),
                    contact: None,
                    volume: ambition_platformer2d_core::Aabb::new(
                        ambition_platformer2d_core::Vec2::ZERO,
                        ambition_platformer2d_core::Vec2::ONE,
                    )
                    .into(),
                    pieces: Vec::new(),
                    damage: 1,
                    source: crate::HitSource::Melee,
                    attacker: Some(source),
                    room: None,
                    target: crate::HitTarget::UnresolvedFeatures,
                    mode: crate::HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                    strike_sfx: None,
                    attacker_move_instance: None,
                });
            })
            .unwrap();
        world.despawn(source);
        let hit = world
            .resource::<Messages<HitEvent>>()
            .iter_current_update_messages()
            .next()
            .unwrap();
        let origin = hit.origin.as_ref().unwrap();
        assert_eq!(origin.attacker.as_ref().unwrap().id.as_ref(), Some(&id));
        assert_eq!(
            origin.attacker.as_ref().unwrap().session,
            Some(SessionScopeId(4))
        );
        assert!(origin.projectile.is_none());
    }
}
