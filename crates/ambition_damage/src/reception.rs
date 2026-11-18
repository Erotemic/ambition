//! Native hit decisions with identity captured at the reception cut.

use crate::{BodyHitResolution, BodyHitResolved};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::{ecs::system::SystemParam, prelude::*};

/// The shared publication seam for every native damage receiver.
#[derive(SystemParam)]
pub struct BodyHitReporter<'w, 's> {
    messages: MessageWriter<'w, BodyHitResolved>,
    identities: Query<'w, 's, (Option<&'static SimId>, Option<&'static SessionScopedEntity>)>,
    rooms: LiveRooms<'w, 's>,
    tick: Option<Res<'w, ambition_time::SimTick>>,
}

impl BodyHitReporter<'_, '_> {
    fn identity(&self, entity: Entity) -> ambition_combat::HitIdentity {
        let Ok((id, session)) = self.identities.get(entity) else {
            return ambition_combat::HitIdentity {
                id: None,
                session: None,
                room: None,
            };
        };
        ambition_combat::HitIdentity {
            id: id.cloned(),
            session: session.map(|session| session.0),
            room: self.rooms.of(entity),
        }
    }

    /// Publish the decision before death or travel changes the recipient.
    pub fn record(
        &mut self,
        body: Entity,
        hit: &ambition_combat::HitEvent,
        resolution: BodyHitResolution,
    ) {
        let identity = self.identity(body);
        let attacker_identity = match &hit.origin {
            Some(origin) => origin.attacker.clone(),
            None => hit.attacker.map(|attacker| self.identity(attacker)),
        };
        self.messages.write(BodyHitResolved {
            projectile: hit
                .origin
                .as_ref()
                .and_then(|origin| origin.projectile.clone()),
            body,
            attacker: hit.attacker,
            identity,
            attacker_identity,
            observed_at: self.tick.as_deref().map_or(0, |tick| tick.0),
            resolution,
            source: hit.source.clone(),
            raw_damage: hit.damage,
            attacker_move_instance: hit.attacker_move_instance,
        });
    }
}
