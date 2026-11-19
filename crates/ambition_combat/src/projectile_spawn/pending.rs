//! Pending emissions are simulation state; message cursors are observations.

use super::{ProjectileEmission, ProjectilePresentation, ProjectileStart};
use ambition_platformer2d_core::snapshot::{self as wire, SnapshotState};
use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    prelude::*,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct PendingProjectileEmissions(pub(super) Vec<ProjectileEmission>);

impl PendingProjectileEmissions {
    pub(super) fn cancel(
        &mut self,
        reply: &ambition_platformer2d_shared_tangle::action_results::ActionReply,
    ) -> bool {
        let before = self.0.len();
        self.0.retain(|emission| {
            !emission.request.reply.as_ref().is_some_and(|queued| {
                queued.ticket == reply.ticket
                    && queued.recipient == reply.recipient
            })
        });
        self.0.len() != before
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The emissions that start at `start`. During a room replay (`room`),
    /// only that room's: the other rooms are hidden and do not run the
    /// replayed ticks, so theirs wait for the live tick.
    pub(super) fn take(
        &mut self,
        start: ProjectileStart,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Vec<ProjectileEmission> {
        let (ready, pending) = std::mem::take(&mut self.0).into_iter().partition(|emission| {
            emission.request.start == start && room.is_none_or(|room| emission.source.credit.room == Some(room))
        });
        self.0 = pending;
        ready
    }

    pub fn checksum(&self) -> u64 {
        let mut out = Vec::new();
        wire::put_u64(&mut out, self.0.len() as u64);
        for emission in &self.0 {
            let request = &emission.request;
            wire::put_bool(&mut out, request.reply.is_some());
            if let Some(reply) = &request.reply {
                reply.recipient.occurrence.encode(&mut out);
                wire::put_opt_str(&mut out, reply.recipient.subject.as_deref());
                ambition_extension_sdk::structure::put(&reply.ticket, &mut out);
            }
            request.projectile.body.kin.encode(&mut out);
            request.projectile.body.game.encode(&mut out);
            request.projectile.body.acceleration.encode(&mut out);
            wire::put_bool(&mut out, request.owner != Entity::PLACEHOLDER);
            wire::put_u8(
                &mut out,
                match request.start {
                    ProjectileStart::StepThisTick => 0,
                    ProjectileStart::StepNextTick => 1,
                },
            );
            match &request.presentation {
                ProjectilePresentation::NamedKind(kind) => {
                    wire::put_u8(&mut out, 0);
                    kind.encode(&mut out);
                }
                ProjectilePresentation::NamedCharged(kind, tier) => {
                    wire::put_u8(&mut out, 1);
                    kind.encode(&mut out);
                    wire::put_u8(&mut out, *tier);
                }
                ProjectilePresentation::OpenVisual(id) => {
                    wire::put_u8(&mut out, 2);
                    wire::put_str(&mut out, id);
                }
            }
            wire::put_bool(&mut out, request.move_instance.is_some());
            if let Some(instance) = request.move_instance {
                wire::put_u32(&mut out, instance);
            }
            wire::put_bool(&mut out, request.weapon_shot);
            wire::put_bool(&mut out, request.launch.is_some());
            if let Some(launch) = &request.launch {
                launch.encode(&mut out);
            }
            let source = &emission.source;
            // Session counters and native entity handles do not cross peers.
            wire::put_bool(&mut out, source.scope.id().is_some());
            wire::put_bool(&mut out, source.scope.room().is_some());
            if let Some(room) = source.scope.room() {
                wire::put_u32(&mut out, room.ordinal());
            }
            wire::put_u8(&mut out, match source.scope.visibility() {
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnVisibility::Published => 0,
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnVisibility::HiddenCandidate => 1,
            });
            wire::put_bool(&mut out, source.round.id().is_some());
            if let Some(round) = source.round.id() {
                wire::put_u64(&mut out, round.0);
            }
            source.credit.encode(&mut out);
            wire::put_bool(&mut out, source.allegiance.is_some());
            if let Some(side) = &source.allegiance {
                side.encode(&mut out);
            }
            wire::put_opt_str(
                &mut out,
                source
                    .presentation
                    .as_ref()
                    .map(|source| source.id().as_str()),
            );
            wire::put_bool(&mut out, source.identity.is_some());
            if let Some((id, origin)) = &source.identity {
                wire::put_str(&mut out, id.as_str());
                origin.encode(&mut out);
            }
        }
        wire::checksum_bytes(&out)
    }
}
impl MapEntities for PendingProjectileEmissions {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        for emission in &mut self.0 {
            if emission.request.owner != Entity::PLACEHOLDER {
                emission.request.owner = mapper.get_mapped(emission.request.owner);
            }
        }
    }
}

/// The ticks of queue a room rewind can reach back to: past the room rollback
/// host's horizon (`ambition_hosted_world::REWIND_HORIZON`, 60).
const CARRIED_TICKS: u64 = 120;

/// What [`PendingProjectileEmissions`] held at the start of each recent tick
/// (only the ticks it held something). A room rewind sets its room's rows of
/// the queue back to these ([`retract_room_projectile_emissions`]).
#[derive(Resource, Default)]
pub struct CarriedProjectileEmissions(std::collections::BTreeMap<u64, Vec<ProjectileEmission>>);

/// At the head of each tick: note what the queue carries into it.
pub fn carry_pending_projectile_emissions(
    tick: Option<Res<ambition_time::SimTick>>,
    pending: Option<Res<PendingProjectileEmissions>>,
    mut carried: ResMut<CarriedProjectileEmissions>,
) {
    let (Some(tick), Some(pending)) = (tick, pending) else { return };
    let tick = tick.0 + 1;
    if pending.0.is_empty() {
        carried.0.remove(&tick);
    } else {
        carried.0.insert(tick, pending.0.clone());
    }
    carried.0.retain(|at, _| *at + CARRIED_TICKS > tick);
}

/// A room rewound to the start of the tick after `saved_at`: its queued
/// emissions are the speculative timeline's (the replay queues its own), and
/// the ones it carried into that tick were taken by the speculative run, so
/// its rows of the queue go back to those. Other rooms' rows stand.
pub fn retract_room_projectile_emissions(
    world: &mut World,
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    saved_at: u64,
) {
    let back: Vec<ProjectileEmission> = world
        .get_resource::<CarriedProjectileEmissions>()
        .and_then(|carried| carried.0.get(&(saved_at + 1)))
        .map(|carried| carried.iter().filter(|emission| emission.source.credit.room == Some(room)).cloned().collect())
        .unwrap_or_default();
    let Some(mut pending) = world.get_resource_mut::<PendingProjectileEmissions>() else { return };
    pending.0.retain(|emission| emission.source.credit.room != Some(room));
    pending.0.extend(back);
}
