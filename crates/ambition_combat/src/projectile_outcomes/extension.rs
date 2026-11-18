//! Bounded delivery of native projectile facts at the combat read cut.

use super::ProjectileOutcome;
use crate::{HitIdentity, extension::status::BodyStatuses, rules::ProjectileOutcomeRules};
use ambition_combat_port::projectile_outcome::{
    ProjectileOutcomes, ProjectileOutcomesPort, ProjectileParticipant,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations};
use ambition_extension_sdk::{Port, phases::HIT_REACTION};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, RoomInstanceRoot, SessionScopeId, SessionScopedEntity},
    sim_id::SimId,
};
use ambition_platformer2d_world::governing_rules::RulesOf;
use bevy::prelude::*;

pub fn install(app: &mut App) {
    app.add_message::<ProjectileOutcome>();
    app.install_extension_trigger::<ProjectileOutcomesPort, _>(
        HIT_REACTION,
        "ambition_combat",
        queue_outcomes,
    );
}
fn identity(body: &HitIdentity, session: Option<SessionScopeId>) -> ProjectileParticipant {
    let local = body.session == session;
    ProjectileParticipant {
        id: body
            .id
            .as_ref()
            .filter(|_| local)
            .map(|id| id.as_str().into()),
        room: body.room.filter(|_| local).map(|room| room.ordinal()),
    }
}
fn fact(
    report: &ProjectileOutcome,
    session: Option<SessionScopeId>,
) -> ambition_combat_port::projectile_outcome::ProjectileOutcome {
    ambition_combat_port::projectile_outcome::ProjectileOutcome {
        observed_at: report.observed_at,
        projectile: identity(&report.projectile, session),
        credit: report.credit.as_ref().map(|body| identity(body, session)),
        previous_credit: report
            .previous_credit
            .as_ref()
            .map(|body| identity(body, session)),
        interceptor: report
            .interceptor
            .as_ref()
            .map(|body| identity(body, session)),
        kind: report.kind,
        terminal: report.terminal,
        position: report.position.to_array(),
    }
}

/// Pending delivery belongs to its observer and rewinds with that observer.
#[derive(Component, Clone, Debug, Default)]
pub struct PendingProjectileOutcomes {
    events: Vec<(
        u64,
        ambition_combat_port::projectile_outcome::ProjectileOutcome,
    )>,
    truncated: bool,
}
impl PendingProjectileOutcomes {
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot as wire;
        let mut bytes = Vec::new();
        wire::put_bool(&mut bytes, self.truncated);
        wire::put_u64(&mut bytes, self.events.len() as u64);
        for (sequence, event) in &self.events {
            wire::put_u64(&mut bytes, *sequence);
            ambition_extension_sdk::structure::put(event, &mut bytes);
        }
        ambition_platformer2d_core::snapshot::checksum_bytes(&bytes)
    }
}

pub(super) fn enqueue(world: &mut World, shot: Entity, report: &ProjectileOutcome) {
    use ambition_platformer2d_shared_tangle::lifecycle::live_room_root_for;
    use bevy::ecs::system::SystemState;
    let bound: std::collections::BTreeSet<_> = world
        .get_resource::<AdmittedExtensions>()
        .into_iter()
        .flat_map(|admitted| &admitted.0.entries)
        .filter(|entry| entry.descriptor.trigger.port == ProjectileOutcomesPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.to_string())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut candidates: Vec<_> = [
        report.credit_body,
        report.previous_credit_body,
        report.interceptor_body,
    ]
    .into_iter()
    .flatten()
    .collect();
    let root = report
        .projectile
        .session
        .zip(report.projectile.room)
        .and_then(|(session, room)| live_room_root_for(world, session, room));
    candidates.extend(root);
    candidates.sort();
    candidates.dedup();
    // Use default visibility. Replay must not write to hidden live observers.
    let mut state = SystemState::<(RulesOf<ProjectileOutcomeRules>, Query<()>)>::new(world);
    let (rules, visible) = state
        .get(world)
        .expect("projectile outcome rules must be readable");
    if !visible.contains(shot) {
        return;
    }
    let limits: Vec<_> = candidates
        .iter()
        .filter(|entity| visible.contains(**entity))
        .map(|entity| (*entity, rules.of(*entity).cloned().unwrap_or_default()))
        .collect();
    for (scope, limits) in limits {
        if !limits.problems().is_empty() {
            continue;
        }
        let Some(session) = world.get::<SessionScopedEntity>(scope).map(|scope| scope.0) else {
            continue;
        };
        if Some(session) != report.projectile.session {
            continue;
        }
        let selectors = if Some(scope) == root {
            &limits.room_observers
        } else {
            let Some(statuses) = world.get::<BodyStatuses>(scope) else {
                continue;
            };
            &statuses.0
        };
        if !selectors.iter().any(|key| bound.contains(key)) {
            continue;
        }
        let mut entity = world.entity_mut(scope);
        let mut pending = entity
            .entry::<PendingProjectileOutcomes>()
            .or_default()
            .into_mut();
        pending
            .events
            .push((report.sequence, fact(report, Some(session))));
        pending
            .events
            .sort_by(|a, b| a.1.observed_at.cmp(&b.1.observed_at).then(a.0.cmp(&b.0)));
        if pending.events.len() > limits.max_events as usize {
            pending.events.pop();
            pending.truncated = true;
        }
    }
}

fn queue_outcomes(
    admitted: Res<AdmittedExtensions>,
    mut scopes: Query<(
        Entity,
        &SimId,
        Option<&BodyStatuses>,
        Option<&SessionScopedEntity>,
        Has<RoomInstanceRoot>,
        &mut PendingProjectileOutcomes,
    )>,
    rooms: LiveRooms,
    rules: RulesOf<ProjectileOutcomeRules>,
    mut invocations: ResMut<ExtensionInvocations>,
) {
    let bound: std::collections::BTreeSet<_> = admitted
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == ProjectileOutcomesPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.to_string())
        .collect();
    let mut order: Vec<_> = scopes
        .iter()
        .map(|(entity, id, _, session, _, _)| {
            (
                session.map(|scope| scope.0),
                rooms.of(entity),
                id.clone(),
                entity,
            )
        })
        .collect();
    order.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    for (_, _, _, scope) in order {
        let Ok((_, _, statuses, _, root, mut pending)) = scopes.get_mut(scope) else {
            continue;
        };
        if pending.events.is_empty() {
            continue;
        }
        let mut batch = ProjectileOutcomes {
            events: std::mem::take(&mut pending.events)
                .into_iter()
                .map(|(_, event)| event)
                .collect(),
            truncated: std::mem::take(&mut pending.truncated),
        };
        let limits = rules.of(scope).cloned().unwrap_or_default();
        if !limits.problems().is_empty() {
            continue;
        }
        let selectors = if root {
            &limits.room_observers
        } else {
            let Some(statuses) = statuses else {
                continue;
            };
            &statuses.0
        };
        let mut keys: Vec<_> = selectors
            .iter()
            .filter(|key| bound.contains(*key))
            .collect();
        keys.sort();
        keys.dedup();
        if batch.events.len() > limits.max_events as usize {
            batch.events.truncate(limits.max_events as usize);
            batch.truncated = true;
        }
        for key in keys {
            invocations.trigger::<ProjectileOutcomesPort>(
                &HIT_REACTION,
                key.clone(),
                scope,
                None,
                false,
                batch.clone(),
            );
        }
    }
}

#[cfg(test)]
mod tests;
