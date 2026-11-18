//! Script delivery of native reception decisions after damage resolution.

use crate::{reception::HitReceiverIdentity, BodyHitResolution, BodyHitResolved};
use ambition_combat::{extension::status::BodyStatuses, rules::HitReceptionRules};
use ambition_combat_port::{
    HitReception, HitReceptions, HitReceptionsPort, ReceptionBody, ReceptionDecision,
    ReceptionSource,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations};
use ambition_extension_sdk::{phases::HIT_REACTION, Port};
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, RoomInstanceRoot, SessionScopedEntity},
    sim_id::SimId,
};
use ambition_platformer2d_world::governing_rules::RulesOf;
use bevy::prelude::*;

pub fn install(app: &mut App) {
    app.add_message::<BodyHitResolved>();
    app.install_extension_trigger::<HitReceptionsPort, _>(
        HIT_REACTION,
        "ambition_damage",
        queue_receptions,
    );
}

fn identity(
    value: &HitReceiverIdentity,
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
) -> ReceptionBody {
    if value.session != session {
        return ReceptionBody {
            id: None,
            room: None,
        };
    }
    ReceptionBody {
        id: value.id.as_ref().map(|id| id.as_str().into()),
        room: value.room.map(|room| room.ordinal()),
    }
}

fn fact(
    hit: &BodyHitResolved,
    scope: Entity,
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
) -> HitReception {
    let decision = match hit.resolution {
        BodyHitResolution::Ignored => ReceptionDecision::Ignored,
        BodyHitResolution::Protected => ReceptionDecision::Protected,
        BodyHitResolution::Blocked => ReceptionDecision::Blocked,
        BodyHitResolution::Armored => ReceptionDecision::Armored,
        BodyHitResolution::WalletShielded { .. } => ReceptionDecision::WalletShielded,
        BodyHitResolution::Damaged { .. } => ReceptionDecision::Damaged,
    };
    HitReception {
        observed_at: hit.observed_at,
        victim: identity(&hit.identity, session),
        attacker: hit
            .attacker_identity
            .as_ref()
            .map(|value| identity(value, session)),
        source: match hit.source {
            ambition_combat::HitSource::Melee => ReceptionSource::Melee,
            ambition_combat::HitSource::Projectile => ReceptionSource::Projectile,
            ambition_combat::HitSource::Contact => ReceptionSource::Contact,
            ambition_combat::HitSource::Hazard => ReceptionSource::Hazard,
            ambition_combat::HitSource::LeftTheWorld => ReceptionSource::LeftTheWorld,
            ambition_combat::HitSource::Pogo => ReceptionSource::Pogo,
        },
        decision,
        raw_damage: hit.raw_damage,
        damage: hit.resolution.damage_applied(),
        wallet_spent: match hit.resolution {
            BodyHitResolution::WalletShielded { spent } => spent,
            _ => 0,
        },
        health_depleted: matches!(
            hit.resolution,
            BodyHitResolution::Damaged { died: true, .. }
        ),
        move_instance: hit.attacker_move_instance,
        incoming: hit.body == scope,
        outgoing: hit.attacker == Some(scope),
    }
}

fn queue_receptions(
    mut reports: ReplayMessageReader<BodyHitResolved>,
    admitted: Res<AdmittedExtensions>,
    scopes: Query<
        (
            Entity,
            &SimId,
            Option<&BodyStatuses>,
            Option<&SessionScopedEntity>,
            Has<RoomInstanceRoot>,
        ),
        Or<(With<BodyStatuses>, With<RoomInstanceRoot>)>,
    >,
    rooms: LiveRooms,
    rules: RulesOf<HitReceptionRules>,
    mut invocations: ResMut<ExtensionInvocations>,
) {
    let mut reports: Vec<_> = reports.read().cloned().collect();
    if reports.is_empty() {
        return;
    }
    // Keep each receiver's decision order. Sort independent receivers by scope
    // and semantic identity so room observers do not depend on ECS storage order.
    reports.sort_by(|a, b| {
        a.observed_at
            .cmp(&b.observed_at)
            .then(a.identity.session.cmp(&b.identity.session))
            .then(a.identity.room.cmp(&b.identity.room))
            .then(a.identity.id.cmp(&b.identity.id))
    });
    let bound: std::collections::BTreeSet<_> = admitted
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == HitReceptionsPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.to_string())
        .collect();
    let mut scopes: Vec<_> = scopes.iter().collect();
    scopes.sort_by(|a, b| {
        a.3.map(|s| s.0)
            .cmp(&b.3.map(|s| s.0))
            .then(rooms.of(a.0).cmp(&rooms.of(b.0)))
            .then(a.1.cmp(b.1))
    });
    for (scope, _, statuses, session, root) in scopes {
        let session = session.map(|session| session.0);
        let room = rooms.of(scope);
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
        if keys.is_empty() {
            continue;
        }
        let mut matching = reports.iter().filter(|hit| {
            if root {
                hit.identity.session == session && hit.identity.room == room
            } else {
                (hit.body == scope && hit.identity.session == session && hit.identity.room == room)
                    || (hit.attacker == Some(scope)
                        && hit
                            .attacker_identity
                            .as_ref()
                            .is_some_and(|source| source.session == session && source.room == room))
            }
        });
        let events: Vec<_> = matching
            .by_ref()
            .take(limits.max_events as usize)
            .map(|hit| fact(hit, scope, session))
            .collect();
        if events.is_empty() {
            continue;
        }
        let batch = HitReceptions {
            events,
            truncated: matching.next().is_some(),
        };
        for key in keys {
            invocations.trigger::<HitReceptionsPort>(
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
