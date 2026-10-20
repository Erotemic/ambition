//! The adapters of a body's statuses: `ambition.body.hold_status`,
//! `ambition.body.status_tick` and `ambition.body.time_scale`. See the port
//! cards in `ambition_combat_port::status`.
//!
//! A status is a name a module holds on a body; its state is that module's
//! records on the body. This keeps only which names a body holds, so a move
//! that lives on a body for a while needs no component of its own here.

use std::collections::BTreeMap;

use ambition_combat_port::{HoldStatusPort, StatusTick, StatusTickPort, TimeScalePort};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::Port as _;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Install the status trigger and its requests in `technique_execution`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<StatusTickPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", queue_status_ticks);
    app.install_extension_request::<HoldStatusPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_held_statuses);
    app.install_extension_request::<TimeScalePort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_time_scales);
}

/// The statuses a body holds, by kind, in order. Never empty: a body that
/// holds none carries no component.
///
/// Rollback state: whether a body is ticked for a status decides what its
/// module does next.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyStatuses(pub Vec<String>);

impl BodyStatuses {
    pub fn holds(&self, kind: &str) -> bool {
        self.0.iter().any(|held| held == kind)
    }
}

/// The rollback value projection: every kind, in order.
pub fn body_statuses_probe(statuses: &BodyStatuses) -> u64 {
    // FNV-1a over the kinds and a separator, so ["ab"] and ["a", "b"] differ.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for kind in &statuses.0 {
        for byte in kind.bytes().chain(std::iter::once(0xff)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// One tick of each status that an admitted entry binds, on each body that
/// holds it, in the order of the bodies' simulation identity, then entity.
pub fn queue_status_ticks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<(
        Entity,
        &BodyStatuses,
        &ae::BodyKinematics,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
) {
    // A paused world ticks no status (see the port card).
    if time.sim_dt() <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == StatusTickPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut holders: Vec<_> = bodies.iter().collect();
    holders.sort_by(|a, b| a.3.cmp(&b.3).then(a.0.cmp(&b.0)));
    for (body, statuses, kin, _) in holders {
        for kind in statuses.0.iter().filter(|kind| bound.contains(&kind.as_str())) {
            invocations.trigger::<StatusTickPort>(
                &TECHNIQUE_EXECUTION,
                kind.clone(),
                body,
                None,
                false,
                StatusTick { position: [kin.pos.x, kin.pos.y], facing: kin.facing.signum() },
            );
        }
    }
}

/// Hold or let go of each status a module asked for, in the order asked.
///
/// The requests of a tick are applied to one staged copy per body before any
/// is written back, so a let-go and a later hold of one kind leave it held.
pub fn lower_held_statuses(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut holders: Query<&mut BodyStatuses>,
    bodies: Query<(), With<ae::BodyKinematics>>,
) {
    let mut staged: BTreeMap<Entity, Vec<String>> = BTreeMap::new();
    for submitted in outbox.drain::<HoldStatusPort>(&TECHNIQUE_EXECUTION) {
        let hold = submitted.value;
        if hold.kind.is_empty() {
            warn!("extension entry {} asked to hold a status with no kind; refused", submitted.entry);
            continue;
        }
        if !bodies.contains(submitted.scope) {
            continue;
        }
        let kinds = staged
            .entry(submitted.scope)
            .or_insert_with(|| holders.get(submitted.scope).map(|held| held.0.clone()).unwrap_or_default());
        let at = kinds.binary_search(&hold.kind);
        match (hold.held, at) {
            (true, Err(slot)) => kinds.insert(slot, hold.kind),
            (false, Ok(slot)) => {
                kinds.remove(slot);
            }
            _ => {}
        }
    }
    for (body, kinds) in staged {
        match (holders.get_mut(body), kinds.is_empty()) {
            (Ok(mut held), false) => {
                if held.0 != kinds {
                    held.0 = kinds;
                }
            }
            (Ok(_), true) => {
                commands.entity(body).try_remove::<BodyStatuses>();
            }
            (Err(_), false) => {
                commands.entity(body).try_insert(BodyStatuses(kinds));
            }
            (Err(_), true) => {}
        }
    }
}

/// Set each body's clock scale a module asked for, in the order asked.
pub fn lower_time_scales(mut commands: Commands, mut outbox: ResMut<ExtensionOutbox>) {
    for submitted in outbox.drain::<TimeScalePort>(&TECHNIQUE_EXECUTION) {
        let scale = submitted.value.scale;
        if !scale.is_finite() || scale <= 0.0 {
            warn!("extension entry {} asked for a clock scale of {scale}; refused", submitted.entry);
            continue;
        }
        commands.entity(submitted.scope).try_insert(ambition_time::ProperTimeScale(scale));
    }
}

#[cfg(test)]
mod tests;
