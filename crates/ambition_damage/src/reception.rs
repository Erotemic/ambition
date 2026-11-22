//! Native hit decisions with identity captured at the reception cut.

use crate::{BodyHitResolution, BodyHitResolved};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::{ecs::system::SystemParam, prelude::*};

/// The shared publication seam for every native damage receiver: the
/// reception decision ([`Self::record`]) and the move's outcome
/// ([`Self::connect`], [`Self::block`]). A road states each once, from the hit
/// it resolved; the reporter builds the fields.
#[derive(SystemParam)]
pub struct BodyHitReporter<'w, 's> {
    messages: MessageWriter<'w, BodyHitResolved>,
    /// The hit's RESULT, for the simulation: the match freeze and a move's
    /// `Connected`/`OnHit` read it. `Option` because what it drives is a beat:
    /// a headless fixture with no feel tuning has no freeze to charge, and
    /// every real composition registers it in `SimCoreResourcesPlugin`.
    resolved: Option<MessageWriter<'w, ambition_combat::hitbox::ResolvedBodyHit>>,
    /// The strikes a GUARD ate. A move asking whether it connected reads this
    /// beside `resolved` and never infers a block from a missing connect.
    blocked: Option<MessageWriter<'w, ambition_combat::hitbox::BlockedBodyHit>>,
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

    /// ⭐⭐ THE HIT LANDED: the move that made it connected. Called where the
    /// reaction has been decided, because that is where the hitlag exists;
    /// `hitlag_seconds` is what the reaction actually earned, and zero is a
    /// valid reaction (a boss that takes the blow without a launch, a lethal
    /// strike whose body dies instead of reacting). The damage is the
    /// resolution's, after every defence.
    ///
    /// ⛔ UNCONDITIONAL in every build: an instrument may vanish, a beat may not.
    pub fn connect(
        &mut self,
        body: Entity,
        hit: &ambition_combat::HitEvent,
        resolution: BodyHitResolution,
        hitlag_seconds: f32,
    ) {
        if let Some(resolved) = self.resolved.as_mut() {
            resolved.write(ambition_combat::hitbox::ResolvedBodyHit {
                victim: body,
                attacker: hit.attacker,
                hitlag_seconds,
                source: hit.source.clone(),
                damage: resolution.damage_applied(),
                attacker_move_instance: hit.attacker_move_instance,
            });
        }
    }

    /// A guard consumed this strike. The counterpart of [`Self::connect`]: a
    /// consumer asking "did my move connect" cannot tell a block from a
    /// not-yet-resolved overlap by absence, because the actor road resolves
    /// on the overlap frame and the player road on the next one.
    pub fn block(&mut self, body: Entity, hit: &ambition_combat::HitEvent) {
        if let Some(blocked) = self.blocked.as_mut() {
            blocked.write(ambition_combat::hitbox::BlockedBodyHit {
                victim: body,
                attacker: hit.attacker,
                attacker_move_instance: hit.attacker_move_instance,
            });
        }
    }
}
