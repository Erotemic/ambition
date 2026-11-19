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

    pub(super) fn take(&mut self, start: ProjectileStart) -> Vec<ProjectileEmission> {
        let (ready, pending) = std::mem::take(&mut self.0)
            .into_iter()
            .partition(|emission| emission.request.start == start);
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
